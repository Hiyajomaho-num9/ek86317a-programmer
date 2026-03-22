//! I2C bus trait and implementations
//!
//! - `I2cBus` trait: abstraction for I2C communication
//! - `MockI2cBus`: simulates EK86317A register behavior for development
//! - `Ft232hI2cBus`: real FT232H hardware (feature-gated)

use std::collections::HashMap;

use crate::ek86317a::registers::*;

// ============================================================================
// I2C Bus Trait
// ============================================================================

/// Generic I2C bus interface. All addresses are 7-bit.
pub trait I2cBus: Send {
    /// Write data to the given 7-bit slave address.
    /// `data` contains the register address followed by value byte(s).
    /// When `data` is empty, performs an address-only probe (START + addr_W + STOP).
    fn write(&mut self, addr: u8, data: &[u8]) -> Result<(), String>;

    /// Read `buf.len()` bytes from the given 7-bit slave address.
    fn read(&mut self, addr: u8, buf: &mut [u8]) -> Result<(), String>;

    /// Combined write-then-read transaction (repeated START).
    /// Writes `write_data` then reads into `read_buf`.
    fn write_read(
        &mut self,
        addr: u8,
        write_data: &[u8],
        read_buf: &mut [u8],
    ) -> Result<(), String>;

    /// Perform I2C bus recovery: 9 SCL clock pulses + STOP.
    /// Frees the bus if a slave is holding SDA low from a previously
    /// interrupted transaction. Default implementation is a no-op.
    fn bus_recovery(&mut self) -> Result<(), String> {
        Ok(())
    }
}

// ============================================================================
// Mock I2C Bus — simulates EK86317A registers
// ============================================================================

/// Mock I2C bus that simulates EK86317A register read/write behavior.
/// Internally maintains separate register maps for PMIC (0x20) and VCOM (0x74) slaves,
/// plus DAC vs EEPROM bank selection.
pub struct MockI2cBus {
    /// PMIC DAC registers (what the chip is currently outputting)
    pmic_dac: HashMap<u8, u8>,
    /// PMIC EEPROM registers (non-volatile storage)
    pmic_eeprom: HashMap<u8, u8>,
    /// VCOM slave registers
    vcom_regs: HashMap<u8, u8>,
    /// Current read source selection: false=DAC, true=EEPROM
    read_eeprom: bool,
    /// Tracks which slave addresses respond (for probe)
    active_slaves: Vec<u8>,
}

impl MockI2cBus {
    /// Create a new MockI2cBus with default EK86317A register values.
    pub fn new() -> Self {
        let mut pmic_dac = HashMap::new();
        let mut pmic_eeprom = HashMap::new();

        // Initialize all PMIC registers with their default values
        for reg in PMIC_REGISTERS {
            pmic_dac.insert(reg.address, reg.default_value);
            pmic_eeprom.insert(reg.address, reg.default_value);
        }

        let mut vcom_regs = HashMap::new();
        vcom_regs.insert(VCOM_REG_CONTROL, 0x00u8);
        vcom_regs.insert(VCOM_REG_VCOM1_NT, 0x00u8);
        vcom_regs.insert(VCOM_REG_FAULT, 0x00u8); // no faults

        Self {
            pmic_dac,
            pmic_eeprom,
            vcom_regs,
            read_eeprom: false,
            active_slaves: vec![0x20, 0x74],
        }
    }

    /// Check if a slave address is active (for probe simulation).
    pub fn is_slave_active(&self, addr: u8) -> bool {
        self.active_slaves.contains(&addr)
    }

