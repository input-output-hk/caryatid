// Sample Caraytid module
use caryatid_sdk::*;
use std::sync::Arc;
use anyhow::Result;

#[module]
pub struct SampleModule;

impl Module for SampleModule {
    fn init(&self, context: &Context) -> Result<()> {
        println!("SampleModule initialized!");

        // Register an observer on the message bus to listen for messages
        // on "sample_topic"
        context.message_bus.register("sample_topic",
                                     |message: Arc<serde_json::Value>| {
            println!("SampleModule received: {:?}", message);
        })?;

        Ok(())
    }
}

