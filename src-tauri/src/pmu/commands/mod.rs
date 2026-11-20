//! Tauri command handlers and shared types

pub mod device;
pub mod eeprom;
pub mod firmware;
pub mod register;
pub mod uart;

use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::error::AppError;
use crate::pmu::chip::ChipModel;
use crate::pmu::device::ChipDevice;

/// The UART state and device state share this record, so replacing a UART
/// configuration can invalidate the PMU session that uses the previous bus.
/// Other bridge kinds have no UART key and are not affected by UART closure.
pub struct DeviceConnection {
    pub device: ChipDevice,
    pub uart_config_key: Option<String>,
}

pub type SharedDevice = Arc<Mutex<Option<DeviceConnection>>>;
pub type Mismatches = Vec<(u8, u8, u8)>;
pub type BankMismatches = (Mismatches, Mismatches);

/// Global device state managed by Tauri.
pub struct DeviceState {
    pub device: SharedDevice,
}

impl DeviceState {
    pub fn new() -> Self {
        Self {
            device: Arc::new(Mutex::new(None)),
        }
    }

    pub fn shared_device(&self) -> SharedDevice {
        Arc::clone(&self.device)
    }

    /// Remove the connection on disconnect; return whether the UART bus shared
    /// with the raw console should be closed too.
    pub fn disconnect(shared: &SharedDevice) -> Result<bool, String> {
        let mut guard = shared.lock().map_err(|_| AppError::LockError.to_string())?;
        Ok(guard
            .take()
            .is_some_and(|connection| connection.uart_config_key.is_some()))
    }

    pub fn close_uart_session(shared: &SharedDevice) -> Result<(), String> {
        let mut guard = shared.lock().map_err(|_| AppError::LockError.to_string())?;
        if guard
            .as_ref()
            .is_some_and(|connection| connection.uart_config_key.is_some())
        {
            *guard = None;
        }
        Ok(())
    }

    pub fn close_uart_session_if_config_changed(
        shared: &SharedDevice,
        config_key: &str,
    ) -> Result<(), String> {
        let mut guard = shared.lock().map_err(|_| AppError::LockError.to_string())?;
        if guard.as_ref().is_some_and(|connection| {
            connection
                .uart_config_key
                .as_deref()
                .is_some_and(|active_key| active_key != config_key)
        }) {
            *guard = None;
        }
        Ok(())
    }
}

impl Default for DeviceState {
    fn default() -> Self {
        Self::new()
    }
}

/// Run a device operation on a blocking worker thread so FTDI/I2C access and
/// datasheet-mandated sleeps do not occupy the async runtime executor.
pub async fn with_device<T, F>(state: &State<'_, DeviceState>, op: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce(&mut ChipDevice) -> Result<T, String> + Send + 'static,
{
    let device_handle = Arc::clone(&state.device);

    tokio::task::spawn_blocking(move || {
        let mut guard = device_handle
            .lock()
            .map_err(|_| AppError::LockError.to_string())?;
        let connection = guard
            .as_mut()
            .ok_or_else(|| AppError::DeviceNotConnected.to_string())?;

        op(&mut connection.device)
    })
    .await
    .map_err(|e| format!("Device task failed: {}", e))?
}

/// Device information returned after connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub pmic_detected: bool,
    pub vcom_detected: Option<bool>,
    pub device_id: String,
    pub chip_model: ChipModel,
}

/// Register data for frontend display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterData {
    pub address: u8,
    pub value: u8,
    pub name: String,
    pub voltage: Option<f64>,
}

/// Firmware preview information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwarePreview {
    pub file_name: String,
    pub size: usize,
    pub register_count: usize,
    pub registers: Vec<RegisterData>,
}

/// Result of firmware programming.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramResult {
    pub success: bool,
    pub registers_written: usize,
    pub eeprom_written: bool,
}

/// Result of firmware verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyResult {
    pub success: bool,
    /// Number of verifiable registers for this chip, including absent image bytes.
    pub total: usize,
    pub compared: usize,
    pub matched: usize,
    pub missing: Vec<u8>,
    pub mismatches: Vec<(u8, u8, u8)>,
    pub partial: bool,
}

/// Result of verify-all (DAC + EEPROM). `total` is the address count per bank;
/// `compared + missing == total` independently for each bank.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyAllResult {
    pub success: bool,
    pub total: usize,
    pub compared: usize,
    pub missing: Vec<u8>,
    pub partial: bool,
    pub dac_compared: usize,
    pub dac_matched: usize,
    pub dac_missing: Vec<u8>,
    pub dac_mismatches: Vec<(u8, u8, u8)>,
    pub eeprom_compared: usize,
    pub eeprom_matched: usize,
    pub eeprom_missing: Vec<u8>,
    pub eeprom_mismatches: Vec<(u8, u8, u8)>,
}

/// Result of batch DAC write.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteAllDacResult {
    pub success: bool,
    pub registers_written: usize,
}

#[cfg(test)]
mod tests {
    use super::{DeviceConnection, DeviceState};
    use crate::bridges::ftdi::MockI2cBus;
    use crate::pmu::chip::{spec_for_model, ChipModel};
    use crate::pmu::device::ChipDevice;

    fn connection(uart_config_key: Option<&str>) -> DeviceConnection {
        DeviceConnection {
            device: ChipDevice::new(
                Box::new(MockI2cBus::new(ChipModel::Ek86317a)),
                spec_for_model(ChipModel::Ek86317a),
            ),
            uart_config_key: uart_config_key.map(str::to_string),
        }
    }

    fn has_uart_session(state: &DeviceState) -> bool {
        state
            .device
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|connection| connection.uart_config_key.is_some())
    }

    #[test]
    fn uart_config_change_invalidates_only_its_old_pmu_session() {
        let state = DeviceState::new();
        *state.device.lock().unwrap() = Some(connection(Some("old")));

        DeviceState::close_uart_session_if_config_changed(&state.device, "old").unwrap();
        assert!(has_uart_session(&state));

        DeviceState::close_uart_session_if_config_changed(&state.device, "new").unwrap();
        assert!(state.device.lock().unwrap().is_none());
    }

    #[test]
    fn uart_close_preserves_non_uart_device_connection() {
        let state = DeviceState::new();
        *state.device.lock().unwrap() = Some(connection(None));
        DeviceState::close_uart_session(&state.device).unwrap();
        assert!(state.device.lock().unwrap().is_some());

        DeviceState::close_uart_session_if_config_changed(&state.device, "new").unwrap();
        assert!(state.device.lock().unwrap().is_some());
    }

    #[test]
    fn disconnect_reports_whether_shared_uart_bus_must_close() {
        let uart = DeviceState::new();
        *uart.device.lock().unwrap() = Some(connection(Some("config")));
        assert!(DeviceState::disconnect(&uart.device).unwrap());
        assert!(uart.device.lock().unwrap().is_none());

        let usb = DeviceState::new();
        *usb.device.lock().unwrap() = Some(connection(None));
        assert!(!DeviceState::disconnect(&usb.device).unwrap());
        assert!(usb.device.lock().unwrap().is_none());
    }
}
