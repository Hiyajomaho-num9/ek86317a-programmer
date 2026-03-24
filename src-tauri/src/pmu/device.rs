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

        let pmic_ok = self.bus.write(self.spec.pmic_addr, &[]).is_ok();
        let vcom_ok = self
            .spec
            .vcom_addr
            .map(|addr| self.bus.write(addr, &[]).is_ok());

        Ok((pmic_ok, vcom_ok))
    }
}
