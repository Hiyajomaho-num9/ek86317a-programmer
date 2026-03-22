pub const REG_AVDD: u8 = 0x00;
pub const REG_VBK1: u8 = 0x01;
pub const REG_HAVDD: u8 = 0x02;
pub const REG_VGH_NT: u8 = 0x03;
pub const REG_VGH_LT: u8 = 0x04;
pub const REG_VGL_NT: u8 = 0x05;
pub const REG_VGL_LT_HT: u8 = 0x06;
pub const REG_VSS1: u8 = 0x07;
pub const REG_VCOM_NT: u8 = 0x08;
pub const REG_VCOM_HT: u8 = 0x09;
pub const REG_VCOM_MAX: u8 = 0x0A;
pub const REG_VCOM_MIN: u8 = 0x0B;
pub const REG_CHANNEL_EN: u8 = 0x28;
pub const REG_CHANNEL_SET: u8 = 0x29;
pub const REG_DELAY1: u8 = 0x2A;
pub const REG_DELAY2: u8 = 0x2B;
pub const REG_DISCHARGE: u8 = 0x2C;
pub const REG_CONFIG1: u8 = 0x2D;
pub const REG_NTC_VGH_VGL: u8 = 0x2E;
pub const REG_NTC_VCOM: u8 = 0x2F;
pub const REG_CONFIG2: u8 = 0x30;
pub const REG_CONTROL: u8 = 0xFF;

pub const CTRL_WRITE_ALL_EEPROM: u8 = 0x80;
pub const CTRL_WRITE_VCOM1_EEPROM: u8 = 0x40;
pub const CTRL_READ_EEPROM: u8 = 0x01;
pub const CTRL_READ_DAC: u8 = 0x00;

pub const DEFAULT_REG_AVDD_VALUE: u8 = 0x29;
pub const DEFAULT_REG_VBK1_VALUE: u8 = 0x1E;
pub const DEFAULT_REG_HAVDD_VALUE: u8 = 0x40;
pub const DEFAULT_REG_VGH_NT_VALUE: u8 = 0x0A;
pub const DEFAULT_REG_VGH_LT_VALUE: u8 = 0x0F;
pub const DEFAULT_REG_VGL_NT_VALUE: u8 = 0x0A;
pub const DEFAULT_REG_VGL_LT_HT_VALUE: u8 = 0x12;
pub const DEFAULT_REG_VSS1_VALUE: u8 = 0x06;
pub const DEFAULT_REG_VCOM_NT_VALUE: u8 = 0x7E;
pub const DEFAULT_REG_VCOM_HT_VALUE: u8 = 0xBA;
pub const DEFAULT_REG_VCOM_MAX_VALUE: u8 = 0x3F;
pub const DEFAULT_REG_VCOM_MIN_VALUE: u8 = 0x26;

pub static PMIC_REG_ADDRESSES: &[u8] = &[
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B,
    0x0C, 0x0D, 0x0E, 0x0F, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17,
    0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1E, 0x1F, 0x20, 0x21, 0x22, 0x23,
    0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2A, 0x2B, 0x2C, 0x2D, 0x2E, 0x2F,
    0x30,
];

pub fn default_register_map() -> Vec<(u8, u8)> {
    let mut defaults = vec![
        (REG_AVDD, DEFAULT_REG_AVDD_VALUE),
        (REG_VBK1, DEFAULT_REG_VBK1_VALUE),
        (REG_HAVDD, DEFAULT_REG_HAVDD_VALUE),
        (REG_VGH_NT, DEFAULT_REG_VGH_NT_VALUE),
        (REG_VGH_LT, DEFAULT_REG_VGH_LT_VALUE),
        (REG_VGL_NT, DEFAULT_REG_VGL_NT_VALUE),
        (REG_VGL_LT_HT, DEFAULT_REG_VGL_LT_HT_VALUE),
        (REG_VSS1, DEFAULT_REG_VSS1_VALUE),
        (REG_VCOM_NT, DEFAULT_REG_VCOM_NT_VALUE),
        (REG_VCOM_HT, DEFAULT_REG_VCOM_HT_VALUE),
        (REG_VCOM_MAX, DEFAULT_REG_VCOM_MAX_VALUE),
        (REG_VCOM_MIN, DEFAULT_REG_VCOM_MIN_VALUE),
        (REG_CHANNEL_EN, 0xFF),
        (REG_CHANNEL_SET, 0x00),
        (REG_DELAY1, 0x00),
        (REG_DELAY2, 0x00),
        (REG_DISCHARGE, 0xFF),
        (REG_CONFIG1, 0x81),
        (REG_NTC_VGH_VGL, 0xD8),
        (REG_NTC_VCOM, 0x41),
        (REG_CONFIG2, 0x1A),
    ];

    for channel in 0..14u8 {
        let high = 0x0C + channel * 2;
        defaults.push((high, 0x02));
        defaults.push((high + 1, 0x00));
    }

    defaults.sort_by_key(|(addr, _)| *addr);
    defaults
}

