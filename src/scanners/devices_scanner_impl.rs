use crate::scanners::tcp_devices_scanner::TcpDevicesScanner;
use crate::scanners::usb_devices_scanner::UsbDevicesScanner;

#[allow(dead_code)]
pub enum DevicesScannerImpl {
    UsbScanner(UsbDevicesScanner),
    TcpScanner(TcpDevicesScanner),
}