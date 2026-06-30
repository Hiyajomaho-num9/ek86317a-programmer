//! Device connection and bridge selection commands.

use std::sync::Arc;

use tauri::State;

use crate::bridges::ch347f::Ch347I2cBus;
#[cfg(feature = "ftdi")]
use crate::bridges::ftdi::FtdiI2cBus;
#[cfg(debug_assertions)]
use crate::bridges::ftdi::MockI2cBus;
use crate::bridges::I2cBus;
use crate::error::AppError;
use crate::pmu::chip::{spec_for_model, ChipModel};
use crate::pmu::device::ChipDevice;

use super::{DeviceInfo, DeviceState};

const FTDI_BRIDGE_PREFIX: &str = "bridge:ftdi:";
const CH347F_BRIDGE_PREFIX: &str = "bridge:ch347f:";
const MOCK_BRIDGE_ID: &str = "bridge:mock:development";

/// Parse the numeric bridge index from a device ID string.
///
/// Only called from the FTDI bridge path (which needs a numeric index) and
/// from unit tests. Under a ch347f-only build the function has no non-test
/// callers, so we silence the dead-code warning for that configuration
/// rather than gating the function behind a feature flag (which would force
/// duplicating the test module).
#[cfg_attr(not(any(test, feature = "ftdi")), allow(dead_code))]
fn parse_bridge_index(prefix: &str, device_id: &str) -> Result<u32, String> {
    parse_bridge_selector(prefix, device_id)?
        .parse()
        .map_err(|_| format!("Invalid device ID: {}", device_id))
}

fn parse_bridge_selector(prefix: &str, device_id: &str) -> Result<String, String> {
    let parts: Vec<&str> = device_id.splitn(4, ':').collect();
    if !device_id.starts_with(prefix) {
        return Err(format!(
            "Device ID {} does not match prefix {}",
            device_id, prefix
        ));
    }

    parts
        .get(2)
        .filter(|s| !s.is_empty())
        .map(|s| (*s).to_string())
        .ok_or_else(|| format!("Invalid device ID: {}", device_id))
}

#[tauri::command]
pub async fn scan_devices() -> Result<Vec<String>, String> {
    log::info!("Scanning available bridge devices...");

    tokio::task::spawn_blocking(|| {
        let mut devices = Vec::new();

        #[cfg(feature = "ftdi")]
        {
            match FtdiI2cBus::list_devices() {
                Ok(ftdi_devices) => {
                    for (idx, desc) in &ftdi_devices {
                        let id = format!("{}{}:{}", FTDI_BRIDGE_PREFIX, idx, desc);
                        log::info!("Found FTDI bridge: {}", id);
                        devices.push(id);
                    }
                }
                Err(e) => {
                    log::warn!("Failed to enumerate FTDI bridges: {}", e);
                }
            }
        }

        #[cfg(feature = "ch347f")]
        {
            match Ch347I2cBus::list_devices() {
                Ok(ch347_devices) => {
                    for (selector, desc) in &ch347_devices {
                        let id = format!("{}{}:{}", CH347F_BRIDGE_PREFIX, selector, desc);
                        log::info!("Found CH34x/CH347 bridge: {}", id);
                        devices.push(id);
                    }
                }
                Err(e) => {
                    log::warn!("Failed to enumerate CH34x/CH347 bridges: {}", e);
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
    log::info!(
        "Connecting to device bridge: {} as {}",
        device_id,
        chip_model.display_name()
    );

    let device_handle = Arc::clone(&state.device);

    tokio::task::spawn_blocking(move || {
        let bus: Box<dyn I2cBus> = if device_id.starts_with(FTDI_BRIDGE_PREFIX) {
            #[cfg(feature = "ftdi")]
            {
                let index = parse_bridge_index(FTDI_BRIDGE_PREFIX, &device_id)?;
                log::info!("Opening FTDI bridge index={}, clock={}Hz", index, clock_hz);
                let ft_bus = FtdiI2cBus::open(index, clock_hz)
                    .map_err(|e| format!("Failed to open FTDI bridge: {}", e))?;
                Box::new(ft_bus)
            }
            #[cfg(not(feature = "ftdi"))]
            {
                return Err(
                    "FTDI bridge support not compiled in. Rebuild with --features ftdi".to_string(),
                );
            }
        } else if device_id.starts_with(CH347F_BRIDGE_PREFIX) {
            #[cfg(feature = "ch347f")]
            {
                let selector = parse_bridge_selector(CH347F_BRIDGE_PREFIX, &device_id)?;
                log::info!(
                    "Opening CH34x/CH347 bridge selector={}, clock={}Hz",
                    selector,
                    clock_hz
                );
                let ch347_bus = Ch347I2cBus::open(&selector, clock_hz)
                    .map_err(|e| format!("Failed to open CH34x/CH347 bridge: {}", e))?;
                Box::new(ch347_bus)
            }
            #[cfg(not(feature = "ch347f"))]
            {
                return Err(
                    "CH347F bridge support not compiled in. Rebuild with --features ch347f"
                        .to_string(),
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

#[cfg(test)]
mod tests {
    use super::{
        parse_bridge_index, parse_bridge_selector, CH347F_BRIDGE_PREFIX, FTDI_BRIDGE_PREFIX,
    };

    #[test]
    fn parses_ftdi_bridge_index() {
        assert_eq!(
            parse_bridge_index(FTDI_BRIDGE_PREFIX, "bridge:ftdi:3:demo").unwrap(),
            3
        );
    }

    #[test]
    fn parses_ch347_bridge_index() {
        assert_eq!(
            parse_bridge_index(CH347F_BRIDGE_PREFIX, "bridge:ch347f:5:bridge").unwrap(),
            5
        );
    }

    #[test]
    fn parses_ch347_bridge_selector_path() {
        assert_eq!(
            parse_bridge_selector(CH347F_BRIDGE_PREFIX, "bridge:ch347f:/dev/hidraw0:bridge")
                .unwrap(),
            "/dev/hidraw0"
        );
    }
}