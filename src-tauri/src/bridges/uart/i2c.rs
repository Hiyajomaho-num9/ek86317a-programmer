//! UART (serial console) I2C bridge.
//!
//! 走面板主芯片的串口控制台，用它自带的 `i2cget` / `i2cset` 去访问 I2C 总线。
//!
//! 设计约束（都是踩过的坑）：
//!
//! 1. **不在链路上传文件**。115200 的线只有 ~11 KB/s，传脚本比手敲还慢，
//!    而且传过去的脚本一旦有一个字节错位就整段废掉。这里所有操作都是
//!    「发一行短命令 + 读回一行文本」，单条命令不超过 40 字节。
//! 2. **不依赖提示符**。用 `echo <beg>; <cmd>; echo <end>$?` 打标记框住输出，
//!    所以 busybox / toybox / bash / ash 都通用，提示符长什么样都无所谓。
//! 3. **抗丢字节**。115200 长时间粘贴文本会掉字符（实测把 for 循环粘进去直接
//!    乱码）。这里逐字节发送且可配字节间延时，另外每一帧都校验首尾标记，
//!    对不上就重试，不会把半截输出当成数据。
//! 4. **toybox 和 i2c-tools 的语法不一致**，必须分开处理：
//!      * `i2cget`：toybox 是 `BUS CHIP [ADDR]`，多给一个 mode 会报
//!        "Max 3 arguments"；i2c-tools 是 `BUS CHIP ADDR [MODE]`。
//!      * `i2cset`：两边都接受尾部的 `b`，但 toybox 把最后一个参数当 MODE，
//!        不给会报 "mode too long"。

use std::io::Write;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serialport::{ClearBuffer, SerialPort};

use crate::bridges::I2cBus;

/// 提权方式：每条命令包一层 `su -c`。
pub const SU_WRAP: &str = "wrap";
/// 提权方式：连接时切一次 root shell。
pub const SU_SESSION: &str = "session";

/// toybox 精简版 i2c-tools（Android / TV 板常见）。
pub const DIALECT_TOYBOX: &str = "toybox";
/// 标准 i2c-tools（Debian / 自编译 busybox 环境常见）。
pub const DIALECT_I2CTOOLS: &str = "i2ctools";

fn default_baud() -> u32 {
    115_200
}
fn default_bus() -> u8 {
    1
}
fn default_dialect() -> String {
    DIALECT_TOYBOX.to_string()
}
fn default_line_ending() -> String {
    "\r".to_string()
}
fn default_timeout() -> u64 {
    2_000
}
fn default_retries() -> u32 {
    3
}
fn default_settle() -> u64 {
    120
}

/// 串口 I2C 桥的连接参数。整个结构体从 UI 传进来，也写进 localStorage。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UartConfig {
    /// 串口设备名，例如 `/dev/ttyUSB0` 或 `COM7`。
    pub port: String,
    #[serde(default = "default_baud")]
    pub baud: u32,
    /// 主芯片上的 I2C 总线号（`i2cdetect -l` 第一列）。
    #[serde(default = "default_bus")]
    pub bus: u8,
    /// `toybox` 或 `i2ctools`。决定 i2cget 的命令拼法。
    #[serde(default = "default_dialect")]
    pub dialect: String,
    /// 地址位移：0 = 直接发 7 位地址；1 = 发 `7bit << 1`（有些板子的 userland 要 8 位写地址）。
    #[serde(default)]
    pub addr_shift: u8,
    /// 行尾。toybox 控制台一般 `\r` 就够，`\r\n` 也认。
    #[serde(default = "default_line_ending")]
    pub line_ending: String,
    /// 逐字节发送时每个字节之间的延时（毫秒）。0 = 整块发送。
    #[serde(default)]
    pub char_delay_ms: u64,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(default = "default_retries")]
    pub retries: u32,
    /// 发命令前等控制台安静下来的时间。
    #[serde(default = "default_settle")]
    pub settle_ms: u64,
    /// 需要登录时填；留空表示串口直接就是 shell。
    #[serde(default)]
    pub login_user: String,
    #[serde(default)]
    pub login_pass: String,
    /// 提权方式。Android / TV 板上 `/dev/i2c-*` 通常只有 root 能开，
    /// 不处理就会拿到 "Permission denied"。
    ///
    /// * `""`        —— 不提权
    /// * `"session"` —— 打开时切一次 root shell，之后所有命令都在 root 里跑（默认）
    /// * `"wrap"`    —— 每条命令包一层 `su -c "..."`（部分 ROM 的 su 不支持 -c）
    #[serde(default)]
    pub su_mode: String,
}