pub fn decode_avdd(value: u8) -> f64 {
    13.5 + (value & 0x3F) as f64 * 0.1
}

pub fn decode_vbk1(value: u8) -> f64 {
    1.8 + (value & 0x1F) as f64 * 0.05
}

pub fn decode_havdd(value: u8, avdd: f64) -> f64 {
    avdd / 512.0 * (((value & 0x7F) as f64) + 192.0)
}

pub fn decode_vgh(value: u8) -> f64 {
    20.0 + (value & 0x1F) as f64
}

pub fn decode_vgl(value: u8) -> f64 {
    -3.0 - (value & 0x1F) as f64 * 0.5
}

pub fn decode_vss1(value: u8) -> f64 {
    -3.0 - (value & 0x1F) as f64 * 0.5
}

pub fn decode_vcom_limit(value: u8, avdd: f64) -> f64 {
    avdd * (value & 0x7F) as f64 / 128.0
}

pub fn decode_vcom_output(value: u8, vcom_min: f64, vcom_max: f64) -> f64 {
    let (min_v, max_v) = if vcom_min <= vcom_max {
        (vcom_min, vcom_max)
    } else {
        (vcom_max, vcom_min)
    };
    let step = (max_v - min_v) / 127.0;
    min_v + ((value >> 1) & 0x7F) as f64 * step
}

pub fn get_register_name(addr: u8) -> &'static str {
    match addr {
        REG_AVDD => "AVDD",
        REG_VBK1 => "VBK1",
        REG_HAVDD => "HAVDD",
        REG_VGH_NT => "VGH_NT",
        REG_VGH_LT => "VGH_LT",
        REG_VGL_NT => "VGL_NT",
        REG_VGL_LT_HT => "VGL_LT_HT",
        REG_VSS1 => "VSS1",
        REG_VCOM_NT => "VCOM_NT",
        REG_VCOM_HT => "VCOM_HT",
        REG_VCOM_MAX => "VCOM_MAX",
        REG_VCOM_MIN => "VCOM_MIN",
        REG_CHANNEL_EN => "CHANNEL_EN",
        REG_CHANNEL_SET => "CHANNEL_SET",
        REG_DELAY1 => "DELAY1",
        REG_DELAY2 => "DELAY2",
        REG_DISCHARGE => "DISCHARGE",
        REG_CONFIG1 => "CONFIG1",
        REG_NTC_VGH_VGL => "NTC_VGH_VGL",
        REG_NTC_VCOM => "NTC_VCOM",
        REG_CONFIG2 => "CONFIG2",
        REG_CONTROL => "CONTROL",
        0x0C..=0x27 => {
            if addr & 1 == 0 {
                "GMA_H"
            } else {
                "GMA_L"
            }
        }
        _ => "UNKNOWN",
    }
}

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
        REG_VGH_NT | REG_VGH_LT => Some(decode_vgh(value)),
        REG_VGL_NT | REG_VGL_LT_HT => Some(decode_vgl(value)),
        REG_VSS1 => Some(decode_vss1(value)),
        REG_VCOM_NT | REG_VCOM_HT => Some(decode_vcom_output(value, vcom_min, vcom_max)),
        REG_VCOM_MAX | REG_VCOM_MIN => Some(decode_vcom_limit(value, avdd)),
        _ => None,
    }
}
