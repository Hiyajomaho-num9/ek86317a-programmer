use std::thread;
use std::time::Duration;

use crate::bridges::I2cBus;
use crate::pmu::chip::{self, ChipModel, ChipSpec};
use crate::pmu::detect;

pub struct ChipDevice {
    bus: Box<dyn I2cBus>,
    spec: &'static ChipSpec,
}

impl ChipDevice {
    pub fn new(bus: Box<dyn I2cBus>, spec: &'static ChipSpec) -> Self {
        Self { bus, spec }
    }

    pub fn chip_model(&self) -> ChipModel {
        self.spec.model
    }

    pub fn spec(&self) -> &'static ChipSpec {
        self.spec
    }

    pub fn get_register_name(&self, addr: u8) -> &'static str {
        chip::get_register_name(self.spec.model, addr)
    }

    pub fn decode_register_voltage(
        &self,
        addr: u8,
        value: u8,
        avdd_value: Option<u8>,
        vcom_min_value: Option<u8>,
        vcom_max_value: Option<u8>,
        mode_value: Option<u8>,
    ) -> Option<f64> {
        chip::decode_register_voltage(
            self.spec.model,
            addr,
            value,
            avdd_value,
            vcom_min_value,
            vcom_max_value,
            mode_value,
        )
    }

    pub fn read_dac_register(&mut self, reg: u8) -> Result<u8, String> {
        self.bus.write(
            self.spec.pmic_addr,
            &[self.spec.control_reg, self.spec.ctrl_read_dac],
        )?;
        thread::sleep(Duration::from_millis(self.spec.read_delay_ms));

        let mut buf = [0u8; 1];
        self.bus.write_read(self.spec.pmic_addr, &[reg], &mut buf)?;
        Ok(buf[0])
    }

    pub fn write_dac_register(&mut self, reg: u8, value: u8) -> Result<(), String> {
        self.bus.write(self.spec.pmic_addr, &[reg, value])
    }

    pub fn write_dac_registers(&mut self, start_reg: u8, data: &[u8]) -> Result<(), String> {
        let mut payload = Vec::with_capacity(1 + data.len());
        payload.push(start_reg);
        payload.extend_from_slice(data);
        self.bus.write(self.spec.pmic_addr, &payload)
    }

    pub fn read_all_dac(&mut self) -> Result<Vec<(u8, u8)>, String> {
        self.bus.write(
            self.spec.pmic_addr,
            &[self.spec.control_reg, self.spec.ctrl_read_dac],
        )?;
        thread::sleep(Duration::from_millis(self.spec.read_delay_ms));

        let mut results = Vec::new();
        for &addr in chip::register_addresses(self.spec.model) {
            if addr == self.spec.control_reg {
                continue;
            }
            let mut buf = [0u8; 1];
            self.bus
                .write_read(self.spec.pmic_addr, &[addr], &mut buf)?;
            results.push((addr, buf[0]));
        }
        Ok(results)
    }

    pub fn read_eeprom_register(&mut self, reg: u8) -> Result<u8, String> {
        self.bus.write(
            self.spec.pmic_addr,
            &[self.spec.control_reg, self.spec.ctrl_read_eeprom],
        )?;
        thread::sleep(Duration::from_millis(self.spec.read_delay_ms));

        let mut buf = [0u8; 1];
        self.bus.write_read(self.spec.pmic_addr, &[reg], &mut buf)?;
        Ok(buf[0])
    }

    pub fn read_all_eeprom(&mut self) -> Result<Vec<(u8, u8)>, String> {
        self.bus.write(
            self.spec.pmic_addr,
            &[self.spec.control_reg, self.spec.ctrl_read_eeprom],
        )?;
        thread::sleep(Duration::from_millis(self.spec.read_delay_ms));

        let mut results = Vec::new();
        for &addr in chip::register_addresses(self.spec.model) {
            if addr == self.spec.control_reg {
                continue;
            }
            let mut buf = [0u8; 1];
            self.bus
                .write_read(self.spec.pmic_addr, &[addr], &mut buf)?;
            results.push((addr, buf[0]));
        }
        Ok(results)
    }

    pub fn write_all_to_eeprom(&mut self) -> Result<(), String> {
        self.bus.write(
            self.spec.pmic_addr,
            &[self.spec.control_reg, self.spec.ctrl_write_all_eeprom],
        )?;
        thread::sleep(Duration::from_millis(self.spec.write_delay_ms));
        Ok(())
    }

    pub fn write_vcom1_to_eeprom(&mut self) -> Result<(), String> {
        self.bus.write(
            self.spec.pmic_addr,
            &[self.spec.control_reg, self.spec.ctrl_write_vcom_eeprom],
        )?;
        thread::sleep(Duration::from_millis(self.spec.write_delay_ms));
        Ok(())
    }

    pub fn read_fault_flags(&mut self) -> Result<u8, String> {
        let vcom_addr = self.spec.vcom_addr.ok_or_else(|| {
            format!(
                "{} does not expose a separate VCOM slave",
                self.spec.display_name
            )
        })?;
        let fault_reg = self
            .spec
            .vcom_fault_reg
            .ok_or_else(|| format!("{} does not expose fault flags", self.spec.display_name))?;

        let mut buf = [0u8; 1];
        self.bus.write_read(vcom_addr, &[fault_reg], &mut buf)?;
        Ok(buf[0])
    }

    /// Registers absent from a sparse/short image are explicitly uncovered;
    /// they are neither compared nor implicitly treated as matching.
    pub fn firmware_missing(&self, data: &[u8]) -> Vec<u8> {
        chip::register_addresses(self.spec.model)
            .iter()
            .copied()
            .filter(|&addr| addr != self.spec.control_reg && addr as usize >= data.len())
            .collect()
    }

    pub fn firmware_total(&self) -> usize {
        chip::register_addresses(self.spec.model)
            .iter()
            .filter(|&&addr| addr != self.spec.control_reg)
            .count()
    }

    pub fn verify_firmware(&mut self, data: &[u8]) -> Result<Vec<(u8, u8, u8)>, String> {
        let eeprom = self.read_all_eeprom()?;
        let mut mismatches = Vec::new();

        for &(addr, actual) in &eeprom {
            let idx = addr as usize;
            if idx < data.len() {
                let expected = data[idx];
                if expected != actual {
                    mismatches.push((addr, expected, actual));
                }
            }
        }

        Ok(mismatches)
    }

    pub fn verify_all(
        &mut self,
        data: &[u8],
    ) -> Result<crate::pmu::commands::BankMismatches, String> {
        let dac = self.read_all_dac()?;
        let mut dac_mismatches = Vec::new();
        for &(addr, actual) in &dac {
            let idx = addr as usize;
            if idx < data.len() {
                let expected = data[idx];
                if expected != actual {
                    dac_mismatches.push((addr, expected, actual));
                }
            }
        }

        let eeprom = self.read_all_eeprom()?;
        let mut eeprom_mismatches = Vec::new();
        for &(addr, actual) in &eeprom {
            let idx = addr as usize;
            if idx < data.len() {
                let expected = data[idx];
                if expected != actual {
                    eeprom_mismatches.push((addr, expected, actual));
                }
            }
        }

        Ok((dac_mismatches, eeprom_mismatches))
    }

    pub fn write_all_dac_registers(&mut self, registers: &[(u8, u8)]) -> Result<usize, String> {
        let mut count = 0;
        for &(addr, value) in registers {
            self.write_dac_register(addr, value)?;
            count += 1;
        }
        Ok(count)
    }

    pub fn probe(&mut self) -> Result<(bool, Option<bool>), String> {
        if let Err(e) = self.bus.bus_recovery() {
            log::warn!("Bus recovery failed (non-fatal): {}", e);
        }

        // Select the DAC bank first: an unselected page can read back as a flat
        // 0xFF, which is indistinguishable from an unloaded bus in the presence
        // comparison below. This is the same bank select the register reads use.
        if let Err(e) = self.bus.write(
            self.spec.pmic_addr,
            &[self.spec.control_reg, self.spec.ctrl_read_dac],
        ) {
            log::debug!("DAC bank select before probe failed (ignored): {}", e);
        }
        thread::sleep(Duration::from_millis(self.spec.read_delay_ms));

        // Seeing anything other than a successful read used to be enough to
        // report "PMU present". On CH341 that read also succeeds on an empty
        // bus (the DLL never reports NACK), so presence now goes through the
        // floating-baseline comparison in `detect`.
        let pmic_regs = probe_registers(
            self.spec.avdd_reg,
            self.spec.vcom_max_reg,
            self.spec.vcom_min_reg,
        );
        let pmic_ok = detect::device_responds(self.bus.as_mut(), self.spec.pmic_addr, &pmic_regs);

        // Probe the optional VCOM slave (0x74 / E8-E9h) with a register read
        // when the chip exposes one. EK86317A and iML8947K carry a separate
        // VCOM slave and report Some(found/missing); LP6281 has no such slave
        // (vcom_addr = None) and returns None, which the UI renders as N/A.
        let vcom_detected = self.spec.vcom_addr.map(|addr| {
            let fault_reg = self.spec.vcom_fault_reg;
            let output_reg = self.spec.vcom_output_reg;
            let control_reg = self.spec.vcom_control_reg;
            let regs = probe_registers(
                fault_reg.or(output_reg).or(control_reg).unwrap_or(0x00),
                output_reg,
                control_reg,
            );
            detect::device_responds(self.bus.as_mut(), addr, &regs)
        });

        Ok((pmic_ok, vcom_detected))
    }
}

