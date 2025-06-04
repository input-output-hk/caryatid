//! Simple Caraytid module - subscriber side
use caryatid_sdk::{Context, Module, module};
use std::sync::Arc;
use anyhow::Result;
use config::Config;
use tracing::{info, error};
use tokio::task;

/// Standard message type
type MType = serde_json::Value;

/// Sample module
// Define it as a module, with a name and description
#[module(
    message_type(MType),
    name = "subscriber",
    description = "Multi subscriber module"
)]
pub struct Subscriber;

impl Subscriber {

    // Implement the single initialisation function, with application
    // Context and this module's Config
    async fn async_init(context: Arc<Context<MType>>, config: Arc<Config>) -> Result<()> {
        // Get configuration
        let topic1 = config.get_string("topic1").unwrap_or("test1".to_string());
        let topic2 = config.get_string("topic2").unwrap_or("test2".to_string());

        info!("Creating subscription on '{}'", topic1);
        let mut subscription1 = context.message_bus.register(&topic1).await?;

        info!("Creating subscription on '{}'", topic2);
        let mut subscription2 = context.message_bus.register(&topic2).await?;

        context.run(async move {
            loop {
                // Start reads of messages together to avoid delays later on
                let (message1, message2) = (subscription1.read(), subscription2.read());
                let Ok(message1) = message1.await else { return; };
                info!("Message from {}: {:?}", topic1,  message1);
                let Ok(message2) = message2.await else { return; };
                info!("Message from {}: {:?}", topic2,  message2);
            }
        });

        Ok(())
    }

    fn init(&self, context: Arc<Context<MType>>, config: Arc<Config>) -> Result<()> {
        task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                Self::async_init(context, config)
                    .await.unwrap_or_else(|e| error!("Failed: {e}"));
            })
        });

        Ok(())
    }
}

