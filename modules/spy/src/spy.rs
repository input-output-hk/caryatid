//! Caryatid Spy module

use caryatid_sdk::{Context, Module, module, MessageBounds, MessageBusExt};
use std::sync::Arc;
use anyhow::Result;
use config::Config;
use tracing::{info, error};

/// Spy module
/// Parameterised by the outer message enum used on the bus
#[module(
    message_type(M),
    name = "spy",
    description = "Message spy"
)]
pub struct Spy<M: MessageBounds + std::fmt::Debug>;

impl<M: MessageBounds + std::fmt::Debug> Spy<M>
{
    fn init(&self, context: Arc<Context<M>>, config: Arc<Config>) -> Result<()> {
        match config.get_string("topic") {
            Ok(topic) => {
                info!("Creating message spy on '{}'", topic);
                context.message_bus.subscribe(&topic,
                                              |message: Arc<M>| async move {
                                                  info!("{:?}", message);
                                              })?;
            },

            _ => error!("No topic given for Spy module - no effect")
        }

        Ok(())
    }
}
