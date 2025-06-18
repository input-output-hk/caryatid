//! Caryatid Clock module
//! Generates regular clock.tick events

use anyhow::Result;
use caryatid_sdk::{module, Context, MessageBounds, Module};
use chrono::{DateTime, Utc};
use config::Config;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::time::{interval_at, Duration, Instant};
use tracing::{debug, error};

const DEFAULT_TOPIC: &str = "clock.tick";

pub mod messages;
use messages::ClockTickMessage;

/// Clock module
/// Parameterised by the outer message enum used on the bus
#[module(message_type(M), name = "clock", description = "System clock")]
pub struct Clock<M: From<ClockTickMessage> + MessageBounds>;

impl<M: From<ClockTickMessage> + MessageBounds> Clock<M> {
    async fn init(&self, context: Arc<Context<M>>, config: Arc<Config>) -> Result<()> {
        let message_bus = context.message_bus.clone();
        let topic = config
            .get_string("topic")
            .unwrap_or(DEFAULT_TOPIC.to_string());

        context.run(async move {
            let start_instant = Instant::now();
            let start_system = SystemTime::now();

            let mut interval = interval_at(start_instant, Duration::from_secs(1));
            let mut number = 0u64;

            loop {
                let scheduled_instant = interval.tick().await;

                // Get wall clock equivalent for this
                let wall_clock = start_system + scheduled_instant.duration_since(start_instant);

                // Convert to a chrono DateTime<Utc>
                let datetime = DateTime::<Utc>::from(wall_clock);

                // Construct message
                let message = ClockTickMessage {
                    time: datetime,
                    number: number,
                };

                debug!("Clock sending {:?}", message);

                let message_enum: M = message.into(); // 'Promote' to outer enum
                message_bus
                    .publish(&topic, Arc::new(message_enum))
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
    use caryatid_sdk::mock_bus::MockBus;
    use config::{Config, FileFormat};
    use tokio::sync::{mpsc, watch::Sender, Notify};
    use tokio::time::{timeout, Duration};
    use tracing::Level;
    use tracing_subscriber;

    // Message type which includes a ClockTickMessage
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub enum Message {
        None(()),
        Clock(ClockTickMessage), // Clock tick
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
        module: Arc<dyn Module<Message>>,
        context: Arc<Context<Message>>,
    }

    impl TestSetup {
        async fn new(config_str: &str) -> Self {
            // Set up tracing
            let _ = tracing_subscriber::fmt()
                .with_max_level(Level::DEBUG)
                .with_test_writer()
                .try_init();

            // Parse config
            let config = Arc::new(
                Config::builder()
                    .add_source(config::File::from_str(config_str, FileFormat::Toml))
                    .build()
                    .unwrap(),
            );

            // Create mock bus
            let bus = Arc::new(MockBus::<Message>::new(&config));

            // Create a context
            let context = Arc::new(Context::new(
                config.clone(),
                bus.clone(),
                Sender::<bool>::new(false),
            ));

            // Create the clock
            let clock = Clock::<Message> {
                _marker: std::marker::PhantomData,
            };
            assert!(clock.init(context.clone(), config).await.is_ok());

            Self {
                module: Arc::new(clock),
                context,
            }
        }

        fn start(&self) {
            let _ = self.context.startup_watch.send(true);
        }
    }

    #[tokio::test]
    async fn construct_a_clock() {
        let setup = TestSetup::new("").await;
        assert_eq!(setup.module.get_name(), "clock");
        assert_eq!(setup.module.get_description(), "System clock");
    }

    #[tokio::test]
    async fn clock_sends_tick_messages() {
        let setup = TestSetup::new("").await;
        let notify = Arc::new(Notify::new());

        // Register for clock.tick
        let notify_clone = notify.clone();
        let subscription = setup.context.subscribe("clock.tick").await;
        assert!(subscription.is_ok());
        let Ok(mut subscription) = subscription else {
            return;
        };
        setup.context.run(async move {
            loop {
                let Ok(_) = subscription.read().await else {
                    return;
                };
                notify_clone.notify_one();
            }
        });

        setup.start();

        // Wait for it to be received, or timeout
        assert!(
            timeout(Duration::from_secs(1), notify.notified())
                .await
                .is_ok(),
            "Didn't receive a clock.tick message"
        );

        // Wait for another one
        assert!(
            timeout(Duration::from_secs(2), notify.notified())
                .await
                .is_ok(),
            "Didn't receive a second clock.tick message"
        );
    }

    #[tokio::test]
    async fn clock_messages_have_increasing_times_and_ticks() {
        let setup = TestSetup::new("topic = 'tick'").await; // Also test configured topic
        let (tx, mut rx) = mpsc::channel(10);

        // Register for tick
        let subscription = setup.context.subscribe("tick").await;
        assert!(subscription.is_ok());
        let Ok(mut subscription) = subscription else {
            return;
        };
        setup.context.run(async move {
            loop {
                let Ok((_, message)) = subscription.read().await else {
                    return;
                };
                let _ = tx.send(message).await;
            }
        });

        setup.start();

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
            second_clock.number == first_clock.number + 1,
            "Second tick number was not incremented from the first"
        );
    }
}
