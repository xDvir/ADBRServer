#[derive(Clone, Copy)]
pub enum AdbDeviceType {
    Usb,
    Emulator,
}

impl AdbDeviceType {
    pub fn is_usb(&self) -> bool {
        matches!(self, AdbDeviceType::Usb)
    }

    pub fn is_emulator(&self) -> bool {
        matches!(self, AdbDeviceType::Emulator)
    }
}