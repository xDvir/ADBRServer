
use tokio::net::TcpStream;
use tracing::{info, warn,error};
use crate::adb::io::socket::{send_fail_with_response, send_full_response, send_ok_with_response};
use crate::adb::models::adb_device::AdbDevice;
use crate::adb::server::server::AdbServer;
use crate::constants::OKAY;

const REMOUNT_COMMAND_OPERATION_TIMEOUT_SEC: f64 = 0.1;
const ROOT_COMMAND_OPERATION_TIMEOUT_SEC: f64 = 0.1;
const UNROOT_COMMAND_OPERATION_TIMEOUT_SEC: f64 = 0.1;

impl AdbServer {
    pub async fn handle_reboot_command(adb_device: &AdbDevice, reboot_command: &str, socket: &mut TcpStream) {
        match adb_device.adb_device_connection().adb_reboot(reboot_command, None).await {
            Ok(_) => {
                info!("Reboot command succeeded");
                if let Err(e) = send_ok_with_response(socket, None).await {
                    warn!("Failed to send OK response after reboot: {:?}", e);
                }
            }
            Err(err) => {
                error!("Reboot command failed: {}", err);
                if let Err(e) = send_fail_with_response(socket, Some(err.to_string())).await {
                    warn!("Failed to send FAIL response: {:?}", e);
                }
            }
        }
    }

    pub async fn handle_serialno_command(adb_device: &AdbDevice, socket: &mut TcpStream) {
        let serial_number = adb_device.device_serial_number().to_string();
        if let Err(e) = send_ok_with_response(socket, Some(serial_number)).await {
            warn!("Failed to send OK response with serial number: {:?}", e);
        }
    }

    pub async fn handle_remount_command(adb_device: &AdbDevice, socket: &mut TcpStream) {
        let root_command_response_result = adb_device.adb_device_connection().
            adb_remount(Some(REMOUNT_COMMAND_OPERATION_TIMEOUT_SEC)).await;
        match root_command_response_result {
            Ok(response) => {
                info!("Remount command succeeded");
                if let Err(err) = send_full_response(socket, format!("{}{}", OKAY, response)).await {
                    warn!("Failed to send OK response: {:?}", err);
                }
            }
            Err(err) => {
                error!("Remount command failed: {}", err);
                if let Err(e) = send_fail_with_response(socket, Some(err.to_string())).await {
                    warn!("Failed to send Fail response: {:?}", e);
                }
            }
        }
    }

    pub async fn handle_root_command(adb_device: &AdbDevice, socket: &mut TcpStream) {
        let root_command_response_result = adb_device.adb_device_connection().
            adb_root(Some(ROOT_COMMAND_OPERATION_TIMEOUT_SEC)).await;
        match root_command_response_result {
            Ok(response) => {
                info!("Root command succeeded");
                if let Err(err) = send_full_response(socket, format!("{}{}", OKAY, response)).await {
                    warn!("Failed to send OK response: {:?}", err);
                }
            }
            Err(err) => {
                error!("Root command failed: {}", err);
                if let Err(e) = send_fail_with_response(socket, Some(err.to_string())).await {
                    warn!("Failed to send FAIL response: {:?}", e);
                }
            }
        }
    }

    pub async fn handle_unroot_command(adb_device: &AdbDevice, socket: &mut TcpStream) {
        let unroot_command_response_result = adb_device.adb_device_connection().
            adb_unroot(Some(UNROOT_COMMAND_OPERATION_TIMEOUT_SEC)).await;
        match unroot_command_response_result {
            Ok(response) => {
                info!("Unroot command succeeded");
                if let Err(err) = send_full_response(socket, format!("{}{}", OKAY, response)).await {
                    warn!("Failed to send OK response: {:?}", err);
                }
            }
            Err(err) => {
                warn!("Unroot command failed: {}", err);
                if let Err(e) = send_fail_with_response(socket, Some(err.to_string())).await {
                    warn!("Failed to send FAIL response: {:?}", e);
                }
            }
        }
    }

    pub async fn handle_get_devpath(adb_device: &AdbDevice, socket: &mut TcpStream) {
        let dev_path_result = adb_device.adb_device_connection().adb_get_devpath(None).await;
        match dev_path_result {
            Ok(dev_path) => {
                info!("Get devpath command succeeded");
                if let Err(err) = send_ok_with_response(socket, Some(dev_path)).await {
                    warn!("Failed to send OK response: {:?}", err);
                }
            }
            Err(err) => {
                error!("Get devpath command failed: {}", err);
                if let Err(e) = send_fail_with_response(socket, Some(err.to_string())).await {
                    warn!("Failed to send FAIL response: {:?}", e);
                }
            }
        }
    }
}

