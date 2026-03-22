//! Error types for EK86317A Programmer

use serde::Serialize;

/// Application-level error type
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Device not connected")]
    DeviceNotConnected,

    #[error("Device connection failed: {0}")]
    ConnectionFailed(String),

    #[error("I2C communication error: {0}")]
    I2cError(String),

    #[error("Invalid register address: 0x{0:02X}")]
    InvalidRegister(u8),

    #[error("Firmware error: {0}")]
    FirmwareError(String),

    #[error("EEPROM error: {0}")]
    EepromError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Encode error: {0}")]
    EncodeError(String),

    #[error("Lock error: device state is poisoned")]
    LockError,
}

// Implement Serialize for AppError so it can be returned from Tauri commands
impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
