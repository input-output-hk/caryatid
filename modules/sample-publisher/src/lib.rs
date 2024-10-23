//! Sample Caraytid module - publisher side
use caryatid_sdk::*;
use std::sync::Arc;
use anyhow::Result;
use config::Config;
use tracing::{info};
use serde_json::json;

/// Sample module
// Define it as a module, with a name and description
#[module(
    name = "sample-publisher",
    description = "Sample publisher module"
)]
pub struct SamplePublisher;

impl SamplePublisher {

    // Implement the single initialisation function, with application
    // Context and this module's Config
    fn init(&self, context: &Context, config: &Config) -> Result<()> {
        let message_bus = context.message_bus.clone();

        // Get configuration
        let topic = config.get_string("topic").unwrap_or("test".to_string());
        info!("Initialising sample publisher on '{}'", topic);

        // Send a test JSON message to the message bus on 'sample_topic'
        // Let this run async
        tokio::spawn(async move {

            let test_message = Arc::new(json!({
                "message": "Hello, world!",
            }));

            info!("Sending {:?}", test_message);

            message_bus.publish(&topic, test_message)
                .await.expect("Failed to publish message");
        });

        Ok(())
    }
}

