// In-memory pub-sub bus with multi-threaded async workers
use tokio::sync::{mpsc, Mutex};
use std::sync::Arc;
use anyhow::Result;
use config::Config;
use tracing::{info, error, debug};
use futures::future::BoxFuture;
use caryatid_sdk::message_bus::{MessageBus, Subscriber, MessageBounds};
use caryatid_sdk::match_topic::match_topic;

const DEFAULT_WORKERS: i64 = 4;
const DEFAULT_DISPATCH_QUEUE_SIZE: i64 = 1000;
const DEFAULT_WORKER_QUEUE_SIZE: i64 = 1000;

/// Subscriber on a particular topic pattern
struct PatternSubscriber<M: MessageBounds> {
    pattern: String,
    subscriber: Arc<Subscriber<M>>,
}

/// In-memory, zero-copy pub-sub bus
pub struct InMemoryBus<M: MessageBounds> {

    /// Subscribers
    subscribers: Arc<Mutex<Vec<Arc<PatternSubscriber<M>>>>>,

    /// Sender for received messages
    sender: mpsc::Sender<(String, Arc<M>)>,
}

impl<M: MessageBounds> InMemoryBus<M> {
    pub fn new(config: &Config) -> Self {
        let num_workers = config.get_int("workers")
            .unwrap_or(DEFAULT_WORKERS) as usize;
        let dispatch_queue_size = config.get_int("dispatch-queue-size")
            .unwrap_or(DEFAULT_DISPATCH_QUEUE_SIZE) as usize;
        let worker_queue_size = config.get_int("worker-queue-size")
            .unwrap_or(DEFAULT_WORKER_QUEUE_SIZE) as usize;

        let (sender, mut receiver) =
            mpsc::channel::<(String, Arc<M>)>(dispatch_queue_size);

        info!("Creating in-memory message bus with {} workers", num_workers);

        let subscribers: Arc<Mutex<Vec<Arc<PatternSubscriber<M>>>>> =
            Arc::new(Mutex::new(Vec::new()));

        // Create a task queue channel for each worker
        let mut worker_txs = Vec::new();
        for _ in 0..num_workers {
            let (worker_tx, mut worker_rx) =
                mpsc::channel::<(Arc<Subscriber<M>>,
                                 String,
                                 Arc<M>)>(worker_queue_size);
            worker_txs.push(worker_tx);

            // Spawn worker tasks that handle individual subscriber invocations
            tokio::spawn(async move {
                while let Some((subscriber, topic, message)) = worker_rx.recv().await {
                    subscriber(&topic, message.clone()).await;
                }
            });
        }

        // Single receiver task to handle incoming messages
        let subs_clone = subscribers.clone();
        tokio::spawn(async move {
            let mut round_robin_index = 0;

            while let Some((topic, message)) = receiver.recv().await {

                // Get matching subscribers, limiting lock duration
                let matching: Vec<_> = {
                    let subscribers = subs_clone.lock().await;
                    subscribers.iter()
                        .filter(|patsub| match_topic(&patsub.pattern, &topic))
                        .map(Arc::clone)
                        .collect()
                };

                // Lock the subscribers for the topic
                for patsub in matching {
                    // Loop in case worker queues are full
                    for i in 0..num_workers+1 {  // ends with i=num_workers
                        // Dispatch the task to a worker
                        let worker_tx = &worker_txs[(round_robin_index+i) % num_workers];

                        // Send the subscriber and the message to a worker
                        let data = (patsub.subscriber.clone(), topic.clone(),
                                    message.clone());

                        // If we've looped right round then they're all full -
                        // just block on the first one
                        if i == num_workers {
                            debug!("All worker queues full - blocking");
                            if let Err(e) = worker_tx.send(data).await {
                                error!("Failed to send message to worker: {}", e);
                            }
                        } else {
                            // Try each one in turn, stop if it accepts it
                            match worker_tx.try_send(data) {
                                Ok(_) => break,
                                Err(mpsc::error::TrySendError::Full(_)) => {},
                                Err(e) => error!("Failed to send message to worker: {e}")
                            }
                        }

                        round_robin_index += 1;
                    }
                }
            }
        });

        InMemoryBus { subscribers, sender }
    }
}

impl<M: MessageBounds> MessageBus<M> for InMemoryBus<M> {

    /// Publish a message on a given topic
    fn publish(&self, topic: &str, message: Arc<M>)
               -> BoxFuture<'static, Result<()>> {
        let topic = topic.to_string();
        let sender = self.sender.clone();
        let message = message.clone();

        Box::pin(async move {
            sender.send((topic, message)).await?;
            Ok(())
        })
    }

    /// Subscribe for a message with an subscriber function
    fn register_subscriber(&self, topic: &str, subscriber: Arc<Subscriber<M>>)
                           -> BoxFuture<'static, Result<()>> {
        let subscribers = self.subscribers.clone();
        let topic = topic.to_string();
        Box::pin(async move {
            let mut subscribers = subscribers.lock().await;
            subscribers.push(Arc::new(PatternSubscriber {
                pattern: topic,
                subscriber: subscriber
            }));
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
