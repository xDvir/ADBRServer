use crate::adb::connections::adb_device_connection::AdbDeviceConnection;
use crate::adb::errors::adb_connection_error::AdbConnectionError;
use crate::adb::errors::adb_connection_error::AdbConnectionError::UnexpectedError;
use crate::constants::{DISABLE_VERITY_COMMAND, ENABLE_VERITY_COMMAND};

impl AdbDeviceConnection {
    pub async fn _adb_disable_verity(&self, operation_timeout_s: Option<f64>) -> Result<String, AdbConnectionError> {
        let mut adb_disable_verity_command_transaction_info = self.send_open_command(DISABLE_VERITY_COMMAND, operation_timeout_s).await?;
        let adb_message_response = self.read_okay_response(&adb_disable_verity_command_transaction_info, operation_timeout_s).await?;
        adb_disable_verity_command_transaction_info.set_receive_packet_id(adb_message_response.arg0());
        let enable_disable_response = self.read_wrte_response(&adb_disable_verity_command_transaction_info, operation_timeout_s).await?;
        self.send_clse_command(&adb_disable_verity_command_transaction_info, operation_timeout_s).await?;
        String::from_utf8(enable_disable_response.data().to_vec())
            .map_err(|e| UnexpectedError(e.to_string()))
    }

    pub async fn _adb_enable_verity(&self, operation_timeout_s: Option<f64>) -> Result<String, AdbConnectionError> {
        let mut adb_enable_verity_command_transaction_info = self.send_open_command(ENABLE_VERITY_COMMAND, operation_timeout_s).await?;
        let adb_message_response = self.read_okay_response(&adb_enable_verity_command_transaction_info, operation_timeout_s).await?;
        adb_enable_verity_command_transaction_info.set_receive_packet_id(adb_message_response.arg0());
        let enable_verity_response = self.read_wrte_response(&adb_enable_verity_command_transaction_info, operation_timeout_s).await?;
        self.send_clse_command(&adb_enable_verity_command_transaction_info, operation_timeout_s).await?;
        String::from_utf8(enable_verity_response.data().to_vec())
            .map_err(|e| UnexpectedError(e.to_string()))
    }
}