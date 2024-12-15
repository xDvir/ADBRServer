use core::pin::Pin;
use std::error::Error;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::mpsc;
use std::time::{Duration};
use rusb::{Context, Device, UsbContext};
use crate::scanners::devices_scanner_trait::{DevicesScannerTrait, SCANNING_CHANNEL_CAPACITY, SCANNING_INTERVAL_MS};
use crate::transport::transport::Transport;
use crate::transport::usb_transport::UsbTransport;
use tokio::select;
use tokio::time::{sleep, timeout};
use tracing::error;
use crate::transport::enums::interface_type::InterfaceType;

const HANDLE_DEVICE_TIMEOUT_SECONDS: u64 = 10;

pub struct UsbDevicesScanner {
    stop_scanning_tx: Option<mpsc::Sender<()>>,
}

impl DevicesScannerTrait for UsbDevicesScanner {
    async fn start_scanning(
        &mut self,
        device_type: InterfaceType,
        is_new_device: Arc<dyn Fn(String) -> bool + Send + Sync>,
        on_find_device: Arc<dyn Fn(String, Box<dyn Transport>, InterfaceType) -> Pin<Box<dyn Future<Output=()> + Send>> + Send + Sync>,
    ) {
        let (stop_scanning_tx, mut stop_scanning_rx) = mpsc::channel(SCANNING_CHANNEL_CAPACITY);
        self.stop_scanning_tx = Some(stop_scanning_tx);

        loop {
            select! {
        _ = UsbDevicesScanner::_start_scanning(device_type, is_new_device.clone(), on_find_device.clone()) => {
        }
        _ = stop_scanning_rx.recv() => {
            break;
        }
    }
            sleep(Duration::from_millis(SCANNING_INTERVAL_MS)).await;
        }
    }

    fn stop_scanning(&mut self) {
        if let Some(stop_scanning_tx) = self.stop_scanning_tx.take() {
            let _ = stop_scanning_tx.send(());
        }
    }
}

impl UsbDevicesScanner {
    pub fn new() -> UsbDevicesScanner {
        Self {
            stop_scanning_tx: None,
        }
    }

    async fn _handle_device(
        device: Device<Context>,
        device_type: InterfaceType,
        is_new_device: Arc<dyn Fn(String) -> bool + Send + Sync>,
        on_find_device: Arc<dyn Fn(String, Box<dyn Transport>, InterfaceType) -> Pin<Box<dyn Future<Output=()> + Send>> + Send + Sync>,
    ) -> Result<(), Box<dyn Error + Send>> {
        let (class_code, sub_class_code, protocol_code) = match device_type {
            InterfaceType::AndroidUsb(c, sc, p) | InterfaceType::AndroidTcp(c, sc, p) => (c, sc, p),
        };

        let device_desc = device.device_descriptor()
            .map_err(|err| Box::new(err) as Box<dyn Error + Send>)?;

        let config_desc = device.active_config_descriptor()
            .map_err(|err| Box::new(err) as Box<dyn Error + Send>)?;

        let matching_interface = config_desc.interfaces().find(|interface| {
            if let Some(interface_desc) = interface.descriptors().next() {
                interface_desc.class_code() == class_code &&
                    interface_desc.sub_class_code() == sub_class_code &&
                    interface_desc.protocol_code() == protocol_code
            } else {
                false
            }
        });

        if let Some(_) = matching_interface {
            let device_handle = device.open()
                .map_err(|err| Box::new(err) as Box<dyn Error + Send>)?;

            let curr_device_serial = device_handle.read_serial_number_string_ascii(&device_desc)
                .map_err(|err| Box::new(err) as Box<dyn Error + Send>)?;

            if is_new_device(curr_device_serial.clone()) {
                let transport = UsbTransport::new(Some(device.clone()));
                on_find_device(curr_device_serial, Box::new(transport), device_type).await;
            }
        }

        Ok(())
    }

    async fn _start_scanning(
        device_type: InterfaceType,
        is_new_device: Arc<dyn Fn(String) -> bool + Send + Sync>,
        on_find_device: Arc<dyn Fn(String, Box<dyn Transport>, InterfaceType) -> Pin<Box<dyn Future<Output=()> + Send>> + Send + Sync>,
    ) {
        let mut handle_devices_tasks = vec![];

        let context = match Context::new() {
            Ok(context) => context,
            Err(err) => {
                error!("Failed to create USB context {}", err);
                return;
            }
        };

        let devices = match context.devices() {
            Ok(devices) => devices,
            Err(err) => {
                error!("Failed to retrieve USB devices: {}", err);
                return;
            }
        };

        for device in devices.iter() {
            let task_device = device.clone();
            let device_type = device_type.clone();
            let is_new_device = is_new_device.clone();
            let on_find_device = on_find_device.clone();

            let handle_device_task = tokio::spawn(async move {
                if let Err(err) = UsbDevicesScanner::_handle_device(
                    task_device,
                    device_type,
                    is_new_device,
                    on_find_device,
                ).await {
                    error!("Error handling device: {:?}", err);
                }
            });

            handle_devices_tasks.push(handle_device_task);
        }

        for task in handle_devices_tasks {
            if let Err(_) = timeout(
                Duration::from_secs(HANDLE_DEVICE_TIMEOUT_SECONDS),
                task,
            ).await {

            }
        }
    }
}
