use std::thread;
use std::time::Duration;

use crate::bridges::I2cBus;
use crate::pmu::chip::{self, ChipModel, ChipSpec};

pub struct ChipDevice {
    bus: Box<dyn I2cBus>,
    spec: &'static ChipSpec,
}

impl ChipDevice {
    pub fn new(bus: Box<dyn I2cBus>, spec: &'static ChipSpec) -> Self {
        Self { bus, spec }
    }

    pub fn chip_model(&self) -> ChipModel {
        self.spec.model
    }

    pub fn spec(&self) -> &'static ChipSpec {
        self.spec
    }

    pub fn get_register_name(&self, addr: u8) -> &'static str {
        chip::get_register_name(self.spec.model, addr)
    }

    pub fn decode_register_voltage(
        &self,
        addr: u8,
        value: u8,
        avdd_value: Option<u8>,
        vcom_min_value: Option<u8>,
        vcom_max_value: Option<u8>,
        mode_value: Option<u8>,
    ) -> Option<f64> {
        chip::decode_register_voltage(
            self.spec.model,
            addr,
            value,
            avdd_value,
            vcom_min_value,
            vcom_max_value,
            mode_value,
        )
    }

    pub fn read_dac_register(&mut self, reg: u8) -> Result<u8, String> {
        self.bus.write(
            self.spec.pmic_addr,
            &[self.spec.control_reg, self.spec.ctrl_read_dac],
        )?;
        thread::sleep(Duration::from_millis(self.spec.read_delay_ms));

        let mut buf = [0u8; 1];
        self.bus.write_read(self.spec.pmic_addr, &[reg], &mut buf)?;
        Ok(buf[0])
    }

    pub fn write_dac_register(&mut self, reg: u8, value: u8) -> Result<(), String> {
        self.bus.write(self.spec.pmic_addr, &[reg, value])
    }

    pub fn write_dac_registers(&mut self, start_reg: u8, data: &[u8]) -> Result<(), String> {
        let mut payload = Vec::with_capacity(1 + data.len());
        payload.push(start_reg);
        payload.extend_from_slice(data);
        self.bus.write(self.spec.pmic_addr, &payload)
    }

    pub fn read_all_dac(&mut self) -> Result<Vec<(u8, u8)>, String> {
        self.bus.write(
            self.spec.pmic_addr,
            &[self.spec.control_reg, self.spec.ctrl_read_dac],
        )?;
        thread::sleep(Duration::from_millis(self.spec.read_delay_ms));

        let mut results = Vec::new();
        for &addr in chip::register_addresses(self.spec.model) {
            if addr == self.spec.control_reg {
                continue;
            }
            let mut buf = [0u8; 1];
            self.bus
                .write_read(self.spec.pmic_addr, &[addr], &mut buf)?;
            results.push((addr, buf[0]));
        }
        Ok(results)
    }

    pub fn read_eeprom_register(&mut self, reg: u8) -> Result<u8, String> {
        self.bus.write(
            self.spec.pmic_addr,
            &[self.spec.control_reg, self.spec.ctrl_read_eeprom],
        )?;
        thread::sleep(Duration::from_millis(self.spec.read_delay_ms));

        let mut buf = [0u8; 1];
        self.bus.write_read(self.spec.pmic_addr, &[reg], &mut buf)?;
        Ok(buf[0])
    }

    pub fn read_all_eeprom(&mut self) -> Result<Vec<(u8, u8)>, String> {
        self.bus.write(
            self.spec.pmic_addr,
            &[self.spec.control_reg, self.spec.ctrl_read_eeprom],
        )?;
        thread::sleep(Duration::from_millis(self.spec.read_delay_ms));

        let mut results = Vec::new();
        for &addr in chip::register_addresses(self.spec.model) {
            if addr == self.spec.control_reg {
                continue;
            }
            let mut buf = [0u8; 1];
            self.bus
                .write_read(self.spec.pmic_addr, &[addr], &mut buf)?;
            results.push((addr, buf[0]));
        }
        Ok(results)
    }

    pub fn write_all_to_eeprom(&mut self) -> Result<(), String> {
        self.bus.write(
            self.spec.pmic_addr,
            &[self.spec.control_reg, self.spec.ctrl_write_all_eeprom],
        )?;
        thread::sleep(Duration::from_millis(self.spec.write_delay_ms));
        Ok(())
    }

    pub fn write_vcom1_to_eeprom(&mut self) -> Result<(), String> {
        self.bus.write(
            self.spec.pmic_addr,
            &[self.spec.control_reg, self.spec.ctrl_write_vcom_eeprom],
        )?;
        thread::sleep(Duration::from_millis(self.spec.write_delay_ms));
        Ok(())
    }

    pub fn read_fault_flags(&mut self) -> Result<u8, String> {
        let vcom_addr = self.spec.vcom_addr.ok_or_else(|| {
            format!(
                "{} does not expose a separate VCOM slave",
                self.spec.display_name
            )
        })?;
        let fault_reg = self
            .spec
            .vcom_fault_reg
            .ok_or_else(|| format!("{} does not expose fault flags", self.spec.display_name))?;

        let mut buf = [0u8; 1];
        self.bus.write_read(vcom_addr, &[fault_reg], &mut buf)?;
        Ok(buf[0])
    }

    pub fn verify_firmware(&mut self, data: &[u8]) -> Result<Vec<(u8, u8, u8)>, String> {
        let eeprom = self.read_all_eeprom()?;
        let mut mismatches = Vec::new();

        for &(addr, actual) in &eeprom {
            let idx = addr as usize;
            if idx < data.len() {
                let expected = data[idx];
                if expected != actual {
                    mismatches.push((addr, expected, actual));
                }
            }
        }

        Ok(mismatches)
    }

    pub fn verify_all(
        &mut self,
        data: &[u8],
    ) -> Result<(Vec<(u8, u8, u8)>, Vec<(u8, u8, u8)>), String> {
        let dac = self.read_all_dac()?;
        let mut dac_mismatches = Vec::new();
        for &(addr, actual) in &dac {
            let idx = addr as usize;
            if idx < data.len() {
                let expected = data[idx];
                if expected != actual {
                    dac_mismatches.push((addr, expected, actual));
                }
            }
        }

        let eeprom = self.read_all_eeprom()?;
        let mut eeprom_mismatches = Vec::new();
        for &(addr, actual) in &eeprom {
            let idx = addr as usize;
            if idx < data.len() {
                let expected = data[idx];
                if expected != actual {
                    eeprom_mismatches.push((addr, expected, actual));
                }
            }
        }

        Ok((dac_mismatches, eeprom_mismatches))
    }

    pub fn write_all_dac_registers(&mut self, registers: &[(u8, u8)]) -> Result<usize, String> {
        let mut count = 0;
        for &(addr, value) in registers {
            self.write_dac_register(addr, value)?;
            count += 1;
        }
        Ok(count)
    }

    pub fn probe(&mut self) -> Result<(bool, Option<bool>), String> {
        if let Err(e) = self.bus.bus_recovery() {
            log::warn!("Bus recovery failed (non-fatal): {}", e);
        }

        let mut buf = [0u8; 1];
        let pmic_ok = self
            .bus
            .write_read(self.spec.pmic_addr, &[self.spec.avdd_reg], &mut buf)
            .is_ok();

        // Probe the optional VCOM slave (0x74 / E8-E9h) with a register read
        // when the chip exposes one. EK86317A and iML8947K carry a separate
        // VCOM slave and report Some(found/missing); LP6281 has no such slave
        // (vcom_addr = None) and returns None, which the UI renders as N/A.
        // A register read is used instead of an address-only write so a
        // missing slave produces a clean NACK on a real transaction rather
        // than an ambiguous empty write on the bus.
        let vcom_detected = self.spec.vcom_addr.map(|addr| {
            let probe_reg = self
                .spec
                .vcom_fault_reg
                .or(self.spec.vcom_output_reg)
                .or(self.spec.vcom_control_reg)
                .unwrap_or(0x00);
            let mut vbuf = [0u8; 1];
            self.bus
                .write_read(addr, &[probe_reg], &mut vbuf)
                .is_ok()
        });

        Ok((pmic_ok, vcom_detected))
    }
}

