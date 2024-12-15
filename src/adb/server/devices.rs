use tracing::{error};
use tokio::net::TcpStream;
use crate::adb::io::socket::send_ok_with_response;
use crate::adb::server::server::{ADB_SERVER_INSTANCE, AdbServer};

impl AdbServer{
    pub async fn adb_devices(client_socket: &mut TcpStream) {
        let response = Self::get_active_devices();
        if let Err(err) = send_ok_with_response(client_socket, Some(response)).await {
            error!("Failed get adb devices {}", err);
        }
    }
     fn get_active_devices() -> String {
        let mut devices_result = String::new();
        if !ADB_SERVER_INSTANCE.adb_devices_hashmap.is_empty() {
            for entry in ADB_SERVER_INSTANCE.adb_devices_hashmap.iter() {
                let (serial_number, adb_device) = entry.pair();
                match adb_device {
                    None => {}
                    Some(adb_device) => {
                        devices_result.push_str(&format!("{} {}\n", serial_number, adb_device.adb_device_status()));
                    }
                }
            }
            devices_result
        } else {
            String::from("No devices found")
        }
    }
}