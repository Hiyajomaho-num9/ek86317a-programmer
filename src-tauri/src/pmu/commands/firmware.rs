//! Firmware programming commands

use std::path::Path;

use tauri::State;

use crate::pmu::chip::{self, spec_for_model, ChipModel};
use crate::pmu::firmware::FirmwareImage;

use super::{
    DeviceState, FirmwarePreview, ProgramResult, RegisterData, VerifyAllResult, VerifyResult,
    WriteAllDacResult,
};

#[tauri::command]
pub async fn load_firmware(path: String, chip_model: ChipModel) -> Result<FirmwarePreview, String> {
    log::info!(
        "Loading firmware from: {} as {}",
        path,
        chip_model.display_name()
    );

    let fw = FirmwareImage::from_file(&path, chip_model)?;
    let spec = spec_for_model(chip_model);

    let file_name = Path::new(&path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.clone());

    let all_regs = fw.get_all_registers();
    let avdd_value = all_regs
        .iter()
        .find(|(a, _)| *a == spec.avdd_reg)
        .map(|(_, v)| *v);
    let vcom_min_value = spec.vcom_min_reg.and_then(|reg| {
        all_regs
            .iter()
            .find(|(a, _)| *a == reg)
            .map(|(_, value)| *value)
    });
    let vcom_max_value = spec.vcom_max_reg.and_then(|reg| {
        all_regs
            .iter()
            .find(|(a, _)| *a == reg)
            .map(|(_, value)| *value)
    });
    let mode_value = spec.mode_reg.and_then(|reg| {
        all_regs
            .iter()
            .find(|(a, _)| *a == reg)
            .map(|(_, value)| *value)
    });

    let register_data: Vec<RegisterData> = all_regs
        .iter()
        .map(|(addr, value)| RegisterData {
            address: *addr,
            value: *value,
            name: crate::pmu::chip::get_register_name(chip_model, *addr).to_string(),
            voltage: crate::pmu::chip::decode_register_voltage(
                chip_model,
                *addr,
                *value,
                avdd_value,
                vcom_min_value,
                vcom_max_value,
                mode_value,
            ),
        })
        .collect();

    Ok(FirmwarePreview {
        file_name,
        size: fw.size,
        register_count: fw.register_count,
        registers: register_data,
    })
}

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

    super::with_device(&state, move |device| {
        let fw = FirmwareImage::from_file(&path, device.chip_model())?;
        let all_regs = fw.get_all_registers();
        let mut count = 0;

        for (addr, value) in all_regs {
            device.write_dac_register(*addr, *value)?;
            count += 1;
        }

        let eeprom_written = if write_eeprom {
            device.write_all_to_eeprom()?;
            true
        } else {
            false
        };

        Ok(ProgramResult {
            success: true,
            registers_written: count,
            eeprom_written,
        })
    })
    .await
}

#[tauri::command]
pub async fn verify_firmware(
    state: State<'_, DeviceState>,
    path: String,
) -> Result<VerifyResult, String> {
    log::info!("Verifying firmware from: {}", path);

    super::with_device(&state, move |device| {
        let fw = FirmwareImage::from_file(&path, device.chip_model())?;
        let total = fw.register_count;
        let mismatches = device.verify_firmware(fw.as_bytes())?;
        let matched = total.saturating_sub(mismatches.len());

        Ok(VerifyResult {
            success: mismatches.is_empty(),
            total,
            matched,
            mismatches,
        })
    })
    .await
}

#[tauri::command]
pub async fn export_registers(
    path: String,
    chip_model: ChipModel,
    entries: Vec<(u8, u8)>,
) -> Result<(), String> {
    log::info!(
        "Exporting UI registers to BIN: {} as {}",
        path,
        chip_model.display_name()
    );

    tokio::task::spawn_blocking(move || {
        let bin = build_export_bin(chip_model, &entries)?;
        std::fs::write(&path, &bin).map_err(|e| format!("Failed to write file: {}", e))
    })
    .await
    .map_err(|e| format!("Export task failed: {}", e))?
}

fn build_export_bin(chip_model: ChipModel, entries: &[(u8, u8)]) -> Result<Vec<u8>, String> {
    let spec = spec_for_model(chip_model);
    let addresses = chip::register_addresses(chip_model);
    let max_addr = addresses
        .iter()
        .copied()
        .filter(|addr| *addr != spec.control_reg)
        .max()
        .ok_or_else(|| format!("No exportable registers for {}", chip_model.display_name()))?
        as usize;

    let mut bin = vec![0u8; max_addr + 1];
    for (addr, value) in chip::default_register_map(chip_model) {
        if addr != spec.control_reg && (addr as usize) < bin.len() {
            bin[addr as usize] = value;
        }
    }

    for (addr, value) in entries {
        if *addr == spec.control_reg || !addresses.contains(addr) {
            continue;
        }
        bin[*addr as usize] = *value;
    }

    Ok(bin)
}

#[tauri::command]
pub async fn verify_all(
    state: State<'_, DeviceState>,
    path: String,
) -> Result<VerifyAllResult, String> {
    log::info!("Verify ALL (DAC + EEPROM) from: {}", path);

    super::with_device(&state, move |device| {
        let fw = FirmwareImage::from_file(&path, device.chip_model())?;
        let total = fw.register_count;
        let (dac_mismatches, eeprom_mismatches) = device.verify_all(fw.as_bytes())?;

        Ok(VerifyAllResult {
            total,
            dac_matched: total.saturating_sub(dac_mismatches.len()),
            eeprom_matched: total.saturating_sub(eeprom_mismatches.len()),
            dac_mismatches,
            eeprom_mismatches,
        })
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::build_export_bin;
    use crate::pmu::chip::ChipModel;

    #[test]
    fn export_bin_uses_ui_value_without_device() {
        let bin = build_export_bin(ChipModel::Ek86317a, &[(0x00, 0x12)]).unwrap();
        assert_eq!(bin[0x00], 0x12);
    }

    #[test]
    fn export_bin_keeps_defaults_for_missing_ui_values() {
        let bin = build_export_bin(ChipModel::Ek86317a, &[]).unwrap();
        assert_eq!(bin[0x0a], 0x3f);
    }
}

#[tauri::command]
pub async fn write_all_dac_registers(
    state: State<'_, DeviceState>,
    entries: Vec<(u8, u8)>,
) -> Result<WriteAllDacResult, String> {
    log::info!("Batch writing {} DAC registers", entries.len());

    super::with_device(&state, move |device| {
        if entries
            .iter()
            .any(|(addr, _)| *addr == device.spec().control_reg)
        {
            return Err(
                "Control register 0xFF is reserved for dedicated EEPROM commands".to_string(),
            );
        }

        let count = device.write_all_dac_registers(&entries)?;
        Ok(WriteAllDacResult {
            success: true,
            registers_written: count,
        })
    })
    .await
}
