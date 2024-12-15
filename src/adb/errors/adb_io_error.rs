use std::fmt;

#[derive(Debug)]
pub enum AdbIoError {
    DeviceConnectionError(String),
    SocketError(String),
    CommunicationError(String),
    TimeoutError,
    ConnectionClosed(String),
    ParseError(String),
    UnexpectedError(String),
}

impl fmt::Display for AdbIoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AdbIoError::DeviceConnectionError(msg) => write!(f, "{}", msg),
            AdbIoError::SocketError(msg) => write!(f, "{}", msg),
            AdbIoError::CommunicationError(msg) => write!(f, "{}", msg),
            AdbIoError::TimeoutError => write!(f, ""),
            AdbIoError::ConnectionClosed(msg) => write!(f, "{}", msg),
            AdbIoError::ParseError(msg) => write!(f, "{}", msg),
            AdbIoError::UnexpectedError(msg) => write!(f, "{}", msg),
        }
    }
}
impl std::error::Error for AdbIoError {}