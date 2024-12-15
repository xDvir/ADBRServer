use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use crate::adb::connections::adb_connection::AdbConnection;
use crate::adb::enums::adb_device_status::AdbDeviceStatus;
use crate::adb::enums::adb_device_type::AdbDeviceType;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use dashmap::DashMap;
use crate::adb::models::adb_port_forward::AdbPortForward;
use crate::adb::models::adb_port_reverse::AdbPortReverse;


pub struct AdbDevice {
    device_serial_number: String,
    adb_device_connection: Arc<dyn AdbConnection>,
    adb_device_status: AdbDeviceStatus,
    adb_device_type: AdbDeviceType,
    adb_ports_forward_hs: DashMap<String, AdbPortForward>,
    adb_ports_reverse_hs: DashMap<String, AdbPortReverse>,
    last_monitored_at: AtomicU64,
    monitoring_interval: Duration,
}

#[allow(dead_code)]
impl AdbDevice {
    pub fn new(device_serial_number:String,adb_device_connection: Arc<dyn AdbConnection>, adb_device_status: AdbDeviceStatus, adb_device_type: AdbDeviceType, monitoring_interval: Duration) -> Self {
        Self {
            device_serial_number,
            adb_device_connection,
            adb_device_status,
            adb_device_type,
            adb_ports_forward_hs: DashMap::new(),
            adb_ports_reverse_hs: DashMap::new(),
            last_monitored_at: AtomicU64::new(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()),
            monitoring_interval,
        }
    }

    pub fn adb_device_connection(&self) -> &Arc<dyn AdbConnection> {
        &self.adb_device_connection
    }

    pub fn adb_device_status(&self) -> &AdbDeviceStatus {
        &self.adb_device_status
    }

    pub fn adb_device_type(&self) -> &AdbDeviceType {
        &self.adb_device_type
    }

    pub fn adb_ports_forward_hs(&self) -> &DashMap<String, AdbPortForward> {
        &self.adb_ports_forward_hs
    }

    pub fn set_device_status(&mut self, device_status: AdbDeviceStatus) {
        self.adb_device_status = device_status;
    }


    pub fn is_emulator_device(&self) -> bool {
        self.adb_device_type.is_emulator()
    }

    pub fn is_usb_device(&self) -> bool {
        self.adb_device_type.is_usb()
    }

    pub fn is_monitoring_interval_passed(&self) -> bool {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let last_monitored = self.last_monitored_at.load(Ordering::Relaxed);
        (now - last_monitored) >= self.monitoring_interval.as_secs()
    }

    pub fn update_last_monitored(&self) {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        self.last_monitored_at.store(now, Ordering::Relaxed);
    }

    pub async fn close_device_gracefully(&self) {
        for port_forward in self.adb_ports_forward_hs.iter() {
            port_forward.value().stop()
        }
    }

    pub fn get_port_forward(&self, key: &str) -> Option<dashmap::mapref::one::Ref<'_, String, AdbPortForward>> {
        self.adb_ports_forward_hs.get(key)
    }

    pub fn get_all_port_forwards(&self) -> dashmap::iter::Iter<'_, String, AdbPortForward> {
        self.adb_ports_forward_hs.iter()
    }

    pub fn has_port_forward(&self, key: &str) -> bool {
        self.adb_ports_forward_hs.contains_key(key)
    }

    pub fn port_forwards_count(&self) -> usize {
        self.adb_ports_forward_hs.len()
    }

    pub fn get_port_forward_mut(&self, key: &str) -> Option<dashmap::mapref::one::RefMut<'_, String, AdbPortForward>> {
        self.adb_ports_forward_hs.get_mut(key)
    }

    pub fn insert_port_forward(&self, key: String, port_forward: AdbPortForward) -> Option<AdbPortForward> {
        self.adb_ports_forward_hs.insert(key, port_forward)
    }

    pub fn remove_port_forward(&self, key: &str) {
        self.adb_ports_forward_hs.remove(key);
    }

    pub fn get_port_reverse(&self, key: &str) -> Option<dashmap::mapref::one::Ref<'_, String, AdbPortReverse>> {
        self.adb_ports_reverse_hs.get(key)
    }

    pub fn has_port_reverse(&self, key: &str) -> bool {
        self.adb_ports_reverse_hs.contains_key(key)
    }

    pub fn insert_port_reverse(&self, key: String, port_reverse: AdbPortReverse) -> Option<AdbPortReverse> {
        self.adb_ports_reverse_hs.insert(key, port_reverse)
    }

    pub fn remove_port_reverse(&self, key: &str) {
        self.adb_ports_reverse_hs.remove(key);
    }

    pub fn adb_ports_reverse_hs(&self) -> &DashMap<String, AdbPortReverse> {
        &self.adb_ports_reverse_hs
    }

    pub fn device_serial_number(&self)->&str {
        &self.device_serial_number
    }


}