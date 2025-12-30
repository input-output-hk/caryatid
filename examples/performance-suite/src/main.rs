//! 'main' for Caryatid performance test suite

use ::config::{Config, Environment, File};
use anyhow::Result;
use caryatid_process::Process;
use std::sync::Arc;
use tracing::info;

// Modules
mod config;
mod coordinator;
mod message;
mod metrics;
mod publisher_perf;
mod reporter;
mod subscriber_perf;

use coordinator::Coordinator;
use message::PerfMessage;
use publisher_perf::PublisherPerf;
use subscriber_perf::SubscriberPerf;

/// Standard main
#[tokio::main]
pub async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Caryatid Performance Test Suite");

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let config_file = if args.len() > 1 {
        args[1].clone()
    } else {
        "performance-suite.toml".to_string()
    };

    info!("Loading configuration from: {}", config_file);

    // Read the config
    let config = Arc::new(
        Config::builder()
            .add_source(File::with_name(&config_file).required(false))
            .add_source(Environment::with_prefix("CARYATID"))
            .build()?,
    );

    // Create the process
    let mut process = Process::<PerfMessage>::create(config).await;

    // Register modules
    Coordinator::register(&mut process);
    PublisherPerf::register(&mut process);
    SubscriberPerf::register(&mut process);

    // Run it
    info!("Starting performance test...");
    process.run().await?;

    Ok(())
}
