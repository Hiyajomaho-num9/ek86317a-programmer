//! FT232H bridge implementations.
//!
//! - `MockI2cBus`: development-only PMU simulator that satisfies the shared
//!   bridge trait
//! - `Ft232hI2cBus`: real FT232H hardware backend (feature-gated)

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

// ============================================================================
// FT232H I2C Bus (conditional compilation)
// ============================================================================

#[cfg(feature = "ft232h")]
use libftd2xx::{
    ClockBitsIn, ClockBitsOut, ClockDataOut, DeviceType, Ft232h, Ftdi, FtdiCommon, FtdiMpsse,
    MpsseCmdBuilder, MpsseSettings,
};

#[cfg(feature = "ft232h")]
pub struct Ft232hI2cBus {
    device: Ft232h,
}

#[cfg(feature = "ft232h")]
impl Ft232hI2cBus {
    /// List all connected FTDI devices, returning (index, serial_number, description) tuples.
    /// Only lists devices that are not already open.
    pub fn list_devices() -> Result<Vec<(u32, String)>, String> {
        let devices =
            libftd2xx::list_devices().map_err(|e| format!("Failed to list FTDI devices: {}", e))?;
        let mut result = Vec::new();
        for (idx, info) in devices.iter().enumerate() {
            if info.port_open {
                continue; // skip already-open devices
            }
            let desc = if info.serial_number.is_empty() && info.description.is_empty() {
                format!("FTDI #{}", idx)
            } else if info.description.is_empty() {
                info.serial_number.clone()
            } else {
                format!("{} ({})", info.description, info.serial_number)
            };
            result.push((idx as u32, desc));
        }
        Ok(result)
    }

    /// Open a specific FT232H device by index and configure MPSSE for I2C.
    pub fn open(device_index: u32, clock_hz: u32) -> Result<Self, String> {
        // Open via Ftdi generic handle, then try_into Ft232h
        let ftdi = Ftdi::with_index(device_index as i32)
            .map_err(|e| format!("Failed to open FTDI #{}: {}", device_index, e))?;
        let device: Ft232h = ftdi
            .try_into()
            .map_err(|e: libftd2xx::DeviceTypeError| format!("Device is not FT232H: {}", e))?;

        let mut bus = Self { device };
        bus.init_i2c(clock_hz)?;
        Ok(bus)
    }

    fn init_i2c(&mut self, clock_hz: u32) -> Result<(), String> {
        // Use libftd2xx's built-in MPSSE initialization
        let settings = MpsseSettings {
            clock_frequency: Some(clock_hz),
            ..MpsseSettings::default()
        };
        self.device
            .initialize_mpsse(&settings)
            .map_err(|e| format!("MPSSE init failed: {}", e))?;

        // Enable 3-phase data clocking for I2C compatibility
        let cmd = MpsseCmdBuilder::new()
            .enable_3phase_data_clocking()
            // Set initial pin state: SCL=1(AD0), SDA=1(AD1), AD2=input
            .set_gpio_lower(0x03, 0x03)
            .send_immediate();

        self.device
            .write_all(cmd.as_slice())
            .map_err(|e| format!("I2C config failed: {}", e))?;

        // Perform bus recovery to free any stuck slaves
        self.bus_recovery()?;

        log::info!("FT232H MPSSE I2C initialized: clock_hz={}", clock_hz);
        Ok(())
    }

    /// I2C bus recovery: toggle SCL 9 times with SDA released, then STOP.
    /// This frees a slave that may be holding SDA low from a previously
    /// interrupted transaction.
    fn bus_recovery(&mut self) -> Result<(), String> {
        // Release SDA (input), keep SCL as output
        let mut cmd = MpsseCmdBuilder::new().set_gpio_lower(0x00, 0x01); // SCL=out(low), SDA=input(released/high by pull-up)

        // 9 clock pulses — any stuck slave will release SDA after at most 9 clocks
        for _ in 0..9 {
            cmd = cmd
                .set_gpio_lower(0x01, 0x01) // SCL=1
                .set_gpio_lower(0x00, 0x01); // SCL=0
        }

        // Generate STOP: take SDA back as output
        cmd = cmd
            .set_gpio_lower(0x00, 0x03) // SDA=0, SCL=0
            .set_gpio_lower(0x01, 0x03) // SDA=0, SCL=1
            .set_gpio_lower(0x03, 0x03) // SDA=1, SCL=1 → STOP
            .send_immediate();

        self.device
            .write_all(cmd.as_slice())
            .map_err(|e| format!("I2C bus recovery failed: {}", e))?;

        log::debug!("I2C bus recovery completed");
        Ok(())
    }

    /// Send I2C START condition: SDA goes low while SCL is high.
    fn i2c_start(&mut self) -> Result<(), String> {
        let cmd = MpsseCmdBuilder::new()
            .set_gpio_lower(0x03, 0x03) // SDA=1, SCL=1 (idle)
            .set_gpio_lower(0x01, 0x03) // SDA=0, SCL=1 (START)
            .set_gpio_lower(0x00, 0x03) // SDA=0, SCL=0
            .send_immediate();

        self.device
            .write_all(cmd.as_slice())
            .map_err(|e| format!("I2C START failed: {}", e))
    }

