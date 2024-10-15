// Shared context passed to each module

use std::sync::Arc;
use crate::message_bus::MessageBus;

pub struct Context {
    pub message_bus: Arc<dyn MessageBus<serde_json::Value>>,
}

impl Context {
    pub fn new(
        message_bus: Arc<dyn MessageBus<serde_json::Value>>,
    ) -> Self {
        Self { message_bus }
    }
}
