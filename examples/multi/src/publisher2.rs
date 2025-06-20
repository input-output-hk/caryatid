//! Sample Caraytid module - publisher side
use anyhow::Result;
use caryatid_sdk::{module, Context, Module};
use config::Config;
use serde_json::json;
use std::sync::Arc;
use tracing::info;

/// Standard message type
type MType = serde_json::Value;

/// Sample publisher module
// Define it as a module, with a name and description
#[module(
    message_type(MType),
    name = "publisher2",
    description = "Sample publisher module (#2)"
)]
pub struct Publisher;

impl Publisher {
    // Implement the single initialisation function, with application
    // Context and this module's Config
    async fn init(&self, context: Arc<Context<MType>>, config: Arc<Config>) -> Result<()> {
        let message_bus = context.message_bus.clone();

        // Get configuration
        let topic = config.get_string("topic").unwrap_or("test".to_string());
        info!("Creating publisher on '{}'", topic);

        // Send a test JSON message to the message bus on 'sample_topic'
        // Let this run async
        context.run(async move {
            let test_message = Arc::new(json!({
                "message": "Hello, world! from publisher #2",
            }));

            info!("Sending {:?}", test_message);

            message_bus
                .publish(&topic, test_message)
                .await
                .expect("Failed to publish message");
        });

        Ok(())
    }
}
