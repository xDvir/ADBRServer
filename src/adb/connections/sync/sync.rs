use std::str::from_utf8;

use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tracing::{error, info};

use crate::adb::connections::adb_device_connection::AdbDeviceConnection;
use crate::adb::enums::adb_sync_command::SyncCommand;
use crate::adb::errors::adb_connection_error::AdbConnectionError;
use crate::adb::io::socket::{read_exact, read_string, read_u32, send_bytes};
use crate::adb::models::adb_transaction_info::AdbTransactionInfo;
use crate::constants::{B_FAIL, DENT_HEADER_SIZE, DENT_MIN_SIZE, OKAY, SYNC_COMMAND, SYNC_DATA_COMMAND, SYNC_DATA_COMMAND_STR, SYNC_DENT_COMMAND, SYNC_DENT_COMMAND_STR, SYNC_DONE_COMMAND, SYNC_DONE_COMMAND_STR, SYNC_LIST_COMMAND, SYNC_LIST_COMMAND_STR, SYNC_QUIT_COMMAND, SYNC_QUIT_COMMAND_STR, SYNC_RECV_COMMAND, SYNC_RECV_COMMAND_STR, SYNC_RECV_GET_DATA_TIME_SECONDS, SYNC_SEND_COMMAND, SYNC_SEND_COMMAND_STR, SYNC_STAT_COMMAND, SYNC_STAT_COMMAND_STR, ZERO};

impl AdbDeviceConnection {
    const SYNC_MAX_CHUNK_SIZE: usize = 64 * 1024;

    pub async fn _handle_sync_mode(&self, socket: &mut TcpStream, operation_timeout_s: Option<f64>) -> Result<(), AdbConnectionError> {
        info!("Initializing sync mode");
        let sync_command_transaction_info = self.initialize_sync_mode(operation_timeout_s).await?;

        loop {
            let command = Self::read_sync_command_from_socket(socket).await?;
            match command {
                SyncCommand::Stat { path } => {
                    info!("Handling Stat command for path: {}", path);
                    self.handle_stat_command(socket, &sync_command_transaction_info, &path, operation_timeout_s).await?
                }
                SyncCommand::Recv { path } => {
                    info!("Handling Recv command for path: {}", path);
                    self.handle_recv_command(socket, &sync_command_transaction_info, &path, operation_timeout_s).await?
                }
                SyncCommand::Send { mode, size: _, path } => {
                    info!("Handling Send command for path: {}", path);
                    self.handle_send_command(socket, &sync_command_transaction_info, &path, mode, operation_timeout_s).await?
                }
                SyncCommand::Data { .. } => {
                    error!("Received Data command without prerequisite");
                    return Err(AdbConnectionError::SyncError("Missing prerequisite sync command before Data transmission".to_string()));
                }
                SyncCommand::Done { .. } => {
                    error!("Received Done command without prerequisite");
                    return Err(AdbConnectionError::SyncError("Missing prerequisite sync command before Done transmission".to_string()));
                }
                SyncCommand::List { path } => {
                    info!("Handling List command for path: {}", path);
                    self.handle_list_command(socket, &sync_command_transaction_info, &path, operation_timeout_s).await?
                }
                SyncCommand::Dent { .. } => {
                    error!("Received Dent command without prerequisite");
                    return Err(AdbConnectionError::SyncError("Missing prerequisite sync command before Dent transmission".to_string()));
                }
                SyncCommand::Quit => {
                    info!("Handling quit command");
                    self.handle_quit_command(&sync_command_transaction_info, operation_timeout_s).await?;
                    break;
                }
            }
        }
        Ok(())
    }


    async fn initialize_sync_mode(&self, operation_timeout_s: Option<f64>) -> Result<AdbTransactionInfo, AdbConnectionError> {
        let mut sync_command_transaction_info = self.send_open_command(SYNC_COMMAND, operation_timeout_s).await?;
        let adb_message_response = self.read_okay_response(&sync_command_transaction_info, operation_timeout_s).await?;

        sync_command_transaction_info.set_receive_packet_id(adb_message_response.arg0());

        Ok(sync_command_transaction_info)
    }

