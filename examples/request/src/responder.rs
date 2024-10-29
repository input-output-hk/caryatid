//! Simple Caraytid module - responder side
use caryatid_sdk::{Context, MessageBusExt, Module, module};
use std::sync::Arc;
use anyhow::Result;
use config::Config;
use tracing::{info};
use serde_json::Value;

/// Responder module
#[module(
    name = "responder",
    description = "Example responder"
)]
pub struct Responder;

impl Responder {

    async fn handler(message: Arc<Value>) -> Arc<Result<Value>> {
        info!("Handler received {:?}", message);

        let mut message = (*message).clone();

        if let Some(obj) = message.as_object_mut() {
            obj.insert("response".to_string(),
                       Value::String("Loud and clear".to_string()));
        }

        info!("Responding with {:?}", message);
        Arc::new(Ok(message))
    }

    fn init(&self, context: Arc<Context>, config: Arc<Config>) -> Result<()> {

        let topic = config.get_string("topic").unwrap_or("test".to_string());
        info!("Creating responder on '{}'", topic);
        context.message_bus.handle(&topic, Self::handler)?;

        Ok(())
    }
}

