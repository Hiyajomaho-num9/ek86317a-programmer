pub mod registers;

use crate::pmu::chip::{ChipModel, ChipSpec};

pub const SPEC: ChipSpec = ChipSpec {
    model: ChipModel::Lp6281,
    display_name: "LP6281",
    pmic_addr: 0x20,
    vcom_addr: None,
    control_reg: registers::REG_CONTROL,
    ctrl_write_all_eeprom: registers::CTRL_WRITE_ALL_EEPROM,
    ctrl_write_vcom_eeprom: registers::CTRL_WRITE_VCOM1_EEPROM,
    ctrl_read_eeprom: registers::CTRL_READ_EEPROM,
    ctrl_read_dac: registers::CTRL_READ_DAC,
    read_delay_ms: 5,
    write_delay_ms: 200,
    has_vcom_slave: false,
    supports_fault_flags: false,
    supports_vcom2dac: false,
    supports_mnt_mode: false,
    avdd_reg: registers::REG_AVDD,
    vcom_min_reg: Some(registers::REG_VCOM_MIN),
    vcom_max_reg: Some(registers::REG_VCOM_MAX),
    mode_reg: None,
    vcom_control_reg: None,
    vcom_output_reg: None,
    vcom_fault_reg: None,
    vcom_enable_bit: None,
    vcom_load_bit: None,
    vcom_write_bit: None,
    pmic_vcom_register: Some(registers::REG_VCOM_NT),
};
