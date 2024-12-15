use tokio::net::{TcpListener, UnixListener};

pub enum Listener {
    Tcp(TcpListener),
    Unix(UnixListener),
}