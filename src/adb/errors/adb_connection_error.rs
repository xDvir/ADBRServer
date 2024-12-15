#[derive(Debug)]
pub enum AdbConnectionError {
    Timeout,
    DeviceNotAvailable(String),
    Unauthorized(String),
    ConnectionCloseError(String),
    CommunicationError(String),
    PortForwardSetupFailed(String),
    PortReverseSetupFailed(String),
    SyncError(String),
    UnexpectedError(String),

}

impl std::fmt::Display for AdbConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdbConnectionError::Timeout => write!(f, "Operation timed out"),
            AdbConnectionError::DeviceNotAvailable(msg) => write!(f, "Device not available: {}", msg),
            AdbConnectionError::Unauthorized(msg) => write!(f, "Authorization failed: {}",msg),
            AdbConnectionError::ConnectionCloseError(err) => write!(f, "Connection terminated: {}", err),
            AdbConnectionError::CommunicationError(err) => write!(f, "Communication error: {}", err),
            AdbConnectionError::PortForwardSetupFailed(msg) => write!(f, "Port forward setup failed: {}", msg),
            AdbConnectionError::PortReverseSetupFailed(msg) => write!(f, "Port reverse setup failed: {}", msg),
            AdbConnectionError::SyncError(msg) => write!(f, "Sync operation failed: {}", msg),
            AdbConnectionError::UnexpectedError(err) => write!(f, "An unexpected error occurred: {}", err),
        }
    }
}
