use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::scanners::devices_scanner_impl::DevicesScannerImpl;
use crate::scanners::devices_scanner_trait::DevicesScannerTrait;
use crate::scanners::usb_devices_scanner::UsbDevicesScanner;
use crate::transport::enums::interface_type::InterfaceType;
use crate::transport::transport::Transport;

pub struct DevicesScanner {
    scanners: Vec<DevicesScannerImpl>,
}

impl DevicesScanner {
    pub fn new() -> Self {
        let scanners = vec![DevicesScannerImpl::UsbScanner(UsbDevicesScanner::new())];
        DevicesScanner { scanners }
    }

    pub async fn start_scanning(
        &mut self,
        device_type: InterfaceType,
        is_new_device: Arc<dyn Fn(String) -> bool + Send + Sync>,
        on_find_device: Arc<dyn Fn(String, Box<dyn Transport>, InterfaceType) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>
    ) {
        for scanner in &mut self.scanners {
            scanner.start_scanning(
                device_type,
                is_new_device.clone(),
                on_find_device.clone()
            ).await;
        }
    }

    #[allow(dead_code)]
    pub fn stop_scanning(&mut self) {
        for scanner in &mut self.scanners {
            scanner.stop_scanning();
        }
    }
}