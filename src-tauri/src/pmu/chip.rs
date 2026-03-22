use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChipModel {
    Ek86317a,
    Iml8947k,
    Lp6281,
}

impl ChipModel {
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Ek86317a => "EK86317A",
            Self::Iml8947k => "iML8947K",
            Self::Lp6281 => "LP6281",
        }
    }

    pub fn storage_prefix(self) -> &'static str {
        match self {
            Self::Ek86317a => "ek86317a",
            Self::Iml8947k => "iml8947k",
            Self::Lp6281 => "lp6281",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ChipSpec {
    pub model: ChipModel,
    pub display_name: &'static str,
    pub pmic_addr: u8,
    pub vcom_addr: Option<u8>,
    pub control_reg: u8,
    pub ctrl_write_all_eeprom: u8,
    pub ctrl_write_vcom_eeprom: u8,
    pub ctrl_read_eeprom: u8,
    pub ctrl_read_dac: u8,
    pub read_delay_ms: u64,
    pub write_delay_ms: u64,
    pub has_vcom_slave: bool,
    pub supports_fault_flags: bool,
    pub supports_vcom2dac: bool,
    pub supports_mnt_mode: bool,
    pub avdd_reg: u8,
    pub vcom_min_reg: Option<u8>,
    pub vcom_max_reg: Option<u8>,
    pub mode_reg: Option<u8>,
    pub vcom_control_reg: Option<u8>,
    pub vcom_output_reg: Option<u8>,
    pub vcom_fault_reg: Option<u8>,
    pub vcom_enable_bit: Option<u8>,
    pub vcom_load_bit: Option<u8>,
    pub vcom_write_bit: Option<u8>,
    pub pmic_vcom_register: Option<u8>,
}

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

pub fn spec_for_model(model: ChipModel) -> &'static ChipSpec {
    match model {
        ChipModel::Ek86317a => &crate::pmu::ek86317a::SPEC,
        ChipModel::Iml8947k => &crate::pmu::iml8947k::SPEC,
        ChipModel::Lp6281 => &crate::pmu::lp6281::SPEC,
    }
}

pub fn register_addresses(model: ChipModel) -> &'static [u8] {
    match model {
        ChipModel::Ek86317a => crate::pmu::ek86317a::registers::PMIC_REG_ADDRESSES,
        ChipModel::Iml8947k => crate::pmu::iml8947k::registers::PMIC_REG_ADDRESSES,
        ChipModel::Lp6281 => crate::pmu::lp6281::registers::PMIC_REG_ADDRESSES,
    }
}

pub fn default_register_map(model: ChipModel) -> Vec<(u8, u8)> {
    match model {
        ChipModel::Ek86317a => crate::pmu::ek86317a::registers::PMIC_REGISTERS
            .iter()
            .map(|reg| (reg.address, reg.default_value))
            .collect(),
        ChipModel::Iml8947k => crate::pmu::iml8947k::registers::default_register_map(),
        ChipModel::Lp6281 => crate::pmu::lp6281::registers::default_register_map(),
    }
}

pub fn get_register_name(model: ChipModel, addr: u8) -> &'static str {
    match model {
        ChipModel::Ek86317a => crate::pmu::ek86317a::registers::get_register_name(addr),
        ChipModel::Iml8947k => crate::pmu::iml8947k::registers::get_register_name(addr),
        ChipModel::Lp6281 => crate::pmu::lp6281::registers::get_register_name(addr),
    }
}

pub fn decode_register_voltage(
    model: ChipModel,
    addr: u8,
    value: u8,
    avdd_value: Option<u8>,
    vcom_min_value: Option<u8>,
    vcom_max_value: Option<u8>,
    mode_value: Option<u8>,
) -> Option<f64> {
    match model {
        ChipModel::Ek86317a => crate::pmu::ek86317a::registers::decode_register_voltage(
            addr,
            value,
            avdd_value,
            vcom_min_value,
            vcom_max_value,
        ),
        ChipModel::Iml8947k => crate::pmu::iml8947k::registers::decode_register_voltage(
            addr,
            value,
            avdd_value,
            vcom_min_value,
            vcom_max_value,
            mode_value,
        ),
        ChipModel::Lp6281 => crate::pmu::lp6281::registers::decode_register_voltage(
            addr,
            value,
            avdd_value,
            vcom_min_value,
            vcom_max_value,
        ),
    }
}

pub fn decode_fault_flags(model: ChipModel, raw: u8) -> FaultFlags {
    match model {
        ChipModel::Ek86317a => {
            let flags = crate::pmu::ek86317a::registers::FaultFlags::from_raw(raw);
            FaultFlags {
                raw,
                otp: flags.otp,
                vbk1: flags.vbk1,
                avdd: flags.avdd,
                vgh: flags.vgh,
                vgl: flags.vgl,
                vss1: flags.vss1,
                havdd: flags.havdd,
            }
        }
        ChipModel::Iml8947k => FaultFlags {
            raw,
            otp: (raw & 0x80) != 0,
            vbk1: (raw & 0x20) != 0,
            avdd: (raw & 0x10) != 0,
            vgh: (raw & 0x08) != 0,
            vgl: (raw & 0x04) != 0,
            vss1: false,
            havdd: (raw & 0x01) != 0,
        },
        ChipModel::Lp6281 => FaultFlags {
            raw,
            otp: false,
            vbk1: false,
            avdd: false,
            vgh: false,
            vgl: false,
            vss1: false,
            havdd: false,
        },
    }
}
