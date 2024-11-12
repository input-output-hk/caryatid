//! Caryatid Clock module
//! Generates regular clock.tick events

use caryatid_sdk::{Context, Module, module, MessageBounds};
use caryatid_sdk::messages::ClockTickMessage;
use std::sync::Arc;
use anyhow::Result;
use config::Config;
use tracing::{debug, error};
use tokio::time::{interval_at, Duration, Instant};
use std::time::SystemTime;
use chrono::{DateTime, Utc};

/// Clock module
/// Parameterised by the outer message enum used on the bus
#[module(
    message_type(M),
    name = "clock",
    description = "System clock"
)]
pub struct Clock<M: From<ClockTickMessage> + MessageBounds>;

impl<M: From<ClockTickMessage> + MessageBounds> Clock<M>
{
    fn init(&self, context: Arc<Context<M>>, _config: Arc<Config>) -> Result<()> {
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

                // Construct message
                let message = ClockTickMessage {
                    time: datetime,
                    number: number
                };

                debug!("Clock sending {:?}", message);

                let message_enum: M = message.into();  // 'Promote' to outer enum
                message_bus.publish("clock.tick", Arc::new(message_enum))
                    .await
                    .unwrap_or_else(|e| error!("Failed to publish: {e}"));

                number += 1;
            }
        });

        Ok(())
    }
}

