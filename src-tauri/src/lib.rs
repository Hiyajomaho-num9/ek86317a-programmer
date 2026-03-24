#![allow(unused)]

pub mod bridges;
pub mod error;
pub mod pmu;

use pmu::commands::DeviceState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(DeviceState::new())
        .invoke_handler(tauri::generate_handler![
            pmu::commands::device::scan_devices,
            pmu::commands::device::connect_device,
            pmu::commands::device::disconnect_device,
            pmu::commands::device::detect_ic,
            pmu::commands::register::read_dac_register,
            pmu::commands::register::write_dac_register,
            pmu::commands::register::read_all_dac,
            pmu::commands::register::read_all_eeprom,
            pmu::commands::firmware::load_firmware,
            pmu::commands::firmware::program_firmware,
            pmu::commands::firmware::verify_firmware,
            pmu::commands::firmware::verify_all,
            pmu::commands::firmware::write_all_dac_registers,
            pmu::commands::firmware::export_eeprom,
            pmu::commands::eeprom::write_all_to_eeprom,
            pmu::commands::eeprom::write_vcom1_to_eeprom,
            pmu::commands::eeprom::read_fault_flags,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
