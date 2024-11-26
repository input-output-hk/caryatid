//! Mock message bus for tests
use caryatid_sdk::message_bus::{MessageBus, Subscriber, MessageBounds};
use std::sync::Arc;
use futures::future::{ready, BoxFuture};
use anyhow::Result;
use std::sync::Mutex;  // ! Note, not tokio - keeps tests simpler

#[cfg(test)]
pub struct PublishRecord<M: MessageBounds> {
    pub topic: String,
    pub message: Arc<M>
}

#[cfg(test)]
pub struct MockBus<M: MessageBounds> {
    pub publishes: Arc<Mutex<Vec<PublishRecord<M>>>>,
    pub subscribes: Arc<Mutex<Vec<String>>>,  // topics
}

impl<M: MessageBounds> MockBus<M> {
    pub fn new() -> Self {
        Self {
            publishes: Arc::new(Mutex::new(Vec::new())),
            subscribes: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl<M> MessageBus<M> for MockBus<M>
where M: MessageBounds + serde::Serialize + serde::de::DeserializeOwned {

    fn publish(&self, topic: &str, message: Arc<M>) -> BoxFuture<'static, Result<()>> {

        let publishes = self.publishes.clone();
        let topic = topic.to_string();

        let mut publishes = publishes.lock().unwrap();
        publishes.push(PublishRecord{ topic, message });

        Box::pin(ready(Ok(())))
    }

    fn register_subscriber(&self, topic: &str, _subscriber: Arc<Subscriber<M>>) -> Result<()> {
        let subscribes = self.subscribes.clone();
        let topic = topic.to_string();

        let mut subscribes = subscribes.lock().unwrap();
        subscribes.push(topic);

        Ok(())
    }

    fn shutdown(&self) -> BoxFuture<'static, Result<()>> {
        Box::pin(ready(Ok(())))
    }
}

