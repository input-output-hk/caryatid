//! Sample Caraytid module - requester side
use caryatid_sdk::{Context, Module, module};
use std::sync::Arc;
use anyhow::Result;
use config::Config;
use tracing::{info, error};
use serde_json::json;

/// Simple requester module
#[module(
    name = "requester",
    description = "Sample requester"
)]
pub struct Requester;

impl Requester {

    fn init(&self, context: Arc<Context>, config: Arc<Config>) -> Result<()> {
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
                Ok(result) => { info!("Got result: {:?}", result); },
                Err(e) => { error!("Request failed: {e}"); }
            }
        });

        Ok(())
    }
}

