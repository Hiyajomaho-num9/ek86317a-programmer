//! Device connection/disconnection commands

use std::sync::Arc;

use tauri::State;

use crate::bridges::I2cBus;
#[cfg(debug_assertions)]
use crate::bridges::ft232h::MockI2cBus;
use crate::error::AppError;
use crate::pmu::chip::{spec_for_model, ChipModel};
use crate::pmu::device::ChipDevice;

use super::{DeviceInfo, DeviceState};

const FT232H_BRIDGE_PREFIX: &str = "bridge:ft232h:";
const MOCK_BRIDGE_ID: &str = "bridge:mock:development";

#[tauri::command]
pub async fn scan_devices() -> Result<Vec<String>, String> {
    log::info!("Scanning available bridge devices...");

    tokio::task::spawn_blocking(|| {
        let mut devices = Vec::new();

        #[cfg(feature = "ft232h")]
        {
            match crate::bridges::ft232h::Ft232hI2cBus::list_devices() {
                Ok(ftdi_devices) => {
                    for (idx, desc) in &ftdi_devices {
                        let id = format!("{}{}:{}", FT232H_BRIDGE_PREFIX, idx, desc);
                        log::info!("Found FT232H bridge: {}", id);
                        devices.push(id);
                    }
                }
                Err(e) => {
                    log::warn!("Failed to enumerate FT232H bridges: {}", e);
                }
            }
        }

        #[cfg(debug_assertions)]
        devices.push(MOCK_BRIDGE_ID.to_string());

        Ok(devices)
    })
    .await
    .map_err(|e| format!("Device scan task failed: {}", e))?
}

#[tauri::command]
pub async fn connect_device(
    state: State<'_, DeviceState>,
    device_id: String,
    clock_hz: u32,
    chip_model: ChipModel,
) -> Result<DeviceInfo, String> {
    log::info!("Connecting to device bridge: {} as {}", device_id, chip_model.display_name());

    let device_handle = Arc::clone(&state.device);

    tokio::task::spawn_blocking(move || {
        let bus: Box<dyn I2cBus> = if device_id.starts_with(FT232H_BRIDGE_PREFIX) {
            #[cfg(feature = "ft232h")]
            {
                let parts: Vec<&str> = device_id.splitn(4, ':').collect();
                let index: u32 = parts
                    .get(2)
                    .and_then(|s| s.parse().ok())
                    .ok_or_else(|| format!("Invalid device ID: {}", device_id))?;
                log::info!(
                    "Opening FT232H bridge index={}, clock={}Hz",
                    index,
                    clock_hz
                );
                let ft_bus = crate::bridges::ft232h::Ft232hI2cBus::open(index, clock_hz)
                    .map_err(|e| format!("Failed to open FT232H bridge: {}", e))?;
                Box::new(ft_bus)
            }
            #[cfg(not(feature = "ft232h"))]
            {
                return Err(
                    "FT232H bridge support not compiled in. Rebuild with --features ft232h".to_string(),
                );
            }
        } else if device_id == MOCK_BRIDGE_ID {
            #[cfg(debug_assertions)]
            {
                log::info!("Using mock bridge for development");
                Box::new(MockI2cBus::new(chip_model))
            }
            #[cfg(not(debug_assertions))]
            {
                return Err("Mock bridge is only available in debug builds".to_string());
            }
        } else {
            return Err(format!("Unsupported device ID: {}", device_id));
        };

        let spec = spec_for_model(chip_model);
        let mut device = ChipDevice::new(bus, spec);
        let (pmic_detected, vcom_detected) =
            device.probe().map_err(|e| format!("Probe failed: {}", e))?;

        let info = DeviceInfo {
            pmic_detected,
            vcom_detected,
            device_id: device_id.clone(),
            chip_model,
        };

        let mut guard = device_handle
            .lock()
            .map_err(|_| AppError::LockError.to_string())?;
        *guard = Some(device);

        log::info!(
            "Connected: chip={}, PMIC={}, VCOM={:?}",
            chip_model.display_name(),
            pmic_detected,
            vcom_detected
        );

        Ok(info)
    })
    .await
    .map_err(|e| format!("Device connect task failed: {}", e))?
}

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

#[tauri::command]
pub async fn detect_ic(state: State<'_, DeviceState>) -> Result<DeviceInfo, String> {
    log::info!("Detecting IC status...");

    super::with_device(&state, move |device| {
        let (pmic_detected, vcom_detected) =
            device.probe().map_err(|e| format!("Probe failed: {}", e))?;
        let chip_model = device.chip_model();

        log::info!(
            "Detect: chip={}, PMIC={}, VCOM={:?}",
            chip_model.display_name(),
            pmic_detected,
            vcom_detected
        );

        Ok(DeviceInfo {
            pmic_detected,
            vcom_detected,
            device_id: String::new(),
            chip_model,
        })
    })
    .await
}
