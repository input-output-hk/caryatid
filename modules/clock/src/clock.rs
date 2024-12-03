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

const DEFAULT_TOPIC: &str = "clock.tick";

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
    fn init(&self, context: Arc<Context<M>>, config: Arc<Config>) -> Result<()> {
        let message_bus = context.message_bus.clone();
        let topic = config.get_string("topic").unwrap_or(DEFAULT_TOPIC.to_string());

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
                let datetime = DateTime::<Utc>::from(wall_clock);

                // Construct message
                let message = ClockTickMessage {
                    time: datetime,
                    number: number
                };

                debug!("Clock sending {:?}", message);

                let message_enum: M = message.into();  // 'Promote' to outer enum
                message_bus.publish(&topic, Arc::new(message_enum))
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
    use caryatid_sdk::{MessageBus, MessageBusExt};
    use caryatid_sdk::mock_bus::MockBus;
    use tracing::Level;
    use tracing_subscriber;
    use tokio::sync::{Notify, mpsc};
    use tokio::time::{timeout, Duration};

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
        bus: Arc<dyn MessageBus<Message>>,
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
            let bus = Arc::new(MockBus::<Message>::new());

            // Parse config
            let config = Arc::new(Config::builder()
                .add_source(config::File::from_str(config_str, FileFormat::Toml))
                .build()
                .unwrap());

            // Create a context
            let context = Arc::new(Context::new(config.clone(), bus.clone()));

            // Create the clock
            let clock = Clock::<Message>{
                _marker: std::marker::PhantomData,
            };
            assert!(clock.init(context, config).is_ok());

            Self { bus, module: Arc::new(clock) }
        }
    }

    #[tokio::test]
    async fn construct_a_clock() {
        let setup = TestSetup::new("");
        assert_eq!(setup.module.get_name(), "clock");
        assert_eq!(setup.module.get_description(), "System clock");
    }

    #[tokio::test]
    async fn clock_sends_tick_messages() {
        let setup = TestSetup::new("");
        let notify = Arc::new(Notify::new());

        // Register for clock.tick
        let notify_clone = notify.clone();
        assert!(setup.bus.subscribe("clock.tick", move |_message: Arc<Message>| {
            notify_clone.notify_one();
        }).is_ok());

        // Wait for it to be received, or timeout
        assert!(timeout(Duration::from_secs(1), notify.notified()).await.is_ok(),
                "Didn't receive a clock.tick message");

        // Wait for another one
        assert!(timeout(Duration::from_secs(2), notify.notified()).await.is_ok(),
                "Didn't receive a second clock.tick message");
    }

    #[tokio::test]
    async fn clock_messages_have_increasing_times_and_ticks() {
        let setup = TestSetup::new("topic = 'tick'"); // Also test configured topic
        let (tx, mut rx) = mpsc::channel(10);

        // Register for tick
        assert!(setup.bus.subscribe("tick", move |message: Arc<Message>| {
            let tx = tx.clone();
            tokio::spawn(async move {
                let _ = tx.send(message).await;
            });
        }).is_ok());

        // Wait for the first message
        let first_res = timeout(Duration::from_secs(1), rx.recv()).await;
        assert!(first_res.is_ok(), "Didn't receive the first tick message");

        let first_message = first_res.unwrap();
        assert!(first_message.is_some(), "First message was None");

        // Extract and verify the type of the first message
        let first_clock = match first_message.unwrap().as_ref() {
            Message::Clock(clock) => clock.clone(),
            _ => panic!("First message was not a ClockTickMessage"),
        };

        // Wait for the second message
        let second_res = timeout(Duration::from_secs(2), rx.recv()).await;
        assert!(second_res.is_ok(), "Didn't receive the second tick message");

        let second_message = second_res.unwrap();
        assert!(second_message.is_some(), "Second message was None");

        // Extract and verify the type of the second message
        let second_clock = match second_message.unwrap().as_ref() {
            Message::Clock(clock) => clock.clone(),
            _ => panic!("Second message was not a ClockTickMessage"),
        };

        // Compare the timestamps
        let duration = (second_clock.time - first_clock.time).num_milliseconds();
        assert!(
            (900..=1100).contains(&duration),
            "Clock tick interval was out of range: {} ms",
            duration
        );

        // Compare the numbers
        assert!(
            second_clock.number == first_clock.number+1,
            "Second tick number was not incremented from the first"
        );
   }
}