    async fn handle_stat_command(&self, socket: &mut TcpStream, sync_command_transaction_info: &AdbTransactionInfo, path: &str, operation_timeout_s: Option<f64>) -> Result<(), AdbConnectionError> {
        let stat_command = Self::create_stat_command(path);
        self.send_wrte_command(sync_command_transaction_info, &stat_command, operation_timeout_s).await?;
        self.read_okay_response(sync_command_transaction_info, operation_timeout_s).await?;
        let write_response = self.read_wrte_response(sync_command_transaction_info, operation_timeout_s).await?;
        Self::handle_sync_wrte_response(write_response.data(), SYNC_STAT_COMMAND).await?;
        self.send_okay_command(sync_command_transaction_info, operation_timeout_s).await?;
        send_bytes(socket, &write_response.data()).await.map_err(AdbDeviceConnection::map_io_error)
    }

    async fn handle_recv_command(&self, socket: &mut TcpStream, sync_command_transaction_info: &AdbTransactionInfo, path: &str, operation_timeout_s: Option<f64>) -> Result<(), AdbConnectionError> {
        let recv_command = Self::create_recv_command(path);
        self.send_wrte_command(sync_command_transaction_info, &recv_command, operation_timeout_s).await?;
        self.read_okay_response(sync_command_transaction_info, operation_timeout_s).await?;
        self.recv_file_data(socket, sync_command_transaction_info, operation_timeout_s).await
    }

    async fn recv_file_data(&self, socket: &mut TcpStream, sync_command_transaction_info: &AdbTransactionInfo, operation_timeout_s: Option<f64>) -> Result<(), AdbConnectionError> {
        loop {
            let write_response = self.read_wrte_response(sync_command_transaction_info, Some(SYNC_RECV_GET_DATA_TIME_SECONDS)).await;
            match write_response {
                Ok(adb_message) => {
                    send_bytes(socket, adb_message.data()).await
                        .map_err(AdbDeviceConnection::map_io_error)?;
                    self.send_okay_command(sync_command_transaction_info, operation_timeout_s).await?;
                }
                Err(AdbConnectionError::Timeout) => break,
                Err(err) => {
                    error!("Error receiving file data: {}", err);
                    return Err(err);
                }
            }
        }
        Ok(())
    }

    async fn handle_send_command(&self, socket: &mut TcpStream, sync_command_transaction_info: &AdbTransactionInfo, path: &str, mode: u32, operation_timeout_s: Option<f64>) -> Result<(), AdbConnectionError> {
        let init_command = Self::create_init_send_command(path, mode);
        self.send_wrte_command(sync_command_transaction_info, &init_command, operation_timeout_s).await?;
        self.read_okay_response(sync_command_transaction_info, operation_timeout_s).await?;
        self.send_file_data(socket, sync_command_transaction_info, operation_timeout_s).await?;
        send_bytes(socket, &OKAY.as_ref()).await.map_err(AdbDeviceConnection::map_io_error)
    }

    async fn send_file_data(&self, socket: &mut TcpStream, sync_command_transaction_info: &AdbTransactionInfo, operation_timeout_s: Option<f64>) -> Result<(), AdbConnectionError> {
        let mut buffer = Vec::new();
        loop {
            let command = Self::read_sync_command_from_socket(socket).await?;

            match command {
                SyncCommand::Data { size } => {
                    buffer.extend_from_slice(SYNC_DATA_COMMAND);
                    buffer.extend_from_slice(&size.to_le_bytes());

                    let mut data = vec![0u8; size as usize];
                    socket.read_exact(&mut data).await.map_err(|e| AdbConnectionError::SyncError(e.to_string()))?;
                    buffer.extend_from_slice(&data);
                }
                SyncCommand::Done { mtime } => {
                    buffer.extend_from_slice(SYNC_DONE_COMMAND);
                    buffer.extend_from_slice(&mtime.to_le_bytes());
                    break;
                }
                _ => {
                    error!("Expected DATA or DONE command but received something else");
                    return Err(AdbConnectionError::SyncError("Expected DATA or DONE command".to_string()));
                }
            }
        }

        for chunk in buffer.chunks(Self::SYNC_MAX_CHUNK_SIZE) {
            self.send_wrte_command(sync_command_transaction_info, chunk, operation_timeout_s).await?;
            self.read_okay_response(sync_command_transaction_info, operation_timeout_s).await?;
        }
        Ok(())
    }

