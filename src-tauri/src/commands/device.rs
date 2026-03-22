//! Device connection/disconnection commands

use std::sync::Arc;

use tauri::State;

use crate::ek86317a::Ek86317a;
use crate::error::AppError;
#[cfg(debug_assertions)]
use crate::ft232h::MockI2cBus;

use super::{DeviceInfo, DeviceState};

/// Scan for available FT232H devices.
/// Returns a list of device identifiers.
/// When compiled with `ft232h` feature, enumerates real FTDI devices.
/// Includes a mock device only for debug builds.
#[tauri::command]
pub async fn scan_devices() -> Result<Vec<String>, String> {
    log::info!("Scanning for FT232H devices...");

    tokio::task::spawn_blocking(|| {
        let mut devices = Vec::new();

        #[cfg(feature = "ft232h")]
        {
            match crate::ft232h::Ft232hI2cBus::list_devices() {
                Ok(ftdi_devices) => {
                    for (idx, desc) in &ftdi_devices {
                        let id = format!("FT232H:{}:{}", idx, desc);
                        log::info!("Found FTDI device: {}", id);
                        devices.push(id);
                    }
                }
                Err(e) => {
                    log::warn!("Failed to enumerate FTDI devices: {}", e);
                }
            }
        }

        #[cfg(debug_assertions)]
        devices.push("Mock FT232H (development)".to_string());

        Ok(devices)
    })
    .await
    .map_err(|e| format!("Device scan task failed: {}", e))?
}

/// Connect to a device and probe for PMIC/VCOM slaves.
#[tauri::command]
pub async fn connect_device(
    state: State<'_, DeviceState>,
    device_id: String,
    clock_hz: u32,
) -> Result<DeviceInfo, String> {
    log::info!("Connecting to device: {} ", device_id);

    let device_handle = Arc::clone(&state.device);

    tokio::task::spawn_blocking(move || {
        let bus: Box<dyn crate::ft232h::I2cBus> = if device_id.starts_with("FT232H:") {
            #[cfg(feature = "ft232h")]
            {
                let parts: Vec<&str> = device_id.splitn(3, ':').collect();
                let index: u32 = parts
                    .get(1)
                    .and_then(|s| s.parse().ok())
                    .ok_or_else(|| format!("Invalid device ID: {}", device_id))?;
                log::info!(
                    "Opening real FT232H device index={}, clock={}Hz",
                    index,
                    clock_hz
                );
                let ft_bus = crate::ft232h::Ft232hI2cBus::open(index, clock_hz)
                    .map_err(|e| format!("Failed to open FT232H: {}", e))?;
                Box::new(ft_bus)
            }
            #[cfg(not(feature = "ft232h"))]
            {
                return Err(
                    "FT232H support not compiled in. Rebuild with --features ft232h".to_string(),
                );
            }
        } else if device_id == "Mock FT232H (development)" {
            #[cfg(debug_assertions)]
            {
                log::info!("Using MockI2cBus for development");
                Box::new(MockI2cBus::new())
            }
            #[cfg(not(debug_assertions))]
            {
                return Err("Mock device is only available in debug builds".to_string());
            }
        } else {
            return Err(format!("Unsupported device ID: {}", device_id));
        };

        let mut device = Ek86317a::new(bus);
        let (pmic_detected, vcom_detected) =
            device.probe().map_err(|e| format!("Probe failed: {}", e))?;

        let info = DeviceInfo {
            pmic_detected,
            vcom_detected,
            device_id: device_id.clone(),
        };

        let mut guard = device_handle
            .lock()
            .map_err(|_| AppError::LockError.to_string())?;
        *guard = Some(device);

        log::info!("Connected: PMIC={}, VCOM={}", pmic_detected, vcom_detected);

        Ok(info)
    })
    .await
    .map_err(|e| format!("Device connect task failed: {}", e))?
}

/// Disconnect from the current device.
#[tauri::command]
pub async fn disconnect_device(state: State<'_, DeviceState>) -> Result<(), String> {
    log::info!("Disconnecting from device...");

    let device_handle = Arc::clone(&state.device);

    tokio::task::spawn_blocking(move || {
        let mut guard = device_handle
            .lock()
            .map_err(|_| AppError::LockError.to_string())?;
        *guard = None;
        Ok::<(), String>(())
    })
    .await
    .map_err(|e| format!("Device disconnect task failed: {}", e))??;

    log::info!("Disconnected");
    Ok(())
}

/// Detect IC status by re-probing PMIC and VCOM slaves.
#[tauri::command]
pub async fn detect_ic(state: State<'_, DeviceState>) -> Result<DeviceInfo, String> {
    log::info!("Detecting IC status...");

    super::with_device(&state, move |device| {
        let (pmic_detected, vcom_detected) =
            device.probe().map_err(|e| format!("Probe failed: {}", e))?;

        log::info!("Detect: PMIC={}, VCOM={}", pmic_detected, vcom_detected);

        Ok(DeviceInfo {
            pmic_detected,
            vcom_detected,
            device_id: String::new(),
        })
    })
    .await
}
