//! UART（串口控制台）桥的 Tauri 命令。
//!
//! 这一组命令做两件事：
//!
//! 1. **让原来的 PMU 流程跑在串口上**：`connect_uart_device` 用串口桥建出一个
//!    和 FTDI / CH347 完全一样的 `ChipDevice` 塞进 `DeviceState`，之后
//!    Read DAC / Write EEPROM / Verify ALL 这些按钮原样可用，不需要改一行。
//! 2. **裸 I2C 调试**：扫描总线、按 7 位地址读写任意寄存器、透传 shell 命令。
//!    就是替代手工敲 `i2cget` / `i2cset` 的那部分。
//!
//! 串口是独占资源，所以这里用 `SharedUartBus` 把「PMU 会话」和「裸寄存器工具」
//! 接到同一个已经打开的句柄上，避免两边抢端口。

use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::bridges::uart::{SharedUartBus, UartConfig, UartI2cBus, UartPortInfo, UartScanResult};
use crate::bridges::I2cBus;
use crate::error::AppError;
use crate::pmu::chip::{spec_for_model, ChipModel};
use crate::pmu::device::ChipDevice;

use super::{DeviceConnection, DeviceInfo, DeviceState, SharedDevice};

/// 把会影响句柄、登录或命令会话的字段拼成一个无歧义指纹。
///
/// 字符串使用 Debug 转义并带引号，避免分隔符出现在配置值中时发生碰撞。
/// 登录信息、提权和超时参数变了也必须重开，不能复用旧会话。
fn config_key(cfg: &UartConfig) -> String {
    format!(
        "port={:?};baud={};bus={};dialect={:?};addr_shift={};line_ending={:?};\
         char_delay_ms={};timeout_ms={};retries={};settle_ms={};\
         login_user={:?};login_pass={:?};su_mode={:?}",
        cfg.port,
        cfg.baud,
        cfg.bus,
        cfg.dialect,
        cfg.addr_shift,
        cfg.line_ending,
        cfg.char_delay_ms,
        cfg.timeout_ms,
        cfg.retries,
        cfg.settle_ms,
        cfg.login_user,
        cfg.login_pass,
        cfg.su_mode,
    )
}

/// 当前打开的串口桥 + 它对应的配置指纹。指纹变了就重开。
pub type UartSlot = Arc<Mutex<Option<(String, SharedUartBus)>>>;

/// Tauri 侧管理的串口状态。
#[derive(Default)]
pub struct UartState {
    slot: UartSlot,
    device: SharedDevice,
}

impl UartState {
    pub fn new(device: &DeviceState) -> Self {
        Self {
            slot: Arc::new(Mutex::new(None)),
            device: device.shared_device(),
        }
    }

    pub fn slot(&self) -> UartSlot {
        Arc::clone(&self.slot)
    }

    fn device(&self) -> SharedDevice {
        Arc::clone(&self.device)
    }
}

/// 打开（或复用）串口桥。配置变化先使旧 PMU 会话失效；
/// 相同指纹的 PMU 和裸 UART 操作继续复用同一个总线。
fn ensure_bus(
    slot: &UartSlot,
    device: &SharedDevice,
    cfg: &UartConfig,
) -> Result<SharedUartBus, String> {
    let key = config_key(cfg);
    DeviceState::close_uart_session_if_config_changed(device, &key)?;
    let mut guard = slot.lock().map_err(|_| AppError::LockError.to_string())?;
    if let Some((cached_key, bus)) = guard.as_ref() {
        if *cached_key == key {
            return Ok(bus.clone());
        }
    }

    // 参数变了（或首次打开）：先丢掉旧的，再开新的。
    *guard = None;
    let bus = UartI2cBus::open(cfg.clone())?;
    let shared = SharedUartBus::new(bus);
    *guard = Some((key, shared.clone()));
    Ok(shared)
}

#[tauri::command]
pub async fn uart_list_ports() -> Result<Vec<UartPortInfo>, String> {
    tokio::task::spawn_blocking(UartI2cBus::list_ports)
        .await
        .map_err(|e| format!("枚举串口任务失败: {}", e))
}

