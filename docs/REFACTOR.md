# 兼容重构与交付说明

## 范围

保留 React 18 + TypeScript + Vite 6 + Tailwind 4 与 Rust + Tauri 2。选择兼容式重构而非整体换栈，直接复用 FTDI MPSSE、CH341A/CH347、UART 后端，保留三个芯片的 Tab 顺序、主要布局、按钮流程、固件命名、寄存器顺序和 EEPROM 时序。未创建 Git commit、未 push；解压工作树不含 `.git`。

基线：上游 `main` 的 `0d3fe0dbf03ae3855fe279ebcf713b9accb8a89f`，版本 `0.5.1`。原 ZIP 与基线副本均保留在工作区上一级。

## 结构与性能

Provider 组合为 `DeviceProvider → OperationsProvider → RegistersProvider → LogProvider → LegacyAppProvider`。

- `src/context/` 分离设备、寄存器、日志与操作状态；`legacy.tsx` 保留 `useAppContext` 兼容接口，稳定字段供不渲染寄存器表的面板使用。
- `src/lib/register-store.ts` 采用不可变 `ReadonlyMap` 快照，行组件使用 `useSyncExternalStore` 按地址订阅，避免每字节变化让整个页面订阅快照。
- 空 Map 表示未读取；展示默认值不反向填充 store，显式写入的 `0` 始终通过 `??` 保留。
- 私有 UI 操作队列与单例 `hardwareQueue` 分离。硬件 IPC 串行；整个 GAMMA 批次持有一次 FIFO，内层 IPC 明确走 unqueued 边界，避免重入自锁及与其它硬件事务交错。
- GAMMA 保持通道 1→14、高字节→低字节，共访问 `0x0C..0x27` 的 28 个地址。
- UART PMU 配置身份保存在共享设备状态；切换 Tab 不丢失。配置变更只使旧 UART PMU 会话失效，USB 会话保持；显式 Disconnect 继续尝试 PMU 与 UART 两层清理并回报首个失败。
- UART 终端 Stop 使旧轮询失效、合并停止任务并处理组件卸载，测试覆盖 pending poll 后恢复。
- CH347 辅助函数的特性门控同时兼容默认构建和 `--no-default-features`。
- `tauri.conf.json` 显式启用 NSIS，并将 `CH347DLLA64.dll` 映射到安装根目录；`build.rs` 同时复制 DLL 到 Release 与测试二进制目录。

## 已执行的软件验证

环境：Windows x64、Node `24.22.1`、npm `11.19.1`、Rust/MSVC `1.99.1`、Vitest `5.1.0`。

| 检查 | 实际结果 |
| --- | --- |
| `npx tsc --noEmit --pretty false` | 通过 |
| `npm test` | 11 个测试文件、48 项全部通过 |
| `npm run build` | 通过，Vite 生产构建 62 modules |
| `cargo test --manifest-path src-tauri/Cargo.toml` | 59/59 通过 |
| `cargo test --manifest-path src-tauri/Cargo.toml --no-default-features` | 58/58 通过 |
| 两种 Cargo feature 配置的 `cargo clippy --all-targets -- -D warnings` | 均通过 |
| `npm run tauri build` | MSVC Release 与 NSIS 打包成功 |

覆盖范围包括 FIFO 失败恢复、GAMMA 外层事务/内层 IPC 顺序、Mock Scan→Connect→GAMMA Read All、UART 配置失效及 USB 隔离、Configuration 的 `0x31`/`0x46` 默认值与显式零、终端停止生命周期、寄存器换算、稀疏固件 coverage、UART 协议解析与 MCU session 测试。

这些是前端 Mock IPC、Rust 单元测试和构建验证，不是物理芯片验收。测试计数来自上述实际命令；安装归档、交付校验和及桌面检查结果另记于 `desktop/build-artifacts/windows-x64/BUILD_INFO.txt`。

### 复现

在项目根目录运行：

```powershell
npm ci
npx tsc --noEmit --pretty false
npm test
npm run build
npm run tauri dev -- --no-watch
npm run tauri build
```

本工作区的原生构建工具链可加载：

```powershell
. ..\.tooling\msvc-rust-env.ps1
cargo fmt --manifest-path src-tauri\Cargo.toml -- --check
cargo test --manifest-path src-tauri\Cargo.toml
cargo test --manifest-path src-tauri\Cargo.toml --no-default-features
cargo clippy --manifest-path src-tauri\Cargo.toml --all-targets -- -D warnings
cargo clippy --manifest-path src-tauri\Cargo.toml --no-default-features --all-targets -- -D warnings
```

该 PowerShell 环境脚本位于交付目录上一级，不在源码归档中；换机器时安装标准 MSVC/Rust 环境后直接运行 Cargo 命令。

