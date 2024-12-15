use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tracing::{error, info, warn};

use crate::adb::connections::adb_device_connection::AdbDeviceConnection;
use crate::adb::errors::adb_connection_error::AdbConnectionError;
use crate::constants::{DEFAULT_BUFFER_SIZE, OKAY_CODE};

impl AdbDeviceConnection {
    pub async fn _open_shell_session(&self, socket: &mut TcpStream, command: String, command_operation_timeout_s: Option<f64>, input_operation_timeout_s: Option<f64>) -> Result<(), AdbConnectionError> {
        info!("Opening shell session for command: {}", command);
        let expected_responses = &[OKAY_CODE];
        let mut transaction_info = self.send_open_command(&command, command_operation_timeout_s).await?;
        let open_response = self.adb_io_manager.read_adb_message_with_packet_store(expected_responses, None, &transaction_info, command_operation_timeout_s).await.map_err(AdbDeviceConnection::map_io_error)?;
        transaction_info.set_receive_packet_id(open_response.arg0());
        let mut read_time_out_seconds = command_operation_timeout_s;

        loop {
            self.read_and_write_all_response(&transaction_info, socket, read_time_out_seconds).await?;

            let mut buf = [0; DEFAULT_BUFFER_SIZE];
            match socket.read(&mut buf).await {
                Ok(0) => {
                    break;
                }
                Ok(n) => {
                    let data = &buf[..n];
                    let last_byte = data[n - 1];
                    if last_byte == b'\n' {
                        read_time_out_seconds = command_operation_timeout_s;
                    } else {
                        read_time_out_seconds = input_operation_timeout_s
                    }
                    if let Err(err) = self.send_wrte_command(&transaction_info, data, command_operation_timeout_s).await {
                        error!("Failed to send shell command data: {}", err);
                        break;
                    }
                }
                Err(err) => {
                    warn!("Shell session terminated: {}", err);
                    break;
                }
            }
        }

        Ok(())
    }

    pub async fn _adb_shell_command(&self, socket: &mut TcpStream, command: String, operation_timeout_s: Option<f64>) -> Result<(), AdbConnectionError> {
        info!("Executing shell command: {}", command);
        let expected_responses: &[u32] = &[OKAY_CODE];
        let mut send_open_command_transaction_info = self.send_open_command(&command, operation_timeout_s).await?;
        let adb_message_response = self.read_expected_packet(expected_responses, None, &send_open_command_transaction_info, operation_timeout_s).await?;
        send_open_command_transaction_info.set_receive_packet_id(adb_message_response.arg0());
        self.read_and_write_all_response(&send_open_command_transaction_info, socket, operation_timeout_s).await
    }
}