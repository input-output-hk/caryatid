//! Message type definitions for example process

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
    JSON(serde_json::Value),  // Get out of jail free card
}

impl Default for Message {
    fn default() -> Self {
        Message::None(())
    }
}
