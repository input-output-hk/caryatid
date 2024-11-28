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

// -- Tests --
#[cfg(test)]
mod tests {
    use super::*;
    use config::{Config, FileFormat};
    use caryatid_sdk::mock_bus::MockBus;
    use tracing::Level;
    use tracing_subscriber;

    // Message type which includes a ClockTickMessage
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub enum Message {
        None(()),
        Clock(ClockTickMessage),  // Clock tick
    }

    impl Default for Message {
        fn default() -> Self {
            Message::None(())
        }
    }

    impl From<ClockTickMessage> for Message {
        fn from(msg: ClockTickMessage) -> Self {
            Message::Clock(msg)
        }
    }

    // Helper to create a clock talking to a mock message bus
    struct TestSetup {
        mock: Arc<MockBus<Message>>,
        module: Arc<dyn Module<Message>>
    }

    impl TestSetup {

        fn new(config_str: &str) -> Self {

            // Set up tracing
            let _ = tracing_subscriber::fmt()
                .with_max_level(Level::DEBUG)
                .with_test_writer()
                .try_init();

            // Create mock bus
            let mock = Arc::new(MockBus::<Message>::new());

            // Parse config
            let config = Arc::new(Config::builder()
                .add_source(config::File::from_str(config_str, FileFormat::Toml))
                .build()
                .unwrap());

            // Create a context
            let context = Arc::new(Context::new(config.clone(), mock.clone()));

            // Create the clock
            let clock = Clock::<Message>{
                _marker: std::marker::PhantomData,
            };
            assert!(clock.init(context, config).is_ok());

            Self { mock, module: Arc::new(clock) }
        }
    }

    #[tokio::test]
    async fn construct_a_clock() {
        let setup = TestSetup::new("");
        assert_eq!(setup.module.get_name(), "clock");
        assert_eq!(setup.module.get_description(), "System clock");
    }
}
