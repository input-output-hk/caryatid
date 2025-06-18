//! Mock message bus for tests
use crate::match_topic::match_topic;
use crate::message_bus::{MessageBounds, MessageBus, Subscription, SubscriptionBounds};
use anyhow::{bail, Result};
use async_trait::async_trait;
use config::Config;
use futures::future::BoxFuture;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::time::Duration;
use tracing::debug;

const DEFAULT_REQUEST_TIMEOUT: u64 = 5;

pub struct PublishRecord<M: MessageBounds> {
    pub topic: String,
    pub message: Arc<M>,
}

pub struct MockSubscription<M> {
    id: usize,
    receiver: mpsc::UnboundedReceiver<(String, Arc<M>)>,
    deregister: Box<dyn Fn() + Send + Sync>,
}

fn make_deregister<M: Send + Sync + 'static>(
    subscriptions: Arc<Mutex<Vec<SubscriptionRecord<M>>>>,
    id: usize,
) -> Box<dyn Fn() + Send + Sync> {
    let deregister = move || {
        let subscriptions = Arc::clone(&subscriptions);

        // Spawn async task to avoid `.await` in sync closure
        tokio::spawn(async move {
            let mut subs = subscriptions.lock().await;
            subs.retain(|rec| rec.id != id);
        });
    };
    Box::new(deregister)
}

impl<M> Drop for MockSubscription<M> {
    fn drop(&mut self) {
        (self.deregister)();
    }
}

#[derive(Clone)]
pub struct SubscriptionRecord<M> {
    pub id: usize,
    pub topic: String,
    sender: Arc<mpsc::UnboundedSender<(String, Arc<M>)>>,
}

impl<M: MessageBounds> SubscriptionBounds for MockSubscription<M> {}

impl<M: MessageBounds> MockSubscription<M> {
    pub fn new(
        id: usize,
        receiver: mpsc::UnboundedReceiver<(String, Arc<M>)>,
        subscriptions: Arc<Mutex<Vec<SubscriptionRecord<M>>>>,
    ) -> Self {
        let deregister = make_deregister(subscriptions, id);
        Self {
            id,
            receiver,
            deregister,
        }
    }
}

impl<M: MessageBounds> Subscription<M> for MockSubscription<M> {
    fn read(&mut self) -> BoxFuture<anyhow::Result<(String, Arc<M>)>> {
        Box::pin(async move {
            let Some(entry) = self.receiver.recv().await else {
                bail!("Sender {} has unexpectedly closed", self.id);
            };
            Ok(entry)
        })
    }
}

pub struct MockBus<M: MessageBounds> {
    pub publishes: Arc<Mutex<Vec<PublishRecord<M>>>>,
    pub subscriptions: Arc<Mutex<Vec<SubscriptionRecord<M>>>>,
    pub shutdowns: Arc<Mutex<u16>>, // just count them
    request_timeout: Duration,
    next_subscription_id: Arc<AtomicUsize>,
}

impl<M: MessageBounds> MockBus<M> {
    pub fn new(config: &Config) -> Self {
        let timeout = config
            .get::<u64>("request-timeout")
            .unwrap_or(DEFAULT_REQUEST_TIMEOUT);
        let timeout = Duration::from_secs(timeout);

        Self {
            publishes: Arc::new(Mutex::new(Vec::new())),
            subscriptions: Arc::new(Mutex::new(Vec::new())),
            shutdowns: Arc::new(Mutex::new(0)),
            request_timeout: timeout,
            next_subscription_id: Arc::new(AtomicUsize::new(0)),
        }
    }
}

#[async_trait]
impl<M> MessageBus<M> for MockBus<M>
where
    M: MessageBounds + serde::Serialize + serde::de::DeserializeOwned,
{
    async fn publish(&self, topic: &str, message: Arc<M>) -> Result<()> {
        debug!("Mock publish on {topic}");
        let topic = topic.to_string();

        // Limit lock because we can be re-entrant
        {
            let mut publishes = self.publishes.lock().await;
            publishes.push(PublishRecord {
                topic: topic.clone(),
                message: message.clone(),
            });
        }

        // Send to all subscriptions if topic matches
        // Copy relevant subscriptions before calling them to avoid deadlock if a subscription
        // does another publish (as handle() does)
        let mut relevant_subscriptions: Vec<SubscriptionRecord<M>> = Vec::new();
        {
            let subscriptions = self.subscriptions.lock().await;
            for sub in subscriptions.iter() {
                if match_topic(&sub.topic, &topic) {
                    relevant_subscriptions.push(sub.clone())
                }
            }
        }

        for sub in relevant_subscriptions.iter() {
            sub.sender.send((topic.to_string(), message.clone()))?;
        }

        Ok(())
    }

    fn request_timeout(&self) -> Duration {
        self.request_timeout
    }

    async fn subscribe(&self, topic: &str) -> Result<Box<dyn Subscription<M>>> {
        let topic = topic.to_string();
        debug!("Mock subscribe on {topic}");
        let mut subscriptions = self.subscriptions.lock().await;
        let id = self
            .next_subscription_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let (sender, receiver) = mpsc::unbounded_channel();
        let subscription = MockSubscription::new(id, receiver, self.subscriptions.clone());
        subscriptions.push(SubscriptionRecord {
            id,
            topic: topic.to_string(),
            sender: Arc::new(sender),
        });
        Ok(Box::new(subscription) as Box<dyn Subscription<M>>)
    }

    async fn shutdown(&self) -> Result<()> {
        let shutdowns = self.shutdowns.clone();
        let mut shutdowns = shutdowns.lock().await;
        *shutdowns += 1;
        Ok(())
    }
}

