pub mod ch347f;
pub mod ftdi;
pub mod i2c;
pub mod uart;

pub use i2c::I2cBus;
pub use uart::{SharedUartBus, UartConfig, UartI2cBus, UartPortInfo, UartScanResult};
