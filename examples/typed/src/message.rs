//! Message type definitions for example process

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct Test {
    pub data: String,
    pub number: i64
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Message {
    None(()),
    Test(Test),
    String(String),
}

impl Default for Message {
    fn default() -> Self {
        Message::None(())
    }
}
