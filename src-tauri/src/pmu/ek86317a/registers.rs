//! EK86317A register definitions and voltage encoding/decoding
//!
//! Based on EK86317A datasheet register map.
//! PMIC Slave address: 0x20 (7-bit)
//! VCOM Slave address: 0x74 (7-bit)

use serde::{Deserialize, Serialize};

// ============================================================================
// PMIC Slave (0x20) Register Address Constants
// ============================================================================

/// AVDD output voltage, 6-bit [5:0], range 13.5V~19.8V, step 0.1V
pub const REG_AVDD: u8 = 0x00;
/// VBK1 output voltage, 6-bit [5:0], range 0.8V~3.35V, step 0.05V (non-linear mapping)
pub const REG_VBK1: u8 = 0x01;
/// HAVDD output voltage, 7-bit [6:0], HAVDD = AVDD × DAC_CODE / 128
pub const REG_HAVDD: u8 = 0x02;
/// VGH normal temperature, 5-bit [4:0], range 20V~45V, step 1V
pub const REG_VGH_NT: u8 = 0x03;
/// VGH low temperature, bit7=enable, [4:0]=voltage, range 20V~45V
pub const REG_VGH_LT: u8 = 0x04;
/// VGL normal temperature, 5-bit [4:0], range -3V~-18V, step 0.5V
pub const REG_VGL_NT: u8 = 0x05;
/// VGL temperature compensation, bit7=enable, bit6=LT/HT select, [4:0]=voltage
pub const REG_VGL_LT_HT: u8 = 0x06;
/// bit7=VCOM1_HT_EN, [4:0]=VSS1 voltage, range -3V~-16V
pub const REG_VSS1: u8 = 0x07;
/// VCOM1 normal temperature, 7-bit [7:1]
pub const REG_VCOM1_NT: u8 = 0x08;
/// VCOM1 high temperature, 7-bit [7:1]
pub const REG_VCOM1_HT: u8 = 0x09;
/// VCOM upper limit, 7-bit [6:0], VCOM_MAX = AVDD × DAC_CODE / 128
pub const REG_VCOM_MAX: u8 = 0x0A;
/// VCOM lower limit, 7-bit [6:0], VCOM_MIN = AVDD × DAC_CODE / 128
pub const REG_VCOM_MIN: u8 = 0x0B;

// GAMMA registers: 14 channels, each 2 registers (H[1:0] + L[7:0]), 10-bit
/// GAMMA 1 High byte [1:0]
pub const REG_GMA1_H: u8 = 0x0C;
/// GAMMA 1 Low byte [7:0]
pub const REG_GMA1_L: u8 = 0x0D;
/// GAMMA 2 High byte [1:0]
pub const REG_GMA2_H: u8 = 0x0E;
/// GAMMA 2 Low byte [7:0]
pub const REG_GMA2_L: u8 = 0x0F;
/// GAMMA 3 High byte [1:0]
pub const REG_GMA3_H: u8 = 0x10;
/// GAMMA 3 Low byte [7:0]
pub const REG_GMA3_L: u8 = 0x11;
/// GAMMA 4 High byte [1:0]
pub const REG_GMA4_H: u8 = 0x12;
/// GAMMA 4 Low byte [7:0]
pub const REG_GMA4_L: u8 = 0x13;
/// GAMMA 5 High byte [1:0]
pub const REG_GMA5_H: u8 = 0x14;
/// GAMMA 5 Low byte [7:0]
pub const REG_GMA5_L: u8 = 0x15;
/// GAMMA 6 High byte [1:0]
pub const REG_GMA6_H: u8 = 0x16;
/// GAMMA 6 Low byte [7:0]
pub const REG_GMA6_L: u8 = 0x17;
/// GAMMA 7 High byte [1:0]
pub const REG_GMA7_H: u8 = 0x18;
/// GAMMA 7 Low byte [7:0]
pub const REG_GMA7_L: u8 = 0x19;
/// GAMMA 8 High byte [1:0]
pub const REG_GMA8_H: u8 = 0x1A;
/// GAMMA 8 Low byte [7:0]
pub const REG_GMA8_L: u8 = 0x1B;
/// GAMMA 9 High byte [1:0]
pub const REG_GMA9_H: u8 = 0x1C;
/// GAMMA 9 Low byte [7:0]
pub const REG_GMA9_L: u8 = 0x1D;
/// GAMMA 10 High byte [1:0]
pub const REG_GMA10_H: u8 = 0x1E;
/// GAMMA 10 Low byte [7:0]
pub const REG_GMA10_L: u8 = 0x1F;
/// GAMMA 11 High byte [1:0]
pub const REG_GMA11_H: u8 = 0x20;
/// GAMMA 11 Low byte [7:0]
pub const REG_GMA11_L: u8 = 0x21;
/// GAMMA 12 High byte [1:0]
pub const REG_GMA12_H: u8 = 0x22;
/// GAMMA 12 Low byte [7:0]
pub const REG_GMA12_L: u8 = 0x23;
/// GAMMA 13 High byte [1:0]
pub const REG_GMA13_H: u8 = 0x24;
/// GAMMA 13 Low byte [7:0]
pub const REG_GMA13_L: u8 = 0x25;
/// GAMMA 14 High byte [1:0]
pub const REG_GMA14_H: u8 = 0x26;
/// GAMMA 14 Low byte [7:0]
pub const REG_GMA14_L: u8 = 0x27;

