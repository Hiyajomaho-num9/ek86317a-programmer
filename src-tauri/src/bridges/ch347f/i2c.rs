use crate::bridges::I2cBus;

pub struct Ch347I2cBus {
    bus_recovery_logged: bool,
    #[cfg(all(feature = "ch347f", target_os = "windows"))]
    index: u32,
    #[cfg(all(feature = "ch347f", target_os = "windows"))]
    api: windows_api::Ch347Api,
    #[cfg(all(feature = "ch347f", target_os = "linux"))]
    fd: i32,
    #[cfg(all(feature = "ch347f", target_os = "linux"))]
    path: String,
}

impl Ch347I2cBus {
    pub fn list_devices() -> Result<Vec<(u32, String)>, String> {
        #[cfg(all(feature = "ch347f", target_os = "windows"))]
        {
            return windows_api::list_devices();
        }

        #[cfg(all(feature = "ch347f", target_os = "linux"))]
        {
            return linux_api::list_devices();
        }

        #[cfg(all(
            feature = "ch347f",
            not(any(target_os = "windows", target_os = "linux"))
        ))]
        {
            Err("CH347F support is currently implemented for Windows and Linux only".to_string())
        }

        #[cfg(not(feature = "ch347f"))]
        {
            Err("CH347F support not compiled in. Rebuild with --features ch347f".to_string())
        }
    }

    pub fn open(index: u32, clock_hz: u32) -> Result<Self, String> {
        #[cfg(all(feature = "ch347f", target_os = "windows"))]
        {
            let api = windows_api::Ch347Api::load()?;
            api.open_i2c(index, clock_hz)?;
            return Ok(Self {
                bus_recovery_logged: false,
                index,
                api,
            });
        }

        #[cfg(all(feature = "ch347f", target_os = "linux"))]
        {
            let (fd, path) = linux_api::open_i2c(index, clock_hz)?;
            return Ok(Self {
                bus_recovery_logged: false,
                fd,
                path,
            });
        }

        #[cfg(all(
            feature = "ch347f",
            not(any(target_os = "windows", target_os = "linux"))
        ))]
        {
            let _ = (index, clock_hz);
            Err("CH347F support is currently implemented for Windows and Linux only".to_string())
        }

        #[cfg(not(feature = "ch347f"))]
        {
            let _ = (index, clock_hz);
            Err("CH347F support not compiled in. Rebuild with --features ch347f".to_string())
        }
    }

    fn stream_write_only(&mut self, write_buf: &mut [u8]) -> Result<(), String> {
        #[cfg(all(feature = "ch347f", target_os = "windows"))]
        {
            return self
                .api
                .stream_i2c(self.index, write_buf, None)
                .map(|_| ())
                .map_err(|e| format!("CH347 write failed on device #{}: {}", self.index, e));
        }

        #[cfg(all(feature = "ch347f", target_os = "linux"))]
        {
            return linux_api::stream_i2c(self.fd, write_buf, None)
                .map(|_| ())
                .map_err(|e| format!("CH347 write failed on {}: {}", self.path, e));
        }

        #[cfg(any(
            not(feature = "ch347f"),
            all(
                feature = "ch347f",
                not(any(target_os = "windows", target_os = "linux"))
            )
        ))]
        {
            let _ = write_buf;
            Err("CH347F I2C write is unavailable on this build".to_string())
        }
    }

    fn stream_read_only(&mut self, addr: u8, read_buf: &mut [u8]) -> Result<(), String> {
        #[cfg(all(feature = "ch347f", target_os = "windows"))]
        {
            let mut write_buf = [encode_read_addr(addr)];
            return self
                .api
                .stream_i2c(self.index, &mut write_buf, Some(read_buf))
                .map(|_| ())
                .map_err(|e| format!("CH347 read failed on device #{}: {}", self.index, e));
        }

        #[cfg(all(feature = "ch347f", target_os = "linux"))]
        {
            let mut write_buf = [encode_read_addr(addr)];
            return linux_api::stream_i2c(self.fd, &mut write_buf, Some(read_buf))
                .map(|_| ())
                .map_err(|e| format!("CH347 read failed on {}: {}", self.path, e));
        }

        #[cfg(any(
            not(feature = "ch347f"),
            all(
                feature = "ch347f",
                not(any(target_os = "windows", target_os = "linux"))
            )
        ))]
        {
            let _ = (addr, read_buf);
            Err("CH347F I2C read is unavailable on this build".to_string())
        }
    }
}

