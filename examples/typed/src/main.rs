//! Sample 'main' for a Caryatid process, typed message version
//! Loads and runs modules built with caryatid-sdk

use caryatid_process::Process;
use anyhow::Result;
use config::{Config, File, Environment};
use tracing::info;
use tracing_subscriber;
use std::sync::Arc;
use crate::message::Message;

// Modules in the same crate
mod typed_subscriber;
mod typed_publisher;
mod message;

/// Standard main
#[tokio::main]
pub async fn main() -> Result<()> {

    // Initialise tracing
    tracing_subscriber::fmt::init();

    info!("Caryatid modular framework - typed message example process");

    // Read the config
    let config = Arc::new(Config::builder()
        .add_source(File::with_name("typed"))
        .add_source(Environment::with_prefix("CARYATID"))
        .build()
        .unwrap());

    // Create the process
    let process = Process::<Message>::create(config).await;

    // Register modules
    typed_subscriber::register(&process);
    typed_publisher::register(&process);

    // Run it
    process.run().await?;

    // Bye!
    info!("Exiting");
    Ok(())
}

