use std::collections::VecDeque;

use dashmap::DashMap;

pub struct AdbPacketStore {
    packet_store: DashMap<(u32, u32), VecDeque<(u32, Vec<u8>)>>,
}

impl AdbPacketStore {
    pub fn new() -> Self {
        Self {
            packet_store: DashMap::new()
        }
    }

    pub fn put_packet(&self, arg0: u32, arg1: u32, cmd: u32, data: Vec<u8>) {
        self.packet_store.entry((arg0, arg1)).or_insert_with(VecDeque::new).push_back((cmd, data));
    }

    pub fn find_packet(&self, arg0: u32, arg1: u32, expected_cmds: &[u32]) -> Option<(u32, u32)> {
        if self.packet_store.is_empty() {
            return None;
        }

        match (arg1, arg0) {
            (0, 0) => self
                .packet_store
                .iter()
                .find_map(|entry| {
                    let ((key0, key1), val1) = entry.pair();
                    if !val1.is_empty() && val1.iter().any(|(cmd, _)| expected_cmds.contains(cmd)) {
                        Some((*key0, *key1))
                    } else {
                        None
                    }
                }),
            (0, _) => self
                .packet_store
                .iter()
                .find_map(|entry| {
                    let ((key0, key1), val1) = entry.pair();
                    if *key0 == arg0 && !val1.is_empty() {
                        Some((arg0, *key1))
                    } else {
                        None
                    }
                }),
            (_, 0) => self
                .packet_store
                .iter()
                .find_map(|entry| {
                    let ((key0, key1), val1) = entry.pair();
                    if *key1 == arg1 && !val1.is_empty() {
                        Some((*key0, arg1))
                    } else {
                        None
                    }
                }),
            (_, _) => {
                if let Some(val1) = self.packet_store.get(&(arg0, arg1)) {
                    if !val1.is_empty() {
                        return Some((arg0, arg1));
                    }
                }
                None
            }
        }
    }

    pub fn get_packet(&self, arg0: u32, arg1: u32, expected_cmds: &[u32]) -> Option<(u32, u32, u32, Vec<u8>)> {
        let (arg0, arg1) = match (arg0, arg1) {
            (0, 0) | (_, 0) | (0, _) => self.find_packet(arg0, arg1,expected_cmds)?,
            _ => (arg0, arg1),
        };

        if let Some(mut val1) = self.packet_store.get_mut(&(arg0, arg1)) {
            if let Some((cmd, data)) = val1.pop_front() {
                return Some((cmd, arg0, arg1, data));
            }
        }

        None
    }

    pub fn clear_packet(&self, arg0: u32, arg1: u32) {
        self.packet_store.remove(&(arg0, arg1));
    }
}