//! Firmware programming commands

use std::path::Path;

use tauri::State;

use crate::pmu::chip::{self, spec_for_model, ChipModel};
use crate::pmu::device::ChipDevice;
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
        let mismatches = device.verify_firmware(fw.as_bytes())?;
        Ok(verify_result_from_mismatches(
            device,
            fw.as_bytes(),
            mismatches,
        ))
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

fn verify_result_from_mismatches(
    device: &ChipDevice,
    image: &[u8],
    mismatches: Vec<(u8, u8, u8)>,
) -> VerifyResult {
    let total = device.firmware_total();
    let missing = device.firmware_missing(image);
    let compared = total.saturating_sub(missing.len());
    let matched = compared.saturating_sub(mismatches.len());
    let partial = !missing.is_empty();
    VerifyResult {
        success: !partial && mismatches.is_empty(),
        total,
        compared,
        matched,
        missing,
        mismatches,
        partial,
    }
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
        let total = device.firmware_total();
        let missing = device.firmware_missing(fw.as_bytes());
        let compared = total.saturating_sub(missing.len());
        let partial = !missing.is_empty();
        let (dac_mismatches, eeprom_mismatches) = device.verify_all(fw.as_bytes())?;

        Ok(VerifyAllResult {
            success: !partial && dac_mismatches.is_empty() && eeprom_mismatches.is_empty(),
            total,
            compared,
            missing: missing.clone(),
            partial,
            dac_compared: compared,
            dac_matched: compared.saturating_sub(dac_mismatches.len()),
            dac_missing: missing.clone(),
            dac_mismatches,
            eeprom_compared: compared,
            eeprom_matched: compared.saturating_sub(eeprom_mismatches.len()),
            eeprom_missing: missing,
            eeprom_mismatches,
        })
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::{build_export_bin, verify_result_from_mismatches};
    use crate::bridges::ftdi::MockI2cBus;
    use crate::pmu::chip::{self, spec_for_model, ChipModel};
    use crate::pmu::device::ChipDevice;
    use crate::pmu::firmware::FirmwareImage;

    fn baseline_image(model: ChipModel, len: usize) -> Vec<u8> {
        let mut data = vec![0u8; len];
        for (addr, value) in chip::default_register_map(model) {
            if (addr as usize) < data.len() {
                data[addr as usize] = value;
            }
        }
        data
    }

    #[test]
    fn short_firmware_remains_loadable_for_partial_programming() {
        // The loader and verification both retain sparse-register coverage.
        let fw = FirmwareImage::from_bytes(vec![0u8; 0x10], ChipModel::Ek86317a).unwrap();
        assert_eq!(fw.size, 0x10);
        assert!(fw.get_all_registers().iter().all(|(addr, _)| *addr < 0x10));
    }

    #[test]
    fn short_default_image_has_partial_coverage_and_only_real_matches() {
        let model = ChipModel::Ek86317a;
        let mut device = ChipDevice::new(Box::new(MockI2cBus::new(model)), spec_for_model(model));
        let data = baseline_image(model, 0x10);
        let mismatches = device.verify_firmware(&data).unwrap();
        let result = verify_result_from_mismatches(&device, &data, mismatches);

        assert_eq!(result.compared, 16);
        assert_eq!(result.matched, 16);
        assert_eq!(result.total, result.compared + result.missing.len());
        assert!(result.partial);
        assert!(!result.success);
        assert!(result.mismatches.is_empty());
        assert!(result.missing.contains(&0x46));
        assert!(!result.missing.contains(&device.spec().control_reg));
    }

    #[test]
    fn short_image_mismatch_is_not_a_match_or_an_incomplete_file_error() {
        let model = ChipModel::Ek86317a;
        let mut device = ChipDevice::new(Box::new(MockI2cBus::new(model)), spec_for_model(model));
        let mut data = baseline_image(model, 0x10);
        let actual = data[0];
        let expected = actual ^ 0xFF;
        data[0] = expected;

        let mismatches = device.verify_firmware(&data).unwrap();
        let result = verify_result_from_mismatches(&device, &data, mismatches);
        assert_eq!(result.compared, 16);
        assert_eq!(result.matched, 15);
        assert_eq!(result.mismatches, vec![(0, expected, actual)]);
        assert_eq!(result.total, result.compared + result.missing.len());
        assert!(result.partial);
        assert!(!result.success);
    }

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
