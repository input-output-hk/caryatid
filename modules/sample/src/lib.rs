//! Sample Caraytid module
use caryatid_sdk::*;
use std::sync::Arc;
use anyhow::Result;
use config::Config;
use tracing::{info};

/// Sample module
// Define it as a module, with a name and description
#[module(
    name = "sample",
    description = "Simple sample module"
)]
pub struct SampleModule;

impl SampleModule {

    // Implement the single initialisation function, with application
    // Context and this module's Config
    fn init(&self, context: &Context, config: &Config) -> Result<()> {

        // We can log as usual
        info!("Initialising sample module");

        // Check our configuration
        info!("Configuration 'foo' = {}",
              config.get_string("foo").unwrap_or("NOT FOUND".to_string()));

        // Register an observer on the message bus to listen for messages
        // on "sample.test". Messages are passed as JSON objects, in an Arc
        context.message_bus.register("sample.test",
                                     |message: Arc<serde_json::Value>| {
           info!("SampleModule received: {:?}", message);
        })?;

        Ok(())
    }
}

