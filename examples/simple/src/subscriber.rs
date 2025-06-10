//! Simple Caraytid module - subscriber side
use anyhow::Result;
use caryatid_sdk::{module, Context, MessageBusExt, Module};
use config::Config;
use std::sync::Arc;
use tracing::info;

/// Standard message type
type MType = serde_json::Value;

/// Sample module
// Define it as a module, with a name and description
#[module(
    message_type(MType),
    name = "subscriber",
    description = "Sample subscriber module"
)]
pub struct Subscriber;

impl Subscriber {
    // Implement the single initialisation function, with application
    // Context and this module's Config
    async fn init(&self, context: Arc<Context<MType>>, config: Arc<Config>) -> Result<()> {
        // Get configuration
        let topic = config.get_string("topic").unwrap_or("test".to_string());
        info!("Creating subscriber on '{}'", topic);

        // Register a subscriber on the message bus to listen for messages
        // Messages are passed as JSON objects, in an Arc
        context
            .message_bus
            .subscribe(&topic, |message: Arc<serde_json::Value>| async move {
                info!("Received: {:?}", message);
            })?;

        Ok(())
    }
}
