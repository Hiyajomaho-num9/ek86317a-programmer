//! FT232H driver module — I2C bus trait and implementations
//!
//! This module defines the `I2cBus` trait and provides:
//! - `MockI2cBus` — always available, simulates EK86317A registers
//! - `Ft232hI2cBus` — conditionally compiled with feature `ft232h`

pub mod i2c;

pub use i2c::{I2cBus, MockI2cBus};

#[cfg(feature = "ft232h")]
pub use i2c::Ft232hI2cBus;