/// Channel enable, each bit controls one channel
pub const REG_CHANNEL_EN: u8 = 0x28;
/// Frequency/structure selection
pub const REG_CHANNEL_SET: u8 = 0x29;
/// VBK1/VGL/VSS1 delay
pub const REG_DELAY1: u8 = 0x2A;
/// AVDD/VGH/VCOM delay
pub const REG_DELAY2: u8 = 0x2B;
/// Discharge setting
pub const REG_DISCHARGE: u8 = 0x2C;
/// VSS1 discharge / soft start etc.
pub const REG_CONFIG1: u8 = 0x2D;
/// NTC VGH & VGL
pub const REG_NTC_VGH_VGL: u8 = 0x2E;
/// NTC VCOM1
pub const REG_NTC_VCOM1: u8 = 0x2F;
/// AVDD external drive / compensation
pub const REG_CONFIG2: u8 = 0x30;
/// EN / delay shutdown / XON
pub const REG_CONFIG3: u8 = 0x31;
/// VBK1/AVDD/VGH discharge disable
pub const REG_DISCHARGE2: u8 = 0x32;
/// VCOM2DAC output, 7-bit [7:1]
pub const REG_VCOM2DAC: u8 = 0x45;
/// VCOM2DAC enable
pub const REG_VCOM2DAC_EN: u8 = 0x46;
/// Control register (0x80=write all EEPROM, 0x40=write VCOM1, 0x01=read EEPROM, 0x00=read DAC)
pub const REG_CONTROL: u8 = 0xFF;

// ============================================================================
// VCOM Slave (0x74) Register Address Constants
// ============================================================================

/// VCOM control: bit4=RESET, bit3=W_VCOM1_NT, bit1=VCOM1_EN
pub const VCOM_REG_CONTROL: u8 = 0x00;
/// VCOM1_NT 7-bit [7:1]
pub const VCOM_REG_VCOM1_NT: u8 = 0x01;
/// Fault Flag (read-only)
pub const VCOM_REG_FAULT: u8 = 0x02;

// ============================================================================
// Control Register Command Values
// ============================================================================

/// Write all registers to EEPROM
pub const CTRL_WRITE_ALL_EEPROM: u8 = 0x80;
/// Write VCOM1 to EEPROM
pub const CTRL_WRITE_VCOM1_EEPROM: u8 = 0x40;
/// Select EEPROM for reading
pub const CTRL_READ_EEPROM: u8 = 0x01;
/// Select DAC for reading
pub const CTRL_READ_DAC: u8 = 0x00;

// ============================================================================
// Datasheet-backed default register values
// ============================================================================

pub const DEFAULT_REG_AVDD_VALUE: u8 = 0x00;
pub const DEFAULT_REG_VCOM1_NT_VALUE: u8 = 0x7E;
pub const DEFAULT_REG_VCOM1_HT_VALUE: u8 = 0xBA;
pub const DEFAULT_REG_VCOM_MAX_VALUE: u8 = 0x3F;
pub const DEFAULT_REG_VCOM_MIN_VALUE: u8 = 0x26;
pub const DEFAULT_REG_VCOM2DAC_VALUE: u8 = 0x7E;

// ============================================================================
// RegisterInfo for frontend display
// ============================================================================

/// Register metadata for frontend display
#[derive(Debug, Clone, Serialize)]
pub struct RegisterInfo {
    pub address: u8,
    pub name: String,
    pub description: String,
    pub default_value: u8,
    pub bit_width: u8,
}

/// Complete register map definition with defaults
#[derive(Debug, Clone)]
pub struct RegisterDef {
    pub address: u8,
    pub name: &'static str,
    pub description: &'static str,
    pub default_value: u8,
    pub bit_width: u8,
    pub writable: bool,
}