impl Default for UartConfig {
    fn default() -> Self {
        Self {
            port: String::new(),
            baud: default_baud(),
            bus: default_bus(),
            dialect: default_dialect(),
            addr_shift: 0,
            line_ending: default_line_ending(),
            char_delay_ms: 0,
            timeout_ms: default_timeout(),
            retries: default_retries(),
            settle_ms: default_settle(),
            login_user: String::new(),
            login_pass: String::new(),
            // 实测 su -c 在不少定制 ROM 上不通，会话模式兼容性更好，设为默认。
            su_mode: SU_SESSION.to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Dialect {
    Toybox,
    I2cTools,
}

impl Dialect {
    fn parse(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            DIALECT_I2CTOOLS | "i2c-tools" | "tools" => Dialect::I2cTools,
            _ => Dialect::Toybox,
        }
    }
}

/// 串口上的一个端口。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UartPortInfo {
    pub path: String,
    pub description: String,
}

/// 总线扫描结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UartScanResult {
    /// 应答的 7 位地址。
    pub addresses: Vec<u8>,
    /// `i2cdetect` 的原始网格，UI 里直接贴出来。
    pub grid: String,
}

/// 一帧命令的解析结果。
#[derive(Debug, Clone)]
pub(crate) struct Frame {
    pub(crate) rc: i32,
    pub(crate) output: String,
}

/// 串口 I2C 后端。
pub struct UartI2cBus {
    port: Box<dyn SerialPort>,
    cfg: UartConfig,
    dialect: Dialect,
    seq: u64,
    /// 交互终端是否占用了这条串口。
    ///
    /// 终端要自由收发，分帧命令要严格成对标记，两者不能同时在这条线上跑，
    /// 所以用这个开关做互斥：终端开着的时候 exec() 直接拒绝。
    interactive: bool,
    /// 握手时用 `id` 探到的结果。
    is_root: bool,
    /// `id` 的原始输出，UI 里直接显示，省得猜。
    id_line: String,
    /// 打开时发现的问题（比如配了提权却没拿到 root）。
    ///
    /// 不在这里直接返回 Err —— 否则用户连终端都进不去，就没法手工排查 su 到底怎么回事。
    warning: Option<String>,
}

impl UartI2cBus {
    /// 打开串口并（可选）完成登录。
    pub fn open(cfg: UartConfig) -> Result<Self, String> {
        if cfg.port.trim().is_empty() {
            return Err("UART 端口为空".to_string());
        }

        let port = serialport::new(&cfg.port, cfg.baud)
            .timeout(Duration::from_millis(50))
            .open()
            .map_err(|e| format!("打开串口 {} 失败: {}", cfg.port, e))?;

        let dialect = Dialect::parse(&cfg.dialect);
        let mut bus = Self {
            port,
            cfg,
            dialect,
            seq: 0,
            interactive: false,
            is_root: false,
            id_line: String::new(),
            warning: None,
        };

        bus.drain(Duration::from_millis(bus.cfg.settle_ms));

        if !bus.cfg.login_user.is_empty() {
            bus.login()?;
        }

        // 先问 uid。本来就 root 的话，什么都不用折腾。
        // 串口偶发丢字节，探测失败再补一次。
        if !bus.probe_root() {
            bus.probe_root();
        }

        // 不是 root 才提权，而且**先确认机器上真的有 su 再去试**。
        //
        // 之前是闭着眼连试 su / su - / 回车三次，在没有 su 的机器上每次都返回
        // 127（command not found），控制台直接刷出三行 "127|console:/ #"，
        // 看着像程序在乱发命令。
        if !bus.is_root && !bus.cfg.su_mode.is_empty() {
            if bus.has_su() {
                if bus.cfg.su_mode == SU_SESSION {
                    bus.try_session_su();
                }
                bus.probe_root();
            } else {
                bus.warning = Some(
                    "这台机器上找不到 su（command -v su 无输出）。\n\
                     · 如果它本来就是 root，把 Run as root 选 None 即可\n\
                     · 确实要提权的话，进终端手工看 mount / id / whoami"
                        .to_string(),
                );
            }
        }

        log::info!(
            "UART 桥就绪: port={} su_mode={:?} id={:?} root={}",
            bus.cfg.port,
            bus.cfg.su_mode,
            bus.id_line,
            bus.is_root
        );

        // 有 su 但没提到权 —— 记下来交给 UI 显示，但**不**在这里返回 Err。
        // 直接失败会让用户连终端都进不去，而进终端手工排查正是唯一手段。
        if !bus.cfg.su_mode.is_empty() && !bus.is_root && bus.warning.is_none() {
            let mode_label = if bus.cfg.su_mode == SU_SESSION {
                "su 会话"
            } else {
                "su -c"
            };
            let id_out = if bus.id_line.is_empty() {
                "无输出".to_string()
            } else {
                bus.id_line.clone()
            };
            bus.warning = Some(format!(
                "{} 模式下没拿到 root（id: {}）。I2C 读写会被 Permission denied 挡住。\n\
                 可以：换另一个提权方式 / 进下面的终端手工跑 id、which su、su -h 看看真机的 su 怎么用",
                mode_label, id_out
            ));
        }

        Ok(bus)
    }

