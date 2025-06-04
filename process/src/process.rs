//! Main process for a Caryatid framework installation
//! Loads and runs modules built with caryatid-sdk

use caryatid_sdk::{Context, MessageBus, Module, MessageBounds, ModuleRegistry};
use caryatid_sdk::config::{get_sub_config, config_from_value};
use caryatid_sdk::correlation_bus::CorrelationBus;
use anyhow::{Result, anyhow};
use std::sync::Arc;
use std::collections::HashMap;
use config::Config;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::watch::Sender;
use tracing::{info, warn, error};

mod in_memory_bus;
use in_memory_bus::InMemoryBus;

mod rabbit_mq_bus;
use rabbit_mq_bus::RabbitMQBus;

mod routing_bus;
use routing_bus::{RoutingBus, BusInfo};

/// Main Process structure
pub struct Process<M: MessageBounds> {
    /// Global configuration
    config: Arc<Config>,

    /// Global context
    context: Arc<Context<M>>,

    /// Active modules by name
    modules: HashMap<String, Arc<dyn Module<M>>>,
}

impl<M: MessageBounds> Process<M> {

    /// Create a bus of the given type
    async fn create_bus(id: String, class: String, config: &Config) -> Result<BusInfo<M>> {

        let bus: Arc<dyn MessageBus<M>> = match class.as_str() {

            // In-memory
            "in-memory" => Arc::new(InMemoryBus::<M>::new(&config)),

            // RabbitMQ
            "rabbit-mq" => match RabbitMQBus::<M>::new(&config).await {
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
    pub async fn create(config: Arc<Config>) -> Self {

        // Create bus registrations
        let mut buses: Vec<Arc<BusInfo<M>>> = Vec::new();

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
        let routing_bus = Arc::new(RoutingBus::<M>::new(
            &get_sub_config(&config, "message-router"),
            Arc::new(buses)));

        // Create correlation wrapper
        let correlation_bus = Arc::new(CorrelationBus::<M>::new(
            &get_sub_config(&config, "message-correlator"),
            routing_bus.clone()));

        // Create the shared context
        let context = Arc::new(Context::new(config.clone(), correlation_bus.clone(), Sender::new(false)));

        Self { config, context, modules: HashMap::new() }
    }

    /// Run the process
    pub async fn run(&self) -> Result<()> {

        info!("Initialising...");

        // Initialise all the modules from [module.<id>] configuration
        if let Ok(mod_confs) = self.config.get_table("module") {
            for (id, mod_conf) in mod_confs {
                if let Ok(modt) = mod_conf.into_table() {
                    let modc = config_from_value(modt);
                    let mut module_name = id.clone();  // Default
                    if let Ok(class) = modc.get_string("class") {
                        module_name = class;
                    }

                    // Look up the module
                    if let Some(module) = self.modules.get(&module_name) {
                        info!("Initialising module {id}");
                        module.init(self.context.clone(), Arc::new(modc)).await.unwrap();
                    }
                    else {
                        error!("Unrecognised module class: {module_name} in [module.{id}]");
                    }
                } else {
                    warn!("Bad configuration for module {id} ignored");
                }
            }
        }

        info!("Running...");

        // Send startup message if required
        let _ = self.context.startup_watch.send(true);
        if let Ok(topic) = self.config.get_string("startup.topic") {
            self.context.message_bus.publish(&topic, Arc::new(M::default()))
                .await
                .unwrap_or_else(|e| error!("Failed to publish: {e}"));
        }

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

/// Module registry implementation
impl<M: MessageBounds> ModuleRegistry<M> for Process<M> {

    /// Register a module
    fn register(&mut self, module: Arc<dyn Module<M>>) {
        let name = module.get_name();
        self.modules.insert(name.to_string(), module.clone());
    }
}
