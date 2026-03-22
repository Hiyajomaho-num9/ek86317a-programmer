//! FT232H bridge backend.
//!
//! This module provides the FT232H-specific bridge implementation plus a
//! development mock bridge that simulates PMU register behavior.

pub mod i2c;

pub use i2c::MockI2cBus;

#[cfg(feature = "ft232h")]
pub use i2c::Ft232hI2cBus;
