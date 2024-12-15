use std::io::{self, ErrorKind};
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpStream, UnixStream};
use tokio::spawn;
use tokio::sync::{mpsc, oneshot, watch};
use tokio::sync::Mutex;
use tracing::{error, info,warn};

use crate::adb::connections::adb_device_connection::AdbDeviceConnection;
use crate::adb::enums::adb_forward_type::ForwardType;
use crate::adb::errors::adb_connection_error::AdbConnectionError;
use crate::adb::errors::adb_connection_error::AdbConnectionError::{PortReverseSetupFailed, UnexpectedError};
use crate::adb::models::adb_port_reverse_info::AdbPortReverseInfo;
use crate::adb::models::adb_transaction_info::AdbTransactionInfo;
use crate::constants::{ABSTRACT_SOCKET_PREFIX, DEFAULT_BUFFER_SIZE, DEV_SOCKET_PREFIX, LOCAL_IP, OPEN_CODE, RESERVED_SOCKET_PREFIX, REVERSE_FORWARD_COMMAND, ZERO};
use crate::utils::utils::ensure_null_terminated;

trait AsyncStream: AsyncRead + AsyncWrite + Unpin {}

impl<T: AsyncRead + AsyncWrite + Unpin> AsyncStream for T {}

impl AdbDeviceConnection {
    const WAIT_FOR_HOST_RESPONSE_SEC: f64 = 0.5;
    const SLEEP_BETWEEN_RECONNECT_SEC: f64 = 1.0;
    const CHANNEL_BUFFER_SIZE: usize = 10;

    pub async fn _adb_port_reverse_set(self: Arc<Self>, adb_port_reverse_info: &AdbPortReverseInfo, port_reverse_result_sender: oneshot::Sender<Result<(), AdbConnectionError>>, operation_timeout_s: Option<f64>) {
        info!("Initializing port reverse for: {}", adb_port_reverse_info.to_string());
        let reverse_command = format!("{}{};{}", REVERSE_FORWARD_COMMAND, adb_port_reverse_info.device_with_type(), adb_port_reverse_info.host_with_type());

        let init_result: Result<(), AdbConnectionError> = async {
            let mut transaction_info = self.send_open_command(&reverse_command, operation_timeout_s).await?;

            let okay = self.read_okay_response(&transaction_info, operation_timeout_s).await?;
            transaction_info.set_receive_packet_id(okay.arg0());

            self.read_wrte_response(&transaction_info, operation_timeout_s).await?;
            self.send_okay_command(&transaction_info, operation_timeout_s).await?;

            self.read_clse_response(&transaction_info, operation_timeout_s).await?;
            self.send_okay_command(&transaction_info, operation_timeout_s).await
        }.await;

        if let Err(err) = init_result {
            if let Err(send_err) = port_reverse_result_sender.send(Err(PortReverseSetupFailed(format!("Protocol setup failed: {}", err)))) {
                info!("Failed to send error result: {:?}", send_err);
            }
            return;
        }

        let mut port_reverse_result_sender = Some(port_reverse_result_sender);

        let host_stream = loop {
            match Self::connect_to_host_port(adb_port_reverse_info).await {
                Ok(stream) => {
                    if let Some(sender) = port_reverse_result_sender {
                        let _ = sender.send(Ok(()));
                    }
                    break Arc::new(Mutex::new(stream));
                }
                Err(err) => {
                    match err.kind() {
                        ErrorKind::ConnectionRefused => {
                            info!("Connection refused, retrying after delay");
                            if let Some(sender) = port_reverse_result_sender {
                                let _ = sender.send(Ok(()));
                                port_reverse_result_sender = None
                            }
                            tokio::time::sleep(Duration::from_secs_f64(Self::SLEEP_BETWEEN_RECONNECT_SEC)).await;
                            continue;
                        }
                        _ => {
                            let error_msg = match err.kind() {
                                ErrorKind::InvalidInput => {
                                    error!("Invalid host address configuration: {}", err);
                                    format!("Invalid host address configuration: {}", err)
                                }
                                ErrorKind::ConnectionReset => {
                                    error!("Connection reset while connecting to host port {}", adb_port_reverse_info.host_with_type());
                                    format!("Connection reset while connecting to host port {}", adb_port_reverse_info.host_with_type())
                                }
                                ErrorKind::ConnectionAborted => {
                                    error!("Connection aborted while connecting to host port {}", adb_port_reverse_info.host_with_type());
                                    format!("Connection aborted while connecting to host port {}", adb_port_reverse_info.host_with_type())
                                }
                                ErrorKind::NotFound => {
                                    error!("Host socket path not found: {}", err);
                                    format!("Host socket path not found: {}", err)
                                }
                                ErrorKind::PermissionDenied => {
                                    error!("Permission denied accessing host port {}: {}", adb_port_reverse_info.host_with_type(), err);
                                    format!("Permission denied accessing host port {}: {}", adb_port_reverse_info.host_with_type(), err)
                                }
                                ErrorKind::AddrInUse => {
                                    error!("Host address already in use: {}", adb_port_reverse_info.host_with_type());
                                    format!("Host address already in use: {}", adb_port_reverse_info.host_with_type())
                                }
                                ErrorKind::AddrNotAvailable => {
                                    error!("Host address not available: {}", adb_port_reverse_info.host_with_type());
                                    format!("Host address not available: {}", adb_port_reverse_info.host_with_type())
                                }
                                ErrorKind::Unsupported => {
                                    error!("Unsupported connection type: {}", err);
                                    format!("Unsupported connection type: {}", err)
                                }
                                _ => {
                                    error!("Unexpected error connecting to host: {}", err);
                                    format!("Unexpected error connecting to host: {}", err)
                                }
                            };
                            if let Some(sender) = port_reverse_result_sender {
                                let _ = sender.send(Err(PortReverseSetupFailed(error_msg)));
                                return;
                            }
                        }
                    }
                }
            };
        };


        if let Err(err) = self.handle_reverse_message(adb_port_reverse_info, operation_timeout_s, host_stream).await {
            error!("Reverse port message handling error: {}", err);
        }
    }

