use tokio::net::TcpStream;
use tracing::{info, warn};
use crate::adb::io::socket::{send_fail_with_response, send_full_response};
use crate::adb::models::adb_device::AdbDevice;
use crate::adb::server::server::AdbServer;
use crate::constants::OKAY;

const ENABLE_VERITY__COMMAND_OPERATION_TIMEOUT_SEC: f64 = 0.1;
const DISABLE_VERITY__COMMAND_OPERATION_TIMEOUT_SEC: f64 = 0.1;

impl AdbServer {
    pub async fn handle_enable_verity_command(adb_device: &AdbDevice, socket: &mut TcpStream) {
        let enable_verity_command_response_result = adb_device.adb_device_connection().
            adb_enable_verity(Some(ENABLE_VERITY__COMMAND_OPERATION_TIMEOUT_SEC)).await;
        match enable_verity_command_response_result {
            Ok(response) => {
                info!("Enable verity command succeeded");
                if let Err(err) = send_full_response(socket, format!("{}{}", OKAY, response)).await {
                    warn!("Failed to send OK response: {:?}", err);
                }
            }
            Err(err) => {
                warn!("Enable verity command failed: {}", err);
                if let Err(err) = send_fail_with_response(socket, Some(err.to_string())).await {
                    warn!("Failed to send FAIL response: {:?}", err);
                }
            }
        }
    }
    pub async fn handle_disable_verity_command(adb_device: &AdbDevice, socket: &mut TcpStream) {
        let disable_verity_command_response_result = adb_device.adb_device_connection().
            adb_disable_verity(Some(DISABLE_VERITY__COMMAND_OPERATION_TIMEOUT_SEC)).await;
        match disable_verity_command_response_result {
            Ok(response) => {
                info!("Disable verity command succeeded");
                if let Err(err) = send_full_response(socket, format!("{}{}", OKAY, response)).await {
                    warn!("Failed to send OK response: {:?}", err);
                }
            }
            Err(err) => {
                warn!("Disable verity command failed: {}", err);
                if let Err(err) = send_fail_with_response(socket, Some(err.to_string())).await {
                    warn!("Failed to send FAIL response: {:?}", err);
                }
            }
        }
    }
}

