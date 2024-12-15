#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum AdbDeviceTransport {
    Any,
    EmulatorAny,
    Emulator(String),
    UsbAny,
    Usb(String),
}

#[allow(dead_code)]
impl AdbDeviceTransport {
    pub fn usb(serial: &str) -> Self {
        AdbDeviceTransport::Usb(String::from(serial))
    }
    pub fn emulator(serial: &str) -> Self {
        AdbDeviceTransport::Usb(String::from(serial))
    }

    pub fn get_serial(&self) -> Option<&String> {
        match self {
            AdbDeviceTransport::Usb(serial) => Some(serial),
            AdbDeviceTransport::Emulator(serial) => Some(serial),
            _ => None,
        }
    }
}
