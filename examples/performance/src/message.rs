//! Message type definitions for performance testing
use arcstr::ArcStr;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Message {
    None(()),                // Just so we have a simple default
    Stop(()),                // Stop marker
    Data(ArcStr),            // Data string
    Json(serde_json::Value), // For monitor snapshots
}

impl Default for Message {
    fn default() -> Self {
        Message::None(())
    }
}

impl From<caryatid_process::MonitorSnapshot> for Message {
    fn from(snapshot: caryatid_process::MonitorSnapshot) -> Self {
        Message::Json(serde_json::to_value(snapshot).unwrap())
    }
}