    /// 列出本机所有串口。
    pub fn list_ports() -> Vec<UartPortInfo> {
        match serialport::available_ports() {
            Ok(ports) => ports
                .into_iter()
                .map(|p| {
                    let description = match &p.port_type {
                        serialport::SerialPortType::UsbPort(info) => {
                            let mut s = info.product.clone().unwrap_or_else(|| "USB serial".into());
                            if let Some(m) = &info.manufacturer {
                                s.push_str(" / ");
                                s.push_str(m);
                            }
                            s
                        }
                        serialport::SerialPortType::PciPort => "PCI serial".to_string(),
                        serialport::SerialPortType::BluetoothPort => "Bluetooth serial".to_string(),
                        serialport::SerialPortType::Unknown => "serial".to_string(),
                    };
                    UartPortInfo {
                        path: p.port_name,
                        description,
                    }
                })
                .collect(),
            Err(e) => {
                log::warn!("枚举串口失败: {}", e);
                Vec::new()
            }
        }
    }

    /// 把 7 位地址换算成实际要发到命令行里的地址。
    fn wire_addr(&self, addr7: u8) -> u8 {
        if self.cfg.addr_shift == 0 {
            addr7
        } else {
            addr7.wrapping_shl(1)
        }
    }

    /// `i2cget -y -f BUS CHIP ADDR`
    fn get_cmd(&self, addr7: u8, reg: u8) -> String {
        build_get_cmd(self.dialect, self.cfg.bus, self.wire_addr(addr7), reg)
    }

    /// `i2cget -y -f BUS CHIP`（不指定寄存器，读当前指针）
    fn get_cmd_no_reg(&self, addr7: u8) -> String {
        let a = self.wire_addr(addr7);
        format!("i2cget -y -f {} 0x{:02X}", self.cfg.bus, a)
    }

    /// `i2cset -y -f BUS CHIP ADDR VALUE b`
    fn set_cmd(&self, addr7: u8, reg: u8, value: u8) -> String {
        build_set_cmd(self.cfg.bus, self.wire_addr(addr7), reg, value)
    }

    /// 按 `su_mode` 把命令包成 root 形式。
    ///
    /// 只处理 "wrap" 模式；"session" 模式在 open() 时就已经切到 root 了，
    /// 不需要逐条加前缀（否则会出现 su 套 su）。
    fn wrap(&self, cmd: &str) -> String {
        wrap_with_su(&self.cfg.su_mode, cmd)
    }

    /// 会话型提权：依次试 `su` / `su -`，每种都等一会儿看有没有密码提示。
    ///
    /// 不同 ROM 的 su 差别很大：有的要按一次回车确认，有的直接给 root，
    /// 有的只支持 -c 装了 su 却进不去交互。这里把能试的都试一遍。
    fn try_session_su(&mut self) {
        for cmd in ["su", "su -"] {
            if self.write_line(cmd).is_err() {
                continue;
            }
            thread::sleep(Duration::from_millis(900));
            let out = self.drain(Duration::from_millis(500));
            let low = out.to_ascii_lowercase();

            if low.contains("password") || low.contains("密码") {
                let answer = if self.cfg.login_pass.is_empty() {
                    String::new()
                } else {
                    self.cfg.login_pass.clone()
                };
                let _ = self.write_line(&answer);
                thread::sleep(Duration::from_millis(700));
                let _ = self.drain(Duration::from_millis(400));
            }

            if self.probe_root() {
                return;
            }
        }

        // 最后补一个回车：有些 su 只是要确认一次
        let _ = self.write_line("");
        thread::sleep(Duration::from_millis(500));
        let _ = self.drain(Duration::from_millis(300));
        self.probe_root();
    }

    /// 跑一次 `id` 判定是否 root。失败只当没拿到，不往上抛。
    fn probe_root(&mut self) -> bool {
        if let Some(frame) = self.exec_soft("id") {
            self.id_line = frame.output.trim().to_string();
            self.is_root = self.id_line.contains("uid=0");
        } else {
            self.is_root = false;
        }
        self.is_root
    }

    /// 握手时是否拿到了 uid=0。
    pub fn is_root(&self) -> bool {
        self.is_root
    }

    /// `id` 的原始输出，UI 里直接显示。
    pub fn id_line(&self) -> &str {
        &self.id_line
    }