impl I2cBus for Ch347I2cBus {
    fn write(&mut self, addr: u8, data: &[u8]) -> Result<(), String> {
        let mut write_buf = Vec::with_capacity(data.len() + 1);
        write_buf.push(encode_write_addr(addr));
        write_buf.extend_from_slice(data);
        self.stream_write_only(&mut write_buf)
    }

    fn read(&mut self, addr: u8, buf: &mut [u8]) -> Result<(), String> {
        if buf.is_empty() {
            return Ok(());
        }
        self.stream_read_only(addr, buf)
    }

    fn write_read(
        &mut self,
        addr: u8,
        write_data: &[u8],
        read_buf: &mut [u8],
    ) -> Result<(), String> {
        if !write_data.is_empty() {
            let mut write_buf = Vec::with_capacity(write_data.len() + 1);
            write_buf.push(encode_write_addr(addr));
            write_buf.extend_from_slice(write_data);
            self.stream_write_only(&mut write_buf)?;
        }

        if !read_buf.is_empty() {
            self.stream_read_only(addr, read_buf)?;
        }

        Ok(())
    }

    fn bus_recovery(&mut self) -> Result<(), String> {
        if !self.bus_recovery_logged {
            log::warn!(
                "CH347F bus recovery is currently a no-op; continuing without manual SCL recovery pulses"
            );
            self.bus_recovery_logged = true;
        }
        Ok(())
    }
}

#[cfg(all(feature = "ch347f", target_os = "windows"))]
impl Drop for Ch347I2cBus {
    fn drop(&mut self) {
        self.api.close_device(self.index);
    }
}

#[cfg(all(feature = "ch347f", target_os = "linux"))]
impl Drop for Ch347I2cBus {
    fn drop(&mut self) {
        linux_api::close_device(self.fd);
    }
}

fn encode_write_addr(addr: u8) -> u8 {
    addr << 1
}

fn encode_read_addr(addr: u8) -> u8 {
    (addr << 1) | 0x01
}

fn i2c_mode_for_clock(clock_hz: u32) -> u32 {
    match clock_hz {
        0..=35_000 => 0x00,
        35_001..=75_000 => 0x04,
        75_001..=150_000 => 0x01,
        150_001..=300_000 => 0x05,
        300_001..=600_000 => 0x02,
        600_001..=875_000 => 0x03,
        _ => 0x06,
    }
}

#[cfg(all(feature = "ch347f", target_os = "windows"))]
mod windows_api {
    use std::ffi::c_void;

    use libloading::Library;

    const MAX_PATH: usize = 260;
    const DEVICE_SLOTS: u32 = 16;
    const INVALID_HANDLE_VALUE: isize = -1isize;

    type Handle = isize;
    type Bool = i32;
    type OpenDeviceFn = unsafe extern "system" fn(u32) -> Handle;
    type CloseDeviceFn = unsafe extern "system" fn(u32) -> Bool;
    type GetDeviceInfoFn = unsafe extern "system" fn(u32, *mut DeviceInfo) -> Bool;
    type I2cSetFn = unsafe extern "system" fn(u32, u32) -> Bool;
    type StreamI2cFn = unsafe extern "system" fn(u32, u32, *mut c_void, u32, *mut c_void) -> Bool;
    type StreamI2cRetAckFn =
        unsafe extern "system" fn(u32, u32, *mut c_void, u32, *mut c_void, *mut u32) -> Bool;

    pub(super) struct Ch347Api {
        _library: Library,
        open_device: OpenDeviceFn,
        close_device: CloseDeviceFn,
        get_device_info: GetDeviceInfoFn,
        i2c_set: I2cSetFn,
        stream_i2c: StreamI2cFn,
        stream_i2c_ret_ack: Option<StreamI2cRetAckFn>,
    }

    #[repr(C)]
    struct DeviceInfo {
        index: u8,
        device_path: [u8; MAX_PATH],
        usb_class: u8,
        func_type: u8,
        device_id: [u8; 64],
        chip_mode: u8,
        device_handle: Handle,
        bulk_out_ep_max_size: u16,
        bulk_in_ep_max_size: u16,
        usb_speed_type: u8,
        ch347_if_num: u8,
        data_up_ep: u8,
        data_down_ep: u8,
        product_string: [u8; 64],
        manufacturer_string: [u8; 64],
        write_timeout: u32,
        read_timeout: u32,
        func_desc_str: [u8; 64],
        fw_ver: u8,
    }

