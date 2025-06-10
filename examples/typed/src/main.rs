//! Sample 'main' for a Caryatid process, typed message version
//! Loads and runs modules built with caryatid-sdk

use crate::message::Message;
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

mod message;

// External modules
extern crate caryatid_module_clock;
use caryatid_module_clock::Clock;

extern crate caryatid_module_spy;
use caryatid_module_spy::Spy;

/// Standard main
#[tokio::main]
pub async fn main() -> Result<()> {
    // Initialise tracing
    tracing_subscriber::fmt::init();

    info!("Caryatid modular framework - typed message example process");

    // Read the config
    let config = Arc::new(
        Config::builder()
            .add_source(File::with_name("typed"))
            .add_source(Environment::with_prefix("CARYATID"))
            .build()
            .unwrap(),
    );

    // Create the process
    let mut process = Process::<Message>::create(config).await;

    // Register modules
    Subscriber::register(&mut process);
    Publisher::register(&mut process);
    Clock::<Message>::register(&mut process);
    Spy::<Message>::register(&mut process);

    // Run it
    process.run().await?;

    // Bye!
    info!("Exiting");
    Ok(())
}
