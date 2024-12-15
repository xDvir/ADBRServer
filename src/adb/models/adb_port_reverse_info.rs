use std::fmt::Display;
use std::str::FromStr;
use crate::adb::enums::adb_forward_type::ForwardType;

#[derive(Clone, Debug)]
pub struct AdbPortReverseInfo {
    device: ForwardType,
    host: ForwardType,
}

#[allow(dead_code)]
impl AdbPortReverseInfo {
    pub fn new(device: ForwardType, host: ForwardType) -> Self {
        AdbPortReverseInfo { device, host }
    }

    pub fn from_string(reverse_str: &str) -> Result<Self, String> {
        let parts: Vec<&str> = reverse_str.split(';').collect();
        if parts.len() != 2 {
            return Err("Invalid format. Expected 'device;host'.".to_string());
        }

        let device = Self::parse_forward_type(parts[0])?;
        let host = Self::parse_forward_type(parts[1])?;

        Ok(AdbPortReverseInfo { device, host })
    }

    fn parse_forward_type(s: &str) -> Result<ForwardType, String> {
        let parts: Vec<&str> = s.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(format!("Invalid forward type format: {}", s));
        }

        match parts[0] {
            "tcp" => {
                let port = u16::from_str(parts[1])
                    .map_err(|_| format!("Invalid TCP port: {}", parts[1]))?;
                Ok(ForwardType::Tcp(port))
            }
            "localabstract" => Ok(ForwardType::LocalAbstract(parts[1].to_string())),
            "localreserved" => Ok(ForwardType::LocalReserved(parts[1].to_string())),
            "localfilesystem" => Ok(ForwardType::LocalFilesystem(parts[1].to_string())),
            "dev" => Ok(ForwardType::Dev(parts[1].to_string())),
            "jdwp" => {
                let pid = u32::from_str(parts[1])
                    .map_err(|_| format!("Invalid JDWP pid: {}", parts[1]))?;
                Ok(ForwardType::Jdwp(pid))
            }
            _ => Err(format!("Unknown forward type: {}", parts[0])),
        }
    }

    pub fn device(&self) -> &ForwardType {
        &self.device
    }

    pub fn host(&self) -> &ForwardType {
        &self.host
    }

    pub fn device_with_type(&self) -> String {
        self.forward_type_to_string(self.device())
    }

    pub fn host_with_type(&self) -> String {
        self.forward_type_to_string(self.host())
    }

    fn forward_type_to_string(&self, forward_type: &ForwardType) -> String {
        match forward_type {
            ForwardType::Tcp(port) => format!("tcp:{}", port),
            ForwardType::LocalAbstract(name) => format!("localabstract:{}", name),
            ForwardType::LocalReserved(name) => format!("localreserved:{}", name),
            ForwardType::LocalFilesystem(name) => format!("localfilesystem:{}", name),
            ForwardType::Dev(name) => format!("dev:{}", name),
            ForwardType::Jdwp(pid) => format!("jdwp:{}", pid),
        }
    }

}

impl Display for AdbPortReverseInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format!("{};{}", self.device_with_type(), self.host_with_type()))
    }
}