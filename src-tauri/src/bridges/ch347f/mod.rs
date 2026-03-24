//! CH347F bridge backend.
//!
//! This module provides a CH347F-backed I2C bridge implementation for the
//! shared PMU transport layer.

pub mod i2c;

pub use i2c::Ch347I2cBus;