    /// Send I2C STOP condition: SDA goes high while SCL is high.
    fn i2c_stop(&mut self) -> Result<(), String> {
        let cmd = MpsseCmdBuilder::new()
            .set_gpio_lower(0x00, 0x03) // SDA=0, SCL=0
            .set_gpio_lower(0x01, 0x03) // SDA=0, SCL=1
            .set_gpio_lower(0x03, 0x03) // SDA=1, SCL=1 (STOP)
            .send_immediate();

        self.device
            .write_all(cmd.as_slice())
            .map_err(|e| format!("I2C STOP failed: {}", e))
    }

    /// Write one byte and read ACK. Returns true if ACK (SDA=0).
    fn i2c_write_byte(&mut self, byte: u8) -> Result<bool, String> {
        // Clock 8 bits out (MSB first, data changes on -ve edge)
        // Then release SDA and clock in 1 ACK bit
        let cmd = MpsseCmdBuilder::new()
            .clock_data_out(ClockDataOut::MsbNeg, &[byte])
            // Release SDA: set AD1 as input
            .set_gpio_lower(0x00, 0x01) // SCL=out(low), SDA=input
            // Clock in 1 ACK bit (sampled on +ve edge)
            .clock_bits_in(ClockBitsIn::MsbPos, 1)
            // Take SDA back: AD0+AD1 as output
            .set_gpio_lower(0x00, 0x03)
            .send_immediate();

        self.device
            .write_all(cmd.as_slice())
            .map_err(|e| format!("I2C write byte MPSSE failed: {}", e))?;

        let mut ack_buf = [0u8; 1];
        self.device
            .read_all(&mut ack_buf)
            .map_err(|e| format!("I2C read ACK failed: {}", e))?;

        // ACK = bit0 is 0 (SDA pulled low by slave)
        Ok((ack_buf[0] & 0x01) == 0)
    }

    /// Read one byte and send ACK (true) or NACK (false).
    fn i2c_read_byte(&mut self, send_ack: bool) -> Result<u8, String> {
        let ack_bit: u8 = if send_ack { 0x00 } else { 0xFF };

        let cmd = MpsseCmdBuilder::new()
            // Release SDA for reading
            .set_gpio_lower(0x00, 0x01) // SCL=out(low), SDA=input
            // Clock 8 bits in (MSB first, sampled on +ve edge)
            .clock_bits_in(ClockBitsIn::MsbPos, 8)
            // Take SDA back as output
            .set_gpio_lower(0x00, 0x03)
            // Clock out 1 ACK/NACK bit
            .clock_bits_out(ClockBitsOut::MsbNeg, ack_bit, 1)
            .send_immediate();

        self.device
            .write_all(cmd.as_slice())
            .map_err(|e| format!("I2C read byte MPSSE failed: {}", e))?;

        let mut data = [0u8; 1];
        self.device
            .read_all(&mut data)
            .map_err(|e| format!("I2C read data failed: {}", e))?;

        Ok(data[0])
    }
}

#[cfg(feature = "ft232h")]
impl I2cBus for Ft232hI2cBus {
    fn write(&mut self, addr: u8, data: &[u8]) -> Result<(), String> {
        self.i2c_start()?;

        let addr_byte = (addr << 1) & 0xFE; // W bit = 0
        if !self.i2c_write_byte(addr_byte)? {
            self.i2c_stop()?;
            return Err(format!("No ACK from slave 0x{:02X} (write)", addr));
        }

        for &byte in data {
            if !self.i2c_write_byte(byte)? {
                self.i2c_stop()?;
                return Err(format!("NACK during write to slave 0x{:02X}", addr));
            }
        }

        self.i2c_stop()?;
        Ok(())
    }

    fn bus_recovery(&mut self) -> Result<(), String> {
        Ft232hI2cBus::bus_recovery(self)
    }

    fn read(&mut self, addr: u8, buf: &mut [u8]) -> Result<(), String> {
        if buf.is_empty() {
            return Ok(());
        }

        self.i2c_start()?;

        let addr_byte = (addr << 1) | 0x01; // R bit = 1
        if !self.i2c_write_byte(addr_byte)? {
            self.i2c_stop()?;
            return Err(format!("No ACK from slave 0x{:02X} (read)", addr));
        }

        let last = buf.len() - 1;
        for (i, byte) in buf.iter_mut().enumerate() {
            *byte = self.i2c_read_byte(i != last)?; // NACK on last byte
        }

        self.i2c_stop()?;
        Ok(())
    }

    fn write_read(
        &mut self,
        addr: u8,
        write_data: &[u8],
        read_buf: &mut [u8],
    ) -> Result<(), String> {
        // Write phase: START + addr(W) + data
        self.i2c_start()?;

        let addr_w = (addr << 1) & 0xFE;
        if !self.i2c_write_byte(addr_w)? {
            self.i2c_stop()?;
            return Err(format!("No ACK from slave 0x{:02X} (write phase)", addr));
        }

        for &byte in write_data {
            if !self.i2c_write_byte(byte)? {
                self.i2c_stop()?;
                return Err(format!("NACK during write to slave 0x{:02X}", addr));
            }
        }

        if read_buf.is_empty() {
            self.i2c_stop()?;
            return Ok(());
        }

        // Read phase: Repeated START + addr(R) + read
        self.i2c_start()?;

        let addr_r = (addr << 1) | 0x01;
        if !self.i2c_write_byte(addr_r)? {
            self.i2c_stop()?;
            return Err(format!("No ACK from slave 0x{:02X} (read phase)", addr));
        }

        let last = read_buf.len() - 1;
        for (i, byte) in read_buf.iter_mut().enumerate() {
            *byte = self.i2c_read_byte(i != last)?;
        }

        self.i2c_stop()?;
        Ok(())
    }
}
