//! Sample 'main' for a Caryatid process
//! Loads and runs modules built with caryatid-sdk

use anyhow::Result;
use caryatid_process::Process;
use config::{Config, Environment, File};
use std::sync::Arc;
use tracing::info;
use tracing_subscriber;

// Modules in the same crate
mod subscriber;
use subscriber::Subscriber;

mod publisher;
use publisher::Publisher;

/// Standard message type
type MType = serde_json::Value;

/// Standard main
#[tokio::main]
pub async fn main() -> Result<()> {
    // Initialise tracing
    tracing_subscriber::fmt::init();

    info!("Caryatid modular framework - simple pub-sub process");

    // Read the config
    let config = Arc::new(
        Config::builder()
            .add_source(File::with_name("simple"))
            .add_source(Environment::with_prefix("CARYATID"))
            .build()
            .unwrap(),
    );

    // Create the process
    let mut process = Process::<MType>::create(config).await;

    // Register modules
    Subscriber::register(&mut process);
    Publisher::register(&mut process);

    // Run it
    process.run().await?;

    // Bye!
    info!("Exiting");
    Ok(())
}
