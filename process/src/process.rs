//! Main process for a Caryatid framework installation
//! Loads and runs modules built with caryatid-sdk

use caryatid_sdk::Context;
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

mod routing_bus;
use routing_bus::RoutingBus;

mod match_topic;

/// Main Process structure
pub struct Process {
    config: Arc<Config>,
    context: Arc<Context>,
}

impl Process {

    /// Create a process with the given config
    pub async fn create(config: Arc<Config>) -> Arc<Self> {

        // Create individual message buses
        let rabbit_mq_bus = Arc::new(
            RabbitMQBus::<serde_json::Value>::new(&get_sub_config(&config,
                                                 "message-bus.rabbit-mq"))
                .await
                .expect("Can't create RabbitMQ bus"));

        let in_memory_bus = Arc::new(InMemoryBus::<serde_json::Value>::new(
            &get_sub_config(&config, "message-bus.in-memory")));

        // Create routing message bus
        let routing_bus = Arc::new(RoutingBus::<serde_json::Value>::new(
            &get_sub_config(&config, "message-router"),
            in_memory_bus.clone(),
            rabbit_mq_bus.clone()));

        // Create the shared context
        let context = Arc::new(Context::new(config.clone(),
                                            routing_bus.clone()));

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

