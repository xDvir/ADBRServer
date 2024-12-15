use std::pin::Pin;

use futures::Stream;
use futures::stream::unfold;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio_stream::StreamExt;
use tracing::{error, warn};

use crate::adb::connections::adb_device_connection::AdbDeviceConnection;
use crate::adb::errors::adb_connection_error::AdbConnectionError;
use crate::adb::errors::adb_connection_error::AdbConnectionError::UnexpectedError;
use crate::adb::models::adb_message::AdbMessage;
use crate::adb::models::adb_transaction_info::AdbTransactionInfo;
use crate::constants::{AUTH_CODE, CLSE_CODE, OPEN_CODE, WRTE_CODE};

impl AdbDeviceConnection {
    pub async fn read_all_response(&self, adb_transaction_info: &AdbTransactionInfo, operation_timeout_s: Option<f64>) -> Result<Vec<u8>, AdbConnectionError> {
        let mut response = Vec::new();
        let mut stream = self.read_until_no_packet_left(adb_transaction_info, operation_timeout_s).await;

        while let Some(msg_result) = stream.next().await {
            match msg_result {
                Ok(msg) => {
                    if msg.command() == CLSE_CODE {
                        break;
                    }
                    response.extend_from_slice(&msg.data());
                }
                Err(AdbConnectionError::Timeout) => break,
                Err(err) => {
                    error!("Failed to read response : {}", err);
                    return Err(err);
                }
            }
        }

        Ok(response)
    }

    pub async fn read_and_write_all_response(&self, adb_transaction_info: &AdbTransactionInfo, socket: &mut TcpStream, operation_timeout_s: Option<f64>) -> Result<(), AdbConnectionError> {
        let mut stream = self.read_until_no_packet_left(adb_transaction_info, operation_timeout_s).await;

        while let Some(msg_result) = stream.next().await {
            match msg_result {
                Ok(msg) => {
                    if msg.command() == CLSE_CODE {
                        break;
                    }
                    if let Err(err) = self.write_message_to_socket(&msg, socket).await {
                        error!("Failed to write message to socket for {}", err);
                        return Err(err);
                    }
                }
                Err(AdbConnectionError::Timeout) => break,
                Err(err) => {
                    error!("Failed to read message : {}", err);
                    return Err(err);
                }
            }
        }
        Ok(())
    }

    async fn write_message_to_socket(&self, msg: &AdbMessage, socket: &mut TcpStream) -> Result<(), AdbConnectionError> {
        match socket.write_all(msg.data()).await {
            Ok(_) => {
                if let Err(err) = socket.flush().await {
                    warn!("Failed to flush socket: {}", err);
                }
                Ok(())
            }
            Err(err) => Err(UnexpectedError(err.to_string()))
        }
    }

    pub async fn read_expected_packet(&self, expected_responses: &[u32], excepted_data: Option<String>, adb_info: &AdbTransactionInfo, operation_timeout_s: Option<f64>) -> Result<AdbMessage, AdbConnectionError> {
        self.adb_io_manager.read_adb_message_with_packet_store(expected_responses, excepted_data, adb_info, operation_timeout_s).await.map_err(AdbDeviceConnection::map_io_error)
    }

    pub async fn read_until_no_packet_left<'a>(&'a self, adb_info: &'a AdbTransactionInfo, operation_timeout_s: Option<f64>) -> Pin<Box<dyn Stream<Item=Result<AdbMessage, AdbConnectionError>> + Send + 'a>> {
        let stream = self.initialize_read_stream(&[], adb_info, operation_timeout_s).await;
        Self::create_message_stream(stream).await
    }

    pub async fn read_until_auth_or_open<'a>(&'a self, adb_info: &'a AdbTransactionInfo, operation_timeout_s: Option<f64>) -> Pin<Box<dyn Stream<Item=Result<AdbMessage, AdbConnectionError>> + Send + 'a>> {
        let stream = self.initialize_read_stream(&[AUTH_CODE, OPEN_CODE], adb_info, operation_timeout_s).await;
        Self::create_message_stream(stream).await
    }

    async fn create_message_stream<'a>(stream: Pin<Box<dyn Stream<Item=Result<AdbMessage, AdbConnectionError>> + Send + 'a>>) -> Pin<Box<dyn Stream<Item=Result<AdbMessage, AdbConnectionError>> + Send + 'a>> {
        let pinned_stream = Box::pin(stream);

        Box::pin(unfold(pinned_stream, move |mut stream| async move {
            while let Some(result) = stream.next().await {
                return match result {
                    Ok(adb_message) => Some((Ok(adb_message), stream)),
                    Err(err) => Some((Err(err), stream)),
                };
            }
            None
        }))
    }

    async fn initialize_read_stream<'a>(&'a self, expected_cmds: &'a [u32], adb_info: &'a AdbTransactionInfo, operation_timeout_s: Option<f64>) -> Pin<Box<dyn Stream<Item=Result<AdbMessage, AdbConnectionError>> + Send + 'a>> {
        self.read_until(expected_cmds, adb_info, operation_timeout_s)
    }

    fn read_until<'a>(&'a self, expected_cmds: &'a [u32], adb_info: &'a AdbTransactionInfo, operation_timeout_s: Option<f64>) -> Pin<Box<dyn Stream<Item=Result<AdbMessage, AdbConnectionError>> + Send + 'a>> {
        let state = (self, expected_cmds, adb_info, operation_timeout_s);

        Box::pin(unfold(state, move |state| async move {
            let (slf, exp_cmds, info, timeout) = state;

            match slf.read_expected_packet(exp_cmds, None, info, timeout).await {
                Ok(adb_message) => {
                    let next_value = if adb_message.command() == WRTE_CODE {
                        match slf.send_okay_command(info, timeout).await {
                            Ok(()) => Ok(adb_message),
                            Err(err) => Err(err),
                        }
                    } else {
                        Ok(adb_message)
                    };

                    Some((next_value, (slf, exp_cmds, info, timeout)))
                }
                Err(err) => {
                    Some((Err(err), (slf, exp_cmds, info, timeout)))
                }
            }
        }))
    }
}