//! Tauri command handlers and shared types

pub mod device;
pub mod eeprom;
pub mod firmware;
pub mod register;

use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::ek86317a::Ek86317a;
use crate::error::AppError;

/// Global device state managed by Tauri
pub struct DeviceState {
    pub device: Arc<Mutex<Option<Ek86317a>>>,
}

impl DeviceState {
    pub fn new() -> Self {
        Self {
            device: Arc::new(Mutex::new(None)),
        }
    }
}

/// Run a device operation on a blocking worker thread so FTDI/I2C access and
/// datasheet-mandated sleeps do not occupy the async runtime executor.
pub async fn with_device<T, F>(state: &State<'_, DeviceState>, op: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce(&mut Ek86317a) -> Result<T, String> + Send + 'static,
{
    let device_handle = Arc::clone(&state.device);

    tokio::task::spawn_blocking(move || {
        let mut guard = device_handle
            .lock()
            .map_err(|_| AppError::LockError.to_string())?;
        let device = guard
            .as_mut()
            .ok_or_else(|| AppError::DeviceNotConnected.to_string())?;

        op(device)
    })
    .await
    .map_err(|e| format!("Device task failed: {}", e))?
}

/// Device information returned after connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub pmic_detected: bool,
    pub vcom_detected: bool,
    pub device_id: String,
}

/// Register data for frontend display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterData {
    pub address: u8,
    pub value: u8,
    pub name: String,
    pub voltage: Option<f64>,
}

/// Firmware preview information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwarePreview {
    pub file_name: String,
    pub size: usize,
    pub register_count: usize,
    pub registers: Vec<RegisterData>,
}

/// Result of firmware programming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramResult {
    pub success: bool,
    pub registers_written: usize,
    pub eeprom_written: bool,
}

/// Result of firmware verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyResult {
    pub success: bool,
    pub total: usize,
    pub matched: usize,
    pub mismatches: Vec<(u8, u8, u8)>, // (addr, expected, actual)
}

/// Result of verify-all (DAC + EEPROM)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyAllResult {
    pub total: usize,
    pub dac_matched: usize,
    pub eeprom_matched: usize,
    pub dac_mismatches: Vec<(u8, u8, u8)>,
    pub eeprom_mismatches: Vec<(u8, u8, u8)>,
}

/// Result of batch DAC write
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteAllDacResult {
    pub success: bool,
    pub registers_written: usize,
}
