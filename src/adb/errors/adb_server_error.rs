use std::error::Error;
use std::fmt;
use crate::adb::errors::adb_io_error::AdbIoError;

#[derive(Debug)]
pub enum AdbServerError {
    IOError(AdbIoError),
    NoAvailableDevices(),
    DeviceNotFound(String),
    MultipleDeviceDetected(),
    NoTransportSelected(),
    SyncError(String),
    RequestError(String),
    UnexpectedError(String),
}

impl fmt::Display for AdbServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AdbServerError::IOError(err) => write!(f, "Communication failed: {}", err),
            AdbServerError::NoAvailableDevices() => write!(f, "No available devices"),
            AdbServerError::DeviceNotFound(msg) => write!(f, "device '{}' not found", msg),
            AdbServerError::MultipleDeviceDetected() => write!(f, "Multiple devices detected"),
            AdbServerError::NoTransportSelected() => write!(f, "No transport selected"),
            AdbServerError::SyncError(msg) => write!(f, "Sync operation failed: {}", msg),
            AdbServerError::RequestError(msg) => write!(f, "Invalid client request: {}", msg),
            AdbServerError::UnexpectedError(msg) => write!(f, "Unexpected error: {}", msg),
        }
    }
}

impl Error for AdbServerError {}
