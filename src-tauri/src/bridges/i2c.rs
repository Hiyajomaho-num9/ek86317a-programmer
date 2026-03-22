//! Shared I2C bridge abstraction used by PMU drivers and concrete bridge backends.

/// Generic I2C bus interface. All addresses are 7-bit.
pub trait I2cBus: Send {
    fn write(&mut self, addr: u8, data: &[u8]) -> Result<(), String>;
    fn read(&mut self, addr: u8, buf: &mut [u8]) -> Result<(), String>;
    fn write_read(
        &mut self,
        addr: u8,
        write_data: &[u8],
        read_buf: &mut [u8],
    ) -> Result<(), String>;

    fn bus_recovery(&mut self) -> Result<(), String> {
        Ok(())
    }
}
