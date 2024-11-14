//! Sample 'main' for a Caryatid process - request/response example
//! Loads and runs modules built with caryatid-sdk

use caryatid_process::Process;
use anyhow::Result;
use config::{Config, File, Environment};
use tracing::info;
use tracing_subscriber;
use std::sync::Arc;

// Modules in the same crate
mod requester;
use requester::Requester;
mod responder;
use responder::Responder;

/// Standard main
#[tokio::main]
pub async fn main() -> Result<()> {

    // Initialise tracing
    tracing_subscriber::fmt::init();

    info!("Caryatid modular framework - request/response example process");

    // Read the config
    let config = Arc::new(Config::builder()
        .add_source(File::with_name("request"))
        .add_source(Environment::with_prefix("CARYATID"))
        .build()
        .unwrap());

    // Create the process
    let mut process = Process::create(config).await;

    // Register modules
    Requester::register(&mut process);
    Responder::register(&mut process);

    // Run it
    process.run().await?;

    // Bye!
    info!("Exiting");
    Ok(())
}

