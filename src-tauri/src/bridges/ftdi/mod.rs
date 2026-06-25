//! FTDI bridge backend.
//!
//! This module provides the FTDI-specific bridge implementation plus a
//! development mock bridge that simulates PMU register behavior.

pub mod i2c;

pub use i2c::MockI2cBus;

#[cfg(feature = "ftdi")]
pub use i2c::FtdiI2cBus;
