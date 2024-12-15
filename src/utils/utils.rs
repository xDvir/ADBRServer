use std::error::Error;
use std::fs::read_to_string;
use std::net::TcpListener;
use std::str;

use rsa::{PaddingScheme, pkcs8::DecodePrivateKey, RsaPrivateKey};

use crate::constants::{FAIL, NULL_TERMINATOR, OKAY};

pub fn sign_data(private_key_path: &str, data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let keys_as_string = read_to_string(private_key_path)?;
    let private_key = RsaPrivateKey::from_pkcs8_pem(&keys_as_string).unwrap();

    let mut data_to_sign = vec![0x30, 0x21, 0x30, 0x09, 0x06, 0x05, 0x2b, 0x0e, 0x03, 0x02, 0x1a, 0x05, 0x00, 0x04, 0x14];
    data_to_sign.extend_from_slice(data);
    let padding = PaddingScheme::new_pkcs1v15_sign(None);
    let signature = private_key.sign(padding, &data_to_sign)?;

    Ok(signature)
}

pub fn get_adb_key_path() -> Result<String, Box<dyn Error>> {
    let mut path = dirs::home_dir().ok_or("Home directory not found")?;

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    path.push(".android");

    #[cfg(target_os = "windows")]
    path.push("AppData/Local/Android");

    path.to_str()
        .ok_or_else(|| From::from("Path contains invalid UTF-8"))
        .map(|s| s.to_owned())
}

pub fn format_response_with_size(response: String) -> String {
    if response.is_empty() {
        String::new()
    } else {
        let size = format!("{:04x}", response.len());
        format!("{}{}", size, response)
    }
}

pub(crate) fn ensure_null_terminated(mut command: String) -> String {
    if !command.ends_with(NULL_TERMINATOR) {
        command.push(NULL_TERMINATOR);
    }
    command
}

pub async fn is_port_available(address: &str, port: u16) -> bool {
    TcpListener::bind(format!("{}:{}", address, port)).is_ok()
}

pub fn fail() -> String {
    String::from(FAIL)
}

pub fn okay() -> String {
    String::from(OKAY)
}