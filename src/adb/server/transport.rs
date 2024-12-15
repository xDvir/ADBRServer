use std::sync::Arc;
use tokio::net::TcpStream;
use tracing::{error, warn};
use crate::adb::enums::adb_device_status::AdbDeviceStatus;
use crate::adb::enums::adb_device_transport::AdbDeviceTransport;
use crate::adb::errors::adb_server_error::AdbServerError;
use crate::adb::io::socket::{send_fail_with_response, send_ok_with_response};
use crate::adb::models::adb_device::AdbDevice;
use crate::adb::server::server::{ADB_SERVER_INSTANCE, AdbServer};

impl AdbServer {
    pub async fn execute_transport_command(socket: &mut TcpStream, device_transport: AdbDeviceTransport) -> Option<Arc<AdbDevice>> {
        match Self::get_adb_device_by_device_transport(device_transport.clone()) {
            Ok(adb_device) => {
                match send_ok_with_response(socket, None).await {
                    Ok(_) => Some(adb_device),
                    Err(_) => {
                        None
                    }
                }
            }
            Err(err) => {
                if let Err(err) = send_fail_with_response(socket, Some(err.to_string())).await {
                    warn!("Failed to send failure response to client: {}", err);
                }
                None
            }
        }
    }

    fn get_adb_device_by_device_transport(device_transport: AdbDeviceTransport) -> Result<Arc<AdbDevice>, AdbServerError> {
        let all_serials_by_transport = Self::get_serials_number_by_device_transport(device_transport.clone());
        let chosen_serial_number = device_transport.get_serial();

        match (chosen_serial_number, all_serials_by_transport.len()) {
            (None, 0) => Err(AdbServerError::NoAvailableDevices()),
            (None, 1) => {
                let serial = &all_serials_by_transport[0];
                Self::get_adb_device_by_serial(serial)
            }
            (None, _) => Err(AdbServerError::MultipleDeviceDetected()),
            (Some(serial), _) => {
                if all_serials_by_transport.contains(&serial) {
                    Self::get_adb_device_by_serial(serial)
                } else {
                    Err(AdbServerError::DeviceNotFound(serial.to_string()))
                }
            }
        }
    }
    fn get_adb_device_by_serial(serial: &str) -> Result<Arc<AdbDevice>, AdbServerError> {
        match ADB_SERVER_INSTANCE.adb_devices_hashmap.get(serial) {
            Some(entry) => {
                entry
                    .value()
                    .clone()
                    .ok_or_else(|| {
                        error!("Error, Adb Device is none but still returned as available");
                        AdbServerError::DeviceNotFound(serial.to_string())
                    })
            }
            None => Err(AdbServerError::DeviceNotFound(serial.to_string())),
        }
    }
    fn get_serials_number_by_device_transport(device_transport: AdbDeviceTransport) -> Vec<String> {
        ADB_SERVER_INSTANCE.adb_devices_hashmap
            .iter()
            .filter_map(|entry| {
                let (device_serial_number, adb_device_option) = entry.pair();
                if let Some(adb_device) = adb_device_option.as_ref() {
                    if *adb_device.adb_device_status() == AdbDeviceStatus::Available {
                        match &device_transport {
                            AdbDeviceTransport::Any => Some(device_serial_number.clone()),
                            AdbDeviceTransport::EmulatorAny if adb_device.is_emulator_device() => {
                                Some(device_serial_number.clone())
                            }
                            AdbDeviceTransport::UsbAny if adb_device.is_usb_device() => {
                                Some(device_serial_number.clone())
                            }
                            AdbDeviceTransport::Emulator(serial_number)
                            if adb_device.is_emulator_device() && serial_number == device_serial_number => {
                                Some(device_serial_number.clone())
                            }
                            AdbDeviceTransport::Usb(serial_number)
                            if adb_device.is_usb_device() && serial_number == device_serial_number => {
                                Some(device_serial_number.clone())
                            }
                            _ => None,
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    }
    pub async fn handle_no_device_selected(socket: &mut TcpStream) {
        let error_message = AdbServerError::NoTransportSelected().to_string();
        warn!("{}",error_message);
        if let Err(err) = send_fail_with_response(socket, Some(error_message)).await {
            warn!("Failed to send FAIL response: {:?}", err);
        }
    }
}