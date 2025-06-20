//! Typed messages Caraytid module - publisher side
use crate::message::{Message, Test};
use anyhow::Result;
use caryatid_sdk::{module, Context, Module};
use config::Config;
use serde_json::json;
use std::sync::Arc;
use tracing::info;

/// Typed publisher module
#[module(
    message_type(Message),
    name = "publisher",
    description = "Typed publisher module"
)]
pub struct Publisher;

impl Publisher {
    // Implement the single initialisation function, with application
    // Context and this module's Config
    async fn init(&self, context: Arc<Context<Message>>, config: Arc<Config>) -> Result<()> {
        let message_bus = context.message_bus.clone();

        // Get configuration
        let topic = config.get_string("topic").unwrap_or("sample".to_string());
        info!("Creating publisher on '{}'", topic);

        // Send test messages to the message bus on 'sample_topic'
        // Let this run async
        context.run(async move {
            // Custom struct
            let test_message_1 = Message::Test(Test {
                data: "Hello, world!".to_string(),
                number: 42,
            });
            info!("Sending {:?}", test_message_1);
            message_bus
                .publish(&format!("{topic}.test"), Arc::new(test_message_1))
                .await
                .expect("Failed to publish message");

            // Simple string
            let test_message_2 = Message::String("Bye!".to_string());
            info!("Sending {:?}", test_message_2);

            message_bus
                .publish(&format!("{topic}.string"), Arc::new(test_message_2))
                .await
                .expect("Failed to publish message");

            // JSON
            let test_message_3 = Message::JSON(json!({
                "message": "Hello, world!",
            }));
            info!("Sending {:?}", test_message_3);
            message_bus
                .publish(&format!("{topic}.json"), Arc::new(test_message_3))
                .await
                .expect("Failed to publish message");
        });

        Ok(())
    }
}
