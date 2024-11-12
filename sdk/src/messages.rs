//! Definition of core framework messages

// We don't use these messages in the SDK itself
#![allow(dead_code)]

use chrono::{DateTime, Utc};

/// Clock tick message
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClockTickMessage {
    /// Time of tick, UTC
    pub time: DateTime<Utc>,

    /// Tick number
    pub number: u64
}
