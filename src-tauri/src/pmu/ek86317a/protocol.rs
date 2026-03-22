//! EK86317A I2C protocol implementation
//!
//! Implements all device operations using the `I2cBus` trait.

use std::thread;
use std::time::Duration;

use crate::bridges::I2cBus;

use super::registers::*;

/// PMIC I2C 7-bit slave address
pub const PMIC_ADDR: u8 = 0x20;
/// VCOM I2C 7-bit slave address
pub const VCOM_ADDR: u8 = 0x74;

// Timing constants
/// Power-on blocking delay (ms)
pub const T_PON_BLK_MS: u64 = 10;
/// Write-to-MTP blocking delay (ms)
pub const T_WR_MTP_BLK_MS: u64 = 120;
/// Read-from-MTP blocking delay (ms)
pub const T_RD_MTP_BLK_MS: u64 = 5;

/// EK86317A device driver, wrapping an I2C bus.
pub struct Ek86317a {
    bus: Box<dyn I2cBus>,
}

impl Ek86317a {
    /// Create a new EK86317A driver using the given I2C bus.
    pub fn new(bus: Box<dyn I2cBus>) -> Self {
        Self { bus }
    }

    // ========================================================================
    // DAC Operations
    // ========================================================================

    /// Read a single DAC register.
    /// Protocol: write 0xFF=0x00 (select DAC), delay, then read register.
    pub fn read_dac_register(&mut self, reg: u8) -> Result<u8, String> {
        // Select DAC bank
        self.bus.write(PMIC_ADDR, &[REG_CONTROL, CTRL_READ_DAC])?;
        thread::sleep(Duration::from_millis(T_RD_MTP_BLK_MS));

        // Read the register
        let mut buf = [0u8; 1];
        self.bus.write_read(PMIC_ADDR, &[reg], &mut buf)?;
        Ok(buf[0])
    }

    /// Write a single DAC register.
    /// Protocol: [slave_addr_w, reg, value]
    pub fn write_dac_register(&mut self, reg: u8, value: u8) -> Result<(), String> {
        self.bus.write(PMIC_ADDR, &[reg, value])
    }

    /// Write multiple DAC registers starting from `start_reg`.
    pub fn write_dac_registers(&mut self, start_reg: u8, data: &[u8]) -> Result<(), String> {
        let mut payload = Vec::with_capacity(1 + data.len());
        payload.push(start_reg);
        payload.extend_from_slice(data);
        self.bus.write(PMIC_ADDR, &payload)
    }

    /// Read all DAC registers. Returns (address, value) pairs.
    pub fn read_all_dac(&mut self) -> Result<Vec<(u8, u8)>, String> {
        // Select DAC bank
        self.bus.write(PMIC_ADDR, &[REG_CONTROL, CTRL_READ_DAC])?;
        thread::sleep(Duration::from_millis(T_RD_MTP_BLK_MS));

        let mut results = Vec::new();
        for &addr in PMIC_REG_ADDRESSES {
            if addr == REG_CONTROL {
                continue; // Skip control register
            }
            let mut buf = [0u8; 1];
            self.bus.write_read(PMIC_ADDR, &[addr], &mut buf)?;
            results.push((addr, buf[0]));
        }
        Ok(results)
    }

    // ========================================================================
    // EEPROM Operations
    // ========================================================================

    /// Read a single EEPROM register.
    /// Protocol: write 0xFF=0x01 (select EEPROM), delay, then read.
    pub fn read_eeprom_register(&mut self, reg: u8) -> Result<u8, String> {
        // Select EEPROM bank
        self.bus
            .write(PMIC_ADDR, &[REG_CONTROL, CTRL_READ_EEPROM])?;
        thread::sleep(Duration::from_millis(T_RD_MTP_BLK_MS));

        let mut buf = [0u8; 1];
        self.bus.write_read(PMIC_ADDR, &[reg], &mut buf)?;
        Ok(buf[0])
    }

    /// Read all EEPROM registers. Returns (address, value) pairs.
    pub fn read_all_eeprom(&mut self) -> Result<Vec<(u8, u8)>, String> {
        // Select EEPROM bank
        self.bus
            .write(PMIC_ADDR, &[REG_CONTROL, CTRL_READ_EEPROM])?;
        thread::sleep(Duration::from_millis(T_RD_MTP_BLK_MS));

        let mut results = Vec::new();
        for &addr in PMIC_REG_ADDRESSES {
            if addr == REG_CONTROL {
                continue;
            }
            let mut buf = [0u8; 1];
            self.bus.write_read(PMIC_ADDR, &[addr], &mut buf)?;
            results.push((addr, buf[0]));
        }
        Ok(results)
    }

    /// Write all DAC register values to EEPROM.
    /// Protocol: write 0xFF=0x80, then wait T_WR_MTP_BLK_MS.
    pub fn write_all_to_eeprom(&mut self) -> Result<(), String> {
        self.bus
            .write(PMIC_ADDR, &[REG_CONTROL, CTRL_WRITE_ALL_EEPROM])?;
        thread::sleep(Duration::from_millis(T_WR_MTP_BLK_MS));
        Ok(())
    }

    /// Write only VCOM1_NT to EEPROM.
    /// Protocol: write 0xFF=0x40, then wait T_WR_MTP_BLK_MS.
    pub fn write_vcom1_to_eeprom(&mut self) -> Result<(), String> {
        self.bus
            .write(PMIC_ADDR, &[REG_CONTROL, CTRL_WRITE_VCOM1_EEPROM])?;
        thread::sleep(Duration::from_millis(T_WR_MTP_BLK_MS));
        Ok(())
    }

