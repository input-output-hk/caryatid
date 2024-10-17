// Main process for a Caryatid framework installation
// Loads and runs modules built with caryatid-sdk

use caryatid_sdk::*;
use anyhow::Result;
use std::sync::Arc;
use serde_json::json;
use config::{Config, File, Environment};
use tokio::signal::unix::{signal, SignalKind};
use tracing::{info, warn};
use tracing_subscriber;

mod loaded_module;
use loaded_module::LoadedModule;

mod in_memory_bus;
use in_memory_bus::InMemoryBus;

mod rabbit_mq_bus;
use rabbit_mq_bus::RabbitMQBus;

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

/// Main process
#[tokio::main]
async fn main() -> Result<()> {

    // Initialise tracing
    tracing_subscriber::fmt::init();

    // Read the config
    let config = Config::builder()
        .add_source(File::with_name("process/caryatid"))
        .add_source(Environment::with_prefix("CARYATID"))
        .build()
        .unwrap();

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
    let context = Context::new(Arc::new(config),
                               message_bus.clone());

    // Scan for modules
    let mut modules: Vec<LoadedModule> = Vec::new();
    let modules_section = context.config.get_table("modules")?;
    for (key, _value) in modules_section.into_iter() {
        info!("Found module '{}'", key);

        // Get the module's config
        let module_config = get_config(&context.config,
                                       format!("modules.{}", key).as_str());

        // Use their specified name, or default it
        let lib_name = module_config.get_string("lib")
            .unwrap_or(format!("lib{}_module.so", key));

        // Load the module
        match LoadedModule::load(lib_name, &context, &module_config) {
            Ok(module) => {
                info!("Created module {}: {}",
                      module.module.get_name(),
                      module.module.get_description());
                modules.push(module)
            },
            Err(e) => {
                warn!("Can't load module {}: {}", key, e);
            }
        }
    }

    // Send a test JSON message to the message bus on 'sample_topic'
    let test_message = Arc::new(json!({
        "message": "Hello from the Caryatid process!",
    }));

    message_bus.publish("sample.test", test_message)
        .await.expect("Failed to publish message");

    // Wait for SIGTERM
    let mut sigterm = signal(SignalKind::terminate())?;
    sigterm.recv().await;

    info!("SIGTERM received. Shutting down...");

    // Shutdown the message bus and all subscriptions (before losing modules)
    message_bus.shutdown();

    // Clear the modules to drop all the loaded libraries
    modules.clear();

    // Bye!
    info!("Exiting");
    Ok(())
}