/// All PMIC registers in order
pub static PMIC_REGISTERS: &[RegisterDef] = &[
    RegisterDef {
        address: REG_AVDD,
        name: "AVDD",
        description: "AVDD output voltage (13.5V~19.8V, 0.1V step)",
        default_value: DEFAULT_REG_AVDD_VALUE,
        bit_width: 6,
        writable: true,
    },
    RegisterDef {
        address: REG_VBK1,
        name: "VBK1",
        description: "VBK1 output voltage (0.8V~3.35V, non-linear)",
        default_value: 0x00,
        bit_width: 6,
        writable: true,
    },
    RegisterDef {
        address: REG_HAVDD,
        name: "HAVDD",
        description: "HAVDD = AVDD × DAC / 128",
        default_value: 0x00,
        bit_width: 7,
        writable: true,
    },
    RegisterDef {
        address: REG_VGH_NT,
        name: "VGH_NT",
        description: "VGH normal temp (20V~45V, 1V step)",
        default_value: 0x00,
        bit_width: 5,
        writable: true,
    },
    RegisterDef {
        address: REG_VGH_LT,
        name: "VGH_LT",
        description: "VGH low temp, bit7=EN, [4:0]=voltage",
        default_value: 0x00,
        bit_width: 8,
        writable: true,
    },
    RegisterDef {
        address: REG_VGL_NT,
        name: "VGL_NT",
        description: "VGL normal temp (-3V~-18V, 0.5V step)",
        default_value: 0x00,
        bit_width: 5,
        writable: true,
    },
    RegisterDef {
        address: REG_VGL_LT_HT,
        name: "VGL_LT_HT",
        description: "VGL temp comp, bit7=EN, bit6=LT/HT, [4:0]=V",
        default_value: 0x00,
        bit_width: 8,
        writable: true,
    },
    RegisterDef {
        address: REG_VSS1,
        name: "VSS1",
        description: "bit7=VCOM1_HT_EN, [4:0]=VSS1 (-3V~-16V)",
        default_value: 0x00,
        bit_width: 8,
        writable: true,
    },
    RegisterDef {
        address: REG_VCOM1_NT,
        name: "VCOM1_NT",
        description: "VCOM1 normal temp, 7-bit [7:1]",
        default_value: DEFAULT_REG_VCOM1_NT_VALUE,
        bit_width: 8,
        writable: true,
    },
    RegisterDef {
        address: REG_VCOM1_HT,
        name: "VCOM1_HT",
        description: "VCOM1 high temp, 7-bit [7:1]",
        default_value: DEFAULT_REG_VCOM1_HT_VALUE,
        bit_width: 8,
        writable: true,
    },
    RegisterDef {
        address: REG_VCOM_MAX,
        name: "VCOM_MAX",
        description: "VCOM upper limit = AVDD × DAC / 128",
        default_value: DEFAULT_REG_VCOM_MAX_VALUE,
        bit_width: 7,
        writable: true,
    },
    RegisterDef {
        address: REG_VCOM_MIN,
        name: "VCOM_MIN",
        description: "VCOM lower limit = AVDD × DAC / 128",
        default_value: DEFAULT_REG_VCOM_MIN_VALUE,
        bit_width: 7,
        writable: true,
    },
    // GAMMA 1~14 (H + L pairs)
    RegisterDef {
        address: REG_GMA1_H,
        name: "GMA1_H",
        description: "Gamma 1 high [1:0]",
        default_value: 0x00,
        bit_width: 2,
        writable: true,
    },
    RegisterDef {
        address: REG_GMA1_L,
        name: "GMA1_L",
        description: "Gamma 1 low [7:0]",
        default_value: 0x00,
        bit_width: 8,
        writable: true,
    },
    RegisterDef {
        address: REG_GMA2_H,
        name: "GMA2_H",
        description: "Gamma 2 high [1:0]",
        default_value: 0x00,
        bit_width: 2,
        writable: true,
    },
    RegisterDef {
        address: REG_GMA2_L,
        name: "GMA2_L",
        description: "Gamma 2 low [7:0]",
        default_value: 0x00,
        bit_width: 8,
        writable: true,
    },
    RegisterDef {
        address: REG_GMA3_H,
        name: "GMA3_H",
        description: "Gamma 3 high [1:0]",
        default_value: 0x00,
        bit_width: 2,
        writable: true,
    },
    RegisterDef {
        address: REG_GMA3_L,
        name: "GMA3_L",
        description: "Gamma 3 low [7:0]",
        default_value: 0x00,
        bit_width: 8,
        writable: true,
    },
    RegisterDef {
        address: REG_GMA4_H,
        name: "GMA4_H",
        description: "Gamma 4 high [1:0]",
        default_value: 0x00,
        bit_width: 2,
        writable: true,
    },
    RegisterDef {
        address: REG_GMA4_L,
        name: "GMA4_L",
        description: "Gamma 4 low [7:0]",
        default_value: 0x00,
        bit_width: 8,
        writable: true,
    },
    RegisterDef {
        address: REG_GMA5_H,
        name: "GMA5_H",
        description: "Gamma 5 high [1:0]",
        default_value: 0x00,
        bit_width: 2,
        writable: true,
    },
    RegisterDef {
        address: REG_GMA5_L,
        name: "GMA5_L",
        description: "Gamma 5 low [7:0]",
        default_value: 0x00,
        bit_width: 8,
        writable: true,
    },
    RegisterDef {
        address: REG_GMA6_H,
        name: "GMA6_H",
        description: "Gamma 6 high [1:0]",
        default_value: 0x00,
        bit_width: 2,
        writable: true,
    },
    RegisterDef {
        address: REG_GMA6_L,
        name: "GMA6_L",
        description: "Gamma 6 low [7:0]",
        default_value: 0x00,
        bit_width: 8,
        writable: true,
    },
    RegisterDef {
        address: REG_GMA7_H,
        name: "GMA7_H",
        description: "Gamma 7 high [1:0]",
        default_value: 0x00,
        bit_width: 2,
        writable: true,
    },
    RegisterDef {
        address: REG_GMA7_L,
        name: "GMA7_L",
        description: "Gamma 7 low [7:0]",
        default_value: 0x00,
        bit_width: 8,
        writable: true,
    },
    RegisterDef {
        address: REG_GMA8_H,
        name: "GMA8_H",
        description: "Gamma 8 high [1:0]",
        default_value: 0x00,
        bit_width: 2,
        writable: true,
    },
    RegisterDef {
        address: REG_GMA8_L,
        name: "GMA8_L",
        description: "Gamma 8 low [7:0]",
        default_value: 0x00,
        bit_width: 8,
        writable: true,
    },
    RegisterDef {
        address: REG_GMA9_H,
        name: "GMA9_H",
        description: "Gamma 9 high [1:0]",
        default_value: 0x00,
        bit_width: 2,
        writable: true,
    },
    RegisterDef {
        address: REG_GMA9_L,
        name: "GMA9_L",
        description: "Gamma 9 low [7:0]",
        default_value: 0x00,
        bit_width: 8,
        writable: true,
    },
    RegisterDef {
        address: REG_GMA10_H,
        name: "GMA10_H",
        description: "Gamma 10 high [1:0]",
        default_value: 0x00,
        bit_width: 2,
        writable: true,
    },
    RegisterDef {
        address: REG_GMA10_L,
        name: "GMA10_L",
        description: "Gamma 10 low [7:0]",
        default_value: 0x00,
        bit_width: 8,
        writable: true,
    },
    RegisterDef {
        address: REG_GMA11_H,
        name: "GMA11_H",
        description: "Gamma 11 high [1:0]",
        default_value: 0x00,
        bit_width: 2,
        writable: true,
    },
    RegisterDef {
        address: REG_GMA11_L,
        name: "GMA11_L",
        description: "Gamma 11 low [7:0]",
        default_value: 0x00,
        bit_width: 8,
        writable: true,
    },
    RegisterDef {
        address: REG_GMA12_H,
        name: "GMA12_H",
        description: "Gamma 12 high [1:0]",
        default_value: 0x00,
        bit_width: 2,
        writable: true,
    },
    RegisterDef {
        address: REG_GMA12_L,
        name: "GMA12_L",
        description: "Gamma 12 low [7:0]",
        default_value: 0x00,
        bit_width: 8,
        writable: true,
    },
    RegisterDef {
        address: REG_GMA13_H,
        name: "GMA13_H",
        description: "Gamma 13 high [1:0]",
        default_value: 0x00,
        bit_width: 2,
        writable: true,
    },
    RegisterDef {
        address: REG_GMA13_L,
        name: "GMA13_L",
        description: "Gamma 13 low [7:0]",
        default_value: 0x00,
        bit_width: 8,
        writable: true,
    },
    RegisterDef {
        address: REG_GMA14_H,
        name: "GMA14_H",
        description: "Gamma 14 high [1:0]",
        default_value: 0x00,
        bit_width: 2,
        writable: true,
    },
    RegisterDef {
        address: REG_GMA14_L,
        name: "GMA14_L",
        description: "Gamma 14 low [7:0]",
        default_value: 0x00,
        bit_width: 8,
        writable: true,
    },
    // Configuration registers
    RegisterDef {
        address: REG_CHANNEL_EN,
        name: "CHANNEL_EN",
        description: "Channel enable (each bit = 1 channel)",
        default_value: 0xFF,
        bit_width: 8,
        writable: true,
    },
    RegisterDef {
        address: REG_CHANNEL_SET,
        name: "CHANNEL_SET",
        description: "Frequency / structure selection",
        default_value: 0x00,
        bit_width: 8,
        writable: true,
    },
    RegisterDef {
        address: REG_DELAY1,
        name: "DELAY1",
        description: "VBK1/VGL/VSS1 delay",
        default_value: 0x00,
        bit_width: 8,
        writable: true,
    },
    RegisterDef {
        address: REG_DELAY2,
        name: "DELAY2",
        description: "AVDD/VGH/VCOM delay",
        default_value: 0x00,
        bit_width: 8,
        writable: true,
    },
    RegisterDef {
        address: REG_DISCHARGE,
        name: "DISCHARGE",
        description: "Discharge setting",
        default_value: 0x00,
        bit_width: 8,
        writable: true,
    },
    RegisterDef {
        address: REG_CONFIG1,
        name: "CONFIG1",
        description: "VSS1 discharge / soft start",
        default_value: 0x00,
        bit_width: 8,
        writable: true,
    },
    RegisterDef {
        address: REG_NTC_VGH_VGL,
        name: "NTC_VGH_VGL",
        description: "NTC VGH & VGL",
        default_value: 0x00,
        bit_width: 8,
        writable: true,
    },
    RegisterDef {
        address: REG_NTC_VCOM1,
        name: "NTC_VCOM1",
        description: "NTC VCOM1",
        default_value: 0x00,
        bit_width: 8,
        writable: true,
    },
    RegisterDef {
        address: REG_CONFIG2,
        name: "CONFIG2",
        description: "AVDD external drive / compensation",
        default_value: 0x00,
        bit_width: 8,
        writable: true,
    },
    RegisterDef {
        address: REG_CONFIG3,
        name: "CONFIG3",
        description: "EN / delay shutdown / XON",
        default_value: 0x00,
        bit_width: 8,
        writable: true,
    },
    RegisterDef {
        address: REG_DISCHARGE2,
        name: "DISCHARGE2",
        description: "VBK1/AVDD/VGH discharge disable",
        default_value: 0x00,
        bit_width: 8,
        writable: true,
    },
    RegisterDef {
        address: REG_VCOM2DAC,
        name: "VCOM2DAC",
        description: "VCOM2DAC output, 7-bit [7:1]",
        default_value: DEFAULT_REG_VCOM2DAC_VALUE,
        bit_width: 8,
        writable: true,
    },
    RegisterDef {
        address: REG_VCOM2DAC_EN,
        name: "VCOM2DAC_EN",
        description: "VCOM2DAC enable",
        default_value: 0x00,
        bit_width: 8,
        writable: true,
    },
    RegisterDef {
        address: REG_CONTROL,
        name: "CONTROL",
        description: "Control (0x80=WR_ALL, 0x40=WR_VCOM1, 0x01=RD_EE, 0x00=RD_DAC)",
        default_value: 0x00,
        bit_width: 8,
        writable: true,
    },
];

