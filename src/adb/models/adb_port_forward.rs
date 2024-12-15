use tokio::task::JoinHandle;
use crate::adb::models::adb_port_forward_info::AdbPortForwardInfo;

pub struct AdbPortForward {
    adb_port_forward_task: JoinHandle<()>,
    adb_port_forward_info: AdbPortForwardInfo,
}

#[allow(dead_code)]
impl AdbPortForward {
    pub fn new(adb_port_forward_info: AdbPortForwardInfo, adb_port_forward_task: JoinHandle<()>) -> Self {
        AdbPortForward {
            adb_port_forward_info,
            adb_port_forward_task,
        }
    }

    pub fn stop(&self) {
        self.adb_port_forward_task.abort();
    }

    pub fn get_info(&self) -> &AdbPortForwardInfo {
        &self.adb_port_forward_info
    }

    pub fn is_running(&self) -> bool {
        !self.adb_port_forward_task.is_finished()
    }
}