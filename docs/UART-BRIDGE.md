# UART 桥（串口 I2C）

## 为什么要有这条路

原来只能靠 FTDI / CH341A / CH347F 这些专用桥接器去碰 I2C。但面板主芯片本身就在跑
Android/Linux，串口控制台里 `i2cget` / `i2cset` / `i2cdetect` 都是现成的 —— 一根
USB-TTL 就能把整个 PMU 读写流程跑起来，不用飞线、不用转接板。

这一层做的事，就是把「手工敲 i2cget」变成工具里的一个正式后端：连上之后
**Read DAC / Write EEPROM / Verify ALL 全部照常可用**，只是底层从 USB 桥换成了串口。

## 架构

    ┌─────────────┐   UART    ┌──────────────────┐   I2C    ┌─────────┐
    │ 本工具(PC)  │ ────────► │ 面板主芯片 shell │ ───────► │ PMU 0x20│
    │ UartI2cBus  │  115200   │ i2cget / i2cset  │          │ VCOM 0x74│
    └─────────────┘           └──────────────────┘          └─────────┘

`UartI2cBus` 实现了和 FTDI / CH347 一样的 `I2cBus` trait，所以对上层完全透明。

## 三个必须知道的坑

### 1. toybox 和 i2c-tools 的语法不一样

这是实测踩到的：

    # toybox（Android / TV 板）
    i2cget -y -f 1 0x20 0x00          # OK，4 个参数
    i2cget -y -f 1 0x20 0x00 b        # 报错：Max 3 arguments
    i2cset -y -f 1 0x20 0x00 0x1A     # 报错：mode too long
    i2cset -y -f 1 0x20 0x00 0x1A b   # OK，最后一个参数必须是 MODE

也就是说 **i2cget 不能带 mode，i2cset 必须带 mode**。工具里用 `dialect` 参数区分，
选错了会直接看到 `Max 3 arguments` 或 `mode too long`。

### 2. 7 位地址 / 8 位地址

原理图或者文档上写的 `0x40 / 0x41` 往往是一对 8 位读写地址：

    0x20 << 1 | 0 = 0x40   (写)
    0x20 << 1 | 1 = 0x41   (读)

而 `i2cdetect` 和 `i2cget` 用的是 **7 位地址**，所以总线上扫到的是 `0x20`。
本工具内部一律用 7 位地址；如果哪天遇到要 8 位地址的 userland，把
**Addr** 切成 `8-bit (x2)` 即可。

### 3. 115200 上不能传文件

实测把多行 shell 脚本粘进串口控制台，**必掉字节**，粘完直接乱码。所以：

- 这条链路上**永远不传脚本/文件**，只发单行短命令（每条不超过 40 字节）
- 每条命令都用一个成对的标记 `echo __PBn__; <cmd>; echo __PEn__$?` 框住，
  首尾对不上就当失败重试，绝不把半截输出当数据
- 实在还乱码，把 **Char delay** 调到 1~2 ms（逐字节发送）

## 用法

1. 切到 **UART Bridge** 页
2. **Refresh** 列出串口，选口
3. Baud 一般 `115200`；Bus 填 `i2cdetect -l` 第一列的数字
4. Dialect：Android/TV 板选 `toybox`，标准发行版选 `i2c-tools`
5. **Open / Test** —— 握手成功会亮绿灯
6. **Scan Bus** —— 出网格，扫到的地址还会标出已知器件
7. 需要调 PMU 就点 **Connect as PMU**，之后回 Toolbar 用原有的所有按钮

### 裸寄存器调试

直接在终端里敲 i2c-tools 就行，不需要单独的调试面板：

    i2cget  -y -f 1 0x20 0x00        # 读
    i2cset  -y -f 1 0x20 0x03 0x0B b # 写
    for r in $(seq 0 31); do printf "%02x: " $r; i2cget -y -f 1 0x20 $r; done

命令拼法（toybox 的 i2cget 不能带 mode）由 Dialect 决定，但终端是原样透传的，
所以这里要自己按真机的语法写。

## 内置终端

串口一旦能跑命令，就不该只做成「填一条、跑一条」的输入框。这里做成了真终端：

- 连着敲，输出实时回显
- **Tab 直接透传给板子的 shell** —— 路径补全、命令补全都是系统现成的，
  这边不实现任何补全逻辑
- 箭头键有历史，Ctrl-C 中断，Ctrl-L 清屏，Ctrl-A/E 行首行尾，Ctrl-U/W 删词
- **粘贴自动分片**：8 字节一批、每批 12 ms，115200 下不会掉字
- **快捷命令可自定义**：按钮名、命令内容、是否发送前确认都能改，即时保存
- 显示走一个极简 VT 缓冲：§\b§（退格）、§\r§（回车重绘）、§ESC[K§（擦行）都按终端规则处理 ——
  不然 Backspace 删不掉字，提示符会越叠越长

典型用法：调完 PMU，直接在终端里 `cp /sdcard/xxx.bin /vendor/...` 把 bin 换掉，
不用再开串口助手。

### 终端和 I2C 命令是互斥的

终端要自由收发字节，I2C 命令要严格成对标记框住输出，两者不能在同一条线上同时跑。
所以终端开着的时候，Scan / Read / Dump 会直接报「终端正在占用串口」，
点「停止终端」就恢复。

## 安全提示

- **写之前先 Dump 备份。** `0x0f` 这类寄存器可能是保护/状态位，写错会掉电
- 遇到一写就待机/黑屏的地址，那多半是 enable / sequence 寄存器，记下来别再碰
- 有些 PMIC 有写保护，写进去读回还是原值 —— 这时需要先解锁，不是工具的问题

## 相关代码

     src-tauri/src/bridges/uart/i2c.rs     桥实现 + 命令拼接 + 帧解析（11 个单测）
     src-tauri/src/bridges/uart/mod.rs     模块导出
     src-tauri/src/pmu/commands/uart.rs    Tauri 命令 + 串口句柄复用
     src/lib/uart.ts                       前端类型与 invoke 封装
     src/components/uart/UartTab.tsx       UI
