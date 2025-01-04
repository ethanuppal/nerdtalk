use chat::Entry;
use serde::{Deserialize, Serialize};
pub use serde_json::Error as CodingError;

pub trait Codable {
    fn to_bytes(&self) -> Vec<u8>
    where
        Self: Serialize,
    {
        serde_json::to_vec(self).expect("failed to serialize data")
    }

    fn try_from_bytes<'a>(bytes: &'a [u8]) -> Result<Self, CodingError>
    where
        Self: Deserialize<'a>,
    {
        serde_json::from_slice(bytes)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientMessage {
    Post {
        username: String,
        content: String,
    },
    Request {
        count: usize,
        up_to_slot_number: Option<usize>,
    },
}

impl Codable for ClientMessage {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    NewEntry(Entry),
    EntryRange(Vec<chat::Entry>),
}

impl Codable for ServerMessage {}
