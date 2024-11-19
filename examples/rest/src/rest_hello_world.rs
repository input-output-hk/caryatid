//! REST server Caraytid module - simple /hello responder
use caryatid_sdk::{Context, MessageBusExt, Module, module};
use caryatid_sdk::messages::RESTResponse;
use std::sync::Arc;
use anyhow::Result;
use config::Config;
use tracing::{info, error};
use crate::message::Message;
use futures::future;

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

        context.message_bus.handle(&topic, |message: Arc<Message>| {
            let response = match message.as_ref() {
                Message::RESTRequest(request) => {
                    info!("REST hello world received {} {}", request.method, request.path);
                    RESTResponse {
                        code: 200,
                        body: "Hello, world!".to_string()
                    }
                },
                _ => {
                    error!("Unexpected message type {:?}", message);
                    RESTResponse {
                        code: 500,
                        body: "Unexpected message in response".to_string() }
                }
            };

            future::ready(Arc::new(Message::RESTResponse(response)))
        })?;

        Ok(())
    }
}

