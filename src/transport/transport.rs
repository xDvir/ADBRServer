use crate::transport::enums::interface_type::InterfaceType;
use crate::transport::errors::transport_error::TransportError;


pub trait Transport: Send + Sync {
    fn acquire_device(&mut self, device_type: InterfaceType) -> Result<(), TransportError>;
    fn release_device(&self) -> Result<(), TransportError>;
    fn bulk_read(&self, length: usize, transport_timeout_s: f64) -> Result<Vec<u8>, TransportError> ;
    fn bulk_write(&self, data: &[u8], transport_timeout_s: Option<f64>) -> Result<usize, TransportError>;
    fn verify_connection_status(&self) -> Result<(), TransportError>;
}