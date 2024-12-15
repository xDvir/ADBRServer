use std::sync::{Arc, Mutex};
use async_trait::async_trait;
use tracing::error;
use crate::adb::connections::adb_connection::AdbConnection;
use crate::adb::errors::adb_connection_error::AdbConnectionError;
use crate::adb::errors::adb_connection_error::AdbConnectionError::{CommunicationError, Unauthorized, UnexpectedError};
use crate::adb::errors::adb_io_error::AdbIoError;
use crate::constants::{ZERO};

use crate::transport::transport::Transport;
use crate::utils::utils::{get_adb_key_path};
use tokio::net::{TcpStream};
use tokio::sync::oneshot;
use tokio::sync::oneshot::Sender;
use crate::adb::io::daemon::AdbDeviceIo;
use crate::adb::models::adb_port_forward_info::AdbPortForwardInfo;
use crate::adb::models::adb_port_reverse_info::AdbPortReverseInfo;


pub struct AdbDeviceConnection {
    pub adb_keys_path: String,
    pub adb_io_manager: AdbDeviceIo,
    last_local_packet_id: Mutex<u32>,
}

impl AdbDeviceConnection {
    pub fn new(adb_keys_path: Option<String>, transport: Box<dyn Transport>) -> Result<Self, AdbConnectionError> {
        let adb_keys_path = if let Some(path) = adb_keys_path {
            path
        } else {
            get_adb_key_path().map_err(|e| Unauthorized(format!("Error retrieving ADB key path: {}", e)))?
        };

        Ok(AdbDeviceConnection {
            adb_keys_path,
            adb_io_manager: AdbDeviceIo::new(transport),
            last_local_packet_id: Mutex::new(ZERO),
        })
    }

    pub fn get_last_packet_id(&self) -> Result<u32, AdbConnectionError> {
        let mut packet_id = self.last_local_packet_id.lock().map_err(|e| UnexpectedError(e.to_string()))?;
        *packet_id += 1;
        let current_packet_id = *packet_id;
        if *packet_id == u32::MAX {
            *packet_id = 1;
        }
        Ok(current_packet_id)
    }

    pub fn map_io_error(error: AdbIoError) -> AdbConnectionError {
        match error {
            AdbIoError::TimeoutError => AdbConnectionError::Timeout,
            AdbIoError::DeviceConnectionError(msg) => AdbConnectionError::DeviceNotAvailable(msg),
            AdbIoError::SocketError(msg) => AdbConnectionError::ConnectionCloseError(msg),
            AdbIoError::ConnectionClosed(msg) => AdbConnectionError::ConnectionCloseError(msg),
            AdbIoError::CommunicationError(msg) => CommunicationError(msg),
            AdbIoError::ParseError(msg) => UnexpectedError(msg),
            AdbIoError::UnexpectedError(msg) => UnexpectedError(msg),
        }
    }
}

#[async_trait]
impl AdbConnection for AdbDeviceConnection {
    async fn close(&self) {
        if let Err(err) = self.adb_io_manager.release_device().await {
            error!("Error while try close connection with device {}", err);
        }
    }

    async fn verify_connection_status(&self) -> Result<(), AdbConnectionError> {
        self.adb_io_manager.verify_connection_status().await.map_err(|err| AdbConnectionError::DeviceNotAvailable(err.to_string()))
    }

    async fn connect(&mut self, operation_timeout_s: Option<f64>) -> Result<(), AdbConnectionError> {
        self.adb_connect(operation_timeout_s).await
    }

    async fn adb_disable_verity(&self, operation_timeout_s: Option<f64>) -> Result<String, AdbConnectionError> {
        self._adb_disable_verity(operation_timeout_s).await
    }

    async fn adb_enable_verity(&self, operation_timeout_s: Option<f64>) -> Result<String, AdbConnectionError> {
        self._adb_enable_verity(operation_timeout_s).await
    }

    async fn adb_reboot(&self, reboot_command: &str, operation_timeout_s: Option<f64>) -> Result<(), AdbConnectionError> {
        self._adb_reboot(reboot_command, operation_timeout_s).await
    }

    async fn adb_remount(&self, operation_timeout_s: Option<f64>) -> Result<String, AdbConnectionError> {
        self._adb_remount(operation_timeout_s).await
    }

    async fn adb_root(&self, operation_timeout_s: Option<f64>) -> Result<String, AdbConnectionError> {
        self._adb_root(operation_timeout_s).await
    }

    async fn adb_unroot(&self, operation_timeout_s: Option<f64>) -> Result<String, AdbConnectionError> {
        self._adb_unroot(operation_timeout_s).await
    }

    async fn adb_get_devpath(&self, _operation_timeout_s: Option<f64>) -> Result<String, AdbConnectionError> {
        unimplemented!()
    }

    async fn adb_shell_command(&self, socket: &mut TcpStream, command: String, operation_timeout_s: Option<f64>) -> Result<(), AdbConnectionError> {
        self._adb_shell_command(socket, command, operation_timeout_s).await
    }

    async fn open_shell_session(&self, socket: &mut TcpStream, command: String, command_read_timeout_s: Option<f64>, input_read_timeout_s: Option<f64>) -> Result<(), AdbConnectionError> {
        self._open_shell_session(socket, command, command_read_timeout_s, input_read_timeout_s).await
    }

    async fn adb_port_forward_set(&self, adb_port_forward_info: AdbPortForwardInfo, port_forward_result_sender: oneshot::Sender<Result<(), AdbConnectionError>>, operation_timeout_s: Option<f64>) {
        self._adb_port_forwarding_set(&adb_port_forward_info, port_forward_result_sender, operation_timeout_s).await;
    }

    async fn adb_port_reverse_set(self:Arc<Self>, adb_port_reverse_info: AdbPortReverseInfo, port_reverse_result_sender: Sender<Result<(), AdbConnectionError>>, operation_timeout_s: Option<f64>) {
        self._adb_port_reverse_set(&adb_port_reverse_info, port_reverse_result_sender, operation_timeout_s).await;
    }

    async fn handle_sync_mode(&self, socket: &mut TcpStream, operation_timeout_s: Option<f64>) -> Result<(), AdbConnectionError> {
        match self._handle_sync_mode(socket, operation_timeout_s).await {
            Ok(_) => Ok(()),
            Err(AdbConnectionError::ConnectionCloseError(_)) => Ok(()),
            Err(err) => Err(err)
        }
    }
}
