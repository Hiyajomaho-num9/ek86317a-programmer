//! 串口（UART 控制台）I2C 桥后端。
//!
//! 让整个工具链可以绕过 FTDI / CH341A / CH347F 这些专用桥接器，直接借面板
//! 主芯片自己的串口控制台去读写 I2C 总线上的 PMU / VCOM。

pub mod i2c;

pub use i2c::{
    SharedUartBus, UartConfig, UartI2cBus, UartPortInfo, UartScanResult, DIALECT_I2CTOOLS,
    DIALECT_TOYBOX, SU_SESSION, SU_WRAP,
};
