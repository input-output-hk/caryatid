//! Typed messages Caraytid module - publisher side
use caryatid_sdk::{Context, Module, module};
use std::sync::Arc;
use anyhow::Result;
use config::Config;
use tracing::{info};
use serde_json::json;
use crate::message::{Test, Message};

/// Typed publisher module
#[module(
    message_type = "Message",
    name = "typed-publisher",
    description = "Typed publisher module"
)]
pub struct TypedPublisher;

impl TypedPublisher {

    // Implement the single initialisation function, with application
    // Context and this module's Config
    fn init(&self, context: Arc<Context<Message>>, config: Arc<Config>) -> Result<()> {
        let message_bus = context.message_bus.clone();

        // Get configuration
        let topic = config.get_string("topic").unwrap_or("test".to_string());
        info!("Creating typed publisher on '{}'", topic);

        // Send test messages to the message bus on 'sample_topic'
        // Let this run async
        tokio::spawn(async move {

            // Custom struct
            let test_message_1 = Message::Test(Test {
                data: "Hello, world!".to_string(),
                number: 42
            });
            info!("Sending {:?}", test_message_1);
            message_bus.publish(&topic, Arc::new(test_message_1))
                .await.expect("Failed to publish message");

            // Simple string
            let test_message_2 = Message::String("Bye!".to_string());
            info!("Sending {:?}", test_message_2);

            message_bus.publish(&topic, Arc::new(test_message_2))
                .await.expect("Failed to publish message");

            // JSON
            let test_message_3 = Message::JSON(json!({
                "message": "Hello, world!",
            }));
            info!("Sending {:?}", test_message_3);
            message_bus.publish(&topic, Arc::new(test_message_3))
                .await.expect("Failed to publish message");
        });

        Ok(())
    }
}

