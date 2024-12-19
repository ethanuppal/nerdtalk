use serde::{Deserialize, Serialize};

pub trait Codable {
    fn to_bytes(&self) -> Vec<u8>
    where
        Self: Serialize,
    {
        bincode::serialize(self).expect("failed to serialize data")
    }

    fn from_bytes<'a>(bytes: &'a [u8]) -> bincode::Result<Self>
    where
        Self: Deserialize<'a>,
    {
        bincode::deserialize(bytes)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientRequest {
    Append {
        content: String,
        sequence_number: usize,
    },
    Ping {
        last_slot_number: usize,
    },
}

impl Codable for ClientRequest {}
