use std::sync::Arc;
use async_trait::async_trait;
use tokio::net::TcpStream;
use tokio::sync::oneshot;
use crate::adb::errors::adb_connection_error::AdbConnectionError;
use crate::adb::models::adb_port_forward_info::AdbPortForwardInfo;
use crate::adb::models::adb_port_reverse_info::AdbPortReverseInfo;


#[async_trait]
pub trait AdbConnection: Send + Sync {
    async fn close(&self);
    async fn verify_connection_status(&self) -> Result<(), AdbConnectionError>;
    async fn connect(&mut self, operation_timeout_s: Option<f64>) -> Result<(), AdbConnectionError>;
    async fn adb_disable_verity(&self, operation_timeout_s: Option<f64>) -> Result<String, AdbConnectionError>;
    async fn adb_enable_verity(&self, operation_timeout_s: Option<f64>) -> Result<String, AdbConnectionError>;
    async fn adb_reboot(&self, reboot_command: &str, operation_timeout_s: Option<f64>) -> Result<(), AdbConnectionError>;
    async fn adb_remount(&self, operation_timeout_s: Option<f64>) -> Result<String, AdbConnectionError>;
    async fn adb_root(&self, operation_timeout_s: Option<f64>) -> Result<String, AdbConnectionError>;
    async fn adb_unroot(&self, operation_timeout_s: Option<f64>) -> Result<String, AdbConnectionError>;
    async fn adb_get_devpath(&self, operation_timeout_s: Option<f64>) -> Result<String, AdbConnectionError>;
    async fn adb_shell_command(&self, socket: &mut TcpStream, command: String, operation_timeout_s: Option<f64>) -> Result<(), AdbConnectionError>;
    async fn open_shell_session(&self, socket: &mut TcpStream, command: String, command_read_timeout_s: Option<f64>, input_read_timeout_s: Option<f64>) -> Result<(), AdbConnectionError>;
    async fn adb_port_forward_set(&self, adb_port_forward_info: AdbPortForwardInfo, port_forward_result_sender: oneshot::Sender<Result<(), AdbConnectionError>>, operation_timeout_s: Option<f64>);
    async fn adb_port_reverse_set(self:Arc<Self>, adb_port_reverse_info: AdbPortReverseInfo, port_reverse_result_sender: oneshot::Sender<Result<(), AdbConnectionError>>, operation_timeout_s: Option<f64>);
    async fn handle_sync_mode(&self, socket: &mut TcpStream, operation_timeout_s: Option<f64>) -> Result<(), AdbConnectionError>;
}