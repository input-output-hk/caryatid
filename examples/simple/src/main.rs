//! Sample 'main' for a Caryatid process
//! Loads and runs modules built with caryatid-sdk

use caryatid_process::Process;
use anyhow::Result;
use config::{Config, File, Environment};
use tracing::info;
use tracing_subscriber;
use std::sync::Arc;

// Modules in the same crate
mod simple_subscriber;
mod simple_publisher;

// External modules
extern crate clock;

/// Standard main
#[tokio::main]
pub async fn main() -> Result<()> {

    // Initialise tracing
    tracing_subscriber::fmt::init();

    info!("Caryatid modular framework - simple example process");

    // Read the config
    let config = Arc::new(Config::builder()
        .add_source(File::with_name("simple"))
        .add_source(Environment::with_prefix("CARYATID"))
        .build()
        .unwrap());

    // Create the process
    let process = Process::create(config).await;

    // Register modules
    simple_subscriber::register(&process);
    simple_publisher::register(&process);
    clock::register(&process);

    // Run it
    process.run().await?;

    // Bye!
    info!("Exiting");
    Ok(())
}