    impl Default for DeviceInfo {
        fn default() -> Self {
            Self {
                index: 0,
                device_path: [0; MAX_PATH],
                usb_class: 0,
                func_type: 0,
                device_id: [0; 64],
                chip_mode: 0,
                device_handle: 0,
                bulk_out_ep_max_size: 0,
                bulk_in_ep_max_size: 0,
                usb_speed_type: 0,
                ch347_if_num: 0,
                data_up_ep: 0,
                data_down_ep: 0,
                product_string: [0; 64],
                manufacturer_string: [0; 64],
                write_timeout: 0,
                read_timeout: 0,
                func_desc_str: [0; 64],
                fw_ver: 0,
            }
        }
    }

    impl DeviceInfo {
        fn product(&self) -> String {
            decode_c_string(&self.product_string)
        }

        fn manufacturer(&self) -> String {
            decode_c_string(&self.manufacturer_string)
        }

        fn function_desc(&self) -> String {
            decode_c_string(&self.func_desc_str)
        }
    }

    impl Ch347Api {
        pub(super) fn load() -> Result<Self, String> {
            let library = unsafe { Library::new("CH347DLLA64.dll") }
                .map_err(|err| format!("Unable to load CH347DLLA64.dll: {}", err))?;
            Self::from_library(library)
        }

        fn from_library(library: Library) -> Result<Self, String> {
            unsafe {
                let open_device = load_symbol::<OpenDeviceFn>(&library, b"CH347OpenDevice\0")?;
                let close_device = load_symbol::<CloseDeviceFn>(&library, b"CH347CloseDevice\0")?;
                let get_device_info =
                    load_symbol::<GetDeviceInfoFn>(&library, b"CH347GetDeviceInfor\0")?;
                let i2c_set = load_symbol::<I2cSetFn>(&library, b"CH347I2C_Set\0")?;
                let stream_i2c = load_symbol::<StreamI2cFn>(&library, b"CH347StreamI2C\0")?;
                let stream_i2c_ret_ack =
                    load_optional_symbol::<StreamI2cRetAckFn>(&library, b"CH347StreamI2C_RetACK\0");

                Ok(Self {
                    _library: library,
                    open_device,
                    close_device,
                    get_device_info,
                    i2c_set,
                    stream_i2c,
                    stream_i2c_ret_ack,
                })
            }
        }

        pub(super) fn open_i2c(&self, index: u32, clock_hz: u32) -> Result<(), String> {
            let handle = unsafe { (self.open_device)(index) };
            if handle == INVALID_HANDLE_VALUE {
                return Err(format!("Failed to open CH347F device #{}", index));
            }

            let mode = super::i2c_mode_for_clock(clock_hz);
            if unsafe { (self.i2c_set)(index, mode) } == 0 {
                self.close_device(index);
                return Err(format!("Failed to set CH347F I2C clock to {}Hz", clock_hz));
            }

            Ok(())
        }

        pub(super) fn close_device(&self, index: u32) {
            unsafe {
                let _ = (self.close_device)(index);
            }
        }

        pub(super) fn stream_i2c(
            &self,
            index: u32,
            write_buf: &mut [u8],
            read_buf: Option<&mut [u8]>,
        ) -> Result<Option<u32>, String> {
            let read_len = read_buf.as_ref().map(|buf| buf.len()).unwrap_or(0) as u32;
            let read_ptr = read_buf
                .map(|buf| buf.as_mut_ptr() as *mut c_void)
                .unwrap_or(std::ptr::null_mut());

            if let Some(stream_i2c_ret_ack) = self.stream_i2c_ret_ack {
                let mut ack_count = 0u32;
                let ok = unsafe {
                    stream_i2c_ret_ack(
                        index,
                        write_buf.len() as u32,
                        write_buf.as_mut_ptr() as *mut c_void,
                        read_len,
                        read_ptr,
                        &mut ack_count,
                    )
                };
                if ok == 0 {
                    return Err("CH347StreamI2C_RetACK returned failure".to_string());
                }
                return Ok(Some(ack_count));
            }

            let ok = unsafe {
                (self.stream_i2c)(
                    index,
                    write_buf.len() as u32,
                    write_buf.as_mut_ptr() as *mut c_void,
                    read_len,
                    read_ptr,
                )
            };
            if ok == 0 {
                return Err("CH347StreamI2C returned failure".to_string());
            }

            Ok(None)
        }
    }

    pub(super) fn list_devices() -> Result<Vec<(u32, String)>, String> {
        let api = Ch347Api::load()?;
        let mut devices = Vec::new();

        for index in 0..DEVICE_SLOTS {
            let handle = unsafe { (api.open_device)(index) };
            if handle == INVALID_HANDLE_VALUE {
                continue;
            }

            let mut info = DeviceInfo::default();
            let description = if unsafe { (api.get_device_info)(index, &mut info) } != 0 {
                format_device_description(&info)
            } else {
                "CH347 bridge".to_string()
            };

            api.close_device(index);
            devices.push((index, description));
        }

        Ok(devices)
    }

