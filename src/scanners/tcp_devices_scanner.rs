use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::scanners::devices_scanner_trait::DevicesScannerTrait;
use crate::transport::enums::interface_type::InterfaceType;
use crate::transport::transport::Transport;

#[allow(dead_code)]
pub struct TcpDevicesScanner {
    continue_scanning: bool,
}

#[allow(dead_code)]
impl DevicesScannerTrait for TcpDevicesScanner {
    async fn start_scanning(
        &mut self,
        _device_type: InterfaceType,
        _is_new_device: Arc<dyn Fn(String) -> bool + Send + Sync>,
        _on_find_device: Arc<dyn Fn(String, Box<dyn Transport>, InterfaceType) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>
    ){
        todo!()
    }

    fn stop_scanning(&mut self) {
        todo!()
    }
}