    async fn handle_reverse_message(self: Arc<Self>, adb_port_reverse_info: &AdbPortReverseInfo, operation_timeout_s: Option<f64>, host_stream: Arc<Mutex<Box<dyn AsyncStream + Send>>>) -> Result<(), AdbConnectionError> {
        let (reverse_message_sender, reverse_message_receiver) = mpsc::channel(Self::CHANNEL_BUFFER_SIZE);

        let self_clone = self.clone();
        let host_stream_clone = host_stream.clone();
        let adb_port_reverse_info_clone = adb_port_reverse_info.clone();
        let (_shutdown_tx, mut shutdown_rx) = watch::channel(());

        spawn(async move {
            tokio::select! {
            _ = shutdown_rx.changed() => {
                    return;
            }
            _ = self_clone.handle_bidirectional_stream_relay(host_stream_clone,&adb_port_reverse_info_clone, operation_timeout_s, reverse_message_receiver) => {}
        }
        });

        loop {
            let transaction_info = self.initialize_reverse_connection(adb_port_reverse_info, operation_timeout_s).await?;
            reverse_message_sender.send(transaction_info).await
                .map_err(|_| UnexpectedError("Failed to send initial transaction info".into()))?;
        }
    }

    async fn initialize_reverse_connection(&self, adb_port_reverse_info: &AdbPortReverseInfo, operation_timeout_s: Option<f64>) -> Result<AdbTransactionInfo, AdbConnectionError> {
        let open_transaction_info = &AdbTransactionInfo::new(ZERO, ZERO);
        let open_response = self.read_expected_packet(&[OPEN_CODE], Some(ensure_null_terminated(adb_port_reverse_info.host_with_type())), open_transaction_info, None).await?;
        let transaction_info = AdbTransactionInfo::new(self.get_last_packet_id()?, open_response.arg0());
        self.send_okay_command(&transaction_info, operation_timeout_s).await?;
        Ok(transaction_info)
    }

    async fn get_initial_transaction_info(reverse_message_receiver: &mut mpsc::Receiver<AdbTransactionInfo>) -> AdbTransactionInfo {
        loop {
            match reverse_message_receiver.recv().await {
                Some(info) => break info,
                None => {
                    continue;
                }
            }
        }
    }