    /// 打开时发现的问题，UI 上以横幅形式提示。
    pub fn warning(&self) -> Option<&str> {
        self.warning.as_deref()
    }

    fn detect_cmd(&self, first: u8, last: u8) -> String {
        format!(
            "i2cdetect -y -r {} 0x{:02X} 0x{:02X}",
            self.cfg.bus, first, last
        )
    }

    /// 登录流程（只有配了用户名才会走）。
    fn login(&mut self) -> Result<(), String> {
        self.write_line("")?;
        thread::sleep(Duration::from_millis(400));
        let text = self.drain(Duration::from_millis(600)).to_ascii_lowercase();

        if text.contains("login:") {
            let user = self.cfg.login_user.clone();
            self.write_line(&user)?;
            thread::sleep(Duration::from_millis(400));
            let _ = self.drain(Duration::from_millis(400));
        }
        if text.contains("password:") {
            let pass = self.cfg.login_pass.clone();
            self.write_line(&pass)?;
            thread::sleep(Duration::from_millis(600));
            let _ = self.drain(Duration::from_millis(400));
        }
        Ok(())
    }

    /// 把缓冲区里已有的字节读干净。
    fn drain(&mut self, duration: Duration) -> String {
        let deadline = Instant::now() + duration;
        let mut acc = Vec::new();
        let mut buf = [0u8; 512];
        while Instant::now() < deadline {
            match self.port.read(&mut buf) {
                Ok(0) => thread::sleep(Duration::from_millis(2)),
                Ok(n) => acc.extend_from_slice(&buf[..n]),
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {}
                Err(_) => break,
            }
        }
        String::from_utf8_lossy(&acc).into_owned()
    }

    /// 按配置逐字节（或不拆分）发送一整行。
    fn write_line(&mut self, line: &str) -> Result<(), String> {
        let payload = format!("{}{}", line, self.cfg.line_ending);
        if self.cfg.char_delay_ms == 0 {
            self.port
                .write_all(payload.as_bytes())
                .map_err(|e| format!("串口写失败: {}", e))?;
            self.port
                .flush()
                .map_err(|e| format!("串口 flush 失败: {}", e))?;
            return Ok(());
        }
        for byte in payload.as_bytes() {
            self.port
                .write_all(&[*byte])
                .map_err(|e| format!("串口写失败: {}", e))?;
            self.port
                .flush()
                .map_err(|e| format!("串口 flush 失败: {}", e))?;
            thread::sleep(Duration::from_millis(self.cfg.char_delay_ms));
        }
        Ok(())
    }

    /// 执行一条命令，失败自动重试。
    fn exec(&mut self, cmd: &str) -> Result<Frame, String> {
        if self.interactive {
            return Err("终端正在占用串口，先点「停止终端」再做 I2C 操作".to_string());
        }
        let mut last = String::from("未执行");
        for attempt in 0..=self.cfg.retries {
            match self.exec_once(cmd) {
                Ok(frame) if frame.rc == 0 => return Ok(frame),
                Ok(frame) => {
                    last = format!("rc={} 输出={:?}", frame.rc, frame.output.trim());
                }
                Err(e) => last = e,
            }
            if attempt < self.cfg.retries {
                log::warn!("UART 命令重试 {}/{}: {}", attempt + 1, self.cfg.retries, last);
                thread::sleep(Duration::from_millis(60));
                let _ = self.port.clear(ClearBuffer::All);
                let _ = self.drain(Duration::from_millis(40));
            }
        }
        // 权限错误是最常见的坑（/dev/i2c-* 只有 root 能开），直接给下一步动作。
        if last.contains("Permission denied") && !self.is_root {
            last.push_str(
                " —— /dev/i2c-* 需要 root：把 Run as root 切成 \"su -c\"（或 \"su (会话)\"）再试",
            );
        }
        Err(last)
    }

    /// 跑一次，不管成功失败都不重试。
    ///
    /// 专门给「探测类」命令用 —— `command -v su` 找不到时本来就返回 1，
    /// 用带重试的 exec() 会连试四次，把控制台刷得全是失败提示。
    fn exec_soft(&mut self, cmd: &str) -> Option<Frame> {
        if self.interactive {
            return None;
        }
        self.exec_once(cmd).ok()
    }

    /// 机器上到底有没有 su。
    ///
    /// 没有 su 还去盲试，控制台会连刷几行 127（command not found），
    /// 看起来像程序在乱发命令。
    fn has_su(&mut self) -> bool {
        match self.exec_soft("command -v su") {
            Some(frame) => frame.rc == 0 && !frame.output.trim().is_empty(),
            None => false,
        }
    }

