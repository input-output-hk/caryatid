//! Sample Caraytid module - subscriber side
use caryatid_sdk::*;
use std::sync::Arc;
use anyhow::Result;
use config::Config;
use tracing::{info};

/// Sample module
// Define it as a module, with a name and description
#[module(
    name = "sample-subscriber",
    description = "Sample subscriber module"
)]
pub struct SampleSubscriber;

impl SampleSubscriber {

    // Implement the single initialisation function, with application
    // Context and this module's Config
    fn init(&self, context: Arc<Context>, config: Arc<Config>) -> Result<()> {

        // Get configuration
        let topic = config.get_string("topic").unwrap_or("test".to_string());
        info!("Initialising sample subscriber on '{}'", topic);

        // Register a subscriber on the message bus to listen for messages
        // Messages are passed as JSON objects, in an Arc
        context.message_bus.subscribe(&topic,
                                      |message: Arc<serde_json::Value>| {
           info!("Received: {:?}", message);
        })?;

        Ok(())
    }
}

