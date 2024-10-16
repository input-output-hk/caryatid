// Sample Caraytid module
use caryatid_sdk::*;
use std::sync::Arc;
use anyhow::Result;
use config::Config;

#[module(
    name = "sample",
    description = "Simple sample module"
)]
pub struct SampleModule;

impl SampleModule {
    fn init(&self, context: &Context, config: &Config) -> Result<()> {
        println!("Initialising sample module");
        println!("Configuration 'foo' = {}",
                 config.get_string("foo").unwrap_or("NOT FOUND".to_string()));

        // Register an observer on the message bus to listen for messages
        // on "sample_topic"
        context.message_bus.register("sample_topic",
                                     |message: Arc<serde_json::Value>| {
            println!("SampleModule received: {:?}", message);
        })?;

        Ok(())
    }
}

