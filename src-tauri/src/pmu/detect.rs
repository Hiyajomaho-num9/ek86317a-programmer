//! Bus-level presence detection.
//!
//! Why this exists
//! ---------------
//! "The transfer call returned `Ok`" is not proof that a slave is on the bus.
//! FTDI MPSSE, the CH347 engine and the UART bridge all turn a missing ACK into
//! an error, but the CH341 engine does not. Measured against CH347DLLA64.dll on
//! a bus with nothing attached: `CH341StreamI2C`, `CH341ReadI2C` and
//! `CH341WriteI2C` all returned TRUE, and reads came back as `0xFF` from the bus
//! pull-ups — including for reserved address 0x7F, which cannot hold a slave.
//!
//! So for a bus that does not report NACK the only usable evidence is the data
//! itself: read the level of a register that no slave can answer (the floating
//! baseline), then read the real registers. Anything that differs from the
//! baseline came from an actual device driving the bus.

use crate::bridges::I2cBus;

/// I2C reserved address: no device may answer here.
pub const FLOATING_BASELINE_ADDR: u8 = 0x7F;

/// Fallback baseline probe when the reserved address reads back something that
/// is not a plausible idle level. 0x5A is outside every address this tool
/// supports and is not a common PMU/VCOM/EEPROM address.
const SECONDARY_BASELINE_ADDR: u8 = 0x5A;

/// Levels a bus can hold when nobody drives it: pulled up (0xFF) or pulled down
/// / floating low (0x00).
fn is_idle_bus_level(value: u8) -> bool {
    value == 0xFF || value == 0x00
}

/// Read the floating-bus level using `reg` at an address that holds no device.
fn floating_baseline(bus: &mut dyn I2cBus, reg: u8) -> Option<u8> {
    for probe_addr in [FLOATING_BASELINE_ADDR, SECONDARY_BASELINE_ADDR] {
        let mut buf = [0u8; 1];
        if bus.write_read(probe_addr, &[reg], &mut buf).is_err() {
            continue;
        }
        if is_idle_bus_level(buf[0]) {
            log::debug!(
                "floating baseline: 0x{:02X} on reserved address 0x{:02X} (reg 0x{:02X})",
                buf[0],
                probe_addr,
                reg
            );
            return Some(buf[0]);
        }
        log::debug!(
            "reserved address 0x{:02X} answered 0x{:02X}; not an idle bus level",
            probe_addr,
            buf[0]
        );
    }
    None
}

/// Read one register, or `None` when the transfer fails.
fn read_register(bus: &mut dyn I2cBus, addr: u8, reg: u8) -> Option<u8> {
    let mut buf = [0u8; 1];
    bus.write_read(addr, &[reg], &mut buf).ok().map(|()| buf[0])
}

