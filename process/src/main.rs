use caryatid_sdk::message_bus::MessageBus;
use caryatid_sdk::in_memory_bus::InMemoryBus;
use caryatid_sdk::context::Context;
use anyhow::Result;
use std::sync::Arc;
use libloading::{Library, Symbol};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {

    let message_bus = Arc::new(InMemoryBus::new(4));

    // Create the shared context
    let context = Arc::new(Context::new(message_bus.clone()));

    // Dynamically load the module
    unsafe {
        let module_lib = Library::new("./target/debug/libsample_module.so")
            .expect("Failed to load module");
        let module_creator: Symbol<unsafe extern "C" fn(Arc<Context>) -> *mut dyn caryatid_sdk::module::Module> =
            module_lib.get(b"create_module").expect("Failed to load create_module symbol");

        // Create the module
        module_creator(context);
    }

    // Send a test JSON message to the message bus
    let test_message = Arc::new(json!({
        "message": "Hello from main.rs!",
    }));

    // Publish the message to the "sample_topic" where the module is listening
    message_bus.publish("sample_topic", test_message)
        .await.expect("Failed to publish message");

    // Wait for completion
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    Ok(())
}

