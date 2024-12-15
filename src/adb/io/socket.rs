use std::io::ErrorKind;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;
use crate::adb::errors::adb_io_error::AdbIoError;
use crate::adb::errors::adb_server_error::AdbServerError;
use crate::utils::utils::{fail, format_response_with_size, okay};

const CLIENT_READ_TIMEOUT_DURATION_SEC: f64 = 5.0;

pub async fn send_full_response(socket: &mut TcpStream, response: String) -> Result<(), AdbIoError> {
    socket.write_all(response.as_bytes()).await
        .map_err(|e| AdbIoError::CommunicationError(format!("Failed to write to socket: {}", e)))
}

pub async fn send_bytes(socket: &mut TcpStream, data: &[u8]) -> Result<(), AdbIoError> {
    socket.write_all(data).await.map_err(|err| AdbIoError::SocketError(err.to_string()))
}

pub async fn read_request_from_socket(socket: &mut TcpStream) -> Result<String, AdbIoError> {
    let mut length_prefix = [0; 4];
    read_with_timeout(socket.read_exact(&mut length_prefix)).await?;

    let length_str = String::from_utf8_lossy(&length_prefix);
    let request_size = usize::from_str_radix(&length_str, 16)
        .map_err(|e| AdbIoError::ParseError(format!("Failed to parse length prefix: {}", e)))?;

    let mut buffer_request = vec![0; request_size];
    read_with_timeout(socket.read_exact(&mut buffer_request)).await?;

    String::from_utf8(buffer_request)
        .map_err(|e| AdbIoError::ParseError(format!("Failed to convert response to string: {}", e)))
}

async fn read_with_timeout<F, T>(operation: F) -> Result<T, AdbIoError>
    where
        F: std::future::Future<Output=std::io::Result<T>>,
{
    match timeout(Duration::from_secs_f64(CLIENT_READ_TIMEOUT_DURATION_SEC), operation).await {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(err)) => {
            if err.kind() == ErrorKind::UnexpectedEof {
                Err(AdbIoError::ConnectionClosed("Connection closed unexpectedly".to_string()))
            } else {
                Err(AdbIoError::CommunicationError(err.to_string()))
            }
        }
        Err(_) => Err(AdbIoError::TimeoutError),
    }
}


pub async fn read_u32(socket: &mut TcpStream) -> Result<u32, AdbIoError> {
    let mut buf = [0u8; 4];
    socket.read_exact(&mut buf).await
        .map_err(|e| AdbIoError::CommunicationError(format!("Failed to read u32: {}", e)))?;
    Ok(u32::from_le_bytes(buf))
}

pub async fn read_string(socket: &mut TcpStream, len: usize) -> Result<String, AdbIoError> {
    let mut buf = vec![0u8; len];
    socket.read_exact(&mut buf).await
        .map_err(|e| AdbIoError::CommunicationError(format!("Failed to read string: {}", e)))?;
    String::from_utf8(buf)
        .map_err(|e| AdbIoError::ParseError(format!("Failed to convert bytes to string: {}", e)))
}


pub async fn read_exact(socket: &mut TcpStream, len: usize) -> Result<Vec<u8>, AdbIoError> {
    let mut buf = vec![0u8; len];
    match socket.read_exact(&mut buf).await {
        Ok(_) => Ok(buf),
        Err(e) => {
            if e.kind() == ErrorKind::UnexpectedEof {
                Err(AdbIoError::ConnectionClosed("Connection closed by peer".to_string()))
            } else {
                Err(AdbIoError::CommunicationError(format!("Failed to read exact bytes: {}", e)))
            }
        }
    }
}

pub async fn send_ok_with_response(socket: &mut TcpStream, response: Option<String>) -> Result<(), AdbServerError> {
    let formatted_response = match response {
        Some(content) => format_response_with_size(content),
        None => String::new(),
    };

    let full_response = format!("{}{}", okay(), formatted_response);
    send_full_response(socket, full_response).await.map_err(|err| AdbServerError::IOError(err))
}

pub async fn send_fail_with_response(socket: &mut TcpStream, response: Option<String>) -> Result<(), AdbServerError> {
    let formatted_response = match response {
        Some(content) => format_response_with_size(content),
        None => String::new(),
    };

    let full_response = format!("{}{}", fail(), formatted_response);
    send_full_response(socket, full_response).await.map_err(|err| AdbServerError::IOError(err))
}

