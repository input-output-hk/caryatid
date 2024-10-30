//! Caryatid Clock module
//! Generates regular clock.tick events

use caryatid_sdk::{Context, Module, module};
use std::sync::Arc;
use anyhow::Result;
use config::Config;
use tracing::{debug, error};
use serde_json::json;
use tokio::time::{interval_at, Duration, Instant};
use std::time::SystemTime;
use chrono::{DateTime, Utc};

/// Clock module
#[module(
    name = "clock",
    description = "System clock"
)]
pub struct Clock;

impl Clock {

    fn init(&self, context: Arc<Context>, _config: Arc<Config>) -> Result<()> {
        let message_bus = context.message_bus.clone();

        tokio::spawn(async move {
            let start_instant = Instant::now();
            let start_system = SystemTime::now();

            let mut interval = interval_at(start_instant,
                                           Duration::from_secs(1));
            let mut number = 0u64;

            loop {
                let scheduled_instant = interval.tick().await;

                // Get wall clock equivalent for this
                let wall_clock = start_system
                    + scheduled_instant.duration_since(start_instant);

                // Convert to a chrono DateTime<Utc>
                let datetime: DateTime<Utc> =
                    DateTime::<Utc>::from(wall_clock);

                // Get ISO timestamp
                let iso = datetime.to_rfc3339();

                let message = Arc::new(json!({
                    "time": iso,
                    "number": number
                }));

                debug!("Clock sending {:?}", message);

                message_bus.publish("clock.tick", message)
                    .await
                    .unwrap_or_else(|e| error!("Failed to publish: {e}"));

                number += 1;
            }
        });

        Ok(())
    }
}