    async fn handle_list_command(&self, socket: &mut TcpStream, sync_command_transaction_info: &AdbTransactionInfo, device_path: &str, operation_timeout_s: Option<f64>) -> Result<(), AdbConnectionError> {
        if device_path.is_empty() {
            error!("Empty device path provided for list command");
            return Err(AdbConnectionError::SyncError("Cannot list an empty device path".to_string()));
        }

        let list_command = Self::create_list_command(device_path);
        self.send_wrte_command(sync_command_transaction_info, &list_command, operation_timeout_s).await?;
        self.read_okay_response(sync_command_transaction_info, operation_timeout_s).await?;

        let mut buffer = Vec::new();

        loop {
            let write_response = self.read_wrte_response(sync_command_transaction_info, operation_timeout_s).await?;
            buffer.extend_from_slice(write_response.data());

            let mut index = 0;
            while index < buffer.len() {
                if buffer.len() - index < 4 {
                    break;
                }

                let cmd = &buffer[index..index + 4];

                match cmd {
                    SYNC_DENT_COMMAND => {
                        let dent_size = self.handle_dent_command(&buffer, index, socket).await?;
                        if dent_size == 0 {
                            break;
                        }
                        index += dent_size;
                    }
                    SYNC_DONE_COMMAND => {
                        send_bytes(socket, SYNC_DONE_COMMAND).await.map_err(AdbDeviceConnection::map_io_error)?;
                        self.send_okay_command(sync_command_transaction_info, operation_timeout_s).await?;
                    }
                    B_FAIL => {
                        let error_msg = String::from_utf8_lossy(&buffer[index + 4..]).to_string();
                        error!("List command failed: {}", error_msg);
                        send_bytes(socket, &buffer[index + 4..]).await.map_err(AdbDeviceConnection::map_io_error)?;
                        return Err(AdbConnectionError::SyncError(format!("Listing failed: {}", error_msg)));
                    }
                    _ => {
                        error!("Unexpected response in list command: {:?}", cmd);
                        return Err(AdbConnectionError::SyncError(format!("Unexpected response: {:?}", cmd)));
                    }
                }
            }
            buffer.drain(..index);
            self.send_okay_command(sync_command_transaction_info, operation_timeout_s).await?;
        }
    }

    async fn handle_dent_command(&self, buffer: &[u8], index: usize, socket: &mut TcpStream) -> Result<usize, AdbConnectionError> {
        if buffer.len() - index < DENT_MIN_SIZE {
            return Ok(ZERO as usize);
        }

        let dent_size = match buffer[index + DENT_HEADER_SIZE..index + DENT_MIN_SIZE].try_into() {
            Ok(bytes) => u32::from_le_bytes(bytes) as usize + DENT_MIN_SIZE,
            Err(_) => {
                error!("Failed to parse DENT size from buffer");
                return Err(AdbConnectionError::SyncError("Failed to parse DENT size".to_string()));
            }
        };

        if buffer.len() - index < dent_size {
            return Ok(ZERO as usize);
        }

        let dent_entry = &buffer[index..index + dent_size];
        send_bytes(socket, dent_entry).await.map_err(AdbDeviceConnection::map_io_error)?;

        Ok(dent_size)
    }

    async fn handle_quit_command(&self, sync_command_transaction_info: &AdbTransactionInfo, operation_timeout_s: Option<f64>) -> Result<(), AdbConnectionError> {
        let quit_command = Self::create_quit_command();

        self.send_wrte_command(sync_command_transaction_info, &quit_command, operation_timeout_s).await?;
        self.read_okay_response(sync_command_transaction_info, operation_timeout_s).await?;
        self.read_clse_response(sync_command_transaction_info, operation_timeout_s).await
    }

