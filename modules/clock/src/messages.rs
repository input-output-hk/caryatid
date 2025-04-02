//! Messages for Clock module
use chrono::{DateTime, Utc};

/// Clock tick message
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClockTickMessage {
    /// Time of tick, UTC
    pub time: DateTime<Utc>,

    /// Tick number
    pub number: u64
}

