
#[derive(Debug)]
#[allow(dead_code)]
pub enum TransportError {
    Timeout,
    Disconnected,
    Unauthorized(String),
    DeviceNotFound,
    ConnectionError(String),
    CommunicationError(String),
    UnexpectedError(String),
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransportError::Timeout => write!(f, ""),
            TransportError::Disconnected => write!(f, ""),
            TransportError::Unauthorized(err) => write!(f, "{}", err),
            TransportError::DeviceNotFound => write!(f, ""),
            TransportError::ConnectionError(err) => write!(f, "{}", err),
            TransportError::CommunicationError(msg) => write!(f, "{}", msg),
            TransportError::UnexpectedError(msg) => write!(f, "{}", msg),
        }
    }
}