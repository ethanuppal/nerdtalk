pub use bincode::{Error as CodingError, ErrorKind as CodingErrorKind};
use chat::ChatLogEntry;
use serde::{Deserialize, Serialize};

pub trait Codable {
    fn to_bytes(&self) -> Vec<u8>
    where
        Self: Serialize,
    {
        bincode::serialize(self).expect("failed to serialize data")
    }

    fn try_from_bytes<'a>(bytes: &'a [u8]) -> Result<Self, CodingError>
    where
        Self: Deserialize<'a>,
    {
        bincode::deserialize(bytes)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppendChatEntry {
    pub username: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientMessage {
    Append(AppendChatEntry),
    Request {
        count: usize,
        up_to_slot_number: Option<usize>,
    },
}

impl Codable for ClientMessage {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    NewEntry(ChatLogEntry),
}

impl Codable for ServerMessage {}
