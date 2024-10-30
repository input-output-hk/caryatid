//! Main process for a Caryatid framework installation
//! Loads and runs modules built with caryatid-sdk

use caryatid_sdk::{Context, MessageBus};
use caryatid_sdk::config::{get_sub_config, config_from_value};
use anyhow::{Result, anyhow};
use std::sync::Arc;
use config::Config;
use tokio::signal::unix::{signal, SignalKind};
use tracing::{info, error};

mod in_memory_bus;
use in_memory_bus::InMemoryBus;

mod rabbit_mq_bus;
use rabbit_mq_bus::RabbitMQBus;

mod routing_bus;
use routing_bus::{RoutingBus, BusInfo};

mod match_topic;

/// Standard message type
type MType = serde_json::Value;

/// Main Process structure
pub struct Process {
    config: Arc<Config>,
    context: Arc<Context>,
}

impl Process {

    /// Create a bus of the given type
    async fn create_bus(id: String, class: String,
                        config: &Config) -> Result<BusInfo<MType>> {

        let bus: Arc<dyn MessageBus<MType>> = match class.as_str() {

            // In-memory
            "in-memory" => Arc::new(InMemoryBus::<MType>::new(&config)),

            // RabbitMQ
            "rabbit-mq" => match RabbitMQBus::<MType>::new(&config).await {
                Ok(mqb) => Arc::new(mqb),
                Err(e) => {
                    error!("Failed to create RabbitMQ bus: {e}");
                    return Err(e);
                }
            },

            // Unknown
            _ => {
                return Err(anyhow!("Unknown message bus class {class}"));
            }
        };

        return Ok(BusInfo { id, bus });
    }

    /// Create a process with the given config
    pub async fn create(config: Arc<Config>) -> Arc<Self> {

        // Create bus registrations
        let mut buses: Vec<Arc<BusInfo<MType>>> = Vec::new();

        // Get all [[message-bus]]
        if let Ok(mb_confs) = config.get_table("message-bus") {
            for (id, mb_conf) in mb_confs {
                if let Ok(mbt) = mb_conf.into_table() {
                    let mbc = config_from_value(mbt);
                    if let Ok(class) = mbc.get_string("class") {
                        info!("Creating message bus '{id}' ({class})");

                        match Self::create_bus(id, class, &mbc).await {
                            Ok(bus) => {
                                buses.push(Arc::new(bus));
                            },

                            _ => {}
                        }
                    }
                }
            }
        }

        // Create routing message bus
        let routing_bus = Arc::new(RoutingBus::<MType>::new(
            &get_sub_config(&config, "message-router"),
            Arc::new(buses)));

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