/// List of all valid PMIC register addresses (for iteration)
pub static PMIC_REG_ADDRESSES: &[u8] = &[
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1E, 0x1F,
    0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2A, 0x2B, 0x2C, 0x2D, 0x2E, 0x2F,
    0x30, 0x31, 0x32, 0x45, 0x46,
];

// ============================================================================
// VBK1 Look-up Table
// ============================================================================
// VBK1 mapping is non-linear:
//   0x00..0x0F → 1.80V base, step +0.05V  => 1.80, 1.85, ..., 2.55
//   0x10..0x1F → 2.60V base, step +0.05V  => 2.60, 2.65, ..., 3.35
//   0x20..0x2F → 0.80V base, step +0.05V  => 0.80, 0.85, ..., 1.55
//   0x30..0x3F → 1.60V base, step +0.05V  => 1.60, 1.65, ..., 2.35

/// VBK1 segment base voltages indexed by bits [5:4]
const VBK1_BASE: [f64; 4] = [1.80, 2.60, 0.80, 1.60];

// ============================================================================
// Voltage Decode Functions
// ============================================================================

/// Decode AVDD register value to voltage.
/// Formula: AVDD = 13.5 + (value & 0x3F) × 0.1  (V)
pub fn decode_avdd(value: u8) -> f64 {
    13.5 + (value & 0x3F) as f64 * 0.1
}

