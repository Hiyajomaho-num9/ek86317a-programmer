pub mod registers;

use crate::pmu::chip::{ChipModel, ChipSpec};

pub const SPEC: ChipSpec = ChipSpec {
    model: ChipModel::Iml8947k,
    display_name: "iML8947K",
    pmic_addr: 0x20,
    vcom_addr: Some(0x74),
    control_reg: registers::REG_CONTROL,
    ctrl_write_all_eeprom: registers::CTRL_WRITE_ALL_EEPROM,
    ctrl_write_vcom_eeprom: registers::CTRL_WRITE_VCOM1_EEPROM,
    ctrl_read_eeprom: registers::CTRL_READ_EEPROM,
    ctrl_read_dac: registers::CTRL_READ_DAC,
    read_delay_ms: 5,
    write_delay_ms: 200,
    has_vcom_slave: true,
    supports_fault_flags: true,
    supports_vcom2dac: true,
    supports_mnt_mode: true,
    avdd_reg: registers::REG_AVDD,
    vcom_min_reg: Some(registers::REG_VCOM_MIN),
    vcom_max_reg: Some(registers::REG_VCOM_MAX),
    mode_reg: Some(registers::REG_CONFIG4),
    vcom_control_reg: Some(registers::VCOM_REG_CONTROL),
    vcom_output_reg: Some(registers::VCOM_REG_VCOM1),
    vcom_fault_reg: Some(registers::VCOM_REG_FAULT),
    vcom_enable_bit: Some(1),
    vcom_load_bit: Some(4),
    vcom_write_bit: Some(3),
    pmic_vcom_register: Some(registers::REG_VCOM1_NT),
};
