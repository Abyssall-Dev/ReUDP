#[derive(Debug)]
pub enum ReUDPError {
    IoError(std::io::Error),
    ConnectionLost,
    NoResponseFromServer,
}

impl From<std::io::Error> for ReUDPError {
    fn from(error: std::io::Error) -> Self {
        ReUDPError::IoError(error)
    }
}