/// Collect the registers used as presence evidence, keeping order and dropping
/// duplicates so a probe never reads the same register twice.
fn probe_registers(first: u8, second: Option<u8>, third: Option<u8>) -> Vec<u8> {
    let mut registers = Vec::with_capacity(3);
    for reg in [Some(first), second, third].into_iter().flatten() {
        if !registers.contains(&reg) {
            registers.push(reg);
        }
    }
    registers
}

#[cfg(test)]
mod tests {
    use super::ChipDevice;
    use crate::bridges::ftdi::MockI2cBus;
    use crate::bridges::I2cBus;
    use crate::pmu::chip::{self, spec_for_model, ChipModel};

    #[test]
    fn probe_reads_pmic_and_vcom_slave_for_ek86317a() {
        let bus = Box::new(MockI2cBus::new(ChipModel::Ek86317a));
        let mut device = ChipDevice::new(bus, spec_for_model(ChipModel::Ek86317a));

        // EK86317A exposes a VCOM slave at 0x74 (E8/E9h); both must be probed.
        assert_eq!(device.probe().unwrap(), (true, Some(true)));
    }

    #[test]
    fn probe_reads_pmic_and_vcom_slave_for_iml8947k() {
        let bus = Box::new(MockI2cBus::new(ChipModel::Iml8947k));
        let mut device = ChipDevice::new(bus, spec_for_model(ChipModel::Iml8947k));

        // iML8947K also exposes a VCOM slave at 0x74 (E8/E9h).
        assert_eq!(device.probe().unwrap(), (true, Some(true)));
    }

