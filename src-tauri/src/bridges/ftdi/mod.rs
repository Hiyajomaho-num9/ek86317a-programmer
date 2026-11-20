//! FTDI bridge backend.
//!
//! This module provides the FTDI-specific bridge implementation plus a
//! development mock bridge that simulates PMU register behavior.

#[cfg(feature = "ftdi")]
pub mod i2c;
pub mod mock;

#[cfg(debug_assertions)]
pub use mock::MockI2cBus;

#[cfg(feature = "ftdi")]
pub use i2c::FtdiI2cBus;
