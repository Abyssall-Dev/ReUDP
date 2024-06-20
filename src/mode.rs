use std::net::SocketAddr;

#[derive(Clone)]
pub enum Mode {
    Server,
    Client(SocketAddr),
}
