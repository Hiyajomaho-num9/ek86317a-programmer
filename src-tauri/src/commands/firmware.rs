//! Firmware programming commands

use std::path::Path;

use tauri::State;

use crate::ek86317a::firmware::FirmwareImage;
use crate::ek86317a::registers;

use super::{
    DeviceState, FirmwarePreview, ProgramResult, RegisterData, VerifyAllResult, VerifyResult,
    WriteAllDacResult,
};

/// Load and preview a firmware file without programming.
#[tauri::command]
pub async fn load_firmware(path: String) -> Result<FirmwarePreview, String> {
    log::info!("Loading firmware from: {}", path);

    let fw = FirmwareImage::from_file(&path)?;

    let file_name = Path::new(&path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.clone());

    let all_regs = fw.get_all_registers();
    let avdd_value = all_regs
        .iter()
        .find(|(a, _)| *a == registers::REG_AVDD)
        .map(|(_, v)| *v);
    let vcom_min_value = all_regs
        .iter()
        .find(|(a, _)| *a == registers::REG_VCOM_MIN)
        .map(|(_, v)| *v);
    let vcom_max_value = all_regs
        .iter()
        .find(|(a, _)| *a == registers::REG_VCOM_MAX)
        .map(|(_, v)| *v);

    let register_data: Vec<RegisterData> = all_regs
        .iter()
        .map(|(addr, value)| {
            let name = registers::get_register_name(*addr).to_string();
            let voltage = registers::decode_register_voltage(
                *addr,
                *value,
                avdd_value,
                vcom_min_value,
                vcom_max_value,
            );
            RegisterData {
                address: *addr,
                value: *value,
                name,
                voltage,
            }
        })
        .collect();

    Ok(FirmwarePreview {
        file_name,
        size: fw.size,
        register_count: fw.register_count,
        registers: register_data,
    })
}

/// Program firmware to device DAC registers, optionally writing to EEPROM.
#[tauri::command]
pub async fn program_firmware(
    state: State<'_, DeviceState>,
    path: String,
    write_eeprom: bool,
) -> Result<ProgramResult, String> {
    log::info!(
        "Programming firmware from: {}, write_eeprom={}",
        path,
        write_eeprom
    );

    let fw = FirmwareImage::from_file(&path)?;
    let all_regs = fw.get_all_registers();

    super::with_device(&state, move |device| {
        let mut count = 0;
        for (addr, value) in &all_regs {
            device.write_dac_register(*addr, *value)?;
            count += 1;
        }

        let eeprom_written = if write_eeprom {
            device.write_all_to_eeprom()?;
            true
        } else {
            false
        };

        log::info!(
            "Programming complete: {} registers written, EEPROM={}",
            count,
            eeprom_written
        );

        Ok(ProgramResult {
            success: true,
            registers_written: count,
            eeprom_written,
        })
    })
    .await
}

/// Verify firmware against device DAC register contents.
#[tauri::command]
pub async fn verify_firmware(
    state: State<'_, DeviceState>,
    path: String,
) -> Result<VerifyResult, String> {
    log::info!("Verifying firmware from: {}", path);

    let fw = FirmwareImage::from_file(&path)?;
    let total = fw.register_count;
    let fw_bytes = fw.as_bytes().to_vec();

    super::with_device(&state, move |device| {
        let mismatches = device.verify_firmware(&fw_bytes)?;
        let matched = total - mismatches.len();

        Ok(VerifyResult {
            success: mismatches.is_empty(),
            total,
            matched,
            mismatches,
        })
    })
    .await
}

/// Export current EEPROM contents to a binary file.
#[tauri::command]
pub async fn export_eeprom(state: State<'_, DeviceState>, path: String) -> Result<(), String> {
    log::info!("Exporting EEPROM to: {}", path);

    super::with_device(&state, move |device| {
        let eeprom_data = device.read_all_eeprom()?;

        let max_addr = eeprom_data
            .iter()
            .map(|(a, _)| *a as usize)
            .max()
            .unwrap_or(0);

        let mut bin = vec![0u8; max_addr + 1];
        for (addr, value) in &eeprom_data {
            bin[*addr as usize] = *value;
        }

        std::fs::write(&path, &bin).map_err(|e| format!("Failed to write file: {}", e))?;

        log::info!("Exported {} bytes to {}", bin.len(), path);
        Ok(())
    })
    .await
}

/// Verify firmware against both DAC and EEPROM banks.
#[tauri::command]
pub async fn verify_all(
    state: State<'_, DeviceState>,
    path: String,
) -> Result<VerifyAllResult, String> {
    log::info!("Verify ALL (DAC + EEPROM) from: {}", path);

    let fw = FirmwareImage::from_file(&path)?;
    let total = fw.register_count;
    let fw_bytes = fw.as_bytes().to_vec();

    super::with_device(&state, move |device| {
        let (dac_mismatches, eeprom_mismatches) = device.verify_all(&fw_bytes)?;

        Ok(VerifyAllResult {
            total,
            dac_matched: total - dac_mismatches.len(),
            eeprom_matched: total - eeprom_mismatches.len(),
            dac_mismatches,
            eeprom_mismatches,
        })
    })
    .await
}

/// Batch write DAC registers from (address, value) pairs.
#[tauri::command]
pub async fn write_all_dac_registers(
    state: State<'_, DeviceState>,
    entries: Vec<(u8, u8)>,
) -> Result<WriteAllDacResult, String> {
    log::info!("Batch writing {} DAC registers", entries.len());

    if entries
        .iter()
        .any(|(addr, _)| *addr == registers::REG_CONTROL)
    {
        return Err("Control register 0xFF is reserved for dedicated EEPROM commands".to_string());
    }

    super::with_device(&state, move |device| {
        let count = device.write_all_dac_registers(&entries)?;

        log::info!("Batch write complete: {} registers written", count);

        Ok(WriteAllDacResult {
            success: true,
            registers_written: count,
        })
    })
    .await
}
