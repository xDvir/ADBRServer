use tracing::error;

use crate::adb::connections::adb_device_connection::AdbDeviceConnection;
use crate::adb::errors::adb_connection_error::AdbConnectionError;
use crate::adb::errors::adb_connection_error::AdbConnectionError::UnexpectedError;
use crate::adb::models::adb_message::AdbMessage;
use crate::adb::models::adb_transaction_info::AdbTransactionInfo;
use crate::constants::{ADB_SERVER_VERSION, CLSE_CODE, CNXN_CODE, HOST, MAX_ADB_DATA, OKAY_CODE, OPEN_CODE, WRTE_CODE, ZERO};

impl AdbDeviceConnection {
    pub async fn send_cnxn_command(&self, operation_timeout_s: Option<f64>) -> Result<(), AdbConnectionError> {
        let host_name = hostname::get()
            .map_err(|e| {
                error!("Failed to get hostname: {}", e);
                UnexpectedError("Failed to get hostname".to_string())
            })?
            .into_string()
            .map_err(|_| UnexpectedError("Hostname contains invalid UTF-8".to_string()))?;

        let banner = format!("{}{}\x00", HOST, host_name).into_bytes();

        let connect_message = AdbMessage::new(CNXN_CODE, ADB_SERVER_VERSION, MAX_ADB_DATA, banner);
        self.send_adb_message(&connect_message, operation_timeout_s).await?;
        Ok(())
    }

    pub async fn send_open_command(&self, command: &str, operation_timeout_s: Option<f64>) -> Result<AdbTransactionInfo, AdbConnectionError> {
        let last_packet_id = self.get_last_packet_id()?;
        let transaction_info = AdbTransactionInfo::new(last_packet_id, ZERO);
        let open_message = AdbMessage::new(OPEN_CODE, last_packet_id, ZERO, command.as_bytes().to_vec());
        self.send_adb_message(&open_message, operation_timeout_s).await?;
        Ok(transaction_info)
    }


    pub async fn send_okay_command(&self, adb_transaction_info: &AdbTransactionInfo, operation_timeout_s: Option<f64>) -> Result<(), AdbConnectionError> {
        let adb_message = AdbMessage::new(OKAY_CODE, adb_transaction_info.sent_packet_id(), adb_transaction_info.receive_packet_id(), Vec::new());
        self.send_adb_message(&adb_message, operation_timeout_s).await
    }

    pub async fn send_clse_command(&self, adb_transaction_info: &AdbTransactionInfo, operation_timeout_s: Option<f64>) -> Result<(), AdbConnectionError> {
        let adb_message = AdbMessage::new(CLSE_CODE, adb_transaction_info.sent_packet_id(), adb_transaction_info.receive_packet_id(), Vec::new());
        self.send_adb_message(&adb_message, operation_timeout_s).await
    }

    pub async fn send_wrte_command(&self, adb_transaction_info: &AdbTransactionInfo, wrte_data: &[u8], operation_timeout_s: Option<f64>) -> Result<(), AdbConnectionError> {
        let adb_message = AdbMessage::new(WRTE_CODE, adb_transaction_info.sent_packet_id(), adb_transaction_info.receive_packet_id(), wrte_data.to_vec());
        self.send_adb_message(&adb_message, operation_timeout_s).await
    }

    pub async fn send_adb_message(&self, adb_message: &AdbMessage, operation_timeout_s: Option<f64>) -> Result<(), AdbConnectionError> {
        self.adb_io_manager.write_bytes(adb_message, operation_timeout_s).await.map_err(|err| {
            error!("Failed to send ADB message: {}", err);
            AdbDeviceConnection::map_io_error(err)
        })
    }

    pub async fn read_okay_response(&self, adb_transaction_info: &AdbTransactionInfo, operation_timeout_s: Option<f64>) -> Result<AdbMessage, AdbConnectionError> {
        self.read_expected_packet(&[OKAY_CODE], None, adb_transaction_info, operation_timeout_s).await
    }

    pub async fn read_wrte_response(&self, adb_transaction_info: &AdbTransactionInfo, operation_timeout_s: Option<f64>) -> Result<AdbMessage, AdbConnectionError> {
        self.read_expected_packet(&[WRTE_CODE], None, adb_transaction_info, operation_timeout_s).await
    }

    pub async fn read_clse_response(&self, adb_transaction_info: &AdbTransactionInfo, operation_timeout_s: Option<f64>) -> Result<(), AdbConnectionError> {
        match self.read_expected_packet(&[CLSE_CODE], None, adb_transaction_info, operation_timeout_s).await {
            Ok(_) => Ok(()),
            Err(AdbConnectionError::Timeout) => Ok(()),
            Err(err) => Err(err)
        }
    }
}