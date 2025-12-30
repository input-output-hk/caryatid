//! Performance test subscriber module

use crate::config::SubscriberConfig;
use crate::message::PerfMessage;
use crate::metrics::MetricsCollector;
use ::config::Config;
use anyhow::Result;
use caryatid_sdk::{module, Context};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use tokio::time::{interval, sleep, Duration};
use tracing::{info, warn};

/// Performance test subscriber
#[module(
    message_type(PerfMessage),
    name = "subscriber_perf",
    description = "Performance measuring subscriber"
)]
pub struct SubscriberPerf;

impl SubscriberPerf {
    async fn init(&self, context: Arc<Context<PerfMessage>>, config: Arc<Config>) -> Result<()> {
        // Parse configuration
        let subscriber_config: SubscriberConfig = Config::clone(&config).try_deserialize()?;
        subscriber_config.validate()?;

        let subscriber_id = format!("subscriber_{}", uuid::Uuid::new_v4());

        info!(
            "Creating performance subscriber on '{}' (ID: {})",
            subscriber_config.topic, subscriber_id
        );
        info!("  Concurrent tasks: {}", subscriber_config.concurrent_tasks);
        if let Some(rate) = subscriber_config.rate_limit {
            info!("  Rate limit: {} msg/sec (slow consumer mode)", rate);
        }

        // Shared metrics collector
        let metrics = Arc::new(Mutex::new(MetricsCollector::new()));

        // Track last sequence number for queue depth calculation
        let last_sequence = Arc::new(Mutex::new(0u64));

        // Flag to track if test is running
        let test_running = Arc::new(Mutex::new(false));

        // Calculate delay between reads if rate limited
        let delay_between_reads = subscriber_config.rate_limit.map(|rate| {
            let per_task_rate = rate / subscriber_config.concurrent_tasks as f64;
            Duration::from_secs_f64(1.0 / per_task_rate)
        });

        // Spawn resource monitoring task
        let metrics_for_monitor = metrics.clone();
        let test_running_for_monitor = test_running.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(1));
            loop {
                interval.tick().await;
                if *test_running_for_monitor.lock().await {
                    metrics_for_monitor.lock().await.sample_resources();
                }
            }
        });

        // Subscribe to control messages
        let mut control_sub = context.subscribe("perf.control").await?;

        let metrics_for_publisher = metrics.clone();
        let message_bus = context.message_bus.clone();
        let subscriber_id_for_publisher = subscriber_id.clone();
        let test_running_for_control = test_running.clone();

        // Spawn task to handle control messages
        tokio::spawn(async move {
            loop {
                if let Ok((_, msg)) = control_sub.read().await {
                    match msg.as_ref() {
                        PerfMessage::StartTest { .. } => {
                            info!("Subscriber received StartTest signal");
                            *test_running_for_control.lock().await = true;
                        }
                        PerfMessage::StopTest { scenario_id } => {
                            info!("Subscriber received StopTest signal");
                            *test_running_for_control.lock().await = false;

                            // Finalize metrics and publish results
                            let mut metrics_guard = metrics_for_publisher.lock().await;
                            let perf_metrics = metrics_guard.finalize();

                            info!(
                                "Subscriber publishing metrics: {} messages, latency p50={:.2}µs",
                                perf_metrics.total_messages,
                                perf_metrics.latency_nanos.p50 as f64 / 1000.0
                            );

                            let metrics_msg = PerfMessage::metrics(
                                scenario_id.clone(),
                                subscriber_id_for_publisher.clone(),
                                perf_metrics,
                            );

                            if let Err(e) = message_bus
                                .publish("perf.metrics", Arc::new(metrics_msg))
                                .await
                            {
                                warn!("Failed to publish metrics: {}", e);
                            } else {
                                info!("Successfully published metrics to perf.metrics");
                            }

                            break;
                        }
                        _ => {}
                    }
                }
            }
        });

        // Spawn subscriber task(s)
        for _ in 0..subscriber_config.concurrent_tasks {
            // Each task gets its own subscription
            let mut sub = context.subscribe(&subscriber_config.topic).await?;
            let metrics = metrics.clone();
            let last_sequence = last_sequence.clone();
            let delay = delay_between_reads;
            let test_running = test_running.clone();

            context.run(async move {
                let mut msg_count = 0u64;
                loop {
                    if let Ok((_, message)) = sub.read().await {
                        // Process PerfData messages
                        if let PerfMessage::PerfData {
                            sequence,
                            timestamp_nanos,
                            ..
                        } = message.as_ref()
                        {
                            msg_count += 1;
                            if msg_count % 1000 == 0 {
                                info!("Subscriber received {} messages", msg_count);
                            }

                            // Always record metrics (test_running flag is for resource monitoring only)
                            {
                                // Calculate latency
                                let now_nanos = SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .expect("Time went backwards")
                                    .as_nanos()
                                    as i64;

                                let latency_nanos = (now_nanos - timestamp_nanos) as u64;

                                // Calculate queue depth (gap from last sequence)
                                let mut last_seq = last_sequence.lock().await;
                                let queue_depth = if *last_seq > 0 {
                                    sequence.saturating_sub(*last_seq)
                                } else {
                                    1
                                };
                                *last_seq = *sequence;
                                drop(last_seq);

                                // Record metrics
                                let mut metrics_guard = metrics.lock().await;
                                metrics_guard.record_latency(latency_nanos);
                                metrics_guard.record_queue_depth(queue_depth);
                                metrics_guard.increment_message_count();
                                drop(metrics_guard);

                                // Apply rate limiting if configured
                                if let Some(delay_duration) = delay {
                                    info!(
                                        "Subscriber subscription closed after {} messages",
                                        msg_count
                                    );
                                    sleep(delay_duration).await;
                                }
                            }
                        }
                    } else {
                        // Subscription closed
                        break;
                    }
                }
            });
        }

        Ok(())
    }
}
