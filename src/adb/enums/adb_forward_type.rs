#[derive(Debug, Clone, PartialEq)]
pub enum ForwardType {
    Tcp(u16),
    LocalAbstract(String),
    LocalReserved(String),
    LocalFilesystem(String),
    Dev(String),
    Jdwp(u32),
}
#[allow(dead_code)]
impl ForwardType {

    pub fn get_value(self) -> String {
        match self {
            ForwardType::Tcp(port) => port.to_string(),
            ForwardType::LocalAbstract(name) => name,
            ForwardType::LocalReserved(name) => name,
            ForwardType::LocalFilesystem(name) => name,
            ForwardType::Dev(name) => name,
            ForwardType::Jdwp(pid) => pid.to_string(),
        }
    }


    pub fn get_value_ref(&self) -> String {
        match self {
            ForwardType::Tcp(port) => port.to_string(),
            ForwardType::LocalAbstract(name) => name.clone(),
            ForwardType::LocalReserved(name) => name.clone(),
            ForwardType::LocalFilesystem(name) => name.clone(),
            ForwardType::Dev(name) => name.clone(),
            ForwardType::Jdwp(pid) => pid.to_string(),
        }
    }
}