/// Encode AVDD voltage to register value.
/// Returns None if voltage is out of range [13.5, 19.8].
pub fn encode_avdd(voltage: f64) -> Option<u8> {
    if voltage < 13.5 || voltage > 19.8 {
        return None;
    }
    let code = ((voltage - 13.5) / 0.1).round() as u8;
    if code > 0x3F {
        return None;
    }
    Some(code)
}

/// Decode VBK1 register value to voltage (non-linear mapping).
pub fn decode_vbk1(value: u8) -> f64 {
    let seg = ((value >> 4) & 0x03) as usize;
    let offset = (value & 0x0F) as f64;
    VBK1_BASE[seg] + offset * 0.05
}

/// Encode VBK1 voltage to register value.
/// Returns None if voltage is out of range.
pub fn encode_vbk1(voltage: f64) -> Option<u8> {
    // Try each segment to find matching range
    for (seg, &base) in VBK1_BASE.iter().enumerate() {
        let max_v = base + 15.0 * 0.05;
        if voltage >= base && voltage <= max_v + 0.001 {
            let offset = ((voltage - base) / 0.05).round() as u8;
            if offset <= 0x0F {
                return Some(((seg as u8) << 4) | offset);
            }
        }
    }
    None
}

/// Decode VGH register value to voltage.
/// Formula: VGH = 20 + (value & 0x1F)  (V)
pub fn decode_vgh(value: u8) -> f64 {
    20.0 + (value & 0x1F) as f64
}

