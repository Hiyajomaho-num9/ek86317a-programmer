use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::pmu::chip::{self, ChipModel};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareImage {
    pub data: Vec<u8>,
    pub size: usize,
    pub register_count: usize,
    registers: Vec<(u8, u8)>,
}

impl FirmwareImage {
    pub fn from_file(path: &str, chip_model: ChipModel) -> Result<Self, String> {
        if Path::new(path)
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("hex"))
        {
            return Err(
                "Intel HEX files are not supported. Please convert the firmware to a raw .bin file first."
                    .to_string(),
            );
        }

        let data = std::fs::read(path)
            .map_err(|e| format!("Failed to read firmware file: {}", e))?;
        Self::from_bytes(data, chip_model)
    }

    pub fn from_bytes(data: Vec<u8>, chip_model: ChipModel) -> Result<Self, String> {
        if data.is_empty() {
            return Err("Empty firmware file".to_string());
        }

        let size = data.len();
        let control_reg = chip::spec_for_model(chip_model).control_reg;
        let registers = chip::register_addresses(chip_model)
            .iter()
            .filter_map(|&addr| {
                if addr == control_reg || addr as usize >= size {
                    return None;
                }
                Some((addr, data[addr as usize]))
            })
            .collect::<Vec<_>>();

        Ok(Self {
            data,
            size,
            register_count: registers.len(),
            registers,
        })
    }

    pub fn get_all_registers(&self) -> &[(u8, u8)] {
        &self.registers
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn firmware_filters_by_chip_registers() {
        let data = vec![0u8; 0x50];
        let fw = FirmwareImage::from_bytes(data, ChipModel::Ek86317a).unwrap();
        assert!(fw.register_count > 0);
    }

    #[test]
    fn firmware_rejects_empty() {
        assert!(FirmwareImage::from_bytes(vec![], ChipModel::Lp6281).is_err());
    }
}
