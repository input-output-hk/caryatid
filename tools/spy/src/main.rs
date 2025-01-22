//! Caryatid spy tool

use caryatid_process::Process;
use anyhow::Result;
use config::{Config, File, Environment};
use tracing::info;
use tracing_subscriber;
use std::sync::Arc;

// Modules in the same crate
mod subscriber;
use subscriber::Subscriber;

/// Universal message type
type MType = serde_json::Value;

/// Standard main
#[tokio::main]
pub async fn main() -> Result<()> {

    // Initialise tracing
    tracing_subscriber::fmt::init();

    info!("Caryatid message spy");

    // Read the config
    let config = Arc::new(Config::builder()
        .add_source(File::with_name("spy"))
        .add_source(Environment::with_prefix("CARYATID"))
        .build()
        .unwrap());

    // Create the process
    let mut process = Process::<MType>::create(config).await;

    // Register modules
    Subscriber::register(&mut process);

    // Run it
    process.run().await?;

    // Bye!
    info!("Exiting");
    Ok(())
}

