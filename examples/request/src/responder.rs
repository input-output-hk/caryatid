//! Simple Caraytid module - responder side
use caryatid_sdk::{Context, MessageBusExt, Module, module};
use std::sync::Arc;
use anyhow::Result;
use config::Config;
use tracing::{info};

/// Responder module
#[module(
    name = "responder",
    description = "Example responder"
)]
pub struct Responder;

impl Responder {

    async fn handler(message: Arc<serde_json::Value>) -> Arc<serde_json::Value> {
        info!("Handler received {:?}", message);
        message
    }

    fn init(&self, context: Arc<Context>, config: Arc<Config>) -> Result<()> {

        let topic = config.get_string("topic").unwrap_or("test".to_string());
        info!("Creating responder on '{}'", topic);
        context.message_bus.handle(&topic, Self::handler);

        Ok(())
    }
}