    async fn read_sync_command_from_socket(socket: &mut TcpStream) -> Result<SyncCommand, AdbConnectionError> {
        let cmd = read_exact(socket, 4).await.map_err(AdbDeviceConnection::map_io_error)?;

        let cmd_str = from_utf8(&cmd)
            .map_err(|e| AdbConnectionError::SyncError(format!("Invalid UTF-8 in command: {}", e)))?;

        match cmd_str {
            SYNC_SEND_COMMAND_STR => {
                let length = read_u32(socket).await.map_err(AdbDeviceConnection::map_io_error)?;
                let remote_path_with_mode = read_string(socket, length as usize)
                    .await.map_err(AdbDeviceConnection::map_io_error)?;

                let parts: Vec<&str> = remote_path_with_mode.split(',').collect();
                if parts.len() != 2 {
                    error!("Invalid SEND command format received");
                    return Err(AdbConnectionError::SyncError("Invalid SEND command format".to_string()));
                }

                let path = parts[0].to_string();
                match parts[1].parse::<u32>() {
                    Ok(mode) => Ok(SyncCommand::Send { mode, size: 0, path }),
                    Err(err) => {
                        error!("Invalid mode format in SEND command: {}", err);
                        Err(AdbConnectionError::SyncError("Invalid mode format".to_string()))
                    }
                }
            }
            SYNC_DATA_COMMAND_STR => {
                let size = read_u32(socket).await.map_err(AdbDeviceConnection::map_io_error)?;
                Ok(SyncCommand::Data { size })
            }
            SYNC_STAT_COMMAND_STR => {
                let path_length = read_u32(socket).await.map_err(AdbDeviceConnection::map_io_error)?;
                let path = read_string(socket, path_length as usize).await.map_err(AdbDeviceConnection::map_io_error)?;
                Ok(SyncCommand::Stat { path })
            }
            SYNC_DONE_COMMAND_STR => {
                let mtime = read_u32(socket).await.map_err(AdbDeviceConnection::map_io_error)?;
                Ok(SyncCommand::Done { mtime })
            }
            SYNC_RECV_COMMAND_STR => {
                let length = read_u32(socket).await.map_err(AdbDeviceConnection::map_io_error)?;
                let path = read_string(socket, length as usize).await.map_err(AdbDeviceConnection::map_io_error)?;
                Ok(SyncCommand::Recv { path })
            }
            SYNC_LIST_COMMAND_STR => {
                let path_length = read_u32(socket).await.map_err(AdbDeviceConnection::map_io_error)?;
                let path = read_string(socket, path_length as usize).await.map_err(AdbDeviceConnection::map_io_error)?;
                Ok(SyncCommand::List { path })
            }
            SYNC_DENT_COMMAND_STR => {
                let mode = read_u32(socket).await.map_err(AdbDeviceConnection::map_io_error)?;
                let size = read_u32(socket).await.map_err(AdbDeviceConnection::map_io_error)?;
                let mtime = read_u32(socket).await.map_err(AdbDeviceConnection::map_io_error)?;
                let name_length = read_u32(socket).await.map_err(AdbDeviceConnection::map_io_error)?;
                let name = read_string(socket, name_length as usize).await.map_err(AdbDeviceConnection::map_io_error)?;
                Ok(SyncCommand::Dent { mode, size, mtime, name })
            }
            SYNC_QUIT_COMMAND_STR => {
                Ok(SyncCommand::Quit)
            }
            _ => {
                error!("Unknown sync command received: {}", cmd_str);
                Err(AdbConnectionError::SyncError(format!("Unknown sync command: {}", cmd_str)))
            }
        }
    }

    async fn handle_sync_wrte_response(data: &[u8], expected_responses: &[u8]) -> Result<(), AdbConnectionError> {
        match data {
            d if d.starts_with(expected_responses) => {
                Ok(())
            }
            d if d.starts_with(B_FAIL) => {
                let error_message = String::from_utf8_lossy(&d[4..]).to_string();
                error!("Failed to handle sync response: {}", error_message);
                Err(AdbConnectionError::SyncError(format!("Command failed: {}", error_message)))
            }
            d => {
                error!("Unexpected sync response received: {:?}", d);
                Err(AdbConnectionError::SyncError("Unexpected response".to_string()))
            }
        }
    }

    fn create_init_send_command(path: &str, mode: u32) -> Vec<u8> {
        let mut command = Vec::new();
        command.extend_from_slice(SYNC_SEND_COMMAND);
        let file_info = format!("{},{}", path, mode);
        command.extend_from_slice(&(file_info.len() as u32).to_le_bytes());
        command.extend_from_slice(file_info.as_bytes());
        command
    }

    fn create_stat_command(path: &str) -> Vec<u8> {
        let mut stat_command = SYNC_STAT_COMMAND.to_vec();
        stat_command.extend_from_slice(&(path.len() as u32).to_le_bytes());
        stat_command.extend_from_slice(path.as_bytes());
        stat_command
    }

    fn create_list_command(path: &str) -> Vec<u8> {
        let mut stat_command = SYNC_LIST_COMMAND.to_vec();
        stat_command.extend_from_slice(&(path.len() as u32).to_le_bytes());
        stat_command.extend_from_slice(path.as_bytes());
        stat_command
    }

    fn create_recv_command(path: &str) -> Vec<u8> {
        let mut recv_command = Vec::new();
        recv_command.extend_from_slice(SYNC_RECV_COMMAND);
        recv_command.extend_from_slice(&(path.len() as u32).to_le_bytes());
        recv_command.extend_from_slice(path.as_bytes());
        recv_command
    }

    fn create_quit_command() -> Vec<u8> {
        let mut quit_command = Vec::new();
        quit_command.extend_from_slice(SYNC_QUIT_COMMAND);
        quit_command.extend_from_slice(&[0, 0, 0, 0]);
        quit_command
    }
}