## 交付目录与运行

`desktop/build-artifacts/windows-x64/` 收集 Windows x64 NSIS 安装包、便携 ZIP、源码 ZIP、`SHA256SUMS.txt` 与 `BUILD_INFO.txt`。便携版解压后运行 `ek86317a-programmer.exe`，保留同目录 `CH347DLLA64.dll`。Windows 需 WebView2 Runtime；NSIS 安装包已生成，但安装器 UI 和已安装目录的启动另行记录，不把归档检查当安装交互验收。

源码运行使用 `npm ci && npm run tauri dev`。仅 `npm run dev` 启动的是 Vite，不包含 Tauri 注入的硬件 IPC。Debug 扫描始终追加 `bridge:mock:development`，显示 `Mock device bridge (development)`；Release 不提供 Mock。

### 支持矩阵

- Windows x64：已构建 Release/NSIS；FTDI、CH341A/CH347、UART 路径存在。FTDI 白名单为 FT2232C/H、FT232H、FT4232H/HA。
- Linux x64/ARM64：保留 CH347 随附静态库与源码配置，未在本机交叉构建或运行；按 README 准备系统依赖后原生构建。
- macOS/其他架构：CH347 后端和随附库没有对应支持，不作为已交付目标。
- `--no-default-features` 排除 FTDI/CH347，但 UART 实现仍参与构建，不能视为 UART 被移除。
- UI 时钟选项 100kHz/400kHz/1MHz 不是硬件速率验收；CH341/CH347 的 `bus_recovery` 不发 FTDI 的九个 SCL 恢复脉冲。

## 固件、覆盖与兼容边界

- `Browse` 只接受 raw BIN，HEX 被拒绝。`Write DAC` 写已浏览固件；修改 UI 后依次用 `Write UI to DAC`、`Write DAC to EEPROM`。写入前保留原始 BIN。
- 短/稀疏 BIN 保持可加载；缺失地址不计入 matched。Verify ALL 报告 `compared/missing/partial` 及 DAC/EEPROM 分别的匹配数，缺失时为 `PARTIAL`，`success=false`。
- 为保持既有可见操作语义，未读 GAMMA/Configuration/VCOM 的直接写入仍可使用显示回退值；GAMMA 未读高低字节回退为零。未读空/部分 DAC 快照导出的 BIN 以芯片默认值补齐。它们不是“已读回真实芯片值”的证明；先 Read 目标页再写。
- 空寄存器 Map 保持“未读”，不是“芯片为零”；此次修复了 Configuration 子字段与显示面板的芯片默认回退不一致，不把所有既有直接写入流程改为跳过或自动读取。
- EK86317A HAVDD 前端选项 `AVDD / 512 * (code + 192)` 与后端 `RegisterData.voltage` 的 `AVDD * code / 128` 不一致，属于基线既有问题。需 EK 数据手册确定权威公式后统一；未宣称它直接造成当前可见下拉写错硬件。
- EK 的 OTP 故障状态由后端 bit6 解码，但共享 Fault UI 标为 b7，属于既有标注问题，需按芯片显示位号。
- iML8947K VBK1 使用与 EK 一致的四段非线性映射并覆盖边界测试；尚缺数据手册主来源核验，不把映射一致性当作数据手册确认。

## 验证边界

- 没有连接 FTDI、CH341A、CH347、UART-to-I²C、真实 PMU 或 EEPROM；未执行物理读写、EEPROM 烧录、断电重读、电气时序或目标板验证。
- 当前会话没有 Computer Use preset 的窗口工具，因此未宣称原生 UI 自动点击、安装器交互或截图验收。可在新会话选择 **Computer Use** 后执行这些检查。
- Windows GNU 工具链在中文工作区路径下出现 MinGW `ld` 路径解析错误；本次采用已验证的 MSVC。MSVC 链接的 `LNK4098` 为警告，不影响上述退出码。

## 回滚

1. 退出应用和开发进程，保存需要保留的 BIN；不要删除原 ZIP、baseline 或校验清单。
2. 将当前 `ek86317a-programmer-main` 整个改名保存，保留为可回查的重构目录。
3. 将上一级 `ek86317a-programmer-main.zip` 解压回 `ek86317a-programmer-main`，在还原目录执行 `npm ci`。
4. 仅回滚单文件时，从 `../ek86317a-programmer-baseline/ek86317a-programmer-main/` 复制对应文件；新增测试、文档和 `desktop/` 按需单独保留。
5. 验证还原版本的依赖锁与 `src-tauri/tauri.conf.json` 后再构建。没有使用 Git 提交，因此不要依赖不存在的本地提交历史回滚。
