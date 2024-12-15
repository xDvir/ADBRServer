use std::str::FromStr;
use crate::adb::enums::adb_forward_type::ForwardType;

#[derive(Clone,Debug)]
pub struct AdbPortForwardInfo {
    local: ForwardType,
    remote: ForwardType,
}

#[allow(dead_code)]
impl AdbPortForwardInfo {
    pub fn new(local: ForwardType, remote: ForwardType) -> Self {
        AdbPortForwardInfo { local, remote }
    }

    pub fn from_string(port_forward_str: &str) -> Result<Self, String> {
        let parts: Vec<&str> = port_forward_str.split(';').collect();
        if parts.len() != 2 {
            return Err("Invalid format. Expected 'local;remote'.".to_string());
        }

        let local = Self::parse_forward_type(parts[0])?;
        let remote = Self::parse_forward_type(parts[1])?;

        Ok(AdbPortForwardInfo { local, remote })
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

    pub fn local(&self) -> &ForwardType {
        &self.local
    }

    pub fn remote(&self) -> &ForwardType {
        &self.remote
    }

    pub fn set_local(&mut self, local: ForwardType) {
        self.local = local;
    }

    pub fn set_remote(&mut self, remote: ForwardType) {
        self.remote = remote;
    }

    pub fn to_string(&self) -> String {
        format!("{};{}", self.forward_type_to_string(&self.local), self.forward_type_to_string(&self.remote))
    }

    pub fn local_with_type(&self) -> String {
        return self.forward_type_to_string(self.local())
    }

    pub fn remote_with_type(&self) -> String {
        return self.forward_type_to_string(self.remote())
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