    // ========================================================================
    // VCOM Operations (slave 0x74)
    // ========================================================================

    /// Read VCOM control register.
    pub fn read_vcom_control(&mut self) -> Result<u8, String> {
        let mut buf = [0u8; 1];
        self.bus
            .write_read(VCOM_ADDR, &[VCOM_REG_CONTROL], &mut buf)?;
        Ok(buf[0])
    }

    /// Write VCOM control register.
    pub fn write_vcom_control(&mut self, value: u8) -> Result<(), String> {
        self.bus.write(VCOM_ADDR, &[VCOM_REG_CONTROL, value])
    }

    /// Read VCOM1_NT register from VCOM slave.
    pub fn read_vcom1_nt(&mut self) -> Result<u8, String> {
        let mut buf = [0u8; 1];
        self.bus
            .write_read(VCOM_ADDR, &[VCOM_REG_VCOM1_NT], &mut buf)?;
        Ok(buf[0])
    }

    /// Write VCOM1_NT register via VCOM slave.
    pub fn write_vcom1_nt(&mut self, value: u8) -> Result<(), String> {
        self.bus.write(VCOM_ADDR, &[VCOM_REG_VCOM1_NT, value])
    }

    /// Read fault flags from VCOM slave.
    pub fn read_fault_flags(&mut self) -> Result<u8, String> {
        let mut buf = [0u8; 1];
        self.bus
            .write_read(VCOM_ADDR, &[VCOM_REG_FAULT], &mut buf)?;
        Ok(buf[0])
    }

    /// Enable or disable VCOM1 output.
    /// Sets/clears bit1 of VCOM control register.
    pub fn set_vcom1_enable(&mut self, enable: bool) -> Result<(), String> {
        let mut ctrl = self.read_vcom_control()?;
        if enable {
            ctrl |= 0x02; // bit1 = VCOM1_EN
        } else {
            ctrl &= !0x02;
        }
        self.write_vcom_control(ctrl)
    }

    /// Load VCOM1_NT from EEPROM by asserting RESET bit (bit4).
    pub fn load_vcom1_from_eeprom(&mut self) -> Result<(), String> {
        let ctrl = self.read_vcom_control()?;
        self.write_vcom_control(ctrl | 0x10)?; // bit4 = RESET
        thread::sleep(Duration::from_millis(T_RD_MTP_BLK_MS));
        Ok(())
    }

    /// Write VCOM1_NT to EEPROM by asserting W_VCOM1_NT bit (bit3).
    pub fn write_vcom1_nt_to_eeprom(&mut self) -> Result<(), String> {
        let ctrl = self.read_vcom_control()?;
        self.write_vcom_control(ctrl | 0x08)?; // bit3 = W_VCOM1_NT
        thread::sleep(Duration::from_millis(T_WR_MTP_BLK_MS));
        Ok(())
    }

    // ========================================================================
    // Firmware Operations
    // ========================================================================

    /// Write firmware data to DAC registers using multi-byte write,
    /// starting from register 0x00.
    pub fn write_firmware(&mut self, data: &[u8]) -> Result<(), String> {
        if data.is_empty() {
            return Err("Empty firmware data".to_string());
        }

        // Write in chunks — I2C multi-byte write from reg 0x00
        // We write all data registers but skip the control register (0xFF)
        let mut payload = Vec::with_capacity(1 + data.len());
        payload.push(0x00); // start register address
        payload.extend_from_slice(data);
        self.bus.write(PMIC_ADDR, &payload)?;

        Ok(())
    }

    /// Verify firmware by reading back EEPROM and comparing.
    /// Returns a list of mismatches: (address, expected, actual).
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

    /// Verify firmware against both DAC and EEPROM banks.
    /// Returns (dac_mismatches, eeprom_mismatches).
    pub fn verify_all(
        &mut self,
        data: &[u8],
    ) -> Result<(Vec<(u8, u8, u8)>, Vec<(u8, u8, u8)>), String> {
        // Verify DAC bank
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

        // Verify EEPROM bank
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

    /// Write multiple DAC registers from (address, value) pairs.
    pub fn write_all_dac_registers(&mut self, registers: &[(u8, u8)]) -> Result<usize, String> {
        let mut count = 0;
        for &(addr, value) in registers {
            self.write_dac_register(addr, value)?;
            count += 1;
        }
        Ok(count)
    }

    // ========================================================================
    // Probe / Detection
    // ========================================================================

    /// Probe for PMIC and VCOM slaves on the I2C bus.
    /// Uses address-only write (like i2cdetect) — the most reliable detection method.
    /// Returns (pmic_detected, vcom_detected).
    pub fn probe(&mut self) -> Result<(bool, bool), String> {
        // Bus recovery: free any slave holding SDA low from a previous interrupted transaction
        if let Err(e) = self.bus.bus_recovery() {
            log::warn!("Bus recovery failed (non-fatal): {}", e);
        }

        // Address-only probe: START + addr_W + check ACK + STOP
        // This is the i2cdetect standard — minimal transaction, no register side effects
        let pmic_ok = self.bus.write(PMIC_ADDR, &[]).is_ok();
        let vcom_ok = self.bus.write(VCOM_ADDR, &[]).is_ok();

        Ok((pmic_ok, vcom_ok))
    }
}
