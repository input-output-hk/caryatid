//! Main process for a Caryatid framework installation
//! Loads and runs modules built with caryatid-sdk

use caryatid_sdk::*;
use anyhow::Result;
use std::sync::Arc;
use config::Config;
use tokio::signal::unix::{signal, SignalKind};
use tracing::info;

mod in_memory_bus;
use in_memory_bus::InMemoryBus;

mod rabbit_mq_bus;
use rabbit_mq_bus::RabbitMQBus;

/// Main Process structure
pub struct Process {
    config: Arc<Config>,
    context: Arc<Context>,
}

/// Extract a sub-config as a new Config object
/// Defaults to an empty Config if the path does not exist.
fn get_config(config: &Config, path: &str) -> Config {
    // Try to extract the sub-config as a table
    match config.get_table(path) {
        Ok(sub_table) => {
            // Use ConfigBuilder to create a new Config with the sub-table
            let mut builder = Config::builder();
            for (key, value) in sub_table.into_iter() {
                builder = builder.set_override(key, value).unwrap();
            }
            builder.build().unwrap_or_default()
        },
        Err(_) => {
            // Return an empty Config if the path doesn't exist
            Config::default()
        }
    }
}

impl Process {

    /// Create a process with the given config
    pub async fn create(config: Arc<Config>) -> Arc<Self> {

        // Create a message bus according to config
        let message_bus: Arc<dyn MessageBus<serde_json::Value>>;
        if let Ok(_table) = config.get_table("message-bus.rabbit-mq") {
            message_bus = Arc::new(
                RabbitMQBus::new(&get_config(&config, "message-bus.rabbit-mq"))
                    .await
                    .expect("Can't create RabbitMQ bus")
            );
        }
        else {
            message_bus = Arc::new(InMemoryBus::new(
                &get_config(&config, "message-bus.in-memory")));
        }

        // Create the shared context
        let context = Arc::new(Context::new(config.clone(),
                                            message_bus.clone()));

        Arc::new(Self { config, context })
    }

    /// Run the process
    pub async fn run(&self) -> Result<()> {

        info!("Running");

        // Wait for SIGTERM
        let mut sigterm = signal(SignalKind::terminate())
            .expect("Can't set signal");
        sigterm.recv().await;

        info!("SIGTERM received. Shutting down...");

        // Shutdown the message bus and all subscriptions (before losing modules)
        let _ = self.context.message_bus.shutdown().await;

        Ok(())
    }
}

