//! Caryatid performance test - publisher side
use caryatid_sdk::{Context, Module, module};
use std::sync::Arc;
use anyhow::Result;
use config::Config;
use tracing::{info};
use serde_json::json;
use tokio::time::{sleep, Duration};

/// Performance publisher module
#[module(
    name = "perf-publisher",
    description = "Performance test publisher module"
)]
pub struct PerfPublisher;

const DEFAULT_COUNT: i64 = 1000_000;

impl PerfPublisher {

    fn init(&self, context: Arc<Context>, config: Arc<Config>) -> Result<()> {
        let message_bus = context.message_bus.clone();

        // Get configuration
        let topic = config.get_string("topic").unwrap_or("test".to_string());
        info!("Creating performance publisher on '{}'", topic);

        let count = config.get_int("count").unwrap_or(DEFAULT_COUNT);

        // Send a test JSON message to the message bus on 'sample_topic'
        tokio::spawn(async move {

            info!("Starting publishing {count} messages");
            for i in 1..=count {

                let message = Arc::new(json!({
                    "index": i
                }));

                message_bus.publish(&topic, message)
                    .await.expect("Failed to publish message");
            }

            // Let the subscriber catch up
            sleep(Duration::from_secs(1)).await;

            // Send a stop
            let message = Arc::new(json!({
                "stop": true
            }));

            message_bus.publish(&topic, message)
                .await.expect("Failed to publish message");
        });

        Ok(())
    }
}

