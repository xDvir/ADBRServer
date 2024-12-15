use tracing::{error, warn};
use tokio::net::TcpStream;
use crate::adb::errors::adb_server_error::AdbServerError;
use crate::adb::io::socket::{send_fail_with_response, send_ok_with_response};
use crate::adb::models::adb_device::AdbDevice;
use crate::adb::server::server::AdbServer;

impl AdbServer {

    const SYNC_COMMAND_TIMEOUT: f64 = 5.0;

    pub async fn handle_sync_command(adb_device: &AdbDevice, socket: &mut TcpStream) {
        match send_ok_with_response(socket, None).await {
            Ok(_) => {
                match adb_device.adb_device_connection().handle_sync_mode(socket, Some(Self::SYNC_COMMAND_TIMEOUT)).await {
                    Ok(_) => {}
                    Err(err) => {
                        error!("Sync operation error: {}", err);
                        if let Err(err) = send_fail_with_response(socket, Some(err.to_string())).await {
                            warn!("Failed to send FAIL response for sync error: {:?}", err);
                        }
                    }
                }
            }
            Err(err) => {
                warn!("Failed to send OK response for sync command: {:?}", err);
            }
        }
    }

    pub async fn handle_invalid_sync_command(socket: &mut TcpStream) {
        let error_message = AdbServerError::SyncError(
            String::from("You should send a SYNC command before any other sync command.")
        ).to_string();

        if let Err(err) = send_fail_with_response(socket, Some(error_message)).await {
            warn!("Failed to send FAIL response for invalid sync command: {:?}", err);
        }
    }
}