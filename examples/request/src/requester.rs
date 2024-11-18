//! Sample Caraytid module - requester side
use caryatid_sdk::{Context, Module, module};
use std::sync::Arc;
use anyhow::Result;
use config::Config;
use tracing::{info, error};
use serde_json::json;

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

    fn init(&self, context: Arc<Context<MType>>, config: Arc<Config>) -> Result<()> {
        let message_bus = context.message_bus.clone();
        let topic = config.get_string("topic").unwrap_or("test".to_string());
        info!("Creating requester on '{}'", topic);

        tokio::spawn(async move {

            // Wait for responder to be ready
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            let test_message = Arc::new(json!({
                "message": "Hello, world!",
            }));

            info!("Sending {:?}", test_message);

            match message_bus.request(&topic, test_message).await {
                Ok(response) => { info!("Got response: {:?}", response); },
                Err(e) => { error!("Got error: {e}"); }
            }
        });

        Ok(())
    }
}

