use std::fmt;

#[derive(Clone)]#[derive(Debug)]
pub enum AdbDeviceStatus {
    Available,
    Offline(String),
    Unauthorized
}

impl fmt::Display for AdbDeviceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AdbDeviceStatus::Available => write!(f, "device"),
            AdbDeviceStatus::Offline(msg) => write!(f, "offline {}",msg),
            AdbDeviceStatus::Unauthorized => write!(f, "unauthorized"),
        }
    }
}

impl PartialEq for AdbDeviceStatus {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Available, Self::Available) => true,
            (Self::Offline(_), Self::Offline(_)) => true,
            (Self::Unauthorized, Self::Unauthorized) => true,
            _ => false,
        }
    }
}