#![allow(unused)]

pub mod commands;
pub mod ek86317a;
pub mod error;
pub mod ft232h;

use commands::DeviceState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(DeviceState::new())
        .invoke_handler(tauri::generate_handler![
            // Device commands
            commands::device::scan_devices,
            commands::device::connect_device,
            commands::device::disconnect_device,
            commands::device::detect_ic,
            // Register commands
            commands::register::read_dac_register,
            commands::register::write_dac_register,
            commands::register::read_all_dac,
            commands::register::read_all_eeprom,
            // Firmware commands
            commands::firmware::load_firmware,
            commands::firmware::program_firmware,
            commands::firmware::verify_firmware,
            commands::firmware::verify_all,
            commands::firmware::write_all_dac_registers,
            commands::firmware::export_eeprom,
            // EEPROM commands
            commands::eeprom::write_all_to_eeprom,
            commands::eeprom::write_vcom1_to_eeprom,
            commands::eeprom::read_fault_flags,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
