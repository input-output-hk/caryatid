//! Mock message bus for tests
use std::sync::Arc;
use futures::future::BoxFuture;
use anyhow::Result;
use tokio::sync::Mutex;
use tracing::debug;
use crate::message_bus::{MessageBus, Subscriber, MessageBounds};
use crate::match_topic::match_topic;

pub struct PublishRecord<M: MessageBounds> {
    pub topic: String,
    pub message: Arc<M>
}

pub struct SubscribeRecord<M: MessageBounds> {
    pub topic: String,
    pub subscriber: Arc<Subscriber<M>>,
}

pub struct MockBus<M: MessageBounds> {
    pub publishes: Arc<Mutex<Vec<PublishRecord<M>>>>,
    pub subscribes: Arc<Mutex<Vec<SubscribeRecord<M>>>>,
    pub shutdowns: Arc<Mutex<u16>>,           // just count them
}

impl<M: MessageBounds> MockBus<M> {
    pub fn new() -> Self {
        Self {
            publishes: Arc::new(Mutex::new(Vec::new())),
            subscribes: Arc::new(Mutex::new(Vec::new())),
            shutdowns: Arc::new(Mutex::new(0)),
        }
    }
}

impl<M> MessageBus<M> for MockBus<M>
where M: MessageBounds + serde::Serialize + serde::de::DeserializeOwned {

    fn publish(&self, topic: &str, message: Arc<M>) -> BoxFuture<'static, Result<()>> {

        debug!("Mock publish on {topic}");
        let publishes = self.publishes.clone();
        let subscribes = self.subscribes.clone();
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

            // Send to all subscribers if topic matches
            // Copy relevant subscribers before calling them to avoid deadlock if a subscriber
            // does another publish (as handle() does)
            let mut relevant_subscribers: Vec<Arc<Subscriber<M>>> = Vec::new();
            {
                let subscribes = subscribes.lock().await;
                for sub in subscribes.iter() {
                    if match_topic(&sub.topic, &topic) {
                        relevant_subscribers.push(sub.subscriber.clone())
                    }
                }
            }

            for sub in relevant_subscribers.iter() {
                sub(&topic, message.clone()).await;
            }

            Ok(())
        })
    }

    fn register_subscriber(&self, topic: &str, subscriber: Arc<Subscriber<M>>)
                           -> BoxFuture<'static, Result<()>> {
        debug!("Mock subscribe on {topic}");
        let subscribes = self.subscribes.clone();
        let topic = topic.to_string();

        Box::pin(async move {
            let mut subscribes = subscribes.lock().await;
            subscribes.push(SubscribeRecord{ topic, subscriber });
            Ok(())
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