/// Encode VGH voltage to register value.
pub fn encode_vgh(voltage: f64) -> Option<u8> {
    if voltage < 20.0 || voltage > 45.0 {
        return None;
    }
    let code = (voltage - 20.0).round() as u8;
    if code > 0x1F {
        return None;
    }
    Some(code)
}

/// Decode VGL register value to voltage.
/// Formula: VGL = -3.0 - (value & 0x1F) × 0.5  (V)
pub fn decode_vgl(value: u8) -> f64 {
    -3.0 - (value & 0x1F) as f64 * 0.5
}

/// Encode VGL voltage to register value.
pub fn encode_vgl(voltage: f64) -> Option<u8> {
    if voltage > -3.0 || voltage < -18.0 {
        return None;
    }
    let code = ((-3.0 - voltage) / 0.5).round() as u8;
    if code > 0x1F {
        return None;
    }
    Some(code)
}

/// Decode VSS1 register value to voltage.
/// Formula: VSS1 = -3.0 - (value & 0x1F) × 0.5  (V)
pub fn decode_vss1(value: u8) -> f64 {
    -3.0 - (value & 0x1F) as f64 * 0.5
}

/// Encode VSS1 voltage to register value.
pub fn encode_vss1(voltage: f64) -> Option<u8> {
    if voltage > -3.0 || voltage < -16.0 {
        return None;
    }
    let code = ((-3.0 - voltage) / 0.5).round() as u8;
    if code > 0x1F {
        return None;
    }
    Some(code)
}

/// Decode HAVDD register value to voltage.
/// Formula: HAVDD = AVDD × (value & 0x7F) / 128
pub fn decode_havdd(value: u8, avdd: f64) -> f64 {
    avdd * (value & 0x7F) as f64 / 128.0
}

/// Encode HAVDD voltage to register value.
pub fn encode_havdd(voltage: f64, avdd: f64) -> Option<u8> {
    if avdd <= 0.0 {
        return None;
    }
    let code = (voltage * 128.0 / avdd).round() as u8;
    if code > 0x7F {
        return None;
    }
    Some(code)
}

/// Decode VCOM register value to voltage (VCOM1_NT, VCOM1_HT).
/// Data is in bits [7:1], bit0 is reserved.
/// Formula: VCOM = AVDD × ((value >> 1) & 0x7F) / 128
pub fn decode_vcom(value: u8, avdd: f64) -> f64 {
    avdd * ((value >> 1) & 0x7F) as f64 / 128.0
}

/// Encode VCOM voltage to register value (stored in [7:1]).
pub fn encode_vcom(voltage: f64, avdd: f64) -> Option<u8> {
    if avdd <= 0.0 {
        return None;
    }
    let code = (voltage * 128.0 / avdd).round() as u8;
    if code > 0x7F {
        return None;
    }
    Some(code << 1)
}

/// Decode VCOM limit register value to voltage (VCOM_MAX, VCOM_MIN).
/// Formula: VCOM_LIMIT = AVDD × (value & 0x7F) / 128
pub fn decode_vcom_limit(value: u8, avdd: f64) -> f64 {
    avdd * (value & 0x7F) as f64 / 128.0
}

/// Encode VCOM limit voltage to register value.
pub fn encode_vcom_limit(voltage: f64, avdd: f64) -> Option<u8> {
    if avdd <= 0.0 {
        return None;
    }
    let code = (voltage * 128.0 / avdd).round() as u8;
    if code > 0x7F {
        return None;
    }
    Some(code)
}

