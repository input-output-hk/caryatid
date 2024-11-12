//! Simple Caraytid module - subscriber side
use caryatid_sdk::{Context, MessageBusExt, Module, module};
use std::sync::Arc;
use anyhow::Result;
use config::Config;
use tracing::{info, error};
use chrono::Local;
use crate::message::Message;

/// Typed subscriber module
#[module(
    message_type(Message),
    name = "typed-subscriber",
    description = "Typed subscriber module"
)]
pub struct TypedSubscriber;

impl TypedSubscriber {

    // Implement the single initialisation function, with application
    // Context and this module's Config
    fn init(&self, context: Arc<Context<Message>>, config: Arc<Config>) -> Result<()> {

        // Get configuration
        let topic = config.get_string("topic").unwrap_or("test".to_string());
        info!("Creating typed subscriber on '{}'", topic);

        // Register a subscriber on the message bus to listen for messages
        // Message is an enum of all possible messages
        context.message_bus.subscribe(&topic, |message: Arc<Message>| {
            match message.as_ref()
            {
                Message::None(_) => error!("Received empty message!"),
                Message::Test(test) => info!("Received test: {} {}", test.data, test.number),
                Message::String(s) => info!("Received string {s}"),
                Message::JSON(json) => info!("Received JSON {:?}", json),
                _ => error!("Unexpected message type")
            }
        })?;

        // Register for clock ticks too
        context.message_bus.subscribe("clock.tick", |message: Arc<Message>| {
            match message.as_ref() {
                Message::Clock(message) => {
                    let localtime = message.time.with_timezone(&Local);
                    info!("The time sponsored by Caryatid is {}",
                          localtime.format("%H:%M:%S").to_string())
                },
                _ => error!("Unexpected clock message type")
            }
        })?;

        Ok(())
    }
}