/// 打开串口并握手。不建 PMU 会话，只验证链路通不通。
/// 打开串口后的链路信息。`root` 是握手时用 `id` 探到的，UI 直接显示。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UartOpenResult {
    pub path: String,
    /// 是否已经拿到 uid=0
    pub root: bool,
    /// `id` 的原始输出
    pub id_line: String,
    /// 打开时发现的问题（比如提权没成功），UI 用横幅提示
    pub warning: Option<String>,
}

#[tauri::command]
pub async fn uart_open(
    state: State<'_, UartState>,
    config: UartConfig,
) -> Result<UartOpenResult, String> {
    let slot = state.slot();
    let device_handle = state.device();
    let path = config.port.clone();
    tokio::task::spawn_blocking(move || {
        let bus = ensure_bus(&slot, &device_handle, &config)?;
        let (root, id_line, warning) = bus.with(|b| {
            Ok((
                b.is_root(),
                b.id_line().to_string(),
                b.warning().map(|w| w.to_string()),
            ))
        })?;
        Ok::<UartOpenResult, String>(UartOpenResult {
            path,
            root,
            id_line,
            warning,
        })
    })
    .await
    .map_err(|e| format!("打开串口任务失败: {}", e))?
}

#[tauri::command]
pub async fn uart_close(state: State<'_, UartState>) -> Result<(), String> {
    let slot = state.slot();
    DeviceState::close_uart_session(&state.device)?;
    let mut guard = slot.lock().map_err(|_| AppError::LockError.to_string())?;
    *guard = None;
    Ok(())
}

/// 扫描总线。返回应答地址 + 原始网格文本。
#[tauri::command]
pub async fn uart_scan_bus(
    state: State<'_, UartState>,
    config: UartConfig,
    first: Option<u8>,
    last: Option<u8>,
) -> Result<UartScanResult, String> {
    let slot = state.slot();
    let device_handle = state.device();
    tokio::task::spawn_blocking(move || {
        let bus = ensure_bus(&slot, &device_handle, &config)?;
        bus.with(|b| b.scan(first.unwrap_or(0x03), last.unwrap_or(0x77)))
    })
    .await
    .map_err(|e| format!("总线扫描任务失败: {}", e))?
}

/// 读一个寄存器（7 位地址）。
#[tauri::command]
pub async fn uart_read_register(
    state: State<'_, UartState>,
    config: UartConfig,
    addr: u8,
    reg: u8,
) -> Result<u8, String> {
    let slot = state.slot();
    let device_handle = state.device();
    tokio::task::spawn_blocking(move || {
        let bus = ensure_bus(&slot, &device_handle, &config)?;
        bus.with(|b| b.read_register(addr, reg))
    })
    .await
    .map_err(|e| format!("读取任务失败: {}", e))?
}

/// 写一个寄存器并读回校验，返回读回值。
#[tauri::command]
pub async fn uart_write_register(
    state: State<'_, UartState>,
    config: UartConfig,
    addr: u8,
    reg: u8,
    value: u8,
) -> Result<u8, String> {
    let slot = state.slot();
    let device_handle = state.device();
    tokio::task::spawn_blocking(move || {
        let bus = ensure_bus(&slot, &device_handle, &config)?;
        bus.with(|b| b.write_register(addr, reg, value))
    })
    .await
    .map_err(|e| format!("写入任务失败: {}", e))?
}

/// 连续 dump 一段寄存器，UI 里一次看完。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UartDumpResult {
    pub addr: u8,
    pub first: u8,
    pub last: u8,
    pub values: Vec<(u8, u8)>,
}

fn uart_dump_range_is_invalid(first: u8, last: u8) -> bool {
    last < first
}

#[tauri::command]
pub async fn uart_dump(
    state: State<'_, UartState>,
    config: UartConfig,
    addr: u8,
    first: u8,
    last: u8,
) -> Result<UartDumpResult, String> {
    if uart_dump_range_is_invalid(first, last) {
        return Err("结束寄存器小于起始寄存器".to_string());
    }
    let slot = state.slot();
    let device_handle = state.device();
    tokio::task::spawn_blocking(move || {
        let bus = ensure_bus(&slot, &device_handle, &config)?;
        let mut values = Vec::new();
        for reg in first..=last {
            let v = bus.with(|b| b.read_register(addr, reg))?;
            values.push((reg, v));
        }
        Ok(UartDumpResult {
            addr,
            first,
            last,
            values,
        })
    })
    .await
    .map_err(|e| format!("dump 任务失败: {}", e))?
}

