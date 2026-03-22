//! Register read/write commands

use tauri::State;

use crate::ek86317a::registers;

use super::{DeviceState, RegisterData};

#[derive(Clone, Copy)]
struct DecodeContext {
    avdd_value: Option<u8>,
    vcom_min_value: Option<u8>,
    vcom_max_value: Option<u8>,
}

fn needs_vcom_context(addr: u8) -> bool {
    matches!(
        addr,
        registers::REG_VCOM1_NT
            | registers::REG_VCOM1_HT
            | registers::REG_VCOM_MAX
            | registers::REG_VCOM_MIN
            | registers::REG_VCOM2DAC
    )
}

fn build_decode_context_from_regs(regs: &[(u8, u8)]) -> DecodeContext {
    let mut ctx = DecodeContext {
        avdd_value: None,
        vcom_min_value: None,
        vcom_max_value: None,
    };

    for &(addr, value) in regs {
        match addr {
            registers::REG_AVDD => ctx.avdd_value = Some(value),
            registers::REG_VCOM_MIN => ctx.vcom_min_value = Some(value),
            registers::REG_VCOM_MAX => ctx.vcom_max_value = Some(value),
            _ => {}
        }
    }

    ctx
}

fn read_decode_context_for_single(
    device: &mut crate::ek86317a::Ek86317a,
    addr: u8,
    value: u8,
) -> DecodeContext {
    let avdd_value = if addr == registers::REG_AVDD {
        Some(value)
    } else {
        device.read_dac_register(registers::REG_AVDD).ok()
    };

    let vcom_min_value = if needs_vcom_context(addr) {
        if addr == registers::REG_VCOM_MIN {
            Some(value)
        } else {
            device.read_dac_register(registers::REG_VCOM_MIN).ok()
        }
    } else {
        None
    };

    let vcom_max_value = if needs_vcom_context(addr) {
        if addr == registers::REG_VCOM_MAX {
            Some(value)
        } else {
            device.read_dac_register(registers::REG_VCOM_MAX).ok()
        }
    } else {
        None
    };

    DecodeContext {
        avdd_value,
        vcom_min_value,
        vcom_max_value,
    }
}

/// Helper: build RegisterData from address and value
fn build_register_data(addr: u8, value: u8, ctx: DecodeContext) -> RegisterData {
    let name = registers::get_register_name(addr).to_string();
    let voltage = registers::decode_register_voltage(
        addr,
        value,
        ctx.avdd_value,
        ctx.vcom_min_value,
        ctx.vcom_max_value,
    );
    RegisterData {
        address: addr,
        value,
        name,
        voltage,
    }
}

/// Read a single DAC register and return enriched data.
#[tauri::command]
pub async fn read_dac_register(
    state: State<'_, DeviceState>,
    addr: u8,
) -> Result<RegisterData, String> {
    log::info!("Reading DAC register 0x{:02X}", addr);

    super::with_device(&state, move |device| {
        let value = device.read_dac_register(addr)?;
        let ctx = read_decode_context_for_single(device, addr, value);
        Ok(build_register_data(addr, value, ctx))
    })
    .await
}

/// Write a single DAC register.
#[tauri::command]
pub async fn write_dac_register(
    state: State<'_, DeviceState>,
    addr: u8,
    value: u8,
) -> Result<(), String> {
    log::info!("Writing DAC register 0x{:02X} = 0x{:02X}", addr, value);

    if addr == registers::REG_CONTROL {
        return Err("Control register 0xFF is reserved for dedicated EEPROM commands".to_string());
    }

    super::with_device(&state, move |device| device.write_dac_register(addr, value)).await
}

/// Read all DAC registers and return enriched data.
#[tauri::command]
pub async fn read_all_dac(state: State<'_, DeviceState>) -> Result<Vec<RegisterData>, String> {
    log::info!("Reading all DAC registers");

    super::with_device(&state, move |device| {
        let regs = device.read_all_dac()?;
        let ctx = build_decode_context_from_regs(&regs);
        let result: Vec<RegisterData> = regs
            .iter()
            .map(|(addr, value)| build_register_data(*addr, *value, ctx))
            .collect();

        Ok(result)
    })
    .await
}

/// Read all EEPROM registers and return enriched data.
#[tauri::command]
pub async fn read_all_eeprom(state: State<'_, DeviceState>) -> Result<Vec<RegisterData>, String> {
    log::info!("Reading all EEPROM registers");

    super::with_device(&state, move |device| {
        let regs = device.read_all_eeprom()?;
        let ctx = build_decode_context_from_regs(&regs);
        let result: Vec<RegisterData> = regs
            .iter()
            .map(|(addr, value)| build_register_data(*addr, *value, ctx))
            .collect();

        Ok(result)
    })
    .await
}
