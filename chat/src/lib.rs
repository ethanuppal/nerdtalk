use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatLogEntry {
    pub slot_number: usize,
    pub username: String,
    pub timestamp: DateTime<Utc>,
    pub content: String,
}

impl ChatLogEntry {
    pub fn new_timestamped_now(
        slot_number: usize,
        username: String,
        content: String,
    ) -> Self {
        Self {
            slot_number,
            username,
            timestamp: Utc::now(),
            content,
        }
    }
}
