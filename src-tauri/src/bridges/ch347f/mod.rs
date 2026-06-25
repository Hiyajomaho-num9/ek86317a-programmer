//! CH34x/CH347 bridge backend.
//!
//! This module provides a WCH-backed I2C bridge implementation for the
//! shared PMU transport layer.

pub mod i2c;

pub use i2c::Ch347I2cBus;
