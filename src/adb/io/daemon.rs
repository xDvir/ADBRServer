use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;

use crate::adb::errors::adb_io_error::AdbIoError;
use crate::adb::errors::adb_io_error::AdbIoError::{DeviceConnectionError, UnexpectedError};
use crate::adb::models::adb_message::AdbMessage;
use crate::adb::models::adb_transaction_info::AdbTransactionInfo;
use crate::adb::packet_store::AdbPacketStore;
use crate::constants::{ADB_MESSAGE_SIZE, CLSE_CODE};
use crate::transport::enums::interface_type::InterfaceType;
use crate::transport::errors::transport_error::TransportError;
use crate::transport::transport::Transport;

pub struct AdbDeviceIo {
    transport: Arc<Mutex<Box<dyn Transport>>>,
    packet_store: Arc<AdbPacketStore>,
}

impl AdbDeviceIo {
    const ADB_HEADER_TIMEOUT_SECONDS: f64 = 0.05;
    const SLEEP_BETWEEN_PACKET_GETTING_SEC: f64 = 0.5;
    const READING_MIN_TIMEOUT: f64 = 0.0;

    pub fn new(transport: Box<dyn Transport>) -> Self {
        Self {
            transport: Arc::new(Mutex::new(transport)),
            packet_store: Arc::new(AdbPacketStore::new()),
        }
    }

    pub async fn acquire_device(&mut self, device_type: InterfaceType) -> Result<(), AdbIoError> {
        self.transport.lock().await.acquire_device(device_type).map_err(|err| DeviceConnectionError(err.to_string()))
    }

    pub async fn release_device(&self) -> Result<(), AdbIoError> {
        match self.transport.lock().await.release_device() {
            Ok(_) => Ok(()),
            Err(TransportError::ConnectionError(_)) => Ok(()),
            Err(err) => {
                Err(UnexpectedError(format!("Failed to release device: {}", err)))
            }
        }
    }

    pub async fn verify_connection_status(&self) -> Result<(), AdbIoError> {
        self.transport.lock().await.verify_connection_status().map_err(|err| DeviceConnectionError(err.to_string()))
    }

    pub async fn write_bytes(&self, adb_message: &AdbMessage, transport_timeout_s: Option<f64>) -> Result<(), AdbIoError> {
        let packed_message = adb_message.pack_message();
        let transport = self.transport.lock().await;

        transport.bulk_write(&packed_message, transport_timeout_s)
            .map_err(Self::map_transport_error)?;

        if !adb_message.data().is_empty() {
            transport.bulk_write(adb_message.data(), transport_timeout_s)
                .map_err(Self::map_transport_error)?;
        }

        Ok(())
    }

    pub async fn read_adb_message_last_message(&self, data_timeout_s: f64) -> Result<AdbMessage, AdbIoError> {
        let transport = self.transport.lock().await;
        let (header_msg, data_length, data_checksum) = self.read_message_header(&transport, Self::ADB_HEADER_TIMEOUT_SECONDS).await?;

        if data_length == 0 {
            return Ok(header_msg);
        }

        self.read_message_data(&transport, header_msg, data_length, data_checksum, data_timeout_s).await
    }

    async fn read_message_data(&self, transport: &Box<dyn Transport>, mut header_msg: AdbMessage, data_length: u32, data_checksum: u32, timeout_s: f64) -> Result<AdbMessage, AdbIoError> {
        let message_data = transport.bulk_read(data_length as usize, timeout_s)
            .map_err(Self::map_transport_error)?;

        header_msg.set_data(message_data);

        if header_msg.checksum() != data_checksum {
            return Err(UnexpectedError(
                format!("Checksum mismatch: received {} != expected {}",
                        header_msg.checksum(), data_checksum)
            ));
        }

        Ok(header_msg)
    }