/// 透传一条 shell 命令，调试用。输出原样返回。
#[tauri::command]
pub async fn uart_shell(
    state: State<'_, UartState>,
    config: UartConfig,
    command: String,
) -> Result<String, String> {
    let cmd = command.trim().to_string();
    if cmd.is_empty() {
        return Err("命令为空".to_string());
    }
    let slot = state.slot();
    let device_handle = state.device();
    tokio::task::spawn_blocking(move || {
        let bus = ensure_bus(&slot, &device_handle, &config)?;
        bus.with(|b| b.shell(&cmd))
    })
    .await
    .map_err(|e| format!("命令执行任务失败: {}", e))?
}

// ---------------- 交互终端 ----------------
//
// 和上面的分帧命令是互斥的：终端模式下 exec() 会直接拒绝，
// 所以 UI 上必须先「停止终端」才能回到 I2C 操作。

/// 进入交互终端模式。
#[tauri::command]
pub async fn uart_term_start(
    state: State<'_, UartState>,
    config: UartConfig,
) -> Result<(), String> {
    let slot = state.slot();
    let device_handle = state.device();
    tokio::task::spawn_blocking(move || {
        let bus = ensure_bus(&slot, &device_handle, &config)?;
        bus.with(|b| {
            b.set_interactive(true);
            Ok(())
        })
    })
    .await
    .map_err(|e| format!("进入终端任务失败: {}", e))?
}

/// 退出交互终端模式，回到分帧命令。
#[tauri::command]
pub async fn uart_term_stop(state: State<'_, UartState>) -> Result<(), String> {
    let slot = state.slot();
    tokio::task::spawn_blocking(move || {
        let guard = slot.lock().map_err(|_| AppError::LockError.to_string())?;
        if let Some((_, bus)) = guard.as_ref() {
            bus.with(|b| {
                b.set_interactive(false);
                Ok(())
            })?;
        }
        Ok::<(), String>(())
    })
    .await
    .map_err(|e| format!("退出终端任务失败: {}", e))?
}

/// 终端模式：原样发送一段文本（控制字符直接写成 \u0003 这类）。
#[tauri::command]
pub async fn uart_term_send(
    state: State<'_, UartState>,
    config: UartConfig,
    data: String,
) -> Result<(), String> {
    if data.is_empty() {
        return Ok(());
    }
    let slot = state.slot();
    let device_handle = state.device();
    tokio::task::spawn_blocking(move || {
        let bus = ensure_bus(&slot, &device_handle, &config)?;
        bus.with(|b| b.term_write(data.as_bytes()))
    })
    .await
    .map_err(|e| format!("终端发送任务失败: {}", e))?
}

/// 终端模式：把当前可读的字节取回来。
///
/// 回 base64 而不是数字数组：JSON 里数组会把体积放大 4 倍左右，
/// 115200 下这个差别不小。前端用 TextDecoder(stream: true) 拼，
/// 多字节字符被拆到两次轮询之间也不会乱码。
#[tauri::command]
pub async fn uart_term_poll(
    state: State<'_, UartState>,
    config: UartConfig,
    max_ms: Option<u64>,
) -> Result<String, String> {
    use base64::Engine as _;
    let slot = state.slot();
    let device_handle = state.device();
    tokio::task::spawn_blocking(move || {
        let bus = ensure_bus(&slot, &device_handle, &config)?;
        let bytes = bus.with(|b| Ok(b.term_poll(max_ms.unwrap_or(60))))?;
        Ok::<String, String>(base64::engine::general_purpose::STANDARD.encode(bytes))
    })
    .await
    .map_err(|e| format!("终端读取任务失败: {}", e))?
}