/// Returns `true` when a slave at `addr` really answers on the bus.
///
/// `probe_registers` are registers that exist on the target device. With a bus
/// that reports NACK, the first successful read decides. Without NACK reporting
/// (CH341), a read only counts as evidence when
///
/// * its value differs from the floating-bus level measured at a reserved
///   address, and
/// * the same register returns that value twice in a row — a real slave answers
///   deterministically, while crosstalk on a dangling harness usually does not.
///
/// Every probe register reading back as the idle level means nothing is
/// attached.
pub fn device_responds(bus: &mut dyn I2cBus, addr: u8, probe_registers: &[u8]) -> bool {
    let Some(&first_reg) = probe_registers.first() else {
        return false;
    };

    if bus.reports_nack() {
        let mut buf = [0u8; 1];
        let ok = bus.write_read(addr, &[first_reg], &mut buf).is_ok();
        log::debug!(
            "presence at 0x{:02X} (NACK reporting bus): read reg 0x{:02X} -> {}",
            addr,
            first_reg,
            if ok {
                format!("Ok(0x{:02X})", buf[0])
            } else {
                "Err".to_string()
            }
        );
        return ok;
    }

    let Some(baseline) = floating_baseline(bus, first_reg) else {
        // Without a baseline there is nothing to compare against, and treating
        // "no comparison possible" as "device present" is exactly the phantom
        // detection this function exists to prevent.
        log::warn!(
            "presence check for 0x{:02X} aborted: no floating baseline on this bus",
            addr
        );
        return false;
    };

    for &reg in probe_registers {
        let Some(first) = read_register(bus, addr, reg) else {
            continue;
        };
        if first == baseline {
            continue;
        }

        match read_register(bus, addr, reg) {
            Some(second) if second == first => {
                log::info!(
                    "presence at 0x{:02X} confirmed: reg 0x{:02X} = 0x{:02X} twice (floating baseline 0x{:02X})",
                    addr,
                    reg,
                    first,
                    baseline
                );
                return true;
            }
            Some(second) => log::debug!(
                "unstable read at 0x{:02X} reg 0x{:02X}: 0x{:02X} then 0x{:02X}; treated as noise",
                addr,
                reg,
                first,
                second
            ),
            None => log::debug!(
                "second read at 0x{:02X} reg 0x{:02X} failed; treated as noise",
                addr,
                reg
            ),
        }
    }

    log::info!(
        "no device at 0x{:02X}: every probe register read the floating level 0x{:02X}",
        addr,
        baseline
    );
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CH341-like: nothing ever fails, values are scripted per address.
    struct SilentBus {
        acks: bool,
        values: Vec<(u8, u8)>,
        idle: u8,
    }

    impl SilentBus {
        fn new(idle: u8, values: Vec<(u8, u8)>) -> Self {
            Self {
                acks: false,
                values,
                idle,
            }
        }

        fn value_at(&self, addr: u8) -> u8 {
            self.values
                .iter()
                .find(|(a, _)| *a == addr)
                .map(|(_, v)| *v)
                .unwrap_or(self.idle)
        }
    }

    impl I2cBus for SilentBus {
        fn write(&mut self, _addr: u8, _data: &[u8]) -> Result<(), String> {
            Ok(())
        }

        fn read(&mut self, _addr: u8, buf: &mut [u8]) -> Result<(), String> {
            for byte in buf.iter_mut() {
                *byte = self.idle;
            }
            Ok(())
        }

        fn write_read(
            &mut self,
            addr: u8,
            _write_data: &[u8],
            read_buf: &mut [u8],
        ) -> Result<(), String> {
            let value = self.value_at(addr);
            for byte in read_buf.iter_mut() {
                *byte = value;
            }
            Ok(())
        }

        fn reports_nack(&self) -> bool {
            self.acks
        }
    }

    /// Hanging harness with crosstalk: reads are never the idle level, but they
    /// do not repeat either.
    struct NoisyBus {
        counter: u8,
    }

    impl I2cBus for NoisyBus {
        fn write(&mut self, _addr: u8, _data: &[u8]) -> Result<(), String> {
            Ok(())
        }

        fn read(&mut self, _addr: u8, _buf: &mut [u8]) -> Result<(), String> {
            Ok(())
        }

        fn write_read(
            &mut self,
            addr: u8,
            _write_data: &[u8],
            read_buf: &mut [u8],
        ) -> Result<(), String> {
            // Reserved address looks idle; the target keeps changing.
            let value = if addr == FLOATING_BASELINE_ADDR {
                0xFF
            } else {
                self.counter = self.counter.wrapping_add(0x11);
                0x20u8.wrapping_add(self.counter)
            };
            for byte in read_buf.iter_mut() {
                *byte = value;
            }
            Ok(())
        }

        fn reports_nack(&self) -> bool {
            false
        }
    }

    /// Reports NACK the way FTDI/CH347 do: reads at the target address fail.
    struct NackBus {
        present: bool,
    }

    impl I2cBus for NackBus {
        fn write(&mut self, _addr: u8, _data: &[u8]) -> Result<(), String> {
            Ok(())
        }

        fn read(&mut self, _addr: u8, _buf: &mut [u8]) -> Result<(), String> {
            Ok(())
        }

        fn write_read(
            &mut self,
            addr: u8,
            _write_data: &[u8],
            read_buf: &mut [u8],
        ) -> Result<(), String> {
            if self.present {
                for byte in read_buf.iter_mut() {
                    *byte = 0x2A;
                }
                Ok(())
            } else {
                Err(format!("No ACK from slave 0x{:02X}", addr))
            }
        }
    }

    const PROBE_REGS: [u8; 3] = [0x00, 0x0A, 0x0B];

    #[test]
    fn unloaded_bus_with_pullups_is_not_reported_as_present() {
        // Reproduces the report: CH341A, nothing attached, every read returns
        // 0xFF and every call succeeds.
        let mut bus = SilentBus::new(0xFF, Vec::new());

        assert!(!device_responds(&mut bus, 0x20, &PROBE_REGS));
    }

    #[test]
    fn unloaded_bus_without_pullups_is_not_reported_as_present() {
        let mut bus = SilentBus::new(0x00, Vec::new());

        assert!(!device_responds(&mut bus, 0x20, &PROBE_REGS));
    }

    #[test]
    fn a_slave_driving_any_probe_register_counts_as_present() {
        let mut bus = SilentBus::new(0xFF, vec![(0x20, 0x1C)]);

        assert!(device_responds(&mut bus, 0x20, &PROBE_REGS));
    }

    #[test]
    fn a_slave_that_only_answers_low_values_is_still_present() {
        // Pull-ups absent: idle level is 0x00, the PMU answers 0x2A.
        let mut bus = SilentBus::new(0x00, vec![(0x20, 0x2A)]);

        assert!(device_responds(&mut bus, 0x20, &PROBE_REGS));
    }

    #[test]
    fn crosstalk_that_does_not_repeat_is_not_a_device() {
        let mut bus = NoisyBus { counter: 0 };

        assert!(!device_responds(&mut bus, 0x20, &PROBE_REGS));
    }

    #[test]
    fn nack_reporting_bus_keeps_the_plain_read_rule() {
        let mut absent = NackBus { present: false };
        assert!(!device_responds(&mut absent, 0x20, &PROBE_REGS));

        let mut present = NackBus { present: true };
        assert!(device_responds(&mut present, 0x20, &PROBE_REGS));
    }

    #[test]
    fn empty_probe_register_list_is_never_present() {
        let mut bus = SilentBus::new(0xFF, vec![(0x20, 0x2A)]);

        assert!(!device_responds(&mut bus, 0x20, &[]));
    }

    #[test]
    fn idle_levels_are_the_two_passive_bus_states() {
        assert!(is_idle_bus_level(0x00));
        assert!(is_idle_bus_level(0xFF));
        assert!(!is_idle_bus_level(0x2A));
    }
}
