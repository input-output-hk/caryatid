// Shared context passed to each module

use std::sync::Arc;
use config::Config;
use crate::message_bus::MessageBus;

pub struct Context {
    pub config: Arc<Config>,
    pub message_bus: Arc<dyn MessageBus<serde_json::Value>>,
}

impl Context {
    pub fn new(
        config: Arc<Config>,
        message_bus: Arc<dyn MessageBus<serde_json::Value>>,
    ) -> Self {
        Self { config, message_bus }
    }
}

