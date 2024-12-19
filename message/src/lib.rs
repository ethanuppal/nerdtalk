pub use bincode;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Message {
    content: String,
    sequence_number: usize,
}

#[derive(Serialize, Deserialize)]
pub struct UpdateRequest {
    last_slot_number: usize,
}
