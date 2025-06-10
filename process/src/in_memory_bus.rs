// In-memory pub-sub bus with multi-threaded async workers
use anyhow::Result;
use caryatid_sdk::match_topic::match_topic;
use caryatid_sdk::message_bus::{MessageBounds, MessageBus, Subscription, SubscriptionBounds};
use config::Config;
use futures::future::BoxFuture;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tracing::info;

const DEFAULT_SUBSCRIBER_QUEUE_SIZE: i64 = 10;

struct InMemorySubscription<M> {
    receiver: mpsc::Receiver<(String, Arc<M>)>,
}

impl<M: MessageBounds> SubscriptionBounds for InMemorySubscription<M> {}

impl<M: MessageBounds> Subscription<M> for InMemorySubscription<M> {
    fn read(&mut self) -> BoxFuture<anyhow::Result<(String, Arc<M>)>> {
        Box::pin(async move {
            loop {
                if let Some(entry) = self.receiver.recv().await {
                    return Ok(entry);
                }
            }
        })
    }
}

/// Subscriptions on a particular topic pattern
struct PatternSubscription<M: MessageBounds> {
    pattern: String,
    queue: mpsc::Sender<(String, Arc<M>)>,
}

/// In-memory, zero-copy pub-sub bus
pub struct InMemoryBus<M: MessageBounds> {
    /// Subscriptions
    subscriptions: Arc<Mutex<Vec<Arc<PatternSubscription<M>>>>>,

    /// Queue size for subscriber
    subscriber_queue_size: usize,
}

impl<M: MessageBounds> InMemoryBus<M> {
    pub fn new(config: &Config) -> Self {
        info!("Creating in-memory message bus");

        let subscriber_queue_size = config
            .get_int("subscriber-queue-size")
            .unwrap_or(DEFAULT_SUBSCRIBER_QUEUE_SIZE) as usize;

        let subscriptions: Arc<Mutex<Vec<Arc<PatternSubscription<M>>>>> =
            Arc::new(Mutex::new(Vec::new()));

        InMemoryBus {
            subscriptions,
            subscriber_queue_size,
        }
    }
}

impl<M: MessageBounds> MessageBus<M> for InMemoryBus<M> {
    /// Publish a message on a given topic
    fn publish(&self, topic: &str, message: Arc<M>) -> BoxFuture<'static, Result<()>> {
        let subscriptions = self.subscriptions.clone();
        let topic = topic.to_string();
        let message = message.clone();

        Box::pin(async move {
            // Get matching subscriptions, limiting lock duration
            let matching: Vec<_> = {
                subscriptions
                    .lock()
                    .await
                    .iter()
                    .filter(|patsub| match_topic(&patsub.pattern, &topic))
                    .map(Arc::clone)
                    .collect()
            };

            for patsub in matching {
                let topic = topic.clone();
                let message = message.clone();
                patsub.queue.send((topic, message)).await?;
            }

            Ok(())
        })
    }

    /// Subscribe for a message with an subscriber function
    fn register(&self, topic: &str) -> BoxFuture<Result<Box<dyn Subscription<M>>>> {
        let subscriptions = self.subscriptions.clone();
        let topic = topic.to_string();
        let subscriber_queue_size = self.subscriber_queue_size;

        Box::pin(async move {
            let (sender, receiver) = mpsc::channel::<(String, Arc<M>)>(subscriber_queue_size);

            let mut subscriptions = subscriptions.lock().await;
            subscriptions.push(Arc::new(PatternSubscription {
                pattern: topic,
                queue: sender,
            }));
            Ok(Box::new(InMemorySubscription { receiver }) as Box<dyn Subscription<M>>)
        })
    }

    /// Shut down, clearing all subscriptions
    fn shutdown(&self) -> BoxFuture<'static, Result<()>> {
        let subscriptions = self.subscriptions.clone();

        Box::pin(async move {
            let mut subscriptions = subscriptions.lock().await;
            subscriptions.clear();
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use config::{Config, FileFormat};
    use tokio::sync::Notify;
    use tokio::time::{timeout, Duration};
    use tracing::Level;
    use tracing_subscriber;

    // Helper to set up an in-memory bus from given config string
    struct TestSetup<M: MessageBounds> {
        bus: Arc<dyn MessageBus<M>>,
    }

    impl<M: MessageBounds> TestSetup<M> {
        fn new(config_str: &str) -> Self {
            // Set up tracing
            let _ = tracing_subscriber::fmt()
                .with_max_level(Level::DEBUG)
                .with_test_writer()
                .try_init();

            // Parse config
            let config = Config::builder()
                .add_source(config::File::from_str(config_str, FileFormat::Toml))
                .build()
                .unwrap();

            // Create the bus
            let bus = Arc::new(InMemoryBus::<M>::new(&config));

            Self { bus }
        }
    }

    #[tokio::test]
    async fn publish_subscribe_round_trip() {
        let setup = TestSetup::<String>::new("");

        // Subscribe
        let subscription = setup.bus.register("test").await;
        assert!(subscription.is_ok());
        if let Ok(mut subscription) = subscription {
            // Publish
            assert!(setup
                .bus
                .publish("test", Arc::new("Hello, world!".to_string()))
                .await
                .is_ok());

            // Read
            let message = subscription.read().await;
            assert!(message.is_ok());
        }
    }

    #[tokio::test]
    async fn publish_subscribe_with_wrong_topic_doesnt_round_trip() {
        let setup = TestSetup::<String>::new("");

        // Subscribe
        let subscription = setup.bus.register("test").await;
        assert!(subscription.is_ok());
        if let Ok(mut subscription) = subscription {
            // Publish
            assert!(setup
                .bus
                .publish("BOGUS", Arc::new("Hello, world!".to_string()))
                .await
                .is_ok());

            let notify = Arc::new(Notify::new());
            let notify_clone = notify.clone();
            tokio::spawn(async move {
                let _ = subscription.read().await;
                notify_clone.notify_one();
            });

            // Wait for it to be received, or timeout
            assert!(
                timeout(Duration::from_millis(100), notify.notified())
                    .await
                    .is_err(),
                "Received the subscribed message when we shouldn't have!"
            );
        }
    }
}
