// In-memory pub-sub bus with multi-threaded async workers
use tokio::sync::{mpsc, Mutex};
use std::sync::Arc;
use anyhow::Result;
use config::Config;
use tracing::info;
use futures::future::BoxFuture;
use caryatid_sdk::message_bus::{MessageBounds, MessageBus, Subscriber};
use caryatid_sdk::match_topic::match_topic;

const DEFAULT_SUBSCRIBER_QUEUE_SIZE: i64 = 10;

/// Subscriber on a particular topic pattern
struct PatternSubscriber<M: MessageBounds> {
    pattern: String,
    queue: mpsc::Sender<(Arc<Subscriber<M>>, String, Arc<M>)>,
    subscriber: Arc<Subscriber<M>>,
}

/// In-memory, zero-copy pub-sub bus
pub struct InMemoryBus<M: MessageBounds> {

    /// Subscribers
    subscribers: Arc<Mutex<Vec<Arc<PatternSubscriber<M>>>>>,

    /// Queue size for subscriber
    subscriber_queue_size: usize,
}

impl<M: MessageBounds> InMemoryBus<M> {
    pub fn new(config: &Config) -> Self {
        info!("Creating in-memory message bus");

        let subscriber_queue_size = config.get_int("subscriber-queue-size")
            .unwrap_or(DEFAULT_SUBSCRIBER_QUEUE_SIZE) as usize;

        let subscribers: Arc<Mutex<Vec<Arc<PatternSubscriber<M>>>>> =
            Arc::new(Mutex::new(Vec::new()));

        InMemoryBus {
            subscribers,
            subscriber_queue_size,
        }
    }
}

impl<M: MessageBounds> MessageBus<M> for InMemoryBus<M> {

    /// Publish a message on a given topic
    fn publish(&self, topic: &str, message: Arc<M>)
               -> BoxFuture<'static, Result<()>> {
        let subscribers = self.subscribers.clone();
        let topic = topic.to_string();
        let message = message.clone();

        Box::pin(async move {
            // Get matching subscribers, limiting lock duration
            let matching: Vec<_> = {
                subscribers.lock().await.iter()
                    .filter(|patsub| match_topic(&patsub.pattern, &topic))
                    .map(Arc::clone)
                    .collect()
            };

            for patsub in matching {
                let subscriber = patsub.subscriber.clone();
                let topic = topic.clone();
                let message = message.clone();
                patsub.queue.send((subscriber, topic, message)).await?;
            }

            Ok(())
        })
    }

    /// Subscribe for a message with an subscriber function
    fn register_subscriber(&self, topic: &str, subscriber: Arc<Subscriber<M>>)
                           -> BoxFuture<'static, Result<()>> {
        let subscribers = self.subscribers.clone();
        let topic = topic.to_string();
        let subscriber_queue_size = self.subscriber_queue_size;

        Box::pin(async move {

            let (sender, mut receiver) =
                mpsc::channel::<(Arc<Subscriber<M>>,
                                 String,
                                 Arc<M>)>(subscriber_queue_size);

            let mut subscribers = subscribers.lock().await;
            subscribers.push(Arc::new(PatternSubscriber {
                pattern: topic,
                queue: sender,
                subscriber: subscriber
            }));

            tokio::spawn(async move {
                while let Some((subscriber, topic, message)) = receiver.recv().await {
                    subscriber(&topic, message.clone()).await;
                }
            });

            Ok(())
        })
    }

    /// Shut down, clearing all subscribers
    fn shutdown(&self) -> BoxFuture<'static, Result<()>> {
        let subscribers = self.subscribers.clone();

        Box::pin(async move {
            let mut subscribers = subscribers.lock().await;
            subscribers.clear();
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use config::{Config, FileFormat};
    use tracing::Level;
    use tracing_subscriber;
    use tokio::sync::Notify;
    use tokio::time::{timeout, Duration};
    use futures::future::ready;

    // Helper to set up an in-memory bus from given config string
    struct TestSetup<M: MessageBounds> {
        bus: Arc<dyn MessageBus<M>>
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
        let notify = Arc::new(Notify::new());

        // Subscribe
        let notify_clone = notify.clone();
        assert!(setup.bus.register_subscriber(
            "test",
            Arc::new(move |_topic: &str, _message: Arc<String>| {
                notify_clone.notify_one(); // Notify that the callback was triggered
                Box::pin(ready(()))
            }))
                .await
                .is_ok());

        // Publish
        assert!(setup.bus.publish("test", Arc::new("Hello, world!".to_string()))
                .await
                .is_ok());

        // Wait for it to be received, or timeout
        assert!(timeout(Duration::from_millis(100), notify.notified()).await.is_ok(),
                "Didn't receive the subscribed message");
    }

    #[tokio::test]
    async fn publish_subscribe_with_wrong_topic_doesnt_round_trip() {
        let setup = TestSetup::<String>::new("");
        let notify = Arc::new(Notify::new());

        // Subscribe
        let notify_clone = notify.clone();
        assert!(setup.bus.register_subscriber(
            "test",
            Arc::new(move |_topic: &str, _message: Arc<String>| {
                notify_clone.notify_one(); // Notify that the callback was triggered
                Box::pin(ready(()))
            }))
                .await
                .is_ok());

        // Publish
        assert!(setup.bus.publish("BOGUS", Arc::new("Hello, world!".to_string()))
                .await
                .is_ok());

        // Wait for it to be received, or timeout
        assert!(timeout(Duration::from_millis(100), notify.notified()).await.is_err(),
                "Received the subscribed message when we shouldn't have!");
    }

}
