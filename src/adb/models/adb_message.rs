use std::io::Cursor;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use crate::adb::errors::adb_connection_error::AdbConnectionError;
use crate::constants::{ADB_MESSAGE_SIZE};


#[derive(Debug)]
pub struct AdbMessage {
    command: u32,
    magic: u32,
    arg0: u32,
    arg1: u32,
    data: Vec<u8>,
}

impl AdbMessage {

    const BITWISE_INVERT_MASK: u32 = 0xFFFFFFFF;

    pub fn new(command: u32, arg0: u32, arg1: u32, data: Vec<u8>) -> Self {
        let magic = command ^ Self::BITWISE_INVERT_MASK;
        let adb_message = AdbMessage {
            command,
            magic,
            arg0,
            arg1,
            data,
        };
        adb_message
    }

    pub fn pack_message(&self) -> Vec<u8> {
        let mut message = Vec::new();
        message.write_u32::<LittleEndian>(self.command).unwrap();
        message.write_u32::<LittleEndian>(self.arg0).unwrap();
        message.write_u32::<LittleEndian>(self.arg1).unwrap();
        message.write_u32::<LittleEndian>(self.data.len() as u32).unwrap();
        message.write_u32::<LittleEndian>(self.checksum()).unwrap();
        message.write_u32::<LittleEndian>(self.magic).unwrap();
        message
    }

    pub fn checksum(&self) -> u32 {
        self.data.iter().map(|&x| x as u32).sum()
    }

    pub fn command(&self) -> u32 {
        self.command
    }

    #[allow(dead_code)]
    pub fn magic(&self) -> u32 {
        self.magic
    }

    pub fn arg0(&self) -> u32 {
        self.arg0
    }

    pub fn arg1(&self) -> u32 {
        self.arg1
    }

    pub fn data(&self) -> &Vec<u8> {
        &self.data
    }

    pub fn set_data(&mut self, data: Vec<u8>) {
        self.data = data;
    }

    pub fn unpack_message(message_bytes: &Vec<u8>) -> Result<(u32, u32, u32, u32, u32), AdbConnectionError> {
        if message_bytes.len() < ADB_MESSAGE_SIZE {
            return Err(AdbConnectionError::CommunicationError(String::from("Buffer is too short to unpack")));
        }

        let mut cursor = Cursor::new(message_bytes);

        let command = cursor.read_u32::<LittleEndian>().map_err(|e| AdbConnectionError::CommunicationError(e.to_string()))?;
        let arg0 = cursor.read_u32::<LittleEndian>().map_err(|e| AdbConnectionError::CommunicationError(e.to_string()))?;
        let arg1 = cursor.read_u32::<LittleEndian>().map_err(|e| AdbConnectionError::CommunicationError(e.to_string()))?;
        let data_length = cursor.read_u32::<LittleEndian>().map_err(|e| AdbConnectionError::CommunicationError(e.to_string()))?;
        let data_checksum = cursor.read_u32::<LittleEndian>().map_err(|e| AdbConnectionError::CommunicationError(e.to_string()))?;

        Ok((command, arg0, arg1, data_length, data_checksum))
    }
}