// Shared context passed to each module

use std::sync::Arc;
use config::Config;
use crate::message_bus::{MessageBounds, MessageBus};
use std::fmt;

pub struct Context<M: MessageBounds> {
    pub config: Arc<Config>,
    pub message_bus: Arc<dyn MessageBus<M>>,
}

impl<M: MessageBounds> Context<M> {
    pub fn new(
        config: Arc<Config>,
        message_bus: Arc<dyn MessageBus<M>>,
    ) -> Self {
        Self { config, message_bus }
    }
}

/// Minimal implementation of Debug for tracing
impl<M: MessageBounds> fmt::Debug for Context<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Context")
            .finish()
    }
}
