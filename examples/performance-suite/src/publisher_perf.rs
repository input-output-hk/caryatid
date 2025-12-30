//! Performance test publisher module

use crate::config::PublisherConfig;
use crate::message::PerfMessage;
use ::config::Config;
use anyhow::Result;
use caryatid_sdk::{module, Context};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{info, warn};

/// Performance test publisher
#[module(
    message_type(PerfMessage),
    name = "publisher_perf",
    description = "Configurable performance test publisher"
)]
pub struct PublisherPerf;

impl PublisherPerf {
    async fn init(&self, context: Arc<Context<PerfMessage>>, config: Arc<Config>) -> Result<()> {
        // Parse configuration
        let publisher_config: PublisherConfig = Config::clone(&config).try_deserialize()?;
        publisher_config.validate()?;

        info!(
            "Creating performance publisher on '{}' with {} messages of {} bytes",
            publisher_config.topic, publisher_config.message_count, publisher_config.message_size
        );
        info!("  Concurrent tasks: {}", publisher_config.concurrent_tasks);
        if let Some(rate) = publisher_config.rate_limit {
            info!("  Rate limit: {} msg/sec", rate);
        } else {
            info!("  Rate limit: unlimited");
        }

        // Generate payload once
        let payload: Vec<u8> = vec![b'X'; publisher_config.message_size];

        // Calculate messages per task
        let messages_per_task =
            (publisher_config.message_count + publisher_config.concurrent_tasks as u64 - 1)
                / publisher_config.concurrent_tasks as u64;

        // Calculate delay between messages if rate limited
        let delay_between_messages = publisher_config.rate_limit.map(|rate| {
            let total_rate = rate / publisher_config.concurrent_tasks as f64;
            Duration::from_secs_f64(1.0 / total_rate)
        });

        let message_bus = context.message_bus.clone();
        let topic = publisher_config.topic.clone();

        // Wait for StartTest signal
        let mut start_sub = context.subscribe("perf.control").await?;

        tokio::spawn(async move {
            // Wait for start signal
            loop {
                if let Ok((_, msg)) = start_sub.read().await {
                    if matches!(msg.as_ref(), PerfMessage::StartTest { .. }) {
                        info!("Publisher received StartTest signal");
                        break;
                    }
                }
            }

            // Start concurrent publishing tasks
            let mut handles = Vec::new();

            for task_id in 0..publisher_config.concurrent_tasks {
                let message_bus = message_bus.clone();
                let topic = topic.clone();
                let payload = payload.clone();
                let delay = delay_between_messages;

                let handle = tokio::spawn(async move {
                    let start_seq = task_id as u64 * messages_per_task;
                    let end_seq = start_seq + messages_per_task;

                    for seq in start_seq..end_seq {
                        let message =
                            PerfMessage::perf_data(seq, payload.clone(), "default".to_string());

                        if let Err(e) = message_bus.publish(&topic, Arc::new(message)).await {
                            warn!("Failed to publish message {}: {}", seq, e);
                        }

                        // Apply rate limiting if configured
                        if let Some(delay_duration) = delay {
                            sleep(delay_duration).await;
                        }
                    }
                });

                handles.push(handle);
            }

            // Wait for all tasks to complete
            for handle in handles {
                handle.await.ok();
            }

            info!("Publisher completed all messages");
        });

        Ok(())
    }
}
