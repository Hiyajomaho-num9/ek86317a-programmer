//! EEPROM and fault flag commands

use tauri::State;

use crate::ek86317a::registers::FaultFlags;

use super::DeviceState;

/// Write all DAC registers to EEPROM.
#[tauri::command]
pub async fn write_all_to_eeprom(state: State<'_, DeviceState>) -> Result<(), String> {
    log::info!("Writing all DAC registers to EEPROM...");

    super::with_device(&state, move |device| device.write_all_to_eeprom()).await?;

    log::info!("EEPROM write complete");
    Ok(())
}

/// Write only VCOM1_NT to EEPROM.
#[tauri::command]
pub async fn write_vcom1_to_eeprom(state: State<'_, DeviceState>) -> Result<(), String> {
    log::info!("Writing VCOM1_NT to EEPROM...");

    super::with_device(&state, move |device| device.write_vcom1_to_eeprom()).await?;

    log::info!("VCOM1_NT EEPROM write complete");
    Ok(())
}

/// Read fault flags from the VCOM slave.
#[tauri::command]
pub async fn read_fault_flags(state: State<'_, DeviceState>) -> Result<FaultFlags, String> {
    log::info!("Reading fault flags...");

    super::with_device(&state, move |device| {
        let raw = device.read_fault_flags()?;
        let flags = FaultFlags::from_raw(raw);

        log::info!("Fault flags: raw=0x{:02X}, {:?}", raw, flags);
        Ok(flags)
    })
    .await
}
