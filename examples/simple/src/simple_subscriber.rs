//! Simple Caraytid module - subscriber side
use caryatid_sdk::{Context, MessageBusExt, Module, module};
use std::sync::Arc;
use anyhow::Result;
use config::Config;
use tracing::{info};
use chrono::{DateTime, Utc, Local};

/// Simple module
// Define it as a module, with a name and description
#[module(
    name = "simple-subscriber",
    description = "Simple subscriber module"
)]
pub struct SimpleSubscriber;

impl SimpleSubscriber {

    // Implement the single initialisation function, with application
    // Context and this module's Config
    fn init(&self, context: Arc<Context>, config: Arc<Config>) -> Result<()> {

        // Get configuration
        let topic = config.get_string("topic").unwrap_or("test".to_string());
        info!("Creating simple subscriber on '{}'", topic);

        // Register a subscriber on the message bus to listen for messages
        // Messages are passed as JSON objects, in an Arc
        context.message_bus.subscribe(&topic,
                                      |message: Arc<serde_json::Value>| {
           info!("Received: {:?}", message);
        })?;

        // Register for clock ticks too
        context.message_bus.subscribe("clock.tick",
                                      |message: Arc<serde_json::Value>| {
           match message["time"].as_str() {
               Some(iso_time) => {
                   match iso_time.parse::<DateTime<Utc>>() {
                       Ok(datetime) => {
                           let localtime = datetime.with_timezone(&Local);
                           info!("The time sponsored by Caryatid is {}",
                                 localtime.format("%H:%M:%S").to_string())
                       }
                       _ => {}
                   }
               }
               _ => {}
           }
        })?;

        Ok(())
    }
}

