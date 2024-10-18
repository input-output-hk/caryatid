// Shared context passed to each module

use std::sync::Arc;
use config::Config;
use crate::message_bus::MessageBus;
use std::fmt;

pub struct Context {
    pub config: Arc<Config>,
    pub message_bus: Arc<dyn MessageBus<serde_json::Value>>,
    pub tokio_runtime: tokio::runtime::Handle,
}

impl Context {
    pub fn new(
        config: Arc<Config>,
        message_bus: Arc<dyn MessageBus<serde_json::Value>>,
        tokio_runtime: tokio::runtime::Handle,
    ) -> Self {
        Self { config, message_bus, tokio_runtime }
    }
}

/// Minimal implementation of Debug for tracing
impl fmt::Debug for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Context")
            .finish()
    }
}