    /// Handle PMIC control register writes (0xFF).
    /// This simulates the bank switch and EEPROM burn commands.
    fn handle_control_write(&mut self, value: u8) {
        match value {
            CTRL_READ_DAC => {
                // Select DAC bank for reading
                self.read_eeprom = false;
                log::debug!("[MockI2C] Control: selected DAC bank for reading");
            }
            CTRL_READ_EEPROM => {
                // Select EEPROM bank for reading
                self.read_eeprom = true;
                log::debug!("[MockI2C] Control: selected EEPROM bank for reading");
            }
            CTRL_WRITE_ALL_EEPROM => {
                // Copy all DAC registers to EEPROM
                for (&addr, &val) in &self.pmic_dac {
                    if addr != REG_CONTROL {
                        self.pmic_eeprom.insert(addr, val);
                    }
                }
                log::debug!("[MockI2C] Control: wrote all DAC to EEPROM");
            }
            CTRL_WRITE_VCOM1_EEPROM => {
                // Copy only VCOM1_NT to EEPROM
                if let Some(&val) = self.pmic_dac.get(&REG_VCOM1_NT) {
                    self.pmic_eeprom.insert(REG_VCOM1_NT, val);
                }
                log::debug!("[MockI2C] Control: wrote VCOM1_NT to EEPROM");
            }
            _ => {
                log::warn!("[MockI2C] Unknown control command: 0x{:02X}", value);
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

        match addr {
            0x20 => {
                // PMIC slave
                if data.len() == 1 {
                    // Address-only write (setting read pointer) — no-op for mock
                    return Ok(());
                }
                // Write one or more bytes starting from reg_addr
                for (i, &val) in data[1..].iter().enumerate() {
                    let target_reg = reg_addr.wrapping_add(i as u8);
                    if target_reg == REG_CONTROL {
                        self.handle_control_write(val);
                    } else {
                        self.pmic_dac.insert(target_reg, val);
                        log::trace!(
                            "[MockI2C] PMIC DAC write: reg 0x{:02X} = 0x{:02X}",
                            target_reg,
                            val
                        );
                    }
                }
            }
            0x74 => {
                // VCOM slave
                if data.len() >= 2 {
                    let val = data[1];
                    self.vcom_regs.insert(reg_addr, val);
                    log::trace!(
                        "[MockI2C] VCOM write: reg 0x{:02X} = 0x{:02X}",
                        reg_addr,
                        val
                    );

                    // Handle special VCOM control bits
                    if reg_addr == VCOM_REG_CONTROL {
                        if val & 0x10 != 0 {
                            // RESET bit — reload VCOM1_NT from EEPROM
                            if let Some(&eeprom_val) = self.pmic_eeprom.get(&REG_VCOM1_NT) {
                                self.vcom_regs.insert(VCOM_REG_VCOM1_NT, eeprom_val);
                                log::debug!("[MockI2C] VCOM RESET: loaded VCOM1_NT from EEPROM");
                            }
                        }
                        if val & 0x08 != 0 {
                            // W_VCOM1_NT — write VCOM1_NT to EEPROM
                            if let Some(&vcom_val) = self.vcom_regs.get(&VCOM_REG_VCOM1_NT) {
                                self.pmic_eeprom.insert(REG_VCOM1_NT, vcom_val);
                                log::debug!("[MockI2C] VCOM W_VCOM1_NT: saved to EEPROM");
                            }
                        }
                    }
                }
            }
            _ => {
                return Err(format!("Unknown slave address 0x{:02X}", addr));
            }
        }

        Ok(())
    }

    fn read(&mut self, addr: u8, buf: &mut [u8]) -> Result<(), String> {
        if !self.is_slave_active(addr) {
            return Err(format!("No ACK from slave 0x{:02X}", addr));
        }

        // For simplicity, read returns 0x00 for unknown registers
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

        match addr {
            0x20 => {
                // PMIC slave — read from selected bank
                let source = if self.read_eeprom {
                    &self.pmic_eeprom
                } else {
                    &self.pmic_dac
                };
                for (i, byte) in read_buf.iter_mut().enumerate() {
                    let target_reg = reg_addr.wrapping_add(i as u8);
                    *byte = source.get(&target_reg).copied().unwrap_or(0x00);
                    log::trace!(
                        "[MockI2C] PMIC read ({}): reg 0x{:02X} = 0x{:02X}",
                        if self.read_eeprom { "EEPROM" } else { "DAC" },
                        target_reg,
                        *byte
                    );
                }
            }
            0x74 => {
                // VCOM slave
                for (i, byte) in read_buf.iter_mut().enumerate() {
                    let target_reg = reg_addr.wrapping_add(i as u8);
                    *byte = self.vcom_regs.get(&target_reg).copied().unwrap_or(0x00);
                    log::trace!(
                        "[MockI2C] VCOM read: reg 0x{:02X} = 0x{:02X}",
                        target_reg,
                        *byte
                    );
                }
            }
            _ => {
                return Err(format!("Unknown slave address 0x{:02X}", addr));
            }
        }

        Ok(())
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