fn normalized_vcom_range(vcom_min: f64, vcom_max: f64) -> (f64, f64) {
    if vcom_min <= vcom_max {
        (vcom_min, vcom_max)
    } else {
        (vcom_max, vcom_min)
    }
}

/// Decode a VCOM output register (VCOM1/VCOM2DAC) using the datasheet range formula.
pub fn decode_vcom_output(value: u8, vcom_min: f64, vcom_max: f64) -> f64 {
    let (vcom_min, vcom_max) = normalized_vcom_range(vcom_min, vcom_max);
    let dac_code = ((value >> 1) & 0x7F) as f64;
    let step = (vcom_max - vcom_min) / 127.0;
    vcom_min + dac_code * step
}

/// Decode GAMMA register pair to voltage.
/// 10-bit value from high[1:0] + low[7:0].
/// Formula: GAMMA_V = AVDD × DAC_CODE / 1024
pub fn decode_gamma(high: u8, low: u8, avdd: f64) -> f64 {
    let dac_code = ((high & 0x03) as u16) * 256 + low as u16;
    avdd * dac_code as f64 / 1024.0
}

/// Encode GAMMA voltage to register pair (high, low).
pub fn encode_gamma(voltage: f64, avdd: f64) -> Option<(u8, u8)> {
    if avdd <= 0.0 {
        return None;
    }
    let dac_code = (voltage * 1024.0 / avdd).round() as u16;
    if dac_code > 0x3FF {
        return None;
    }
    let high = ((dac_code >> 8) & 0x03) as u8;
    let low = (dac_code & 0xFF) as u8;
    Some((high, low))
}

/// Decode VCOM2DAC register value to voltage.
/// Data is in bits [7:1] and follows the same range formula as VCOM1.
pub fn decode_vcom2dac(value: u8, vcom_min: f64, vcom_max: f64) -> f64 {
    decode_vcom_output(value, vcom_min, vcom_max)
}

/// Encode VCOM2DAC voltage to register value.
pub fn encode_vcom2dac(voltage: f64, vcom_min: f64, vcom_max: f64) -> Option<u8> {
    let (vcom_min, vcom_max) = normalized_vcom_range(vcom_min, vcom_max);
    let range = vcom_max - vcom_min;
    if range <= 0.0 {
        return None;
    }

    let clamped = voltage.clamp(vcom_min, vcom_max);
    let dac_code = ((clamped - vcom_min) / range * 127.0).round() as u8;
    Some(dac_code << 1)
}

// ============================================================================
// Helper: get register name by address
// ============================================================================

/// Look up register name by address from the PMIC register table.
pub fn get_register_name(addr: u8) -> &'static str {
    for reg in PMIC_REGISTERS {
        if reg.address == addr {
            return reg.name;
        }
    }
    "UNKNOWN"
}

/// Look up RegisterInfo by address.
pub fn get_register_info(addr: u8) -> Option<RegisterInfo> {
    for reg in PMIC_REGISTERS {
        if reg.address == addr {
            return Some(RegisterInfo {
                address: reg.address,
                name: reg.name.to_string(),
                description: reg.description.to_string(),
                default_value: reg.default_value,
                bit_width: reg.bit_width,
            });
        }
    }
    None
}

/// Decode a register value to its voltage representation.
/// Returns None if the register doesn't have a direct voltage mapping.
pub fn decode_register_voltage(
    addr: u8,
    value: u8,
    avdd_value: Option<u8>,
    vcom_min_value: Option<u8>,
    vcom_max_value: Option<u8>,
) -> Option<f64> {
    let avdd = decode_avdd(avdd_value.unwrap_or(DEFAULT_REG_AVDD_VALUE));
    let vcom_min = decode_vcom_limit(vcom_min_value.unwrap_or(DEFAULT_REG_VCOM_MIN_VALUE), avdd);
    let vcom_max = decode_vcom_limit(vcom_max_value.unwrap_or(DEFAULT_REG_VCOM_MAX_VALUE), avdd);

    match addr {
        REG_AVDD => Some(decode_avdd(value)),
        REG_VBK1 => Some(decode_vbk1(value)),
        REG_HAVDD => Some(decode_havdd(value, avdd)),
        REG_VGH_NT => Some(decode_vgh(value)),
        REG_VGH_LT => Some(decode_vgh(value)), // voltage part only
        REG_VGL_NT => Some(decode_vgl(value)),
        REG_VGL_LT_HT => Some(decode_vgl(value)), // voltage part only
        REG_VSS1 => Some(decode_vss1(value)),
        REG_VCOM1_NT => Some(decode_vcom_output(value, vcom_min, vcom_max)),
        REG_VCOM1_HT => Some(decode_vcom_output(value, vcom_min, vcom_max)),
        REG_VCOM_MAX => Some(decode_vcom_limit(value, avdd)),
        REG_VCOM_MIN => Some(decode_vcom_limit(value, avdd)),
        REG_VCOM2DAC => Some(decode_vcom2dac(value, vcom_min, vcom_max)),
        // Gamma registers: need pair, handled separately
        _ => None,
    }
}

