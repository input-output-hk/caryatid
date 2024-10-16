// Generic MessageBus trait for any pub-sub bus
use futures::future::{BoxFuture, FutureExt};
use anyhow::Result;
use std::sync::Arc;

// Observer pattern function types
pub type ObserverFn<M> = dyn Fn(Arc<M>) ->
    BoxFuture<'static, ()> + Send + Sync + 'static;
pub type BoxedObserverFn<M> = Box<ObserverFn<M>>;

// Message bounds trait (awaiting trait aliases)
pub trait MessageBounds: Send + Sync + Clone + 'static {}
impl<T: Send + Sync + Clone + 'static> MessageBounds for T {}

// MessageBus trait
pub trait MessageBus<M: MessageBounds>: Send + Sync {

    // Publish a message - note async but not defined as such because this
    // is used dynamically
    fn publish(&self, topic: &str, message: Arc<M>)
               -> BoxFuture<'static, anyhow::Result<()>>;

    // Register an observer function - note sync
    fn register_observer(&self, topic: &str, observer: BoxedObserverFn<M>)
                         -> Result<()>;

    // Shut down
    fn shutdown(&self);
}

// Extension trait to sugar registration - needed because MessageBus must be
// object-safe to be used dynamically, which means we can't accept closures
// through type parameters
pub trait MessageBusExt<M: MessageBounds> {
    fn register<F>(&self, topic: &str, observer: F) -> Result<()>
    where
        F: Fn(Arc<M>) + MessageBounds;
}

impl<M: MessageBounds> MessageBusExt<M> for Arc<dyn MessageBus<M>> {
    // Register a simple lambda/closure
    fn register<F>(&self, topic: &str, observer: F) -> Result<()>
    where
        F: Fn(Arc<M>) + MessageBounds,
    {
        let boxed_observer: BoxedObserverFn<M> =
            Box::new(move |message: Arc<M>| {
            let observer = observer.clone();
            async move {
                observer(message);
            }
            .boxed()
        });

        self.register_observer(topic, boxed_observer)
    }
}

