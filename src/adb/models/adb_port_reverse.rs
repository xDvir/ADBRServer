use tokio::task::JoinHandle;
use crate::adb::models::adb_port_reverse_info::AdbPortReverseInfo;

pub struct AdbPortReverse {
    adb_port_reverse_task: JoinHandle<()>,
    adb_port_reverse_info: AdbPortReverseInfo,
}

#[allow(dead_code)]
impl AdbPortReverse {
    pub fn new(adb_port_reverse_info: AdbPortReverseInfo, adb_port_reverse_task: JoinHandle<()>) -> Self {
        AdbPortReverse {
            adb_port_reverse_info,
            adb_port_reverse_task,
        }
    }

    pub fn stop(&self) { self.adb_port_reverse_task.abort(); }

    pub fn get_info(&self) -> &AdbPortReverseInfo { &self.adb_port_reverse_info }

    pub fn is_running(&self) -> bool { !self.adb_port_reverse_task.is_finished() }
}