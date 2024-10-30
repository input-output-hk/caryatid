//! 'main' for Caryatid performance test

use caryatid_process::Process;
use anyhow::Result;
use config::{Config, File, Environment};
use tracing::info;
use tracing_subscriber;
use std::sync::Arc;

// Modules in the same crate
mod perf_subscriber;
mod perf_publisher;

/// Standard main
#[tokio::main]
pub async fn main() -> Result<()> {

    // Initialise tracing
    tracing_subscriber::fmt::init();

    info!("Caryatid modular framework - performance test");

    // Read the config
    let config = Arc::new(Config::builder()
        .add_source(File::with_name("performance"))
        .add_source(Environment::with_prefix("CARYATID"))
        .build()
        .unwrap());

    // Create the process
    let process = Process::create(config).await;

    // Register modules
    perf_subscriber::register();
    perf_publisher::register();

    // Run it
    process.run().await?;

    // Bye!
    info!("Exiting");
    Ok(())
}
