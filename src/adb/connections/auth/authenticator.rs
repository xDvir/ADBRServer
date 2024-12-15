use std::fs::read_to_string;

use tokio_stream::StreamExt;
use tracing::{error, info};

use crate::adb::connections::adb_device_connection::AdbDeviceConnection;
use crate::adb::errors::adb_connection_error::AdbConnectionError;
use crate::adb::errors::adb_connection_error::AdbConnectionError::{Unauthorized, UnexpectedError};
use crate::adb::models::adb_message::AdbMessage;
use crate::adb::models::adb_transaction_info::AdbTransactionInfo;
use crate::constants::{ADB_PRIVATE_KEY_FILE, ADB_PUBLIC_KEY_FILE, AUTH_CODE, CNXN_CODE, ONE, OPEN_CODE, ZERO};
use crate::transport::enums::interface_type::InterfaceType;
use crate::utils::utils::sign_data;

impl AdbDeviceConnection {
    const AUTH_TIME_OUT_SECONDS: f64 = 5.0;
    const AUTH_SIGNATURE: u32 = 2;
    const AUTH_RSA_PUBLIC_KEY: u32 = 3;

    pub async fn adb_connect(&mut self, operation_timeout_s: Option<f64>) -> Result<(), AdbConnectionError> {
        info!("Initiating ADB connection");
        let android_usb_interface = InterfaceType::android_usb();
        self.adb_io_manager.acquire_device(android_usb_interface)
            .await.map_err(|err| AdbDeviceConnection::map_io_error(err))?;

        self.send_cnxn_command(operation_timeout_s).await?;
        let cnxn_transaction_info = AdbTransactionInfo::new(ZERO, ONE);

        let stream = self.read_until_auth_or_open(&cnxn_transaction_info, operation_timeout_s).await;
        tokio::pin!(stream);

        while let Some(adb_message_result) = stream.next().await {
            return match adb_message_result {
                Ok(adb_message_cnxn_response) => {
                    match adb_message_cnxn_response.command() {
                        AUTH_CODE => {
                            info!("Authentication required");
                            self.adb_authenticate(&adb_message_cnxn_response).await
                                .map_err(|e| Unauthorized(e.to_string()))
                        }
                        OPEN_CODE => Ok(()),
                        _ => {
                            error!("Unknown AUTH response: cmd={} arg0={} arg1={}",
                                  adb_message_cnxn_response.command(),
                                  adb_message_cnxn_response.arg0(),
                                  adb_message_cnxn_response.arg1());
                            Err(UnexpectedError(format!(
                                "Unknown AUTH response: {} {} {}",
                                adb_message_cnxn_response.command(),
                                adb_message_cnxn_response.arg0(),
                                adb_message_cnxn_response.arg1()
                            )))
                        }
                    }
                }
                Err(err) => {
                    error!("Connection error: {}", err);
                    Err(err)
                }
            };
        }

        error!("Failed to establish device connection");
        Err(UnexpectedError("Unexpected error while attempting to establish device connection".to_string()))
    }

    async fn adb_authenticate(&self, adb_message: &AdbMessage) -> Result<(), AdbConnectionError> {
        let private_key_path = format!("{}/{}", self.adb_keys_path, ADB_PRIVATE_KEY_FILE);
        let public_key_path = format!("{}/{}", self.adb_keys_path, ADB_PUBLIC_KEY_FILE);

        let signed_token = sign_data(&private_key_path, adb_message.data()).map_err(|err| {
            error!("Failed to sign ADB keys: {}", err);
            Unauthorized(format!("Failed to sign ADB keys. Please verify that the key is valid and properly configured: {}", err))
        })?;

        let auth_message = AdbMessage::new(AUTH_CODE, Self::AUTH_SIGNATURE, ZERO, signed_token);
        self.send_adb_message(&auth_message, None).await?;

        let adb_message_response = self.adb_io_manager.read_adb_message_last_message(Self::AUTH_TIME_OUT_SECONDS)
            .await.map_err(|err| AdbDeviceConnection::map_io_error(err))?;

        if adb_message_response.command() == CNXN_CODE {
            info!("Authentication successful");
            Ok(())
        } else {
            info!("Initial authentication failed, trying public key");
            let string_public_key = read_to_string(&public_key_path)
                .map_err(|err| {
                    error!("Failed to read public ADB keys: {}", err);
                    Unauthorized(format!("Unable to read public adb keys: {}", err))
                })?;

            let auth_message = AdbMessage::new(AUTH_CODE, Self::AUTH_RSA_PUBLIC_KEY, ZERO, string_public_key.into_bytes());
            self.send_adb_message(&auth_message, None).await?;
            self.adb_io_manager.read_adb_message_last_message(Self::AUTH_TIME_OUT_SECONDS)
                .await.map_err(|e| Unauthorized(e.to_string()))?;
            Ok(())
        }
    }
}