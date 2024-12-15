use std::time::Duration;
use rusb::{Context, Device, DeviceHandle, Direction, InterfaceDescriptor, TransferType};
use crate::constants::{DEFAULT_BUFFER_SIZE};
use crate::transport::transport::Transport;
use crate::transport::enums::interface_type::InterfaceType;
use rusb::Error as UsbError;
use crate::transport::errors::transport_error::TransportError;
use crate::transport::errors::transport_error::TransportError::CommunicationError;

#[derive(Debug)]
pub struct UsbTransport {
    device: Option<Device<Context>>,
    device_handle: Option<DeviceHandle<Context>>,
    interface_number: Option<u8>,
    bulk_in_endpoint: Option<u8>,
    bulk_out_endpoint: Option<u8>,
}

impl UsbTransport {
    pub const WRITING_DEFAULT_TIME_OUT: f64 = 5.0;

    pub fn new(device: Option<Device<Context>>) -> Self {
        UsbTransport {
            device,
            device_handle: None,
            bulk_in_endpoint: None,
            bulk_out_endpoint: None,
            interface_number: None,
        }
    }

    fn _acquire_device(&mut self, device_type: InterfaceType) -> Result<(), TransportError> {
        let (class_code, sub_class_code, protocol_code) = match device_type {
            InterfaceType::AndroidUsb(c, sc, p) => (c, sc, p),
            _ => {
                return Err(TransportError::UnexpectedError("Unsupported device type. Expected AndroidUsb interface.".to_string()));
            }
        };

        let device = match self.device.as_ref() {
            Some(handle) => handle,
            None => return Err(TransportError::UnexpectedError("Device is not initialized".to_string())),
        };
        let config_desc = device.active_config_descriptor().map_err(|e| CommunicationError(e.to_string()))?;

        for interface in config_desc.interfaces() {
            let interface_desc = interface.descriptors().next();
            if let Some(interface_desc) = interface_desc {
                if interface_desc.class_code() == class_code &&
                    interface_desc.sub_class_code() == sub_class_code &&
                    interface_desc.protocol_code() == protocol_code {
                    let device_handle = device.open().map_err(|e| CommunicationError(e.to_string()))?;
                    match device_handle.claim_interface(interface.number()) {
                        Ok(_) => {}
                        Err(UsbError::Busy) => {
                            return Err(CommunicationError("USB interface is busy. The device might be in use by another process".to_string()));
                        }
                        Err(err) => {
                            return Err(CommunicationError(err.to_string()));
                        }
                    }

                    self.setup_device(interface_desc)?;
                    self.device_handle = Some(device_handle);
                    self.interface_number = Some(interface.number());

                    return Ok(());
                }
            }
        }
        Err(TransportError::UnexpectedError("An unexpected error occurred while connecting to the device".to_string()))
    }

    fn _bulk_read(&self, length: usize, transport_timeout_s: f64) -> Result<Vec<u8>, TransportError> {
        match (self.device_handle.as_ref(), self.bulk_in_endpoint) {
            (Some(device_handle), Some(endpoint)) => {
                let mut buffer = Vec::with_capacity(length);

                while buffer.len() < length {
                    let remaining = length - buffer.len();
                    let chunk_size = remaining.min(DEFAULT_BUFFER_SIZE);
                    let mut chunk = vec![0u8; chunk_size];

                    match device_handle.read_bulk(endpoint, &mut chunk, Duration::from_secs_f64(transport_timeout_s)) {
                        Ok(bytes_read) => {
                            if bytes_read == 0 {
                                return Err(TransportError::ConnectionError(
                                    "Device connection lost".to_string()
                                ));
                            }
                            chunk.truncate(bytes_read);
                            buffer.extend_from_slice(&chunk);
                        }
                        Err(UsbError::Timeout) => return Err(TransportError::Timeout),
                        Err(UsbError::NoDevice) => return Err(TransportError::DeviceNotFound),
                        Err(UsbError::Pipe) => return Err(TransportError::ConnectionError("USB pipe error".to_string())),
                        Err(UsbError::Access) => return Err(TransportError::Unauthorized("No permission to access USB device".to_string())),
                        Err(UsbError::Overflow) => return Err(TransportError::ConnectionError("USB buffer overflow".to_string())),
                        Err(UsbError::Io) => return Err(TransportError::ConnectionError("USB I/O error".to_string())),
                        Err(UsbError::InvalidParam) => return Err(TransportError::ConnectionError("Invalid USB parameters".to_string())),
                        Err(UsbError::Busy) =>return Err(TransportError::ConnectionError("USB device/endpoint busy".to_string())),
                        Err(UsbError::Other) => return Err(TransportError::ConnectionError("Platform-specific USB error".to_string())),
                        Err(err) => return Err(CommunicationError(format!("Unexpected error: {}", err))),
                    }
                }

                if buffer.len() > length {
                    buffer.truncate(length);
                }

                Ok(buffer)
            }
            (None, _) => Err(TransportError::DeviceNotFound),
            (_, None) => Err(CommunicationError("USB endpoint not configured".to_string())),
        }
    }

