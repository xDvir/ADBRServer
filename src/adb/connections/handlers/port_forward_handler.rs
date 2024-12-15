use std::io;
use std::io::ErrorKind;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, UnixListener};
use tokio::sync::oneshot;
use tracing::{error, info, warn};

use crate::adb::connections::adb_device_connection::AdbDeviceConnection;
use crate::adb::enums::adb_forward_type::ForwardType;
use crate::adb::enums::adb_listener::Listener;
use crate::adb::errors::adb_connection_error::AdbConnectionError;
use crate::adb::errors::adb_connection_error::AdbConnectionError::{PortForwardSetupFailed, UnexpectedError};
use crate::adb::models::adb_port_forward_info::AdbPortForwardInfo;
use crate::constants::{CLSE_CODE, DEFAULT_BUFFER_SIZE, LOCAL_IP, OKAY_CODE};
use crate::utils::utils::ensure_null_terminated;

impl AdbDeviceConnection {
    pub async fn _adb_port_forwarding_set(&self, adb_port_forward_info: &AdbPortForwardInfo, port_forward_result_sender: oneshot::Sender<Result<(), AdbConnectionError>>, operation_timeout_s: Option<f64>) {
        info!("Setting up port forward: {}", adb_port_forward_info.to_string());
        let listener = match self.port_forward_create_listener(adb_port_forward_info).await {
            Ok(listener) => listener,
            Err(err) => {
                error!("Failed to create port forward listener: {}", err);
                let _ = port_forward_result_sender.send(Err(PortForwardSetupFailed(err.to_string())));
                return;
            }
        };

        if let Err(err) = port_forward_result_sender.send(Ok(())) {
            error!("Failed to send port forward setup result: {:?}", err);
            return;
        }

        match listener {
            Listener::Tcp(tcp) => {
                self.handle_tcp_listener(tcp, adb_port_forward_info, operation_timeout_s).await
            }
            Listener::Unix(unix) => {
                self.handle_unix_listener(unix, adb_port_forward_info, operation_timeout_s).await
            }
        }
    }

    async fn handle_tcp_listener(&self, tcp: TcpListener, adb_port_forward_info: &AdbPortForwardInfo, operation_timeout_s: Option<f64>) {
        info!("TCP listener started for {}", adb_port_forward_info.to_string());
        loop {
            tokio::select! {
                result = tcp.accept() => {
                    match result {
                        Ok((stream, _addr)) => {
                            self.port_forward_handle_client(stream, adb_port_forward_info, operation_timeout_s).await;
                        }
                        Err(err) => error!("TCP accept error: {}", err)
                    }
                }
                _ = tokio::task::yield_now() => {}
            }
        }
    }

    async fn handle_unix_listener(&self, unix: UnixListener, adb_port_forward_info: &AdbPortForwardInfo, operation_timeout_s: Option<f64>) {
        info!("Unix listener started for {}", adb_port_forward_info.to_string());
        loop {
            tokio::select! {
                result = unix.accept() => {
                    match result {
                        Ok((stream, _)) => {
                            self.port_forward_handle_client(stream, adb_port_forward_info, operation_timeout_s).await;
                        }
                        Err(err) => error!("Unix socket accept error: {}", err)
                    }
                }
                _ = tokio::task::yield_now() => {}
            }
        }
    }

    async fn port_forward_create_listener(&self, adb_port_forward_info: &AdbPortForwardInfo) -> io::Result<Listener> {
        match adb_port_forward_info.local() {
            ForwardType::Tcp(port) => {
                let ip = IpAddr::from_str(LOCAL_IP)
                    .map_err(|e| io::Error::new(ErrorKind::InvalidInput, e))?;
                let addr = SocketAddr::new(ip, *port);
                let listener = TcpListener::bind(addr).await?;
                Ok(Listener::Tcp(listener))
            }
            ForwardType::LocalAbstract(name) => {
                Ok(Listener::Unix(UnixListener::bind(format!("\0{}", name))?))
            }
            ForwardType::LocalReserved(name) | ForwardType::LocalFilesystem(name) => {
                Ok(Listener::Unix(UnixListener::bind(name)?))
            }
            _ => {
                error!("Unsupported forward type: {:?}", adb_port_forward_info.local());
                Err(io::Error::new(ErrorKind::Other, "Unsupported forward type"))
            }
        }
    }

    async fn port_forward_handle_client<T: AsyncReadExt + AsyncWriteExt + Unpin>(&self, mut stream: T, adb_port_forward_info: &AdbPortForwardInfo, operation_timeout_s: Option<f64>) {
        let mut response_buffer = [0; DEFAULT_BUFFER_SIZE];

        match stream.read(&mut response_buffer).await {
            Ok(0) => return,
            Ok(size) => {
                match self.handle_port_forward_incoming_message(&response_buffer[..size], adb_port_forward_info, operation_timeout_s).await {
                    Ok(response) => {
                        if let Err(err) = stream.write_all(&response).await {
                            warn!("Write response error: {}", err);
                        }
                    }
                    Err(err) => warn!("Handle message error: {}", err)
                }
            }
            Err(err) => warn!("Read error from client: {}", err)
        }
    }

    async fn handle_port_forward_incoming_message(&self, buffer: &[u8], adb_port_forward_info: &AdbPortForwardInfo, operation_timeout_s: Option<f64>) -> Result<Vec<u8>, AdbConnectionError> {
        let expected_responses = &[OKAY_CODE, CLSE_CODE];
        let remote_info = ensure_null_terminated(adb_port_forward_info.remote_with_type());

        let mut adb_open_command_info = self.send_open_command(&remote_info, operation_timeout_s).await?;
        let adb_response = self.read_expected_packet(expected_responses, None, &adb_open_command_info, operation_timeout_s).await?;

        match adb_response.command() {
            OKAY_CODE => {}
            CLSE_CODE => {
                error!("Port forward rejected: {}", adb_port_forward_info.to_string());
                return Err(PortForwardSetupFailed(format!("Port forwarding rejected by device {}", adb_port_forward_info.to_string())));
            }
            cmd => {
                error!("Unexpected port forward response: {}", cmd);
                return Err(UnexpectedError(format!("Unexpected response: {:?}", cmd)));
            }
        }

        adb_open_command_info.set_receive_packet_id(adb_response.arg0());
        self.send_wrte_command(&adb_open_command_info, buffer, operation_timeout_s).await?;

        self.read_all_response(&adb_open_command_info, operation_timeout_s).await
            .map_err(|err| UnexpectedError(format!("Response read error: {:?}", err)).into())
    }
}