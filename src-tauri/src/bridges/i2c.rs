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

    /// Whether a failed transfer really means "the slave did not ACK".
    ///
    /// FTDI MPSSE, the CH347 engine and the UART bridge turn a missing ACK into
    /// `Err`. The CH341 engine does **not**: measured against CH347DLLA64.dll on
    /// an unloaded bus (nothing attached, cross-checked with reserved address
    /// 0x7F), `CH341StreamI2C`, `CH341ReadI2C` and `CH341WriteI2C` all returned
    /// TRUE and handed back 0xFF from the bus pull-ups. There is therefore no
    /// NACK oracle at the DLL layer for CH341, and presence detection has to
    /// compare reads against the floating-bus level instead — see
    /// [`crate::pmu::detect::device_responds`].
    fn reports_nack(&self) -> bool {
        true
    }

    fn bus_recovery(&mut self) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::I2cBus;

    struct NackReportingBus;

    impl I2cBus for NackReportingBus {
        fn write(&mut self, _addr: u8, _data: &[u8]) -> Result<(), String> {
            Ok(())
        }

        fn read(&mut self, _addr: u8, _buf: &mut [u8]) -> Result<(), String> {
            Ok(())
        }

        fn write_read(
            &mut self,
            _addr: u8,
            _write_data: &[u8],
            _read_buf: &mut [u8],
        ) -> Result<(), String> {
            Ok(())
        }
    }

    /// CH341-like bus: transfers never fail, so the capability flag is what
    /// tells the detection layer to use the floating-bus comparison.
    struct SilentBus;

    impl I2cBus for SilentBus {
        fn write(&mut self, _addr: u8, _data: &[u8]) -> Result<(), String> {
            Ok(())
        }

        fn read(&mut self, _addr: u8, buf: &mut [u8]) -> Result<(), String> {
            for byte in buf.iter_mut() {
                *byte = 0xFF;
            }
            Ok(())
        }

        fn write_read(
            &mut self,
            _addr: u8,
            _write_data: &[u8],
            read_buf: &mut [u8],
        ) -> Result<(), String> {
            for byte in read_buf.iter_mut() {
                *byte = 0xFF;
            }
            Ok(())
        }

        fn reports_nack(&self) -> bool {
            false
        }
    }

    #[test]
    fn bridges_report_nack_by_default() {
        assert!(NackReportingBus.reports_nack());
    }

    #[test]
    fn a_silent_bridge_can_opt_out_of_nack_reporting() {
        assert!(!SilentBus.reports_nack());
    }
}
