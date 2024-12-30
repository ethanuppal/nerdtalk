use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Metadata {
    pub username: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MessageText(pub String);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Content {
    Original(MessageText),
    Edited(MessageText),
    Deleted,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entry {
    pub slot_number: usize,
    pub metadata: Metadata,
    pub content: Content,
}

impl Entry {
    pub fn new_timestamped_now(
        slot_number: usize,
        username: String,
        content: Content,
    ) -> Self {
        Self {
            slot_number,
            metadata: Metadata {
                username,
                timestamp: Utc::now(),
            },
            content,
        }
    }

    pub fn text_content(&self) -> Option<&str> {
        match &self.content {
            Content::Original(message_text) | Content::Edited(message_text) => {
                Some(message_text.0.as_str())
            }
            Content::Deleted => None,
        }
    }
}
