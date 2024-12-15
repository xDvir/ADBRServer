use tokio::net::TcpStream;

pub struct AdbTask {
    pub socket: TcpStream,
}

impl AdbTask {
    pub fn new(socket: TcpStream) -> Self {
        AdbTask
        {
            socket,
        }
    }
}
