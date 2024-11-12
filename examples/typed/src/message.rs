//! Message type definitions for example process

use caryatid_sdk::messages::ClockTickMessage;

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct Test {
    pub data: String,
    pub number: i64
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Message {
    None(()),                 // Just so we have a simple default
    Test(Test),               // A custom struct
    String(String),           // Simple string
    Clock(ClockTickMessage),  // Clock tick
    JSON(serde_json::Value),  // Get out of jail free card
}

impl Default for Message {
    fn default() -> Self {
        Message::None(())
    }
}

// Casts from platform-wide messages
impl From<ClockTickMessage> for Message {
    fn from(msg: ClockTickMessage) -> Self {
        Message::Clock(msg)
    }
}
