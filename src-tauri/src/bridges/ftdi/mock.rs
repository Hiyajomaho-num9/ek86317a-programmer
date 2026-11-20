//! Development-only PMU simulator that satisfies the shared bridge trait.

use std::collections::HashMap;

use crate::bridges::i2c::I2cBus;
use crate::pmu::chip::{self, ChipModel, ChipSpec};

// ============================================================================
// Mock I2C Bus
// ============================================================================

pub struct MockI2cBus {
    spec: &'static ChipSpec,
    pmic_dac: HashMap<u8, u8>,
    pmic_eeprom: HashMap<u8, u8>,
    vcom_regs: HashMap<u8, u8>,
    read_eeprom: bool,
    active_slaves: Vec<u8>,
}

impl MockI2cBus {
    pub fn new(chip_model: ChipModel) -> Self {
        let spec = chip::spec_for_model(chip_model);
        let defaults = chip::default_register_map(chip_model);

        let mut pmic_dac = HashMap::new();
        let mut pmic_eeprom = HashMap::new();
        for (addr, value) in defaults {
            pmic_dac.insert(addr, value);
            pmic_eeprom.insert(addr, value);
        }

        let mut vcom_regs = HashMap::new();
        if spec.has_vcom_slave {
            if let Some(control_reg) = spec.vcom_control_reg {
                vcom_regs.insert(control_reg, 0x00);
            }
            if let Some(output_reg) = spec.vcom_output_reg {
                let default_vcom = spec
                    .pmic_vcom_register
                    .and_then(|reg| pmic_dac.get(&reg).copied())
                    .unwrap_or(0x00);
                vcom_regs.insert(output_reg, default_vcom);
            }
            if let Some(fault_reg) = spec.vcom_fault_reg {
                vcom_regs.insert(fault_reg, 0x00);
            }
        }

        let mut active_slaves = vec![spec.pmic_addr];
        if let Some(vcom_addr) = spec.vcom_addr {
            active_slaves.push(vcom_addr);
        }

        Self {
            spec,
            pmic_dac,
            pmic_eeprom,
            vcom_regs,
            read_eeprom: false,
            active_slaves,
        }
    }

    pub fn is_slave_active(&self, addr: u8) -> bool {
        self.active_slaves.contains(&addr)
    }

    fn handle_control_write(&mut self, value: u8) {
        match value {
            v if v == self.spec.ctrl_read_dac => {
                self.read_eeprom = false;
            }
            v if v == self.spec.ctrl_read_eeprom => {
                self.read_eeprom = true;
            }
            v if v == self.spec.ctrl_write_all_eeprom => {
                for (&addr, &reg_value) in &self.pmic_dac {
                    if addr != self.spec.control_reg {
                        self.pmic_eeprom.insert(addr, reg_value);
                    }
                }
            }
            v if v == self.spec.ctrl_write_vcom_eeprom => {
                if let Some(vcom_reg) = self.spec.pmic_vcom_register {
                    if let Some(&reg_value) = self.pmic_dac.get(&vcom_reg) {
                        self.pmic_eeprom.insert(vcom_reg, reg_value);
                    }
                }
            }
            _ => {
                log::warn!(
                    "[MockI2C] Unknown control command for {}: 0x{:02X}",
                    self.spec.display_name,
                    value
                );
            }
        }
    }

    fn handle_vcom_control_write(&mut self, value: u8) {
        let Some(control_reg) = self.spec.vcom_control_reg else {
            return;
        };
        self.vcom_regs.insert(control_reg, value);

        if let (Some(load_bit), Some(output_reg), Some(pmic_reg)) = (
            self.spec.vcom_load_bit,
            self.spec.vcom_output_reg,
            self.spec.pmic_vcom_register,
        ) {
            if value & (1 << load_bit) != 0 {
                if let Some(&eeprom_value) = self.pmic_eeprom.get(&pmic_reg) {
                    self.vcom_regs.insert(output_reg, eeprom_value);
                }
            }
        }

        if let (Some(write_bit), Some(output_reg), Some(pmic_reg)) = (
            self.spec.vcom_write_bit,
            self.spec.vcom_output_reg,
            self.spec.pmic_vcom_register,
        ) {
            if value & (1 << write_bit) != 0 {
                if let Some(&output_value) = self.vcom_regs.get(&output_reg) {
                    self.pmic_eeprom.insert(pmic_reg, output_value);
                }
            }
        }
    }
}

impl I2cBus for MockI2cBus {
    fn write(&mut self, addr: u8, data: &[u8]) -> Result<(), String> {
        if !self.is_slave_active(addr) {
            return Err(format!("No ACK from slave 0x{:02X}", addr));
        }

        if data.is_empty() {
            return Ok(());
        }

        let reg_addr = data[0];

        if addr == self.spec.pmic_addr {
            if data.len() == 1 {
                return Ok(());
            }

            for (offset, &val) in data[1..].iter().enumerate() {
                let target_reg = reg_addr.wrapping_add(offset as u8);
                if target_reg == self.spec.control_reg {
                    self.handle_control_write(val);
                } else {
                    self.pmic_dac.insert(target_reg, val);
                }
            }
            return Ok(());
        }

        if self.spec.vcom_addr == Some(addr) {
            if data.len() >= 2 {
                let value = data[1];
                if self.spec.vcom_control_reg == Some(reg_addr) {
                    self.handle_vcom_control_write(value);
                } else {
                    self.vcom_regs.insert(reg_addr, value);
                }
            }
            return Ok(());
        }

        Err(format!("Unknown slave address 0x{:02X}", addr))
    }

    fn read(&mut self, addr: u8, buf: &mut [u8]) -> Result<(), String> {
        if !self.is_slave_active(addr) {
            return Err(format!("No ACK from slave 0x{:02X}", addr));
        }

        for byte in buf.iter_mut() {
            *byte = 0x00;
        }
        Ok(())
    }

    fn write_read(
        &mut self,
        addr: u8,
        write_data: &[u8],
        read_buf: &mut [u8],
    ) -> Result<(), String> {
        if !self.is_slave_active(addr) {
            return Err(format!("No ACK from slave 0x{:02X}", addr));
        }

        if write_data.is_empty() {
            return self.read(addr, read_buf);
        }

        let reg_addr = write_data[0];

        if addr == self.spec.pmic_addr {
            let source = if self.read_eeprom {
                &self.pmic_eeprom
            } else {
                &self.pmic_dac
            };
            for (offset, byte) in read_buf.iter_mut().enumerate() {
                let target_reg = reg_addr.wrapping_add(offset as u8);
                *byte = source.get(&target_reg).copied().unwrap_or(0x00);
            }
            return Ok(());
        }

        if self.spec.vcom_addr == Some(addr) {
            for (offset, byte) in read_buf.iter_mut().enumerate() {
                let target_reg = reg_addr.wrapping_add(offset as u8);
                *byte = self.vcom_regs.get(&target_reg).copied().unwrap_or(0x00);
            }
            return Ok(());
        }

        Err(format!("Unknown slave address 0x{:02X}", addr))
    }
}