// -- Tests --
#[cfg(test)]
mod tests {
    use super::*;
    use config::FileFormat;
    use tokio::time::sleep;
    use tracing::Level;
    use tracing_subscriber;

    // Helper to set up a correlation bus with a mock sub-bus, from given config string
    struct TestSetup<M: MessageBounds> {
        mock: Arc<MockBus<M>>,
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
            let mock = Arc::new(MockBus::<M>::new(&config));

            Self { mock }
        }
    }

    #[tokio::test]
    async fn publish_passes_straight_through() {
        let setup = TestSetup::<String>::new("");

        // Send a message
        assert!(setup
            .mock
            .publish("test", Arc::new("Hello, world!".to_string()))
            .await
            .is_ok());

        // Check the mock got it
        let mock_publishes = setup.mock.publishes.lock().await;
        assert_eq!(mock_publishes.len(), 1);
        let pub0 = &mock_publishes[0];
        assert_eq!(pub0.topic, "test");
        assert_eq!(pub0.message.as_ref(), "Hello, world!");
    }

    #[tokio::test]
    async fn subscribe_passes_straight_through() {
        let setup = TestSetup::<String>::new("");

        // Subscribe
        let subscription = setup.mock.subscribe("test").await;

        // Check the mock got it
        let mock_subscriptions = setup.mock.subscriptions.lock().await;
        assert_eq!(mock_subscriptions.len(), 1);
        let sub0 = &mock_subscriptions[0];
        assert_eq!(sub0.topic, "test");
        drop(subscription);
    }

    #[tokio::test]
    async fn unsubscribe_removes_subscription() {
        let setup = TestSetup::<String>::new("");

        // Subscribe
        let subscription = setup.mock.subscribe("test").await;
        assert!(subscription.is_ok());
        let Ok(subscription) = subscription else {
            return;
        };

        // Check the mock got it
        let mock_subscriptions = setup.mock.subscriptions.lock().await;
        assert_eq!(mock_subscriptions.len(), 1);
        let sub0 = &mock_subscriptions[0];
        assert_eq!(sub0.topic, "test");
        drop(mock_subscriptions);

        drop(subscription);

        // Check the mock removed it, after waiting a little because it's async
        sleep(Duration::from_millis(100)).await;
        let mock_subscriptions = setup.mock.subscriptions.lock().await;
        assert_eq!(mock_subscriptions.len(), 0);
    }

    #[tokio::test]
    async fn request_times_out_with_no_response() {
        let setup = TestSetup::<String>::new("request-timeout = 1");

        // Request with no response should timeout
        assert!(setup
            .mock
            .request("test", Arc::new("Hello, world!".to_string()))
            .await
            .is_err());
    }

    #[tokio::test]
    async fn shutdown_is_passed_to_sub_bus() {
        let setup = TestSetup::<String>::new("");

        // Shut it down
        assert!(setup.mock.shutdown().await.is_ok());

        // Check the mock got it
        assert_eq!(*setup.mock.shutdowns.lock().await, 1);
    }

    #[tokio::test]
    async fn publish_lets_you_send_multiple_messages() -> Result<()> {
        let setup = TestSetup::<String>::new("");
        const TOPIC: &str = "topic";

        let mut subscriber = setup.mock.subscribe(TOPIC).await?;

        setup
            .mock
            .publish(TOPIC, Arc::new("Hello".to_string()))
            .await?;
        setup
            .mock
            .publish(TOPIC, Arc::new("World!".to_string()))
            .await?;

        assert_eq!(
            subscriber.read().await?,
            (TOPIC.to_string(), Arc::new("Hello".to_string()))
        );
        assert_eq!(
            subscriber.read().await?,
            (TOPIC.to_string(), Arc::new("World!".to_string()))
        );

        Ok(())
    }
}
