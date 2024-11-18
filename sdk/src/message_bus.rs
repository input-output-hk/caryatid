//! Generic MessageBus trait for any pub-sub bus
use futures::future::{BoxFuture, Future, ready};
use anyhow::{Result, anyhow};
use std::sync::Arc;

/// Subscriber pattern function types - takes topic and message
pub type Subscriber<M> = dyn Fn(&str, Arc<M>) ->
    BoxFuture<'static, ()> + Send + Sync + 'static;

/// Message bounds trait (awaiting trait aliases)
pub trait MessageBounds: Send + Sync + Clone + Default +
    serde::Serialize + serde::de::DeserializeOwned + 'static {}
impl<T: Send + Sync + Clone + Default + serde::Serialize +
     serde::de::DeserializeOwned + 'static> MessageBounds for T {}

/// Generic MessageBus trait
pub trait MessageBus<M: MessageBounds>: Send + Sync {

    /// Publish a message - note async but not defined as such because this
    /// is used dynamically
    fn publish(&self, topic: &str, message: Arc<M>) -> BoxFuture<'static, anyhow::Result<()>>;

    /// Request/response - as publish() but returns a result
    /// Note only implemented in CorrelationBus
    fn request(&self, _topic: &str, _message: Arc<M>)
               -> BoxFuture<'static, anyhow::Result<Arc<M>>> {
       Box::pin(ready(Err(anyhow!("Not implemented"))))
    }

    /// Register an subscriber function - note sync
    fn register_subscriber(&self, topic: &str, subscriber: Arc<Subscriber<M>>) -> Result<()>;

    /// Shut down
    fn shutdown(&self) -> BoxFuture<'static, anyhow::Result<()>>;
}

/// Extension trait to sugar registration
/// Needed because MessageBus must be object-safe to be used dynamically,
/// which means we can't accept closures through type parameters
pub trait MessageBusExt<M: MessageBounds> {
    /// Register a simple lambda/closure with no result
    fn subscribe<F>(&self, topic: &str, subscriber: F) -> Result<()>
    where
        F: Fn(Arc<M>) + Send + Sync + 'static;

    /// Register a handler function which returns a future for
    /// the result
    fn handle<F, Fut>(&self, topic: &str, subscriber: F) -> Result<()>
    where
        F: Fn(Arc<M>) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = Result<Arc<M>>> + Send + 'static;
}

impl<M: MessageBounds> MessageBusExt<M> for Arc<dyn MessageBus<M>> {

    fn subscribe<F>(&self, topic: &str, subscriber: F) -> Result<()>
    where
        F: Fn(Arc<M>) + Send + Sync + 'static
    {
        let arc_subscriber: Arc<Subscriber<M>> =
            Arc::new(move |_topic: &str, message: Arc<M>| {
                subscriber(message);
                Box::pin(ready(()))
            });

        self.register_subscriber(topic, arc_subscriber)
    }

    fn handle<F, Fut>(&self, topic: &str, handler: F) -> Result<()>
    where
        F: Fn(Arc<M>) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = Result<Arc<M>>> + Send + 'static,
    {
        let arc_self = self.clone();
        let arc_subscriber: Arc<Subscriber<M>> =
            Arc::new(move |topic, message: Arc<M>| {

                let arc_self = arc_self.clone();
                let handler = handler.clone();
                let response_topic = topic.to_owned() + ".response";

                tokio::spawn(async move {
                    match handler(message).await.as_ref() {
                        Ok(response) => {
                            // Return the result with response topic
                            let _ = arc_self.publish(&response_topic, response.clone()).await;
                        },
                        Err(e) => {
                            // Return an error response
                            // !!! How can we reliably create an error message?
                            // !!! Needs a trait in MessageBounds which ensures M::error()
                            arc_self.publish(&response_topic, Arc::new(M::default())).await;
                        }
                    }
                });

                Box::pin(ready(()))
            });

        // Subscribe for all request IDs in this topic
        let request_pattern = format!("{topic}.*");
        self.register_subscriber(&request_pattern, arc_subscriber)
    }
}

