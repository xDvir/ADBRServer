use std::sync::Arc;
use std::future::Future;
use std::pin::Pin;
use crate::scanners::devices_scanner_impl::DevicesScannerImpl;
use crate::transport::enums::interface_type::InterfaceType;
use crate::transport::transport::Transport;

pub const SCANNING_CHANNEL_CAPACITY: usize = 1;
pub const SCANNING_INTERVAL_MS: u64 = 1000;

pub trait DevicesScannerTrait {
    async fn start_scanning(
        &mut self,
        device_type: InterfaceType,
        is_new_device: Arc<dyn Fn(String) -> bool + Send + Sync>,
        on_find_device: Arc<dyn Fn(String, Box<dyn Transport>, InterfaceType) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>
    );
    fn stop_scanning(&mut self);
}

impl DevicesScannerTrait for DevicesScannerImpl {
    async fn start_scanning(
        &mut self,
        device_type: InterfaceType,
        is_new_device: Arc<dyn Fn(String) -> bool + Send + Sync>,
        on_find_device: Arc<dyn Fn(String, Box<dyn Transport>, InterfaceType) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>
    ) {
        match self {
            DevicesScannerImpl::UsbScanner(scanner) => {
                scanner.start_scanning(device_type, is_new_device, on_find_device).await
            },
            DevicesScannerImpl::TcpScanner(scanner) => {
                scanner.start_scanning(device_type, is_new_device, on_find_device).await
            },
        }
    }

    fn stop_scanning(&mut self) {
        match self {
            DevicesScannerImpl::UsbScanner(scanner) => scanner.stop_scanning(),
            DevicesScannerImpl::TcpScanner(scanner) => scanner.stop_scanning(),
        }
    }
}