#[cfg(test)]
mod tests {
    use super::ChipDevice;
    use crate::bridges::ftdi::MockI2cBus;
    use crate::pmu::chip::{spec_for_model, ChipModel};

    #[test]
    fn probe_reads_pmic_and_vcom_slave_for_ek86317a() {
        let bus = Box::new(MockI2cBus::new(ChipModel::Ek86317a));
        let mut device = ChipDevice::new(bus, spec_for_model(ChipModel::Ek86317a));

        // EK86317A exposes a VCOM slave at 0x74 (E8/E9h); both must be probed.
        assert_eq!(device.probe().unwrap(), (true, Some(true)));
    }

    #[test]
    fn probe_reads_pmic_and_vcom_slave_for_iml8947k() {
        let bus = Box::new(MockI2cBus::new(ChipModel::Iml8947k));
        let mut device = ChipDevice::new(bus, spec_for_model(ChipModel::Iml8947k));

        // iML8947K also exposes a VCOM slave at 0x74 (E8/E9h).
        assert_eq!(device.probe().unwrap(), (true, Some(true)));
    }

    #[test]
    fn probe_skips_vcom_slave_for_lp6281() {
        let bus = Box::new(MockI2cBus::new(ChipModel::Lp6281));
        let mut device = ChipDevice::new(bus, spec_for_model(ChipModel::Lp6281));

        // LP6281 has no separate VCOM slave (no E8/E9h); vcom_detected stays
        // None and the UI renders it as N/A rather than "missing".
        assert_eq!(device.probe().unwrap(), (true, None));
    }
}
