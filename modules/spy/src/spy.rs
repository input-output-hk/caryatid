//! Caryatid Spy module

use anyhow::Result;
use caryatid_sdk::{module, Context, MessageBounds, Module};
use config::Config;
use std::sync::Arc;
use tracing::{error, info};

/// Spy module
/// Parameterised by the outer message enum used on the bus
#[module(message_type(M), name = "spy", description = "Message spy")]
pub struct Spy<M: MessageBounds + std::fmt::Debug>;

impl<M: MessageBounds + std::fmt::Debug> Spy<M> {
    async fn init(&self, context: Arc<Context<M>>, config: Arc<Config>) -> Result<()> {
        match config.get_string("topic") {
            Ok(topic) => {
                info!("Creating message spy on '{}'", topic);
                let mut subscription = context.subscribe(&topic).await?;
                context.run(async move {
                    loop {
                        let Ok((_, message)) = subscription.read().await else {
                            return;
                        };
                        info!("{:?}", message);
                    }
                });
            }

            _ => error!("No topic given for Spy module - no effect"),
        }

        Ok(())
    }
}