    fn exec_once(&mut self, cmd: &str) -> Result<Frame, String> {
        self.seq += 1;
        let beg = format!("__PB{}__", self.seq);
        let end = format!("__PE{}__", self.seq);
        let sent = format!("echo {}; {}; echo {}$?", beg, self.wrap(cmd), end);

        let _ = self.port.clear(ClearBuffer::Input);
        self.write_line(&sent)?;

        let deadline = Instant::now() + Duration::from_millis(self.cfg.timeout_ms);
        let mut acc: Vec<u8> = Vec::new();
        let mut buf = [0u8; 512];

        while Instant::now() < deadline {
            match self.port.read(&mut buf) {
                Ok(0) => thread::sleep(Duration::from_millis(2)),
                Ok(n) => {
                    acc.extend_from_slice(&buf[..n]);
                    let text = String::from_utf8_lossy(&acc);
                    if let Some(frame) = parse_frame(&text, &beg, &end, &sent) {
                        return Ok(frame);
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {}
                Err(e) => return Err(format!("串口读失败: {}", e)),
            }
        }

        Err(format!(
            "等待 {} 超时，收到 {:?}",
            end,
            String::from_utf8_lossy(&acc)
                .chars()
                .take(200)
                .collect::<String>()
        ))
    }

    // ---------------- 交互终端 ----------------

    /// 进入 / 退出交互终端模式。
    ///
    /// 进入时会先清掉接收缓冲，免得把上一条命令的残渣当成终端输出。
    pub fn set_interactive(&mut self, on: bool) {
        self.interactive = on;
        if on {
            let _ = self.port.clear(ClearBuffer::Input);
        }
    }

    pub fn is_interactive(&self) -> bool {
        self.interactive
    }

    /// 终端模式：原样写一段字节。
    ///
    /// 不做任何转义或加换行 —— 用户敲的什么就发什么，回车、退格、Tab、
    /// Ctrl-C 都靠前端直接发对应的控制字符过来。
    pub fn term_write(&mut self, bytes: &[u8]) -> Result<(), String> {
        if bytes.is_empty() {
            return Ok(());
        }
        self.port
            .write_all(bytes)
            .map_err(|e| format!("串口写失败: {}", e))?;
        self.port
            .flush()
            .map_err(|e| format!("串口 flush 失败: {}", e))?;
        Ok(())
    }

    /// 终端模式：把当前能读到的字节全捞出来。
    ///
    /// 最多等 \`max_ms\`，或攒够 8 KB 就返回，保证 UI 的轮询不会卡住。
    pub fn term_poll(&mut self, max_ms: u64) -> Vec<u8> {
        let deadline = Instant::now() + Duration::from_millis(max_ms);
        let mut acc: Vec<u8> = Vec::new();
        let mut buf = [0u8; 1024];

        loop {
            match self.port.read(&mut buf) {
                Ok(0) => {}
                Ok(n) => acc.extend_from_slice(&buf[..n]),
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {}
                Err(_) => break,
            }
            if acc.len() >= 8192 || Instant::now() >= deadline {
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }

        acc
    }

    /// 扫描总线。
    pub fn scan(&mut self, first: u8, last: u8) -> Result<UartScanResult, String> {
        let cmd = self.detect_cmd(first, last);
        let frame = self.exec(&cmd)?;
        let grid = frame.output.clone();
        Ok(UartScanResult {
            addresses: parse_detect_grid(&grid),
            grid,
        })
    }

    /// 直接透传一条 shell 命令，调试用。
    pub fn shell(&mut self, cmd: &str) -> Result<String, String> {
        let frame = self.exec(cmd)?;
        Ok(frame.output)
    }

    /// 读一个寄存器。
    pub fn read_register(&mut self, addr7: u8, reg: u8) -> Result<u8, String> {
        let cmd = self.get_cmd(addr7, reg);
        let frame = self.exec(&cmd)?;
        parse_hex_byte(&frame.output).ok_or_else(|| {
            format!(
                "读 0x{:02X}[0x{:02X}] 无法解析输出: {:?}",
                addr7,
                reg,
                frame.output.trim()
            )
        })
    }

    /// 写一个寄存器，返回读回值（用于校验）。
    pub fn write_register(&mut self, addr7: u8, reg: u8, value: u8) -> Result<u8, String> {
        let cmd = self.set_cmd(addr7, reg, value);
        self.exec(&cmd)?;
        // toybox 的 i2cset 成功时没有任何输出，必须自己读回确认。
        self.read_register(addr7, reg)
    }
}

impl I2cBus for UartI2cBus {
    fn write(&mut self, addr: u8, data: &[u8]) -> Result<(), String> {
        if data.len() < 2 {
            // 只写寄存器地址（"设置读指针"）没有任何数据，shell 工具做不了也
            // 不需要做，PMU 驱动后面一定会跟一次 write_read。
            return Ok(());
        }
        let reg = data[0];
        let payload = &data[1..];
        if payload.len() == 1 {
            let cmd = self.set_cmd(addr, reg, payload[0]);
            self.exec(&cmd)?;
            return Ok(());
        }
        // 多字节：shell 工具没有通用的批量写，逐个寄存器写。
        // 对寄存器映射型器件与自增写等价，而且每一步都能单独校验。
        for (i, value) in payload.iter().enumerate() {
            let r = reg.wrapping_add(i as u8);
            let cmd = self.set_cmd(addr, r, *value);
            self.exec(&cmd)?;
        }
        Ok(())
    }

    fn read(&mut self, addr: u8, buf: &mut [u8]) -> Result<(), String> {
        if buf.is_empty() {
            return Ok(());
        }
        // 没有寄存器地址：读当前指针
        let cmd = self.get_cmd_no_reg(addr);
        let frame = self.exec(&cmd)?;
        buf[0] = parse_hex_byte(&frame.output)
            .ok_or_else(|| format!("读 0x{:02X} 无法解析输出: {:?}", addr, frame.output.trim()))?;
        for slot in buf.iter_mut().skip(1) {
            *slot = 0;
        }
        Ok(())
    }

    fn write_read(&mut self, addr: u8, write_data: &[u8], read_buf: &mut [u8]) -> Result<(), String> {
        if write_data.is_empty() || read_buf.is_empty() {
            return Ok(());
        }
        let reg = write_data[0];
        for (i, slot) in read_buf.iter_mut().enumerate() {
            let r = reg.wrapping_add(i as u8);
            let cmd = self.get_cmd(addr, r);
            let frame = self.exec(&cmd)?;
            *slot = parse_hex_byte(&frame.output).ok_or_else(|| {
                format!(
                    "读 0x{:02X}[0x{:02X}] 无法解析输出: {:?}",
                    addr,
                    r,
                    frame.output.trim()
                )
            })?;
        }
        Ok(())
    }
}

/// 共享句柄：让 PMU 会话和 UI 里的裸寄存器工具用同一个打开的串口。
///
/// `I2cBus` 的方法要 `&mut self`，而两条路径都要访问，所以套一层 `Arc<Mutex<_>>`。
#[derive(Clone)]
pub struct SharedUartBus(pub Arc<Mutex<UartI2cBus>>);

impl SharedUartBus {
    pub fn new(bus: UartI2cBus) -> Self {
        Self(Arc::new(Mutex::new(bus)))
    }

    pub fn with<T, F>(&self, f: F) -> Result<T, String>
    where
        F: FnOnce(&mut UartI2cBus) -> Result<T, String>,
    {
        let mut guard = self.0.lock().map_err(|_| "UART 桥锁失效".to_string())?;
        f(&mut guard)
    }
}

impl I2cBus for SharedUartBus {
    fn write(&mut self, addr: u8, data: &[u8]) -> Result<(), String> {
        self.with(|bus| bus.write(addr, data))
    }

    fn read(&mut self, addr: u8, buf: &mut [u8]) -> Result<(), String> {
        self.with(|bus| bus.read(addr, buf))
    }

    fn write_read(&mut self, addr: u8, write_data: &[u8], read_buf: &mut [u8]) -> Result<(), String> {
        self.with(|bus| bus.write_read(addr, write_data, read_buf))
    }
}

/// 按 `su_mode` 把一条命令包成 root 形式。
///
/// 抽成自由函数是为了单测不用起串口就能验证。
fn wrap_with_su(su_mode: &str, cmd: &str) -> String {
    if su_mode == SU_WRAP {
        format!("su -c \"{}\"", cmd.replace('"', "\\\""))
    } else {
        cmd.to_string()
    }
}

/// 拼 `i2cget` 命令。抽成自由函数是为了单测能不起串口直接验证语法差异。
fn build_get_cmd(dialect: Dialect, bus: u8, wire_addr: u8, reg: u8) -> String {
    match dialect {
        // toybox 只吃 4 个参数，多一个 mode 直接报 "Max 3 arguments"
        Dialect::Toybox => format!("i2cget -y -f {} 0x{:02X} 0x{:02X}", bus, wire_addr, reg),
        Dialect::I2cTools => format!("i2cget -y -f {} 0x{:02X} 0x{:02X} b", bus, wire_addr, reg),
    }
}

/// 拼 `i2cset` 命令。toybox 把最后一个参数当 MODE，所以 `b` 不能省。
fn build_set_cmd(bus: u8, wire_addr: u8, reg: u8, value: u8) -> String {
    format!(
        "i2cset -y -f {} 0x{:02X} 0x{:02X} 0x{:02X} b",
        bus, wire_addr, reg, value
    )
}

/// 去掉 ANSI 转义序列、`\r` 和 NUL，方便按行解析。
fn normalize(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if ('@'..='~').contains(&next) {
                        break;
                    }
                }
                continue;
            }
            chars.next();
            continue;
        }
        if c == '\r' || c == '\u{0}' {
            continue;
        }
        out.push(c);
    }
    out
}

