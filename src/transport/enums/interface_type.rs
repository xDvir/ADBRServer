#[derive(Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum InterfaceType {
    AndroidUsb(u8, u8, u8),
    AndroidTcp(u8, u8, u8),
}

#[allow(dead_code)]
impl InterfaceType {
    pub fn android_usb() -> Self {
        InterfaceType::AndroidUsb(0xff, 0x42, 0x01)
    }

    pub fn is_android_usb_device(interface_type: InterfaceType) -> bool {
        match interface_type {
            InterfaceType::AndroidUsb(_, _, _) => true,
            _ => false,
        }
    }

    pub fn is_android_tcp_device(interface_type: InterfaceType) -> bool {
        match interface_type {
            InterfaceType::AndroidTcp(_, _, _) => true,
            _ => false,
        }
    }
}
