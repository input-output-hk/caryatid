//! REST server Caraytid module - simple /hello responder
use caryatid_sdk::{Context, MessageBusExt, Module, module};
use std::sync::Arc;
use anyhow::Result;
use config::Config;
use tracing::{info, error};
use crate::message::Message;

/// Typed subscriber module
#[module(
    message_type(Message),
    name = "rest-hello-world",
    description = "REST Hello, world! responder"
)]
pub struct RESTHelloWorld;

impl RESTHelloWorld {

    fn init(&self, context: Arc<Context<Message>>, config: Arc<Config>) -> Result<()> {

        let topic = config.get_string("topic").unwrap_or("test".to_string());
        info!("Creating REST Hello, world! responder on '{}'", topic);

        context.message_bus.subscribe(&topic, |message: Arc<Message>| {
            match message.as_ref()
            {
                Message::RESTRequest(request) =>
                    info!("REST hello world received {} {}", request.method, request.path),
                _ => error!("Unexpected message type")
            }
        })?;

        Ok(())
    }
}

