#[derive(Debug, Clone, Copy)]
pub struct AdbTransactionInfo {
    sent_packet_id: u32,
    receive_packet_id: u32,
}

impl AdbTransactionInfo {
    pub fn new(sent_packet_id: u32, receive_packet_id: u32) -> Self {
        AdbTransactionInfo {
            sent_packet_id,
            receive_packet_id,
        }
    }

    pub fn sent_packet_id(&self) -> u32 {
        self.sent_packet_id
    }

    pub fn receive_packet_id(&self) -> u32 {
        self.receive_packet_id
    }

    pub fn args_match(&self, arg0: u32, arg1: u32) -> bool {
        arg1 == self.sent_packet_id && (self.receive_packet_id == 0 || arg0 == self.receive_packet_id)
    }

    pub fn set_receive_packet_id(&mut self, receive_packet_id: u32) {
        self.receive_packet_id = receive_packet_id;
    }

}