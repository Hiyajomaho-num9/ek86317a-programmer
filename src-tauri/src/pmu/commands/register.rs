//! Register read/write commands

use tauri::State;

use crate::pmu::chip::{ChipModel, ChipSpec};

use super::{DeviceState, RegisterData};

#[derive(Clone, Copy)]
struct DecodeContext {
    avdd_value: Option<u8>,
    vcom_min_value: Option<u8>,
    vcom_max_value: Option<u8>,
    mode_value: Option<u8>,
}

#[derive(Clone, Copy)]
struct DecodeRequirements {
    avdd: bool,
    vcom_limits: bool,
    mode: bool,
}

fn decode_requirements(model: ChipModel, addr: u8) -> DecodeRequirements {
    let vcom_limits = matches!(addr, 0x08 | 0x09 | 0x45);
    let avdd = addr == 0x02
        || matches!(addr, 0x0A | 0x0B)
        || (model != ChipModel::Iml8947k && vcom_limits);
    let mode = model == ChipModel::Iml8947k && (addr == 0x00 || avdd);
    DecodeRequirements {
        avdd,
        vcom_limits,
        mode,
    }
}

fn decode_context_is_complete(spec: &ChipSpec, addr: u8, ctx: DecodeContext) -> bool {
    let needs = decode_requirements(spec.model, addr);
    (!needs.avdd || ctx.avdd_value.is_some())
        && (!needs.mode || ctx.mode_value.is_some())
        && (!needs.vcom_limits
            || (spec.vcom_min_reg.is_none() || ctx.vcom_min_value.is_some())
                && (spec.vcom_max_reg.is_none() || ctx.vcom_max_value.is_some()))
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

    let needs = decode_requirements(spec.model, addr);
    let avdd_value = if !needs.avdd {
        None
    } else if addr == spec.avdd_reg {
        Some(value)
    } else {
        device.read_dac_register(spec.avdd_reg).ok()
    };

    let vcom_min_value = if needs.vcom_limits {
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

    let vcom_max_value = if needs.vcom_limits {
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

    let mode_value = spec.mode_reg.filter(|_| needs.mode).and_then(|reg| {
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
    let spec = device.spec();
    let complete = decode_context_is_complete(spec, addr, ctx);
    RegisterData {
        address: addr,
        value,
        name: device.get_register_name(addr).to_string(),
        // Preserve the raw reading, but never display a voltage calculated from
        // defaults after a dependent register failed to read.
        voltage: if complete {
            device.decode_register_voltage(
                addr,
                value,
                ctx.avdd_value,
                ctx.vcom_min_value,
                ctx.vcom_max_value,
                ctx.mode_value,
            )
        } else {
            None
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{build_register_data, DecodeContext};
    use crate::bridges::ftdi::MockI2cBus;
    use crate::bridges::I2cBus;
    use crate::pmu::chip::{self, spec_for_model, ChipModel};
    use crate::pmu::device::ChipDevice;

    fn device(model: ChipModel) -> ChipDevice {
        ChipDevice::new(Box::new(MockI2cBus::new(model)), spec_for_model(model))
    }

    fn incomplete_context() -> DecodeContext {
        DecodeContext {
            avdd_value: None,
            vcom_min_value: None,
            vcom_max_value: None,
            mode_value: None,
        }
    }

    #[test]
    fn avdd_keeps_raw_derived_voltage_without_unrelated_context() {
        let device = device(ChipModel::Ek86317a);
        let value = chip::default_register_map(ChipModel::Ek86317a)[0].1;
        let data =
            build_register_data(&device, device.spec().avdd_reg, value, incomplete_context());

        assert_eq!(data.value, value);
        assert_eq!(data.voltage, Some(13.5 + (value & 0x3F) as f64 * 0.1));
    }

    #[test]
    fn independent_vbk1_voltage_does_not_need_decode_context() {
        let device = device(ChipModel::Ek86317a);
        let data = build_register_data(&device, 0x01, 0x00, incomplete_context());
        assert_eq!(data.voltage, Some(1.8));
    }

    #[test]
    fn missing_vcom_context_degrades_voltage_but_retains_raw_value() {
        let device = device(ChipModel::Ek86317a);
        let data = build_register_data(&device, 0x08, 0x5A, incomplete_context());
        assert_eq!(data.value, 0x5A);
        assert_eq!(data.voltage, None);
    }

    #[test]
    fn avdd_read_failure_degrades_dependent_voltage_only() {
        struct FailingContextBus;
        impl I2cBus for FailingContextBus {
            fn write(&mut self, _addr: u8, _data: &[u8]) -> Result<(), String> {
                Ok(())
            }

            fn read(&mut self, _addr: u8, _buf: &mut [u8]) -> Result<(), String> {
                Ok(())
            }

            fn write_read(
                &mut self,
                _addr: u8,
                _write_data: &[u8],
                _read_buf: &mut [u8],
            ) -> Result<(), String> {
                Err("synthetic context read failure".to_string())
            }
        }

        let mut device = ChipDevice::new(
            Box::new(FailingContextBus),
            spec_for_model(ChipModel::Ek86317a),
        );
        let ctx = super::read_decode_context_for_single(&mut device, 0x02, 0x40);
        let data = build_register_data(&device, 0x02, 0x40, ctx);
        assert_eq!(data.value, 0x40);
        assert_eq!(data.voltage, None);
    }

    #[test]
    fn iml_mode_context_is_required_only_for_mode_dependent_registers() {
        let device = device(ChipModel::Iml8947k);
        assert_eq!(
            build_register_data(&device, 0x00, 0x29, incomplete_context()).voltage,
            None
        );
        assert_eq!(
            build_register_data(&device, 0x01, 0x00, incomplete_context()).voltage,
            Some(1.8)
        );
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
            return Err(
                "Control register 0xFF is reserved for dedicated EEPROM commands".to_string(),
            );
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