/// 从一帧原始文本里切出 rc 和输出。
///
/// 帧格式（`<beg>` 和 `<end>` 都是独占一行的标记）：
///
/// `~~~text`
/// echo __PB1__; i2cget -y -f 1 0x20 0x00; echo __PE1__$?     <- 控制台回显，丢弃
/// __PB1__
/// 0x11                                                       <- 真正的输出
/// __PE1__0                                                   <- rc = 0
/// `~~~`
///
/// 首尾标记必须都出现且成对，否则返回 `None` 让上层重试 —— 这样即使半截
/// 数据被当成输出，也不会污染结果。
pub(crate) fn parse_frame(raw: &str, beg: &str, end: &str, sent: &str) -> Option<Frame> {
    let text = normalize(raw);
    let lines: Vec<&str> = text.split('\n').collect();

    let beg_idx = lines.iter().position(|l| l.trim() == beg)?;

    let end_idx = lines
        .iter()
        .enumerate()
        .skip(beg_idx + 1)
        .find(|(_, l)| l.trim().starts_with(end))
        .map(|(i, _)| i)?;

    let rc_text = lines[end_idx].trim().trim_start_matches(end).trim();
    let rc: i32 = rc_text.parse().unwrap_or(0);

    let sent_trimmed = sent.trim();
    let mut body: Vec<&str> = Vec::new();
    for line in &lines[beg_idx + 1..end_idx] {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // 控制台会把命令本身回显一遍，跳过
        if trimmed == sent_trimmed || trimmed.starts_with("echo ") {
            continue;
        }
        body.push(trimmed);
    }

    Some(Frame {
        rc,
        output: body.join("\n"),
    })
}

