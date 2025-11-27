//! Simple Caraytid module - responder side
use anyhow::Result;
use caryatid_sdk::{module, Context};
use config::Config;
use serde_json::Value;
use std::sync::Arc;
use tracing::info;

/// Standard message type
type MType = serde_json::Value;

/// Responder module
#[module(
    message_type(MType),
    name = "responder",
    description = "Example responder"
)]
pub struct Responder;

impl Responder {
    async fn handler(message: Arc<Value>) -> Arc<Value> {
        info!("Handler received {:?}", message);

        let mut message = (*message).clone();

        if let Some(obj) = message.as_object_mut() {
            obj.insert(
                "response".to_string(),
                Value::String("Loud and clear".to_string()),
            );
        }

        info!("Responding with {:?}", message);
        Arc::new(message)
    }

    async fn init(&self, context: Arc<Context<MType>>, config: Arc<Config>) -> Result<()> {
        let topic = config.get_string("topic").unwrap_or("test".to_string());
        info!("Creating responder on '{}'", topic);
        context.handle(&topic, Self::handler);

        Ok(())
    }
}
