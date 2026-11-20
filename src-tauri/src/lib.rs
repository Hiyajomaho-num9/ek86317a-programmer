pub mod bridges;
pub mod error;
pub mod pmu;

use std::fs::File;
use std::path::PathBuf;

use pmu::commands::uart::UartState;
use pmu::commands::DeviceState;

/// Presence-detection engine built into this binary. Printed on startup so a log
/// file always says which build produced it.
const PROBE_ENGINE: &str = "floating-baseline-v2";

/// `%LOCALAPPDATA%\EK86317A Programmer\ek86317a-programmer.log`
///
/// A release build has no console (`windows_subsystem = "windows"`), so without
/// a log file the detection decisions are invisible to the user — and to anyone
/// asked to explain a wrong detection.
fn log_file_path() -> Option<PathBuf> {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("TEMP").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."));
    let dir = base.join("EK86317A Programmer");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join("ek86317a-programmer.log"))
}

fn init_logging() {
    let mut builder =
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"));

    match log_file_path().and_then(|path| File::create(path).ok()) {
        Some(file) => builder.target(env_logger::Target::Pipe(Box::new(file))),
        None => builder.target(env_logger::Target::Stderr),
    }
    .init();

    log::info!(
        "EK86317A Programmer {} starting (probe-engine={})",
        env!("CARGO_PKG_VERSION"),
        PROBE_ENGINE
    );
    if let Some(path) = log_file_path() {
        log::info!("log file: {}", path.display());
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_logging();

    let device_state = DeviceState::new();
    let uart_state = UartState::new(&device_state);

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(device_state)
        .manage(uart_state)
        .invoke_handler(tauri::generate_handler![
            pmu::commands::device::scan_devices,
            pmu::commands::device::connect_device,
            pmu::commands::device::disconnect_device,
            pmu::commands::device::detect_ic,
            pmu::commands::uart::uart_list_ports,
            pmu::commands::uart::uart_open,
            pmu::commands::uart::uart_close,
            pmu::commands::uart::uart_scan_bus,
            pmu::commands::uart::uart_read_register,
            pmu::commands::uart::uart_write_register,
            pmu::commands::uart::uart_dump,
            pmu::commands::uart::uart_shell,
            pmu::commands::uart::uart_term_start,
            pmu::commands::uart::uart_term_stop,
            pmu::commands::uart::uart_term_send,
            pmu::commands::uart::uart_term_poll,
            pmu::commands::uart::connect_uart_device,
            pmu::commands::register::read_dac_register,
            pmu::commands::register::write_dac_register,
            pmu::commands::register::read_all_dac,
            pmu::commands::register::read_all_eeprom,
            pmu::commands::firmware::load_firmware,
            pmu::commands::firmware::program_firmware,
            pmu::commands::firmware::verify_firmware,
            pmu::commands::firmware::verify_all,
            pmu::commands::firmware::write_all_dac_registers,
            pmu::commands::firmware::export_registers,
            pmu::commands::eeprom::write_all_to_eeprom,
            pmu::commands::eeprom::write_vcom1_to_eeprom,
            pmu::commands::eeprom::read_fault_flags,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
