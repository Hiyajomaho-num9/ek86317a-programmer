//! BIN firmware file parsing for EK86317A
//!
//! BIN files are raw data streams, sequentially mapped from address 0x00.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::registers::*;

/// Parsed firmware image
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareImage {
    /// Raw firmware data bytes
    pub data: Vec<u8>,
    /// Total size in bytes
    pub size: usize,
    /// Number of registers covered
    pub register_count: usize,
}

impl FirmwareImage {
    /// Load firmware from a file path.
    pub fn from_file(path: &str) -> Result<Self, String> {
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

        let data =
            std::fs::read(path).map_err(|e| format!("Failed to read firmware file: {}", e))?;
        Self::from_bytes(data)
    }

    /// Parse firmware from raw bytes.
    pub fn from_bytes(data: Vec<u8>) -> Result<Self, String> {
        if data.is_empty() {
            return Err("Empty firmware file".to_string());
        }

        let size = data.len();

        // Count how many valid PMIC register addresses are covered
        let register_count = PMIC_REG_ADDRESSES
            .iter()
            .filter(|&&addr| (addr as usize) < size && addr != REG_CONTROL)
            .count();

        Ok(Self {
            data,
            size,
            register_count,
        })
    }

    /// Get the value for a specific register address.
    /// Returns None if the address is beyond the firmware data.
    pub fn get_register_value(&self, addr: u8) -> Option<u8> {
        let idx = addr as usize;
        if idx < self.data.len() {
            Some(self.data[idx])
        } else {
            None
        }
    }

    /// Get all registers as (address, value) pairs.
    /// Only returns addresses that are valid PMIC registers and within data bounds.
    pub fn get_all_registers(&self) -> Vec<(u8, u8)> {
        PMIC_REG_ADDRESSES
            .iter()
            .filter_map(|&addr| {
                if addr == REG_CONTROL {
                    return None;
                }
                self.get_register_value(addr).map(|val| (addr, val))
            })
            .collect()
    }

    /// Raw data slice.
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_firmware_from_bytes() {
        let data = vec![0u8; 0x50]; // enough to cover all registers
        let fw = FirmwareImage::from_bytes(data).unwrap();
        assert_eq!(fw.size, 0x50);
        assert!(fw.register_count > 0);
    }

    #[test]
    fn test_firmware_empty() {
        let result = FirmwareImage::from_bytes(vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_register_value() {
        let mut data = vec![0u8; 0x50];
        data[0x00] = 0x15; // AVDD
        data[0x01] = 0x20; // VBK1
        let fw = FirmwareImage::from_bytes(data).unwrap();
        assert_eq!(fw.get_register_value(0x00), Some(0x15));
        assert_eq!(fw.get_register_value(0x01), Some(0x20));
        assert_eq!(fw.get_register_value(0xFF), None); // out of range
    }
}
