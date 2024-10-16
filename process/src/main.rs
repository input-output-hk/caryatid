// Main process for a Caryatid framework installation
// Loads and runs modules built with caryatid-sdk

use caryatid_sdk::*;
use anyhow::Result;
use std::sync::Arc;
use libloading::{Library, Symbol};
use serde_json::json;
use config::{Config, File, Environment};

/// Helper function to extract a sub-config as a new Config object
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

    // Read the config
    let config = Config::builder()
        .add_source(File::with_name("process/caryatid"))
        .add_source(Environment::with_prefix("CARYATID"))
        .build()
        .unwrap();

    // Create an in-memory message bus
    let message_bus = Arc::new(InMemoryBus::new(
        &get_config(&config, "message_bus.in_memory")));

    // Create the shared context
    let context = Context::new(Arc::new(config),
                               message_bus.clone());

    // Dynamically load the module
    unsafe {
        let module_lib = Library::new("./target/debug/libsample_module.so")
            .expect("Failed to load module");
        let module_creator: Symbol<unsafe extern "C" fn(&Context)
                                                        -> *mut dyn Module> =
            module_lib.get(b"create_module")
            .expect("Failed to load create_module symbol");

        // Create the module
        let module = module_creator(&context).as_ref()
            .expect("Failed to create module");

        println!("Created module {}", module.get_name());
    }

    // Send a test JSON message to the message bus on 'sample_topic'
    let test_message = Arc::new(json!({
        "message": "Hello from the Caryatid process!",
    }));

    message_bus.publish("sample_topic", test_message)
        .await.expect("Failed to publish message");

    // Wait for completion
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    Ok(())
}

