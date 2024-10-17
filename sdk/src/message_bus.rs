// Generic MessageBus trait for any pub-sub bus
use futures::future::{BoxFuture, FutureExt};
use anyhow::Result;
use std::sync::Arc;

// Subscriber pattern function types
pub type Subscriber<M> = dyn Fn(Arc<M>) ->
    BoxFuture<'static, ()> + Send + Sync + 'static;
pub type BoxedSubscriber<M> = Box<Subscriber<M>>;

// Message bounds trait (awaiting trait aliases)
pub trait MessageBounds: Send + Sync + Clone + 'static {}
impl<T: Send + Sync + Clone + 'static> MessageBounds for T {}

// MessageBus trait
pub trait MessageBus<M: MessageBounds>: Send + Sync {

    // Publish a message - note async but not defined as such because this
    // is used dynamically
    fn publish(&self, topic: &str, message: Arc<M>)
               -> BoxFuture<'static, anyhow::Result<()>>;

    // Register an subscriber function - note sync
    fn register_subscriber(&self, topic: &str, subscriber: BoxedSubscriber<M>)
                         -> Result<()>;

    // Shut down
    fn shutdown(&self) -> BoxFuture<'static, anyhow::Result<()>>;
}

// Extension trait to sugar registration - needed because MessageBus must be
// object-safe to be used dynamically, which means we can't accept closures
// through type parameters
pub trait MessageBusExt<M: MessageBounds> {
    fn subscribe<F>(&self, topic: &str, subscriber: F) -> Result<()>
    where
        F: Fn(Arc<M>) + MessageBounds;
}

impl<M: MessageBounds> MessageBusExt<M> for Arc<dyn MessageBus<M>> {
    // Register a simple lambda/closure
    fn subscribe<F>(&self, topic: &str, subscriber: F) -> Result<()>
    where
        F: Fn(Arc<M>) + MessageBounds,
    {
        let boxed_subscriber: BoxedSubscriber<M> =
            Box::new(move |message: Arc<M>| {
            let subscriber = subscriber.clone();
            async move {
                subscriber(message);
            }
            .boxed()
        });

        self.register_subscriber(topic, boxed_subscriber)
    }
}

