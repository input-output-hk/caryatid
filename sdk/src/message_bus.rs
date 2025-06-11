//! Generic MessageBus trait for any pub-sub bus
use futures::future::BoxFuture;
use anyhow::{anyhow, Result};
use rand::Rng;
use std::sync::Arc;
use tokio::time::{Duration, timeout};

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
pub trait MessageBus<M: MessageBounds>: Send + Sync {

    /// Publish a message with normal QoS
    /// Note async but not defined as such because this is used dynamically
    fn publish(&self, topic: &str, message: Arc<M>) -> BoxFuture<'static, Result<()>>;

    /// Request/response - as publish() but returns a result
    fn request(&self, topic: &str, message: Arc<M>)
               -> BoxFuture<anyhow::Result<Arc<M>>> {
        let topic = topic.to_string();
        let mut rng = rand::thread_rng();
        let random_bytes: [u8; 8] = rng.gen();
        Box::pin(async move {
            let request_id = hex::encode(random_bytes);
            let response_topic = format!("{topic}.{request_id}.response");
            let request_topic = format!("{topic}.{request_id}.request");
            let mut subscription = self.subscribe(&response_topic).await?;
            let subscription_future = subscription.read();
            self.publish(&request_topic, message).await?;
            // !TODO: get timeout from somewhere configurable
            let result = timeout(Duration::from_secs(5u64), subscription_future).await;
            self.unsubscribe(subscription);
            match result {
                Ok(Ok((_, message))) => {
                    Ok(message)
                },
                Ok(Err(e)) => {
                    Err(e)
                },
                _ => Err(anyhow!("Request timed out"))
            }
        })
    }

    /// Subscribe to a topic on the bus
    fn subscribe(&self, topic: &str) -> BoxFuture<Result<Box<dyn Subscription<M>>>>;

    /// Unsubscribe from the bus
    fn unsubscribe(&self, subscription: Box<dyn Subscription<M>>);

    /// Shut down
    fn shutdown(&self) -> BoxFuture<'static, anyhow::Result<()>>;
}