    #[test]
    fn probe_skips_vcom_slave_for_lp6281() {
        let bus = Box::new(MockI2cBus::new(ChipModel::Lp6281));
        let mut device = ChipDevice::new(bus, spec_for_model(ChipModel::Lp6281));

        // LP6281 has no separate VCOM slave (no E8/E9h); vcom_detected stays
        // None and the UI renders it as N/A rather than "missing".
        assert_eq!(device.probe().unwrap(), (true, None));
    }

    #[test]
    fn short_default_image_compares_present_registers_and_reports_missing() {
        let model = ChipModel::Ek86317a;
        let bus = Box::new(MockI2cBus::new(model));
        let mut device = ChipDevice::new(bus, spec_for_model(model));
        let mut data = vec![0u8; 0x10];
        for (addr, value) in chip::default_register_map(model) {
            if (addr as usize) < data.len() {
                data[addr as usize] = value;
            }
        }

        let missing = device.firmware_missing(&data);
        let expected_missing = chip::register_addresses(model)
            .iter()
            .copied()
            .filter(|&addr| addr != device.spec().control_reg && addr as usize >= data.len())
            .collect::<Vec<_>>();
        assert_eq!(missing, expected_missing);
        assert!(missing.contains(&0x46));
        assert!(!missing.contains(&device.spec().control_reg));
        assert_eq!(device.firmware_total() - missing.len(), 16);
        assert!(device.verify_firmware(&data).unwrap().is_empty());

        let (dac, eeprom) = device.verify_all(&data).unwrap();
        assert!(dac.is_empty());
        assert!(eeprom.is_empty());
    }

    #[test]
    fn sparse_image_with_only_first_avdd_byte_has_explicit_coverage() {
        let model = ChipModel::Iml8947k;
        let bus = Box::new(MockI2cBus::new(model));
        let mut device = ChipDevice::new(bus, spec_for_model(model));
        let avdd = chip::default_register_map(model)[0];
        let data = vec![avdd.1];

        let missing = device.firmware_missing(&data);
        assert_eq!(device.firmware_total() - missing.len(), 1);
        assert!(!missing.contains(&avdd.0));
        assert!(device.verify_firmware(&data).unwrap().is_empty());
        let (dac, eeprom) = device.verify_all(&data).unwrap();
        assert!(dac.is_empty());
        assert!(eeprom.is_empty());
    }

