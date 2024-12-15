use crate::adb::connections::adb_device_connection::AdbDeviceConnection;
use crate::adb::errors::adb_connection_error::AdbConnectionError;
use crate::adb::errors::adb_connection_error::AdbConnectionError::UnexpectedError;
use crate::constants::{REMOUNT_COMMAND, ROOT_COMMAND, UNROOT_COMMAND};

impl AdbDeviceConnection {
    pub async fn _adb_reboot(&self, reboot_command: &str, operation_timeout_s: Option<f64>) -> Result<(), AdbConnectionError> {
        match self.send_open_command(reboot_command, operation_timeout_s).await {
            Err(_) => Err(UnexpectedError("Failed to reboot device. Please check the connection and try again.".to_string())),
            Ok(_) => Ok(())
        }
    }

    pub async fn _adb_remount(&self, operation_timeout_s: Option<f64>) -> Result<String, AdbConnectionError> {
        let mut adb_remount_command_transaction_info = self.send_open_command(REMOUNT_COMMAND, operation_timeout_s).await?;
        let adb_message_response = self.read_okay_response(&adb_remount_command_transaction_info, operation_timeout_s).await?;
        adb_remount_command_transaction_info.set_receive_packet_id(adb_message_response.arg0());
        let remount_response = self.read_wrte_response(&adb_remount_command_transaction_info, operation_timeout_s).await?;
        self.send_clse_command(&adb_remount_command_transaction_info, operation_timeout_s).await?;
        String::from_utf8(remount_response.data().to_vec())
            .map_err(|e| UnexpectedError(e.to_string()))
    }

    pub async fn _adb_root(&self, operation_timeout_s: Option<f64>) -> Result<String, AdbConnectionError> {
        let mut adb_root_command_transaction_info = self.send_open_command(ROOT_COMMAND, operation_timeout_s).await?;
        let adb_message_response = self.read_okay_response(&adb_root_command_transaction_info, operation_timeout_s).await?;
        adb_root_command_transaction_info.set_receive_packet_id(adb_message_response.arg0());
        let root_response = self.read_wrte_response(&adb_root_command_transaction_info, operation_timeout_s).await?;
        self.send_clse_command(&adb_root_command_transaction_info, operation_timeout_s).await?;
        String::from_utf8(root_response.data().to_vec())
            .map_err(|e| UnexpectedError(e.to_string()))
    }

    pub async fn _adb_unroot(&self, operation_timeout_s: Option<f64>) -> Result<String, AdbConnectionError> {
        let mut adb_unroot_command_transaction_info = self.send_open_command(UNROOT_COMMAND, operation_timeout_s).await?;
        let adb_message_response = self.read_okay_response(&adb_unroot_command_transaction_info, operation_timeout_s).await?;
        adb_unroot_command_transaction_info.set_receive_packet_id(adb_message_response.arg0());
        let unroot_response = self.read_wrte_response(&adb_unroot_command_transaction_info, operation_timeout_s).await?;
        self.send_clse_command(&adb_unroot_command_transaction_info, operation_timeout_s).await?;
        String::from_utf8(unroot_response.data().to_vec())
            .map_err(|e| UnexpectedError(e.to_string()))
    }
}