    fn decode_c_string(raw: &[u8]) -> String {
        let nul = raw.iter().position(|b| *b == 0).unwrap_or(raw.len());
        String::from_utf8_lossy(&raw[..nul]).trim().to_string()
    }

    fn format_device_description(info: &DeviceInfo) -> String {
        let mut parts = Vec::new();
        let manufacturer = info.manufacturer();
        let product = info.product();
        let function_desc = info.function_desc();

        if !manufacturer.is_empty() {
            parts.push(manufacturer);
        }
        if !product.is_empty() {
            parts.push(product);
        }
        if !function_desc.is_empty() {
            parts.push(function_desc);
        }

        if parts.is_empty() {
            "CH347 bridge".to_string()
        } else {
            parts.join(" / ")
        }
    }

    unsafe fn load_symbol<T: Copy>(library: &Library, name: &[u8]) -> Result<T, String> {
        library.get::<T>(name).map(|symbol| *symbol).map_err(|err| {
            format!(
                "Failed to load symbol {}: {}",
                String::from_utf8_lossy(name),
                err
            )
        })
    }

    unsafe fn load_optional_symbol<T: Copy>(library: &Library, name: &[u8]) -> Option<T> {
        library.get::<T>(name).ok().map(|symbol| *symbol)
    }
}

#[cfg(all(feature = "ch347f", target_os = "linux"))]
mod linux_api {
    use std::ffi::{c_char, c_int, c_uchar, c_void, CString};
    use std::fs;
    use std::path::Path;

    const CH34X_DEVICE_DIR: &str = "/dev";
    const CH34X_DEVICE_PREFIX: &str = "ch34x_pis";
    const DEFAULT_TIMEOUT_MS: u32 = 2_000;

    pub(super) fn list_devices() -> Result<Vec<(u32, String)>, String> {
        let mut devices = Vec::new();

        let entries = fs::read_dir(CH34X_DEVICE_DIR)
            .map_err(|err| format!("Failed to read {}: {}", CH34X_DEVICE_DIR, err))?;

        for entry in entries {
            let entry = entry.map_err(|err| format!("Failed to inspect /dev entry: {}", err))?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let Some(index) = parse_device_index(&name) else {
                continue;
            };

            let path = device_path_from_index(index);
            let fd = match open_device_path(&path) {
                Ok(fd) => fd,
                Err(err) => {
                    log::warn!("Skipping {} while scanning CH347F devices: {}", path, err);
                    continue;
                }
            };

            let description = format_device_description(fd, &path);
            close_device(fd);
            devices.push((index, description));
        }

        devices.sort_by_key(|(index, _)| *index);
        Ok(devices)
    }

    pub(super) fn open_i2c(index: u32, clock_hz: u32) -> Result<(i32, String), String> {
        let path = device_path_from_index(index);
        let fd = open_device_path(&path)?;

        if !unsafe { CH34xSetTimeout(fd, DEFAULT_TIMEOUT_MS, DEFAULT_TIMEOUT_MS) } {
            close_device(fd);
            return Err(format!("Failed to set CH347F timeouts on {}", path));
        }

        let mode = super::i2c_mode_for_clock(clock_hz) as c_int;
        if !unsafe { CH347I2C_Set(fd, mode) } {
            close_device(fd);
            return Err(format!(
                "Failed to set CH347F I2C clock to {}Hz on {}",
                clock_hz, path
            ));
        }

        Ok((fd, path))
    }

    pub(super) fn close_device(fd: i32) {
        unsafe {
            let _ = CH347CloseDevice(fd);
        }
    }

    pub(super) fn stream_i2c(
        fd: i32,
        write_buf: &mut [u8],
        read_buf: Option<&mut [u8]>,
    ) -> Result<Option<u32>, String> {
        let read_len = read_buf.as_ref().map(|buf| buf.len()).unwrap_or(0) as c_int;
        let read_ptr = read_buf
            .map(|buf| buf.as_mut_ptr() as *mut c_void)
            .unwrap_or(std::ptr::null_mut());
        let mut ack_count: c_int = 0;

        let ok = unsafe {
            CH347StreamI2C_RetAck(
                fd,
                write_buf.len() as c_int,
                write_buf.as_mut_ptr() as *mut c_void,
                read_len,
                read_ptr,
                &mut ack_count,
            )
        };
        if !ok {
            return Err("CH347StreamI2C_RetAck returned failure".to_string());
        }

        Ok(Some(ack_count as u32))
    }

