//! Simple Caraytid module - subscriber side
use crate::message::Message;
use anyhow::Result;
use caryatid_sdk::{module, Context, MessageBusExt, Module};
use chrono::Local;
use config::Config;
use std::sync::Arc;
use tracing::{error, info};

/// Typed subscriber module
#[module(
    message_type(Message),
    name = "subscriber",
    description = "Typed subscriber module"
)]
pub struct Subscriber;

impl Subscriber {
    // Implement the single initialisation function, with application
    // Context and this module's Config
    async fn init(&self, context: Arc<Context<Message>>, config: Arc<Config>) -> Result<()> {
        // Get configuration
        let topic = config.get_string("topic").unwrap_or("test".to_string());
        info!("Creating subscriber on '{}'", topic);

        // Register a subscriber on the message bus to listen for messages
        // Message is an enum of all possible messages
        context
            .message_bus
            .subscribe(&topic, |message: Arc<Message>| async move {
                match message.as_ref() {
                    Message::None(_) => error!("Received empty message!"),
                    Message::Test(test) => info!("Received test: {} {}", test.data, test.number),
                    Message::String(s) => info!("Received string {s}"),
                    Message::JSON(json) => info!("Received JSON {:?}", json),
                    _ => error!("Unexpected message type"),
                }
            })?;

        // Register for clock ticks too
        context
            .message_bus
            .subscribe("clock.tick", |message: Arc<Message>| async move {
                match message.as_ref() {
                    Message::Clock(message) => {
                        let localtime = message.time.with_timezone(&Local);
                        info!(
                            "The time sponsored by Caryatid is {}",
                            localtime.format("%H:%M:%S").to_string()
                        )
                    }
                    _ => error!("Unexpected clock message type"),
                }
            })?;

        Ok(())
    }
}
