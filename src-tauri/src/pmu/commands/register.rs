//! Register read/write commands

use tauri::State;

use super::{DeviceState, RegisterData};

#[derive(Clone, Copy)]
struct DecodeContext {
    avdd_value: Option<u8>,
    vcom_min_value: Option<u8>,
    vcom_max_value: Option<u8>,
    mode_value: Option<u8>,
}

fn needs_vcom_context(addr: u8) -> bool {
    matches!(addr, 0x08 | 0x09 | 0x0A | 0x0B | 0x45)
}

fn build_decode_context_from_regs(
    device: &crate::pmu::device::ChipDevice,
    regs: &[(u8, u8)],
) -> DecodeContext {
    let spec = device.spec();
    let mut ctx = DecodeContext {
        avdd_value: None,
        vcom_min_value: None,
        vcom_max_value: None,
        mode_value: None,
    };

    for &(addr, value) in regs {
        if addr == spec.avdd_reg {
            ctx.avdd_value = Some(value);
        }
        if spec.vcom_min_reg == Some(addr) {
            ctx.vcom_min_value = Some(value);
        }
        if spec.vcom_max_reg == Some(addr) {
            ctx.vcom_max_value = Some(value);
        }
        if spec.mode_reg == Some(addr) {
            ctx.mode_value = Some(value);
        }
    }

    ctx
}

fn read_decode_context_for_single(
    device: &mut crate::pmu::device::ChipDevice,
    addr: u8,
    value: u8,
) -> DecodeContext {
    let spec = *device.spec();

    let avdd_value = if addr == spec.avdd_reg {
        Some(value)
    } else {
        device.read_dac_register(spec.avdd_reg).ok()
    };

    let vcom_min_value = if needs_vcom_context(addr) {
        spec.vcom_min_reg.and_then(|reg| {
            if addr == reg {
                Some(value)
            } else {
                device.read_dac_register(reg).ok()
            }
        })
    } else {
        None
    };

    let vcom_max_value = if needs_vcom_context(addr) {
        spec.vcom_max_reg.and_then(|reg| {
            if addr == reg {
                Some(value)
            } else {
                device.read_dac_register(reg).ok()
            }
        })
    } else {
        None
    };

    let mode_value = spec.mode_reg.and_then(|reg| {
        if addr == reg {
            Some(value)
        } else {
            device.read_dac_register(reg).ok()
        }
    });

    DecodeContext {
        avdd_value,
        vcom_min_value,
        vcom_max_value,
        mode_value,
    }
}

fn build_register_data(
    device: &crate::pmu::device::ChipDevice,
    addr: u8,
    value: u8,
    ctx: DecodeContext,
) -> RegisterData {
    RegisterData {
        address: addr,
        value,
        name: device.get_register_name(addr).to_string(),
        voltage: device.decode_register_voltage(
            addr,
            value,
            ctx.avdd_value,
            ctx.vcom_min_value,
            ctx.vcom_max_value,
            ctx.mode_value,
        ),
    }
}

#[tauri::command]
pub async fn read_dac_register(
    state: State<'_, DeviceState>,
    addr: u8,
) -> Result<RegisterData, String> {
    log::info!("Reading DAC register 0x{:02X}", addr);

    super::with_device(&state, move |device| {
        let value = device.read_dac_register(addr)?;
        let ctx = read_decode_context_for_single(device, addr, value);
        Ok(build_register_data(device, addr, value, ctx))
    })
    .await
}

#[tauri::command]
pub async fn write_dac_register(
    state: State<'_, DeviceState>,
    addr: u8,
    value: u8,
) -> Result<(), String> {
    log::info!("Writing DAC register 0x{:02X} = 0x{:02X}", addr, value);

    super::with_device(&state, move |device| {
        if addr == device.spec().control_reg {
            return Err("Control register 0xFF is reserved for dedicated EEPROM commands".to_string());
        }
        device.write_dac_register(addr, value)
    })
    .await
}

#[tauri::command]
pub async fn read_all_dac(state: State<'_, DeviceState>) -> Result<Vec<RegisterData>, String> {
    log::info!("Reading all DAC registers");

    super::with_device(&state, move |device| {
        let regs = device.read_all_dac()?;
        let ctx = build_decode_context_from_regs(device, &regs);
        Ok(regs
            .iter()
            .map(|(addr, value)| build_register_data(device, *addr, *value, ctx))
            .collect())
    })
    .await
}

#[tauri::command]
pub async fn read_all_eeprom(state: State<'_, DeviceState>) -> Result<Vec<RegisterData>, String> {
    log::info!("Reading all EEPROM registers");

    super::with_device(&state, move |device| {
        let regs = device.read_all_eeprom()?;
        let ctx = build_decode_context_from_regs(device, &regs);
        Ok(regs
            .iter()
            .map(|(addr, value)| build_register_data(device, *addr, *value, ctx))
            .collect())
    })
    .await
}
