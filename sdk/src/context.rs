// Shared context passed to each module

use std::sync::Arc;
use config::Config;
use crate::message_bus::{MessageBounds, MessageBus};
use std::fmt;
use std::future::Future;
use tokio::sync::watch::Sender;
use tokio::task;

pub struct Context<M: MessageBounds> {
    pub config: Arc<Config>,
    pub message_bus: Arc<dyn MessageBus<M>>,
    pub startup_watch: Sender<bool>,
}

impl<M: MessageBounds> Context<M> {
    pub fn new(
        config: Arc<Config>,
        message_bus: Arc<dyn MessageBus<M>>,
        startup_watch: Sender<bool>,
    ) -> Self {
        Self { config, message_bus, startup_watch }
    }

    pub fn run<T, F>(&self, func: F) -> task::JoinHandle<T>
    where
        T: Send + 'static,
        F: Future<Output = T> + Send + 'static,
    {
        let mut signal = self.startup_watch.subscribe();
        tokio::spawn(async move {
            loop {
                let _ = signal.changed().await;
                if *signal.borrow_and_update() {
                    break;
                }
            }
            func.await
        })
    }
}

/// Minimal implementation of Debug for tracing
impl<M: MessageBounds> fmt::Debug for Context<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Context")
            .finish()
    }
}
