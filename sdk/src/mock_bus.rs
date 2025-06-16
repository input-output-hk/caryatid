//! Mock message bus for tests
use crate::match_topic::match_topic;
use crate::message_bus::{MessageBounds, MessageBus, Subscription, SubscriptionBounds};
use anyhow::{bail, Result};
use futures::future::BoxFuture;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tracing::debug;

pub struct PublishRecord<M: MessageBounds> {
    pub topic: String,
    pub message: Arc<M>,
}

pub struct MockSubscription<M> {
    topic: String,
    receiver: mpsc::UnboundedReceiver<(String, Arc<M>)>,
}

pub struct SubscriptionRecord<M> {
    pub topic: String,
    pub sender: mpsc::UnboundedSender<(String, Arc<M>)>,
}

impl<M: MessageBounds> SubscriptionBounds for MockSubscription<M> {}

impl<M> MockSubscription<M> {
    pub fn new(topic: String, receiver: mpsc::UnboundedReceiver<(String, Arc<M>)>) -> Self {
        Self { topic, receiver }
    }
}

impl<M: MessageBounds> Subscription<M> for MockSubscription<M> {
    fn read(&mut self) -> BoxFuture<anyhow::Result<(String, Arc<M>)>> {
        Box::pin(async move {
            let Some(entry) = self.receiver.recv().await else {
                bail!("Sender for topic {} has unexpectedly closed", self.topic);
            };
            Ok(entry)
        })
    }
}

pub struct MockBus<M: MessageBounds> {
    pub publishes: Arc<Mutex<Vec<PublishRecord<M>>>>,
    pub subscriptions: Arc<Mutex<Vec<SubscriptionRecord<M>>>>,
    pub shutdowns: Arc<Mutex<u16>>, // just count them
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
where
    M: MessageBounds + serde::Serialize + serde::de::DeserializeOwned,
{
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
                    message: message.clone(),
                });
            }

            // Send to all subscriptions if topic matches
            // Copy relevant subscriptions before calling them to avoid deadlock if a subscription
            // does another publish (as handle() does)
            let mut relevant_subscriptions: Vec<mpsc::UnboundedSender<(String, Arc<M>)>> =
                Vec::new();
            {
                let subscriptions = subscriptions.lock().await;
                for sub in subscriptions.iter() {
                    if match_topic(&sub.topic, &topic) {
                        relevant_subscriptions.push(sub.sender.clone())
                    }
                }
            }

            for sub in relevant_subscriptions.iter() {
                sub.send((topic.clone(), message.clone()))?;
            }

            Ok(())
        })
    }

    fn register(&self, topic: &str) -> BoxFuture<Result<Box<dyn Subscription<M>>>> {
        let topic = topic.to_string();
        Box::pin(async move {
            debug!("Mock subscribe on {topic}");
            let mut subscriptions = self.subscriptions.lock().await;
            let (sender, receiver) = mpsc::unbounded_channel();
            let subscription = MockSubscription::new(topic.clone(), receiver);
            subscriptions.push(SubscriptionRecord {
                topic: topic.to_string(),
                sender,
            });
            Ok(Box::new(subscription) as Box<dyn Subscription<M>>)
        })
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
