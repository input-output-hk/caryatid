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
mod subscriber;
use subscriber::Subscriber;

mod publisher;
use publisher::Publisher;

mod message;

// External modules
extern crate clock;
use clock::Clock;

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
    let mut process = Process::<Message>::create(config).await;

    // Register modules
    Subscriber::register(&mut process);
    Publisher::register(&mut process);
    Clock::<Message>::register(&mut process);

    // Run it
    process.run().await?;

    // Bye!
    info!("Exiting");
    Ok(())
}

