//! Generic MessageBus trait for any pub-sub bus
use futures::future::BoxFuture;
use anyhow::{anyhow, Result};
use std::sync::Arc;
use tokio::time::{Duration, timeout};
use async_trait::async_trait;

/// Subscriber pattern function types - takes topic and message
pub type Subscriber<M> = dyn Fn(&str, Arc<M>) ->
    BoxFuture<'static, ()> + Send + Sync + 'static;

pub trait SubscriptionBounds: Send {}
pub trait Subscription<M>: SubscriptionBounds {
    fn read(&mut self) -> BoxFuture<anyhow::Result<(String, Arc<M>)>>;
}

/// Message bounds trait (awaiting trait aliases)
pub trait MessageBounds: Send + Sync + Clone + Default +
    serde::Serialize + serde::de::DeserializeOwned + 'static {}
impl<T: Send + Sync + Clone + Default + serde::Serialize +
     serde::de::DeserializeOwned + 'static> MessageBounds for T {}

/// Generic MessageBus trait
 #[async_trait]
pub trait MessageBus<M: MessageBounds>: Send + Sync {

    /// Publish a message
    /// Note async but not defined as such because this is used dynamically
    async fn publish(&self, topic: &str, message: Arc<M>) -> Result<()>;

    /// Get the request_timeout
    fn request_timeout(&self) -> Duration;

    /// Request/response - as publish() but returns a result
    async fn request(&self, topic: &str, message: Arc<M>) -> Result<Arc<M>> {
        let topic = topic.to_string();
        let mut random_bytes = [0u8; 8];
        rand::fill(&mut random_bytes);
        let request_timeout = self.request_timeout();
        let request_id = hex::encode(random_bytes);
        let response_topic = format!("{topic}.{request_id}.response");
        let request_topic = format!("{topic}.{request_id}.request");
        let mut subscription = self.subscribe(&response_topic).await?;
        let subscription_future = subscription.read();
        self.publish(&request_topic, message).await?;
        match timeout(request_timeout, subscription_future).await {
            Ok(Ok((_, message))) => {
                Ok(message)
            },
            Ok(Err(e)) => {
                Err(e)
            },
            _ => Err(anyhow!("Request timed out"))
        }
    }

    /// Subscribe to a topic on the bus
    async fn subscribe(&self, topic: &str) -> Result<Box<dyn Subscription<M>>>;

    /// Shut down
    async fn shutdown(&self) -> Result<()>;
}
