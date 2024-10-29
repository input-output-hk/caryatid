//! Generic MessageBus trait for any pub-sub bus
use futures::future::{BoxFuture, FutureExt, Future};
use anyhow::Result;
use std::sync::Arc;

/// Subscriber pattern function types
pub type Subscriber<M> = dyn Fn(Arc<M>) ->
    BoxFuture<'static, Arc<Result<M>>> + Send + Sync + 'static;

/// Message bounds trait (awaiting trait aliases)
pub trait MessageBounds: Send + Sync + Clone + Default + 'static {}
impl<T: Send + Sync + Clone + Default + 'static> MessageBounds for T {}

/// Generic MessageBus trait
pub trait MessageBus<M: MessageBounds>: Send + Sync {

    /// Publish a message - note async but not defined as such because this
    /// is used dynamically
    fn publish(&self, topic: &str, message: Arc<M>)
               -> BoxFuture<'static, anyhow::Result<()>>;

    /// Request/response - as publish() but returns a result
    fn request(&self, topic: &str, message: Arc<M>)
               -> BoxFuture<'static, anyhow::Result<Arc<Result<M>>>>;

    /// Register an subscriber function - note sync
    fn register_subscriber(&self, topic: &str, subscriber: Arc<Subscriber<M>>)
                         -> Result<()>;

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
        F: Fn(Arc<M>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Arc<Result<M>>> + Send + 'static;
}

impl<M: MessageBounds> MessageBusExt<M> for Arc<dyn MessageBus<M>> {

    fn subscribe<F>(&self, topic: &str, subscriber: F) -> Result<()>
    where
        F: Fn(Arc<M>) + Send + Sync + 'static
    {
        let arc_subscriber: Arc<Subscriber<M>> =
            Arc::new(move |message: Arc<M>| {
                subscriber(message);

                async move { Arc::new(Ok(M::default())) }.boxed()
            });

        self.register_subscriber(topic, arc_subscriber)
    }

    fn handle<F, Fut>(&self, topic: &str, handler: F) -> Result<()>
    where
        F: Fn(Arc<M>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Arc<Result<M>>> + Send + 'static,
    {
        let arc_subscriber: Arc<Subscriber<M>> =
            Arc::new(move |message: Arc<M>| {
                handler(message).boxed()
            });

        self.register_subscriber(topic, arc_subscriber)
    }

}