    async fn handle_bidirectional_stream_relay(&self, host_stream: Arc<Mutex<Box<dyn AsyncStream + Send>>>, adb_port_reverse_info: &AdbPortReverseInfo, operation_timeout_s: Option<f64>, mut reverse_message_receiver: mpsc::Receiver<AdbTransactionInfo>) {
        let mut response_buffer = [0; DEFAULT_BUFFER_SIZE];
        let mut current_transaction_info = Self::get_initial_transaction_info(&mut reverse_message_receiver).await;

        loop {
            if let Ok(new_info) = reverse_message_receiver.try_recv() {
                current_transaction_info = new_info;
            }

            let read_result = {
                let mut stream = host_stream.lock().await;
                tokio::time::timeout(Duration::from_secs_f64(Self::WAIT_FOR_HOST_RESPONSE_SEC), stream.read(&mut response_buffer)).await
            };

            match read_result {
                Ok(Ok(size)) if size > ZERO as usize => {
                    let data_to_send = response_buffer[..size].to_vec();
                    if let Ok(_) = self.send_wrte_command(&current_transaction_info, &data_to_send, operation_timeout_s).await {
                        if let Err(err) = self.read_okay_response(&current_transaction_info, operation_timeout_s).await {
                            warn!("Failed to read okay response: {}", err);
                        }
                    } else {
                        warn!("Failed to send write command");
                    }
                }
                Ok(Ok(0)) => {
                    match Self::attempt_reconnection(adb_port_reverse_info).await {
                        Ok(new_stream) => {
                            let mut guard = host_stream.lock().await;
                            *guard = new_stream;
                            info!("Successfully reconnected to host");
                        }
                        Err(err) => {
                            error!("Terminating relay due to reconnection failure: {}", err);
                            return;
                        }
                    }
                }
                Ok(Err(err)) if err.kind() == ErrorKind::ConnectionReset ||
                    err.kind() == ErrorKind::ConnectionAborted ||
                    err.kind() == ErrorKind::BrokenPipe => {
                    info!("Connection lost ({}), attempting reconnection...", err.kind());
                    match Self::attempt_reconnection(adb_port_reverse_info).await {
                        Ok(new_stream) => {
                            let mut guard = host_stream.lock().await;
                            *guard = new_stream;
                            info!("Successfully reconnected to host");
                        }
                        Err(err) => {
                            error!("Terminating relay due to reconnection failure: {}", err);
                            return;
                        }
                    }
                }
                _ => {}
            }

            let wrte_response = match self.read_wrte_response(&current_transaction_info, operation_timeout_s).await
            {
                Ok(response) => {
                    if let Err(err) = self.send_okay_command(&current_transaction_info, operation_timeout_s).await {
                        warn!("Failed to send okay command: {}", err);
                    }
                    Some(response)
                }
                Err(AdbConnectionError::Timeout) => None,
                Err(err) => {
                    error!("Failed to read write response: {}", err);
                    break;
                }
            };

            if let Some(write_response) = wrte_response {
                if let Err(err) = host_stream.lock().await.write_all(write_response.data()).await {
                    error!("Failed to write data to host stream: {}", err);
                }
            }
        }
    }

    async fn connect_to_host_port(adb_port_reverse_info: &AdbPortReverseInfo) -> io::Result<Box<dyn AsyncStream + Send>> {
        let path = match adb_port_reverse_info.host() {
            ForwardType::Tcp(port) => {
                let addr = SocketAddr::new(
                    IpAddr::from_str(LOCAL_IP).map_err(|e| io::Error::new(ErrorKind::InvalidInput, e))?,
                    *port,
                );
                return Ok(Box::new(TcpStream::connect(addr).await?));
            }
            ForwardType::LocalAbstract(name) => format!("{}{}", ABSTRACT_SOCKET_PREFIX, name),
            ForwardType::LocalReserved(name) => format!("{}{}", RESERVED_SOCKET_PREFIX, name),
            ForwardType::LocalFilesystem(name) => name.clone(),
            ForwardType::Dev(name) => format!("{}{}", DEV_SOCKET_PREFIX, name),
            ForwardType::Jdwp(pid) => return Err(io::Error::new(
                ErrorKind::Unsupported,
                format!("JDWP not supported for reverse connections (pid: {})", pid),
            )),
        };

        Self::connect_unix_socket(&path).await
    }

    async fn attempt_reconnection(port_info: &AdbPortReverseInfo) -> Result<Box<dyn AsyncStream + Send>, AdbConnectionError> {
        match Self::connect_to_host_port(port_info).await {
            Ok(stream) => {
                Ok(stream)
            }
            Err(err) => Err(UnexpectedError(format!("Reconnection failed: {}", err))),
        }
    }

    async fn connect_unix_socket(path: &str) -> io::Result<Box<dyn AsyncStream + Send>> {
        let stream = UnixStream::connect(path).await
            .map_err(|e| io::Error::new(ErrorKind::ConnectionRefused,
                                        format!("Failed to connect to Unix socket {}: {}", path, e)))?;
        Ok(Box::new(stream))
    }
}