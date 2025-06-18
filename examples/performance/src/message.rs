//! Message type definitions for performance testing
use arcstr::ArcStr;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Message {
    None(()),     // Just so we have a simple default
    Stop(()),     // Stop marker
    Data(ArcStr), // Data string
}

impl Default for Message {
    fn default() -> Self {
        Message::None(())
    }
}
