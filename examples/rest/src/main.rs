//! Sample 'main' for a Caryatid process, REST Hello, world! version

use anyhow::Result;
use caryatid_process::Process;
use config::{Config, Environment, File};
use std::sync::Arc;
use tracing::info;
use tracing_subscriber;

// Modules in the same crate
mod rest_hello_world;
use rest_hello_world::RESTHelloWorld;

mod message;
use message::Message;

// External modules
extern crate caryatid_module_rest_server;
use caryatid_module_rest_server::RESTServer;

/// Standard main
#[tokio::main]
pub async fn main() -> Result<()> {
    // Initialise tracing
    tracing_subscriber::fmt::init();

    info!("Caryatid modular framework - REST Hello, world! process");

    // Read the config
    let config = Arc::new(
        Config::builder()
            .add_source(File::with_name("rest"))
            .add_source(Environment::with_prefix("CARYATID"))
            .build()
            .unwrap(),
    );

    // Create the process
    let mut process = Process::<Message>::create(config).await;

    // Register modules
    RESTServer::<Message>::register(&mut process);
    RESTHelloWorld::register(&mut process);

    // Run it
    process.run().await?;

    // Bye!
    info!("Exiting");
    Ok(())
}