/// Build a complete list of RegisterInfo for all PMIC registers.
pub fn get_all_register_info() -> Vec<RegisterInfo> {
    PMIC_REGISTERS
        .iter()
        .map(|reg| RegisterInfo {
            address: reg.address,
            name: reg.name.to_string(),
            description: reg.description.to_string(),
            default_value: reg.default_value,
            bit_width: reg.bit_width,
        })
        .collect()
}

// ============================================================================
// Fault Flag Decoding
// ============================================================================

/// Decoded fault flags from VCOM slave register 0x02
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaultFlags {
    pub raw: u8,
    pub otp: bool,
    pub vbk1: bool,
    pub avdd: bool,
    pub vgh: bool,
    pub vgl: bool,
    pub vss1: bool,
    pub havdd: bool,
}

impl FaultFlags {
    /// Decode fault flag register value
    pub fn from_raw(raw: u8) -> Self {
        Self {
            raw,
            otp: (raw & 0x40) != 0,
            vbk1: (raw & 0x20) != 0,
            avdd: (raw & 0x10) != 0,
            vgh: (raw & 0x08) != 0,
            vgl: (raw & 0x04) != 0,
            vss1: (raw & 0x02) != 0,
            havdd: (raw & 0x01) != 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_avdd_encode_decode() {
        assert!((decode_avdd(0x00) - 13.5).abs() < 0.001);
        assert!((decode_avdd(0x3F) - 19.8).abs() < 0.001);
        assert_eq!(encode_avdd(13.5), Some(0x00));
        assert_eq!(encode_avdd(15.0), Some(15));
        assert_eq!(encode_avdd(19.8), Some(0x3F));
        assert_eq!(encode_avdd(20.0), None);
    }

    #[test]
    fn test_vbk1_encode_decode() {
        // Segment 0: base 1.80V
        assert!((decode_vbk1(0x00) - 1.80).abs() < 0.001);
        assert!((decode_vbk1(0x0F) - 2.55).abs() < 0.001);
        // Segment 1: base 2.60V
        assert!((decode_vbk1(0x10) - 2.60).abs() < 0.001);
        // Segment 2: base 0.80V
        assert!((decode_vbk1(0x20) - 0.80).abs() < 0.001);
        // Segment 3: base 1.60V
        assert!((decode_vbk1(0x30) - 1.60).abs() < 0.001);

        assert_eq!(encode_vbk1(1.80), Some(0x00));
        assert_eq!(encode_vbk1(0.80), Some(0x20));
    }

    #[test]
    fn test_vgh_encode_decode() {
        assert!((decode_vgh(0x00) - 20.0).abs() < 0.001);
        assert!((decode_vgh(0x19) - 45.0).abs() < 0.001);
        assert_eq!(encode_vgh(20.0), Some(0x00));
        assert_eq!(encode_vgh(45.0), Some(25));
    }

    #[test]
    fn test_vgl_encode_decode() {
        assert!((decode_vgl(0x00) - (-3.0)).abs() < 0.001);
        assert!((decode_vgl(0x1E) - (-18.0)).abs() < 0.001);
        assert_eq!(encode_vgl(-3.0), Some(0x00));
        assert_eq!(encode_vgl(-18.0), Some(30));
    }

    #[test]
    fn test_gamma_encode_decode() {
        let avdd = 15.0;
        let (h, l) = encode_gamma(7.5, avdd).unwrap();
        let decoded = decode_gamma(h, l, avdd);
        assert!((decoded - 7.5).abs() < 0.02);
    }

    #[test]
    fn test_vcom_output_uses_vcom_limits() {
        let avdd = decode_avdd(DEFAULT_REG_AVDD_VALUE);
        let vcom_min = decode_vcom_limit(DEFAULT_REG_VCOM_MIN_VALUE, avdd);
        let vcom_max = decode_vcom_limit(DEFAULT_REG_VCOM_MAX_VALUE, avdd);
        let decoded = decode_vcom_output(DEFAULT_REG_VCOM1_NT_VALUE, vcom_min, vcom_max);
        let expected = vcom_min + (vcom_max - vcom_min) * 63.0 / 127.0;

        assert!((decoded - expected).abs() < 0.001);
        assert!(decoded >= vcom_min);
        assert!(decoded <= vcom_max);
    }
}