    fn _bulk_write(&self, data: &[u8], transport_timeout_s: Option<f64>) -> Result<usize, TransportError> {
        let timeout = transport_timeout_s
            .map(Duration::from_secs_f64)
            .unwrap_or_else(|| Duration::from_secs_f64(Self::WRITING_DEFAULT_TIME_OUT));

        match (self.device_handle.as_ref(), self.bulk_out_endpoint) {
            (Some(device_handle), Some(endpoint)) => {
                device_handle.write_bulk(endpoint, data, timeout)
                    .map_err(|e| match e {
                        rusb::Error::Timeout => TransportError::Timeout,
                        rusb::Error::NoDevice => TransportError::DeviceNotFound,
                        rusb::Error::Pipe => TransportError::ConnectionError("USB pipe error: device disconnected".to_string()),
                        rusb::Error::Busy => CommunicationError("USB device is busy".to_string()),
                        rusb::Error::Access => TransportError::Unauthorized("No permission to access USB device".to_string()),
                        rusb::Error::InvalidParam => TransportError::UnexpectedError("Invalid USB parameter".to_string()),
                        _ => CommunicationError(format!("USB error: {}", e))
                    })
            }
            (None, _) => {
                Err(TransportError::DeviceNotFound)
            }
            (_, None) => {
                Err(CommunicationError("USB endpoint not configured".to_string()))
            }
        }
    }

    fn setup_device(&mut self, interface_desc: InterfaceDescriptor) -> Result<(), TransportError> {
        for endpoint in interface_desc.endpoint_descriptors() {
            if endpoint.transfer_type() == TransferType::Bulk {
                match endpoint.direction() {
                    Direction::Out => self.bulk_out_endpoint = Some(endpoint.address()),
                    Direction::In => self.bulk_in_endpoint = Some(endpoint.address()),
                }
            }
        }

        Ok(())
    }

    pub fn _verify_connection_status(&self) -> Result<(), TransportError> {
        match (self.device_handle.as_ref(), self.interface_number) {
            (Some(device_handle), Some(interface_number)) => {
                if device_handle.active_configuration().is_err() {
                    return Err(TransportError::ConnectionError("Device handle is no longer active".to_string()));
                }
                if device_handle.kernel_driver_active(interface_number).is_err() {
                    return Err(TransportError::ConnectionError("Interface is no longer claimed".to_string()));
                }
                Ok(())
            }
            _ => {
                Err(TransportError::UnexpectedError("Device handle or interface number not set correctly".to_string()))
            }
        }
    }
}

impl Transport for UsbTransport {
    fn acquire_device(&mut self, device_type: InterfaceType) -> Result<(), TransportError> {
        self._acquire_device(device_type)
    }

    fn release_device(&self) -> Result<(), TransportError> {
        if let Some(ref handle) = self.device_handle {
            if let Some(if_num) = self.interface_number {
                return match handle.release_interface(if_num) {
                    Ok(_) => Ok(()),
                    Err(rusb::Error::NotFound) => Err(TransportError::ConnectionError("Device not found".to_string())),
                    Err(rusb::Error::NoDevice) => Err(TransportError::ConnectionError("No such device".to_string())),
                    Err(rusb::Error::Io) => Err(TransportError::ConnectionError("I/O error, device may be disconnected".to_string())),
                    Err(rusb::Error::Busy) => Err(TransportError::ConnectionError("Device is busy".to_string())),
                    Err(err) => Err(TransportError::UnexpectedError(format!("Unexpected error when releasing device: {}", err))),
                };
            }
        }
        Ok(())
    }

    fn bulk_read(&self, length: usize, transport_timeout_s: f64) -> Result<Vec<u8>, TransportError> {
        self._bulk_read(length, transport_timeout_s)
    }


    fn bulk_write(&self, data: &[u8], transport_timeout_s: Option<f64>) -> Result<usize, TransportError> {
        self._bulk_write(data, transport_timeout_s)
    }

    fn verify_connection_status(&self) -> Result<(), TransportError> {
        self._verify_connection_status()
    }
}