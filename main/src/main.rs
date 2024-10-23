//! Standard 'main' for a Caryatid process
//! Loads and runs modules built with caryatid-sdk

use caryatid_process::Process;
use anyhow::Result;
use config::{Config, File, Environment};
use tracing::info;
use tracing_subscriber;
use std::sync::Arc;

extern crate sample_publisher;
extern crate sample_subscriber;

/// Standard main
#[tokio::main]
pub async fn main() -> Result<()> {

    // Initialise tracing
    tracing_subscriber::fmt::init();

    info!("Caryatid modular framework");

    // Read the config
    let config = Arc::new(Config::builder()
        .add_source(File::with_name("main/caryatid"))
        .add_source(Environment::with_prefix("CARYATID"))
        .build()
        .unwrap());

    // Create the process
    let process = Process::create(config).await;

    // Register modules
    sample_publisher::register();
    sample_subscriber::register();

    // Run it
    process.run().await?;

    // Bye!
    info!("Exiting");
    Ok(())
}