/// 用串口桥建立 PMU 会话。
///
/// 成功后所有既有的寄存器 / 固件命令都直接可用，只是底层从 FTDI 换成了串口。
#[tauri::command]
pub async fn connect_uart_device(
    state: State<'_, DeviceState>,
    uart_state: State<'_, UartState>,
    config: UartConfig,
    chip_model: ChipModel,
) -> Result<DeviceInfo, String> {
    log::info!(
        "Connecting via UART {}@{} bus={} as {}",
        config.port,
        config.baud,
        config.bus,
        chip_model.display_name()
    );

    let device_handle = Arc::clone(&state.device);
    let slot = uart_state.slot();
    let device_id = format!("bridge:uart:{}", config.port);

    tokio::task::spawn_blocking(move || {
        let bus = ensure_bus(&slot, &device_handle, &config)?;
        let spec = spec_for_model(chip_model);
        let mut device = ChipDevice::new(Box::new(bus) as Box<dyn I2cBus>, spec);

        let (pmic_detected, vcom_detected) =
            device.probe().map_err(|e| format!("Probe failed: {}", e))?;

        let info = DeviceInfo {
            pmic_detected,
            vcom_detected,
            device_id,
            chip_model,
        };

        let mut guard = device_handle
            .lock()
            .map_err(|_| AppError::LockError.to_string())?;
        *guard = Some(DeviceConnection {
            device,
            uart_config_key: Some(config_key(&config)),
        });

        log::info!(
            "UART connected: chip={}, PMIC={}, VCOM={:?}",
            chip_model.display_name(),
            pmic_detected,
            vcom_detected
        );

        Ok(info)
    })
    .await
    .map_err(|e| format!("UART 连接任务失败: {}", e))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uart_device_id_carries_port() {
        let id = format!("bridge:uart:{}", "/dev/ttyUSB0");
        assert!(id.starts_with("bridge:uart:"));
        assert!(id.ends_with("/dev/ttyUSB0"));
    }

    #[test]
    fn default_config_is_toybox_on_bus_1_at_115200() {
        let cfg = UartConfig::default();
        assert_eq!(cfg.baud, 115_200);
        assert_eq!(cfg.bus, 1);
        assert_eq!(cfg.dialect, "toybox");
        assert_eq!(cfg.addr_shift, 0);
        assert_eq!(cfg.line_ending, "\r");
        assert_eq!(cfg.su_mode, "session");
    }

    #[test]
    fn dump_range_guard_rejects_inverted_range() {
        // 只是把边界判断的逻辑固化下来，防止 UI 传反
        let first = 0x20u8;
        let last = 0x10u8;
        assert!(uart_dump_range_is_invalid(first, last));
    }

    #[test]
    fn config_key_distinguishes_every_session_affecting_setting() {
        let base = UartConfig {
            port: "COM7".to_string(),
            ..UartConfig::default()
        };
        assert_eq!(config_key(&base), config_key(&base.clone()));

        let mut char_delay = base.clone();
        char_delay.char_delay_ms = 2;
        let mut timeout = base.clone();
        timeout.timeout_ms += 1;
        let mut retries = base.clone();
        retries.retries += 1;
        let mut settle = base.clone();
        settle.settle_ms += 1;
        let mut login_user = base.clone();
        login_user.login_user = "root".to_string();
        let mut login_pass = base.clone();
        login_pass.login_pass = "secret".to_string();
        let mut su_mode = base.clone();
        su_mode.su_mode = "wrap".to_string();

        let variants = [
            ("char_delay_ms", char_delay),
            ("timeout_ms", timeout),
            ("retries", retries),
            ("settle_ms", settle),
            ("login_user", login_user),
            ("login_pass", login_pass),
            ("su_mode", su_mode),
        ];
        for (field, config) in variants {
            assert_ne!(
                config_key(&base),
                config_key(&config),
                "{field} change must force a new UART session"
            );
        }
    }

    #[test]
    fn config_key_does_not_collide_on_string_delimiters() {
        let first = UartConfig {
            login_user: "a\".to_string()".to_string(),
            ..UartConfig::default()
        };
        let second = UartConfig {
            login_user: "a".to_string(),
            ..UartConfig::default()
        };
        assert_ne!(config_key(&first), config_key(&second));
    }
}
