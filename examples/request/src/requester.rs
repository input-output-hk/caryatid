//! Sample Caraytid module - requester side
use anyhow::Result;
use caryatid_sdk::{module, Context, Module};
use config::Config;
use serde_json::json;
use std::sync::Arc;
use tracing::{error, info};

/// Standard message type
type MType = serde_json::Value;

/// Simple requester module
#[module(
    message_type(MType),
    name = "requester",
    description = "Sample requester"
)]
pub struct Requester;

impl Requester {
    async fn init(&self, context: Arc<Context<MType>>, config: Arc<Config>) -> Result<()> {
        let message_bus = context.message_bus.clone();
        let topic = config.get_string("topic").unwrap_or("test".to_string());
        info!("Creating requester on '{}'", topic);

        context.run(async move {
            // Wait for responder to be ready
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            let test_message = Arc::new(json!({
                "message": "Hello, world!",
            }));

            info!("Sending {:?} on {topic}", test_message);
            match message_bus.request(&topic, test_message.clone()).await {
                Ok(response) => {
                    info!("Got response: {:?}", response);
                }
                Err(e) => {
                    error!("Got error: {e}");
                }
            }

            // Send another on a bad topic to test timeout
            info!("Sending {:?} on bad.topic", test_message);
            match message_bus.request("bad.topic", test_message).await {
                Ok(response) => {
                    info!("Got response: {:?}", response);
                }
                Err(e) => {
                    error!("Got error: {e}");
                }
            }
        });

        Ok(())
    }
}