    fn format_device_description(fd: i32, path: &str) -> String {
        let chip_name = detect_chip_name(fd).unwrap_or("CH34x bridge".to_string());
        let driver_version =
            read_driver_version(fd).unwrap_or_else(|| "driver unknown".to_string());
        let device_id = read_device_id(fd)
            .map(|id| format!("VID {:04X} PID {:04X}", id & 0xFFFF, id >> 16))
            .unwrap_or_else(|| "VID/PID unknown".to_string());

        format!(
            "{} / {} / {} / {}",
            chip_name, path, device_id, driver_version
        )
    }

    fn detect_chip_name(fd: i32) -> Option<String> {
        let mut chip_type: c_int = 0;
        if unsafe { CH34x_GetChipType(fd, &mut chip_type) } {
            Some(
                match chip_type {
                    0 => "CH341 bridge",
                    1 => "CH347T bridge",
                    2 => "CH347F bridge",
                    3 => "CH339W bridge",
                    4 => "CH346C bridge",
                    _ => "CH34x bridge",
                }
                .to_string(),
            )
        } else {
            None
        }
    }

    fn read_driver_version(fd: i32) -> Option<String> {
        let mut buf = [0u8; 64];
        if unsafe { CH34x_GetDriverVersion(fd, buf.as_mut_ptr()) } {
            let nul = buf.iter().position(|b| *b == 0).unwrap_or(buf.len());
            let text = String::from_utf8_lossy(&buf[..nul]).trim().to_string();
            if text.is_empty() {
                None
            } else {
                Some(format!("driver {}", text))
            }
        } else {
            None
        }
    }

    fn read_device_id(fd: i32) -> Option<u32> {
        let mut device_id = 0u32;
        if unsafe { CH34X_GetDeviceID(fd, &mut device_id) } {
            Some(device_id)
        } else {
            None
        }
    }

    fn device_path_from_index(index: u32) -> String {
        format!("{}/{}{}", CH34X_DEVICE_DIR, CH34X_DEVICE_PREFIX, index)
    }

    fn open_device_path(path: &str) -> Result<i32, String> {
        if !Path::new(path).exists() {
            return Err(format!("{} does not exist", path));
        }

        let c_path = CString::new(path.as_bytes())
            .map_err(|_| format!("{} contains an unexpected NUL byte", path))?;
        let fd = unsafe { CH347OpenDevice(c_path.as_ptr()) };
        if fd < 0 {
            return Err(format!("CH347OpenDevice failed for {}", path));
        }
        Ok(fd)
    }

    fn parse_device_index(name: &str) -> Option<u32> {
        let suffix = name.strip_prefix(CH34X_DEVICE_PREFIX)?;
        suffix.parse().ok()
    }

    unsafe extern "C" {
        fn CH347OpenDevice(pathname: *const c_char) -> c_int;
        fn CH347CloseDevice(fd: c_int) -> bool;
        fn CH34xSetTimeout(fd: c_int, write_timeout: u32, read_timeout: u32) -> bool;
        fn CH34x_GetDriverVersion(fd: c_int, version: *mut c_uchar) -> bool;
        fn CH34x_GetChipType(fd: c_int, chip_type: *mut c_int) -> bool;
        fn CH34X_GetDeviceID(fd: c_int, id: *mut u32) -> bool;
        fn CH347I2C_Set(fd: c_int, mode: c_int) -> bool;
        fn CH347StreamI2C_RetAck(
            fd: c_int,
            write_length: c_int,
            write_buffer: *mut c_void,
            read_length: c_int,
            read_buffer: *mut c_void,
            ret_ack: *mut c_int,
        ) -> bool;
    }
}

#[cfg(test)]
mod tests {
    use super::{encode_read_addr, encode_write_addr, i2c_mode_for_clock};

    #[test]
    fn encodes_i2c_addresses() {
        assert_eq!(encode_write_addr(0x20), 0x40);
        assert_eq!(encode_read_addr(0x20), 0x41);
    }

    #[test]
    fn maps_common_clock_rates() {
        assert_eq!(i2c_mode_for_clock(20_000), 0x00);
        assert_eq!(i2c_mode_for_clock(50_000), 0x04);
        assert_eq!(i2c_mode_for_clock(100_000), 0x01);
        assert_eq!(i2c_mode_for_clock(200_000), 0x05);
        assert_eq!(i2c_mode_for_clock(400_000), 0x02);
        assert_eq!(i2c_mode_for_clock(750_000), 0x03);
        assert_eq!(i2c_mode_for_clock(1_000_000), 0x06);
    }
}