/// 从 `0x11` / `11` / `0X1A` 这类文本里取出一个字节。
pub(crate) fn parse_hex_byte(text: &str) -> Option<u8> {
    for token in text.split_whitespace() {
        let t = token.trim_start_matches("0x").trim_start_matches("0X");
        if t.len() == 2 && t.chars().all(|c| c.is_ascii_hexdigit()) {
            if let Ok(v) = u8::from_str_radix(t, 16) {
                return Some(v);
            }
        }
    }
    None
}

/// 解析 `i2cdetect -y -r` 的网格，返回有应答的 7 位地址。
///
/// 输入长这样：
///
/// `~~~text`
///      0  1  2  3  4  5  6  7  8  9  a  b  c  d  e  f
/// 00:          -- -- -- -- -- -- -- -- -- -- -- -- --
/// 20: 20 -- -- -- -- -- -- -- -- -- -- -- -- -- -- --
/// 70: -- -- -- -- 74 -- -- --
/// `~~~`
pub(crate) fn parse_detect_grid(grid: &str) -> Vec<u8> {
    let mut found = Vec::new();
    for line in normalize(grid).split('\n') {
        let line = line.trim_end();
        let Some(colon) = line.find(':') else {
            continue;
        };
        let (head, rest) = line.split_at(colon);
        let head = head.trim();
        if head.len() != 2 || !head.chars().all(|c| c.is_ascii_hexdigit()) {
            continue;
        }
        let Ok(row_base) = u8::from_str_radix(head, 16) else {
            continue;
        };
        for (i, cell) in rest[1..].split_whitespace().enumerate() {
            if cell.len() == 2 && cell.chars().all(|c| c.is_ascii_hexdigit()) {
                if let Ok(addr) = u8::from_str_radix(cell, 16) {
                    // 必须和行首算出来的地址一致，防止把列号当成地址
                    if addr == row_base.wrapping_add(i as u8) {
                        found.push(addr);
                    }
                }
            }
        }
    }
    found.sort_unstable();
    found.dedup();
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toybox_get_has_no_mode_argument() {
        // toybox 的 i2cget 只吃 4 个参数，多一个 mode 会报 "Max 3 arguments"
        assert_eq!(
            build_get_cmd(Dialect::Toybox, 1, 0x20, 0x00),
            "i2cget -y -f 1 0x20 0x00"
        );
    }

    #[test]
    fn i2ctools_get_keeps_byte_mode() {
        assert_eq!(
            build_get_cmd(Dialect::I2cTools, 1, 0x20, 0x00),
            "i2cget -y -f 1 0x20 0x00 b"
        );
    }

    #[test]
    fn set_always_carries_trailing_b() {
        // toybox 把最后一个参数当 MODE，不给会报 "mode too long"
        assert_eq!(
            build_set_cmd(1, 0x20, 0x03, 0x0B),
            "i2cset -y -f 1 0x20 0x03 0x0B b"
        );
    }

    #[test]
    fn address_shift_converts_7bit_to_8bit_pair() {
        // 0x40 / 0x41 就是 0x20 的 8 位写/读地址
        assert_eq!(0x20u8.wrapping_shl(1), 0x40);
        assert_eq!(
            build_get_cmd(Dialect::Toybox, 1, 0x20u8.wrapping_shl(1), 0x00),
            "i2cget -y -f 1 0x40 0x00"
        );
    }

    #[test]
    fn parses_full_frame_with_echo() {
        let raw = "echo __PB1__; i2cget -y -f 1 0x20 0x00; echo __PE1__$?\r\n\
                   __PB1__\r\n0x11\r\n__PE1__0\r\n";
        let f = parse_frame(
            raw,
            "__PB1__",
            "__PE1__",
            "echo __PB1__; i2cget -y -f 1 0x20 0x00; echo __PE1__$?",
        )
        .expect("frame");
        assert_eq!(f.rc, 0);
        assert_eq!(f.output, "0x11");
    }

    #[test]
    fn parses_frame_with_error_code() {
        let raw = "__PB7__\r\nError: Read failed\r\n__PE7__1\r\n";
        let f = parse_frame(raw, "__PB7__", "__PE7__", "").expect("frame");
        assert_eq!(f.rc, 1);
        assert_eq!(f.output, "Error: Read failed");
    }

    #[test]
    fn rejects_half_frame_so_caller_retries() {
        // 只有开头标记没有结尾：必须返回 None，绝不能把半截当结果
        let raw = "__PB3__\r\n0x11\r\n";
        assert!(parse_frame(raw, "__PB3__", "__PE3__", "").is_none());
    }

    #[test]
    fn strips_ansi_and_carriage_returns() {
        let raw = "\u{1b}[1;32m__PB9__\u{1b}[0m\r\n0x1A\r\n__PE9__0\r\n";
        let f = parse_frame(raw, "__PB9__", "__PE9__", "").expect("frame");
        assert_eq!(f.output, "0x1A");
        assert_eq!(f.rc, 0);
    }

    #[test]
    fn parses_hex_byte_variants() {
        assert_eq!(parse_hex_byte("0x11"), Some(0x11));
        assert_eq!(parse_hex_byte("11"), Some(0x11));
        assert_eq!(parse_hex_byte("0X1a"), Some(0x1a));
        assert_eq!(parse_hex_byte("Error: Read failed"), None);
    }

    #[test]
    fn parses_real_toybox_detect_output() {
        // 真机 bus 1 的输出
        let grid = "     0  1  2  3  4  5  6  7  8  9  a  b  c  d  e  f\n\
                    00:          -- -- -- -- -- -- -- -- -- -- -- -- --\n\
                    10: -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- --\n\
                    20: 20 -- -- -- -- -- -- -- -- -- -- -- -- -- -- --\n\
                    30: -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- --\n\
                    40: -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- --\n\
                    50: -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- --\n\
                    60: -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- --\n\
                    70: -- -- -- -- 74 -- -- --";
        assert_eq!(parse_detect_grid(grid), vec![0x20, 0x74]);
    }

    #[test]
    fn su_wrap_prefixes_single_command() {
        // Android 上 /dev/i2c-* 只有 root 能开
        assert_eq!(
            wrap_with_su(SU_WRAP, "i2cdetect -y -r 1"),
            "su -c \"i2cdetect -y -r 1\""
        );
    }

    #[test]
    fn su_session_does_not_double_wrap() {
        // 会话模式已经在 open() 切过 root，再包一层会变成 su 套 su
        assert_eq!(
            wrap_with_su(SU_SESSION, "i2cget -y -f 1 0x20 0x00"),
            "i2cget -y -f 1 0x20 0x00"
        );
        assert_eq!(
            wrap_with_su("", "i2cget -y -f 1 0x20 0x00"),
            "i2cget -y -f 1 0x20 0x00"
        );
    }

    #[test]
    fn su_wrap_escapes_inner_quotes() {
        assert_eq!(
            wrap_with_su(SU_WRAP, "echo \"a\""),
            "su -c \"echo \\\"a\\\"\""
        );
    }

    #[test]
    fn detect_grid_ignores_reserved_markers() {
        let grid = "40: UU -- -- -- 44 -- -- --";
        assert_eq!(parse_detect_grid(grid), vec![0x44]);
    }
}