    #[test]
    fn short_image_does_not_hide_real_bus_read_errors() {
        struct FailingReadBus;
        impl I2cBus for FailingReadBus {
            fn write(&mut self, _addr: u8, _data: &[u8]) -> Result<(), String> {
                Ok(())
            }

            fn read(&mut self, _addr: u8, _buf: &mut [u8]) -> Result<(), String> {
                Err("synthetic read failure".to_string())
            }

            fn write_read(
                &mut self,
                _addr: u8,
                _write_data: &[u8],
                _read_buf: &mut [u8],
            ) -> Result<(), String> {
                Err("synthetic read failure".to_string())
            }
        }

        let mut device = ChipDevice::new(
            Box::new(FailingReadBus),
            spec_for_model(ChipModel::Ek86317a),
        );
        assert_eq!(
            device.verify_firmware(&[0x29]).unwrap_err(),
            "synthetic read failure"
        );
    }

    #[test]
    fn verify_still_reports_mismatches_in_a_complete_image() {
        let model = ChipModel::Ek86317a;
        let bus = Box::new(MockI2cBus::new(model));
        let mut device = ChipDevice::new(bus, spec_for_model(model));
        let mut data = vec![0u8; 0x47];
        for (addr, value) in chip::default_register_map(model) {
            if let Some(entry) = data.get_mut(addr as usize) {
                *entry = value;
            }
        }
        let actual = data[0x00];
        let expected = actual ^ 0xFF;
        data[0x00] = expected;

        let mismatches = device.verify_firmware(&data).unwrap();
        assert_eq!(mismatches, vec![(0x00, expected, actual)]);

        let (dac, eeprom) = device.verify_all(&data).unwrap();
        assert_eq!(dac, mismatches);
        assert_eq!(eeprom, mismatches);
    }

    /// Mimics the CH341 failure mode this probe has to survive: every transfer
    /// reports success, no NACK is ever signalled, and reads come back as bus
    /// pull-up data (0xFF) — the unloaded bus that used to look like a PMU.
    struct EmptyBusWithoutNackReporting;

    impl I2cBus for EmptyBusWithoutNackReporting {
        fn write(&mut self, _addr: u8, _data: &[u8]) -> Result<(), String> {
            Ok(())
        }

        fn read(&mut self, _addr: u8, buf: &mut [u8]) -> Result<(), String> {
            for byte in buf.iter_mut() {
                *byte = 0xFF;
            }
            Ok(())
        }

        fn write_read(
            &mut self,
            _addr: u8,
            _write_data: &[u8],
            read_buf: &mut [u8],
        ) -> Result<(), String> {
            for byte in read_buf.iter_mut() {
                *byte = 0xFF;
            }
            Ok(())
        }

        fn reports_nack(&self) -> bool {
            false
        }
    }

    /// Same bus, but a PMU answers at 0x20 with a real register value.
    struct LoadedBusWithoutNackReporting;

    impl I2cBus for LoadedBusWithoutNackReporting {
        fn write(&mut self, _addr: u8, _data: &[u8]) -> Result<(), String> {
            Ok(())
        }

        fn read(&mut self, _addr: u8, buf: &mut [u8]) -> Result<(), String> {
            for byte in buf.iter_mut() {
                *byte = 0x00;
            }
            Ok(())
        }

        fn write_read(
            &mut self,
            addr: u8,
            _write_data: &[u8],
            read_buf: &mut [u8],
        ) -> Result<(), String> {
            for byte in read_buf.iter_mut() {
                *byte = if addr == 0x20 { 0x1C } else { 0xFF };
            }
            Ok(())
        }

        fn reports_nack(&self) -> bool {
            false
        }
    }

    #[test]
    fn probe_reports_no_pmic_on_a_bus_that_never_reports_a_nack() {
        // Reported symptom: an unloaded CH341A claimed a PMU at 0x20. "The read
        // returned Ok" must not be enough when every read is the idle level.
        let mut device = ChipDevice::new(
            Box::new(EmptyBusWithoutNackReporting),
            spec_for_model(ChipModel::Ek86317a),
        );

        assert_eq!(device.probe().unwrap(), (false, Some(false)));
    }

    #[test]
    fn probe_still_finds_a_pmic_on_a_bus_that_never_reports_a_nack() {
        // The other half of the contract: a real slave drives the bus, so the
        // probe must not turn into a permanent "missing".
        let mut device = ChipDevice::new(
            Box::new(LoadedBusWithoutNackReporting),
            spec_for_model(ChipModel::Ek86317a),
        );

        assert_eq!(device.probe().unwrap(), (true, Some(false)));
    }
}
