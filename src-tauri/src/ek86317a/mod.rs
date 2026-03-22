//! EK86317A TCON driver module
//!
//! Provides the `Ek86317a` device abstraction with I2C protocol operations.

pub mod firmware;
pub mod protocol;
pub mod registers;

pub use protocol::Ek86317a;
pub use registers::FaultFlags;