    async fn read_message_header(&self, transport: &Box<dyn Transport>, timeout_s: f64) -> Result<(AdbMessage, u32, u32), AdbIoError> {
        let bytes_response = transport.bulk_read(ADB_MESSAGE_SIZE, timeout_s)
            .map_err(Self::map_transport_error)?;

        let (command, arg0, arg1, data_length, data_checksum) = AdbMessage::unpack_message(&bytes_response)
            .map_err(|err| AdbIoError::ParseError(err.to_string()))?;

        let header_msg = AdbMessage::new(command, arg0, arg1, vec![]);
        Ok((header_msg, data_length, data_checksum))
    }

    pub async fn read_adb_message_with_packet_store(&self, expected_cmds: &[u32], excepted_data: Option<String>, adb_info: &AdbTransactionInfo, operation_timeout_s: Option<f64>) -> Result<AdbMessage, AdbIoError> {
        let excepted_data = excepted_data.as_ref();
        let start_time = std::time::Instant::now();

        loop {
            if let Some(timeout) = operation_timeout_s {
                if start_time.elapsed().as_secs_f64() > timeout {
                    return Err(AdbIoError::TimeoutError);
                }
            }

            {
                if let Some((cmd, arg0, arg1, data)) = self.packet_store.get_packet(adb_info.receive_packet_id(), adb_info.sent_packet_id(), expected_cmds) {
                    if (expected_cmds.contains(&cmd) || expected_cmds.is_empty()) &&
                        (excepted_data.map_or(true, |expected| expected.as_bytes() == data)) {
                        if cmd == CLSE_CODE {
                            self.packet_store.clear_packet(arg0, arg1);
                        }
                        return Ok(AdbMessage::new(cmd, arg0, arg1, data));
                    } else {
                        self.packet_store.put_packet(arg0, arg1, cmd, data.clone());
                    }
                }
            }

            let remaining_timeout = if let Some(timeout) = operation_timeout_s {
                (timeout - start_time.elapsed().as_secs_f64()).max(Self::READING_MIN_TIMEOUT)
            } else {
                Self::READING_MIN_TIMEOUT
            };

            let adb_message = match self.read_adb_message_last_message(remaining_timeout).await {
                Ok(adb_message) => {
                    adb_message
                }
                Err(AdbIoError::TimeoutError) => {
                    if let Some(timeout) = operation_timeout_s {
                        let elapsed = start_time.elapsed().as_secs_f64();
                        if elapsed + Self::SLEEP_BETWEEN_PACKET_GETTING_SEC <= timeout {
                            tokio::time::sleep(Duration::from_secs_f64(Self::SLEEP_BETWEEN_PACKET_GETTING_SEC)).await;
                        }
                    } else {
                        tokio::time::sleep(Duration::from_secs_f64(Self::SLEEP_BETWEEN_PACKET_GETTING_SEC)).await;
                    }
                    continue;
                }
                Err(err) => {
                    return Err(err);
                }
            };

            if !adb_info.args_match(adb_message.arg0(), adb_message.arg1()) {
                self.packet_store.put_packet(
                    adb_message.arg0(),
                    adb_message.arg1(),
                    adb_message.command(),
                    adb_message.data().clone(),
                );
            } else if expected_cmds.contains(&adb_message.command()) || expected_cmds.is_empty() &&
                (excepted_data.map_or(true, |data| data.as_bytes() == adb_message.data())) {
                if adb_message.command() == CLSE_CODE {
                    self.packet_store.clear_packet(adb_message.arg0(), adb_message.arg1());
                }
                return Ok(adb_message);
            } else {
                self.packet_store.put_packet(
                    adb_message.arg0(),
                    adb_message.arg1(),
                    adb_message.command(),
                    adb_message.data().clone(),
                );
            }
        }
    }

    fn map_transport_error(err: TransportError) -> AdbIoError {
        match err {
            TransportError::Timeout => AdbIoError::TimeoutError,
            TransportError::DeviceNotFound => DeviceConnectionError("Device not found".to_string()),
            TransportError::ConnectionError(msg) => AdbIoError::ConnectionClosed(msg),
            TransportError::Unauthorized(msg) => DeviceConnectionError(msg),
            TransportError::CommunicationError(msg) => AdbIoError::CommunicationError(msg),
            _ => UnexpectedError(err.to_string()),
        }
    }
}


