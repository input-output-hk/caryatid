//! Main process for a Caryatid framework installation
//! Loads and runs modules built with caryatid-sdk

use caryatid_sdk::{Context, MessageBus};
use caryatid_sdk::config::get_sub_config;
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

impl Process {

    /// Create a process with the given config
    pub async fn create(config: Arc<Config>) -> Arc<Self> {

        // Create a message bus according to config
        let message_bus: Arc<dyn MessageBus<serde_json::Value>>;
        if let Ok(_table) = config.get_table("message-bus.rabbit-mq") {
            message_bus = Arc::new(
                RabbitMQBus::new(&get_sub_config(&config,
                                                 "message-bus.rabbit-mq"))
                    .await
                    .expect("Can't create RabbitMQ bus")
            );
        }
        else {
            message_bus = Arc::new(InMemoryBus::new(
                &get_sub_config(&config, "message-bus.in-memory")));
        }

        // Create the shared context
        let context = Arc::new(Context::new(config.clone(),
                                            message_bus.clone()));

        Arc::new(Self { config, context })
    }

    /// Run the process
    pub async fn run(&self) -> Result<()> {

        info!("Running");

        // Create all the modules
        caryatid_sdk::module_registry::initialise_modules(self.context.clone(),
                                                          self.config.clone());

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

