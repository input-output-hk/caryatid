//! Generic MessageBus trait for any pub-sub bus
use futures::future::{BoxFuture, Future, ready};
use anyhow::{Result, anyhow};
use std::sync::Arc;
use tracing::error;

/// Subscriber pattern function types - takes topic and message
pub type Subscriber<M> = dyn Fn(&str, Arc<M>) ->
    BoxFuture<'static, ()> + Send + Sync + 'static;

/// Message quality-of-service
#[derive(Clone, Copy)]
pub enum QoS {
    Normal,       // Normal messages generated one at a time
    Bulk,         // Messages generated quickly in large volumes
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
    fn publish(&self, topic: &str, message: Arc<M>) -> BoxFuture<'static, Result<()>> {
        self.publish_with_qos(topic, message, QoS::Normal)
    }

    /// Publish a message with given QoS
    fn publish_with_qos(&self, topic: &str, message: Arc<M>, qos: QoS) 
        -> BoxFuture<'static, Result<()>>;

    /// Request/response - as publish() but returns a result
    /// Note only implemented in CorrelationBus
    fn request(&self, _topic: &str, _message: Arc<M>)
               -> BoxFuture<'static, anyhow::Result<Arc<M>>> {
       Box::pin(ready(Err(anyhow!("Not implemented"))))
    }

    /// Register an subscriber function
    fn register_subscriber(&self, topic: &str, subscriber: Arc<Subscriber<M>>)
                           -> BoxFuture<'static, Result<()>>;

    /// Shut down
    fn shutdown(&self) -> BoxFuture<'static, anyhow::Result<()>>;
}

/// Extension trait to sugar registration
/// Needed because MessageBus must be object-safe to be used dynamically,
/// which means we can't accept closures through type parameters
pub trait MessageBusExt<M: MessageBounds> {
    /// Register a simple asynchronous lambda/closure with no result
    fn subscribe<F, Fut>(&self, topic: &str, subscriber: F) -> Result<()>
    where
        F: Fn(Arc<M>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static;

    /// Register a handler function which returns a future for
    /// the result.  They must return an M, not a Result, because
    /// we have no generic way of passing back an error.
    fn handle<F, Fut>(&self, topic: &str, subscriber: F) -> Result<()>
    where
        F: Fn(Arc<M>) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = Arc<M>> + Send + 'static;
}

impl<M: MessageBounds> MessageBusExt<M> for Arc<dyn MessageBus<M>> {

    fn subscribe<F, Fut>(&self, topic: &str, subscriber: F) -> Result<()>
    where
        F: Fn(Arc<M>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static
     {
        let arc_self = self.clone();
        let topic = topic.to_string();

        tokio::spawn(async move {
            let arc_subscriber: Arc<Subscriber<M>> =
                Arc::new(move |_topic: &str, message: Arc<M>| {
                    Box::pin(subscriber(message))
                });

            arc_self.register_subscriber(&topic, arc_subscriber).await
        });

        Ok(())
    }

    fn handle<F, Fut>(&self, topic: &str, handler: F) -> Result<()>
    where
        F: Fn(Arc<M>) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = Arc<M>> + Send + 'static,
    {
        let arc_self = self.clone();
        let topic = topic.to_string();

        tokio::spawn(async move {
            let arc_self_2 = arc_self.clone();
            let arc_subscriber: Arc<Subscriber<M>> =
                Arc::new(move |topic, message: Arc<M>| {

                    let arc_self = arc_self_2.clone();
                    let handler = handler.clone();
                    let response_topic = topic.to_owned() + ".response";

                    Box::pin(async move {
                        let response = handler(message).await;
                        if let Err(e) = arc_self.publish(&response_topic, response.clone()).await {
                            error!("Response on {response_topic} failed {e} - timed out?");
                        }
                    })
                });

            // Subscribe for all request IDs in this topic
            let request_pattern = format!("{topic}.*");
            arc_self.register_subscriber(&request_pattern, arc_subscriber).await
        });

        Ok(())
    }
}

