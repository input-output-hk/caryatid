//! Mock message bus for tests
use std::collections::VecDeque;
use std::sync::Arc;
use futures::future::BoxFuture;
use anyhow::Result;
use tokio::sync::{Mutex, Notify};
use tracing::debug;
use crate::message_bus::{MessageBus, MessageBounds, Subscription, SubscriptionBounds};
use crate::match_topic::match_topic;

pub struct PublishRecord<M: MessageBounds> {
    pub topic: String,
    pub message: Arc<M>
}

#[derive(Clone)]
pub struct MockSubscription<M> {
    messages: Arc<Mutex<VecDeque<(String, Arc<M>)>>>,
    notify: Arc<Notify>,
}

pub struct SubscriptionRecord<M> {
    pub topic: String,
    pub subscription: MockSubscription<M>,
}

impl<M: MessageBounds> SubscriptionBounds for MockSubscription<M> {}

impl<M> MockSubscription<M> {
    pub fn new() -> Self {
        Self {
            messages: Arc::new(Mutex::new(VecDeque::new())),
            notify: Arc::new(Notify::new()),
        }
    }

    pub async fn push(&self, topic: &str, message: Arc<M>) {
        self.messages.lock().await.push_back((topic.to_string(), message));
        self.notify.notify_one();
    }
}

impl<M: MessageBounds> Subscription<M> for MockSubscription<M> {
    fn read(&mut self) -> BoxFuture<anyhow::Result<(String, Arc<M>)>> {
        Box::pin(async move {
            loop {
                self.notify.notified().await;
                if let Some(entry) = self.messages.lock().await.pop_front() {
                    return Ok(entry);
                }
            }
        })
    }
}

pub struct MockBus<M: MessageBounds> {
    pub publishes: Arc<Mutex<Vec<PublishRecord<M>>>>,
    pub subscriptions: Arc<Mutex<Vec<SubscriptionRecord<M>>>>,
    pub shutdowns: Arc<Mutex<u16>>,           // just count them
}

impl<M: MessageBounds> MockBus<M> {
    pub fn new() -> Self {
        Self {
            publishes: Arc::new(Mutex::new(Vec::new())),
            subscriptions: Arc::new(Mutex::new(Vec::new())),
            shutdowns: Arc::new(Mutex::new(0)),
        }
    }
}

impl<M> MessageBus<M> for MockBus<M>
where M: MessageBounds + serde::Serialize + serde::de::DeserializeOwned {

    fn publish(&self, topic: &str, message: Arc<M>) -> BoxFuture<'static, Result<()>> {

        debug!("Mock publish on {topic}");
        let publishes = self.publishes.clone();
        let subscriptions = self.subscriptions.clone();
        let topic = topic.to_string();

        Box::pin(async move {

            // Limit lock because we can be re-entrant
            {
                let mut publishes = publishes.lock().await;
                publishes.push(PublishRecord {
                    topic: topic.clone(),
                    message: message.clone()
                });
            }

            // Send to all subscriptions if topic matches
            // Copy relevant subscriptions before calling them to avoid deadlock if a subscription
            // does another publish (as handle() does)
            let mut relevant_subscriptions: Vec<MockSubscription<M>> = Vec::new();
            {
                let subscriptions = subscriptions.lock().await;
                for sub in subscriptions.iter() {
                    if match_topic(&sub.topic, &topic) {
                        relevant_subscriptions.push(sub.subscription.clone())
                    }
                }
            }

            for sub in relevant_subscriptions.iter() {
                sub.push(&topic, message.clone()).await;
            }

            Ok(())
        })
    }

    fn subscribe(&self, topic: &str) -> BoxFuture<Result<Box<dyn Subscription<M>>>> {
        let topic = topic.to_string();
        Box::pin(async move {
            debug!("Mock subscribe on {topic}");
            let mut subscriptions = self.subscriptions.lock().await;
            let subscription = MockSubscription::new();
            subscriptions.push(SubscriptionRecord {
                topic: topic.to_string(),
                subscription: subscription.clone(),
            });
            Ok(Box::new(subscription) as Box<dyn Subscription<M>>)
        })
    }

    fn unsubscribe(&self, _subscription: Box<dyn Subscription<M>>) {
        // !TODO
    }

    fn shutdown(&self) -> BoxFuture<'static, Result<()>> {
        let shutdowns = self.shutdowns.clone();
        Box::pin(async move {
            let mut shutdowns = shutdowns.lock().await;
            *shutdowns += 1;
            Ok(())
        })
    }
}

// -- Tests --
#[cfg(test)]
mod tests {
    use super::*;
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

            // Create mock bus
            let mock = Arc::new(MockBus::<M>::new());

            Self { mock }
        }
    }

    #[tokio::test]
    async fn publish_passes_straight_through() {
        let setup = TestSetup::<String>::new("");

        // Send a message
        assert!(setup.mock.publish("test", Arc::new("Hello, world!".to_string())).await.is_ok());

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
        let _subscription = setup.mock.subscribe("test").await;

        // Check the mock got it
        let mock_subscriptions = setup.mock.subscriptions.lock().await;
        assert_eq!(mock_subscriptions.len(), 1);
        let sub0 = &mock_subscriptions[0];
        assert_eq!(sub0.topic, "test");
    }

    #[tokio::test]
    async fn request_times_out_with_no_response() {
        let setup = TestSetup::<String>::new("timeout = 1");

        // Request with no response should timeout
        assert!(setup.mock.request("test", Arc::new("Hello, world!".to_string()))
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
}
