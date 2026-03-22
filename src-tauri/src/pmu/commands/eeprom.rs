//! EEPROM and fault flag commands

use tauri::State;

use crate::pmu::chip::{self, FaultFlags};

use super::DeviceState;

#[tauri::command]
pub async fn write_all_to_eeprom(state: State<'_, DeviceState>) -> Result<(), String> {
    log::info!("Writing all DAC registers to EEPROM...");

    super::with_device(&state, move |device| device.write_all_to_eeprom()).await?;

    log::info!("EEPROM write complete");
    Ok(())
}

#[tauri::command]
pub async fn write_vcom1_to_eeprom(state: State<'_, DeviceState>) -> Result<(), String> {
    log::info!("Writing VCOM1 to EEPROM...");

    super::with_device(&state, move |device| device.write_vcom1_to_eeprom()).await?;

    log::info!("VCOM EEPROM write complete");
    Ok(())
}

#[tauri::command]
pub async fn read_fault_flags(state: State<'_, DeviceState>) -> Result<FaultFlags, String> {
    log::info!("Reading fault flags...");

    super::with_device(&state, move |device| {
        if !device.spec().supports_fault_flags {
            return Err(format!("{} does not support separate fault flags", device.spec().display_name));
        }

        let raw = device.read_fault_flags()?;
        Ok(chip::decode_fault_flags(device.chip_model(), raw))
    })
    .await
}
