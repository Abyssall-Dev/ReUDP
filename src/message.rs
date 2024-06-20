const HEADER_SIZE: usize = 9; // 8 bytes for sequence number, 1 byte for message type

#[derive(Debug, PartialEq, Clone)]
pub enum MessageType {
    Data,
    Ack,
    Heartbeat,
    Unknown(u8),
}

#[derive(Debug, Clone)]
pub struct Message {
    pub sequence: u64,
    pub message_type: MessageType,
    pub payload: Vec<u8>,
}

impl Message {
    pub fn new(sequence: u64, message_type: MessageType, payload: Vec<u8>) -> Self {
        Self {
            sequence,
            message_type,
            payload,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(HEADER_SIZE + self.payload.len());
        bytes.extend_from_slice(&self.sequence.to_be_bytes());
        bytes.push(match self.message_type {
            MessageType::Data => 0,
            MessageType::Ack => 1,
            MessageType::Heartbeat => 2,
            MessageType::Unknown(t) => t,
        });
        bytes.extend_from_slice(&self.payload);
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        let sequence = u64::from_be_bytes(bytes[..8].try_into().unwrap());
        let message_type = match bytes[8] {
            0 => MessageType::Data,
            1 => MessageType::Ack,
            2 => MessageType::Heartbeat,
            t => {
                eprintln!("Unknown message type: {}", t);
                MessageType::Unknown(t)
            }
        };
        let payload = bytes[9..].to_vec();
        Self {
            sequence,
            message_type,
            payload,
        }
    }
}
