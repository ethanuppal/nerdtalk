use chat::Entry;
use chrono::{DateTime, Utc};
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

/// Opaque unique-per-client identifier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientId {
    timestamp: DateTime<Utc>,
}

impl ClientId {
    /// Guaranteed to be unique for each client but not necessarily between
    /// clients.
    pub fn new_unique_per_client() -> Self {
        Self {
            timestamp: Utc::now(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientMessage {
    Post {
        username: String,
        content: String,
    },
    Request {
        client_id: ClientId,
        count: usize,
        up_to_slot_number: Option<usize>,
    },
}

impl Codable for ClientMessage {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    NewEntry(Entry),
    EntryRange {
        client_id: ClientId,
        entries: Vec<chat::Entry>,
    },
}

impl Codable for ServerMessage {}
