use tokio::net::TcpStream;
use crate::adb::errors::adb_connection_error::AdbConnectionError;
use crate::adb::io::socket::{send_bytes, send_ok_with_response};
use crate::adb::models::adb_device::AdbDevice;
use crate::adb::server::server::AdbServer;
use crate::constants::{BINARY_SHELL_COMMAND, BINARY_SHELL_COMMAND_NULL, SHELL_BUGREPORT_COMMAND};
use crate::utils::utils::ensure_null_terminated;
use tracing::warn;
impl AdbServer {
    const SHELL_COMMAND_TIMEOUT: f64 = 15.0;
    const BUGREPORT_TIMEOUT: f64 = 100.0;
    const SHELL_INPUT_OPERATION_TIME_OUT_SECONDS: f64 = 0.05;
    const SHELL_COMMAND_TIME_OUT_SECONDS: f64 = 0.25;

    pub async fn handle_shell_command(adb_device: &AdbDevice, socket: &mut TcpStream, shell_command: String) {
        if let Err(err) = send_ok_with_response(socket, None).await {
            warn!("Failed to send OK response to client: {}", err);
            return;
        }

        let command_bytes = shell_command.as_bytes();
        match command_bytes {
            BINARY_SHELL_COMMAND | BINARY_SHELL_COMMAND_NULL => {
                Self::handle_interactive_shell(adb_device, socket, shell_command).await
            }
            _ => {
                Self::handle_single_command(adb_device, socket, shell_command).await
            }
        }
    }

    async fn handle_single_command(adb_device: &AdbDevice, socket: &mut TcpStream, command: String) {
        let timeout = Self::get_command_timeout(&command);
        let command = ensure_null_terminated(command);

        if let Err(err) = adb_device.adb_device_connection()
            .adb_shell_command(socket, command, Some(timeout)).await
        {
            let error_msg = match err {
                AdbConnectionError::Timeout => format!("Command timed out after {} seconds\n", timeout),
                _ => format!("Command execution failed: {}\n", err),
            };

            if let Err(err) = send_bytes(socket, error_msg.as_bytes()).await {
                warn!("Failed to send error message to client: {}", err);
            }
        }
    }

    async fn handle_interactive_shell(adb_device: &AdbDevice, socket: &mut TcpStream, shell_command: String) {
        if let Err(err) = adb_device.adb_device_connection()
            .open_shell_session(socket, shell_command, Some(Self::SHELL_COMMAND_TIME_OUT_SECONDS), Some(Self::SHELL_INPUT_OPERATION_TIME_OUT_SECONDS)).await
        {
            let error_msg = match err {
                AdbConnectionError::Timeout => "Interactive shell timed out".to_string(),
                _ => format!("Interactive shell session failed: {}\n", err),
            };

            if let Err(err) = send_bytes(socket, error_msg.as_bytes()).await {
                warn!("Failed to send error message to client: {}", err);
            }
        }
    }

    fn get_command_timeout(command: &str) -> f64 {
        match command {
            cmd if cmd.starts_with(SHELL_BUGREPORT_COMMAND) => Self::BUGREPORT_TIMEOUT,
            _ => Self::SHELL_COMMAND_TIMEOUT,
        }
    }
}
