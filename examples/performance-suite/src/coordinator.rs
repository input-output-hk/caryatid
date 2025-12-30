//! Test coordinator module

use crate::config::{CoordinatorConfig, ScenarioConfig};
use crate::message::{PerfMessage, PerformanceMetrics};
use crate::reporter::create_reporter;
use ::config::Config;
use anyhow::Result;
use caryatid_sdk::{module, Context};
use core::panic;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};

/// Test coordinator
#[module(
    message_type(PerfMessage),
    name = "coordinator",
    description = "Performance test orchestrator"
)]
pub struct Coordinator;

impl Coordinator {
    async fn init(&self, context: Arc<Context<PerfMessage>>, config: Arc<Config>) -> Result<()> {
        // Parse configurations from root config (not module-scoped config)
        let coordinator_config: CoordinatorConfig = context
            .config
            .get("coordinator")
            .unwrap_or_else(|_| CoordinatorConfig::default());

        let scenario_config: ScenarioConfig = context
            .config
            .get("scenario")
            .unwrap_or_else(|_| ScenarioConfig::default());

        info!("Performance test coordinator starting");
        info!("  Scenario: {}", scenario_config.name);
        info!("  Warmup: {}s", scenario_config.warmup_secs);
        info!("  Test duration: {}s", scenario_config.test_duration_secs);
        info!("  Cooldown: {}s", scenario_config.cooldown_secs);

        let scenario_id = scenario_config.name.clone();
        let scenario_name = scenario_config.name.clone();
        let message_bus = context.message_bus.clone();

        // Storage for collected metrics
        let collected_metrics: Arc<Mutex<Vec<PerformanceMetrics>>> =
            Arc::new(Mutex::new(Vec::new()));

        // Subscribe to metrics messages BEFORE starting the test
        let mut metrics_sub = context.subscribe("perf.metrics").await?;
        let collected_metrics_for_sub = collected_metrics.clone();

        info!("Subscribed to perf.metrics topic");

        context.run(async move {
            info!("Coordinator metrics collector task started");
            loop {
                match metrics_sub.read().await {
                    Ok((_, msg)) => {
                        if let PerfMessage::Metrics { metrics, .. } = msg.as_ref() {
                            info!(
                                "Coordinator received metrics from subscriber: {} messages",
                                metrics.total_messages
                            );
                            collected_metrics_for_sub.lock().await.push(metrics.clone());
                        }
                    }
                    Err(e) => {
                        warn!("Coordinator metrics subscription error: {:?}", e);
                        break;
                    }
                }
            }
            info!("Coordinator metrics collector task exiting");
        });

        // Subscribe to ready signals FIRST
        let mut ready_sub = context.subscribe("perf.ready").await?;
        let ready_modules: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let ready_modules_for_task = ready_modules.clone();

        tokio::spawn(async move {
            info!("Coordinator waiting for module ready signals...");
            loop {
                match ready_sub.read().await {
                    Ok((_, msg)) => {
                        if let PerfMessage::Ready {
                            module_id,
                            module_type,
                        } = msg.as_ref()
                        {
                            info!("Module ready: {} ({})", module_id, module_type);
                            ready_modules_for_task.lock().await.push(module_id.clone());
                        }
                    }
                    Err(e) => {
                        warn!("Ready subscription error: {:?}", e);
                        break;
                    }
                }
            }
        });

        // Orchestrate test lifecycle
        tokio::spawn(async move {
            // Give all modules time to signal ready (they signal at end of init)
            // init phase: wait for all subscriptions to be ready
            info!(
                "Initialization phase: waiting {}s for subscriptions to be ready...",
                2
            );
            sleep(Duration::from_secs(2)).await;

            // Wait for modules to signal ready
            info!("Waiting for all modules to be ready...");
            let timeout = Duration::from_secs(10);
            let start = std::time::Instant::now();

            loop {
                sleep(Duration::from_millis(500)).await;
                let ready_count = ready_modules.lock().await.len();

                // We expect at least 2 modules: publisher and subscriber
                if ready_count >= 2 {
                    info!("All modules ready ({}), starting test", ready_count);
                    break;
                }

                if start.elapsed() > timeout {
                    error!("Timeout waiting for modules to be ready. Halting test.");
                    std::process::exit(1);
                }
            }

            // Warmup phase
            // info!("Starting warmup phase ({}s)", scenario_config.warmup_secs);
            let start_msg = PerfMessage::start_test(scenario_id.clone());
            if let Err(e) = message_bus
                .publish("perf.control", Arc::new(start_msg))
                .await
            {
                warn!("Failed to send StartTest: {}", e);
                return;
            }

            // sleep(Duration::from_secs(scenario_config.warmup_secs)).await;

            // Test phase
            info!("Running test for {}s", scenario_config.test_duration_secs);
            sleep(Duration::from_secs(scenario_config.test_duration_secs)).await;

            // Cooldown phase
            info!(
                "Test complete. Cooldown for {}s",
                scenario_config.cooldown_secs
            );
            let stop_msg = PerfMessage::stop_test(scenario_id.clone());
            info!("Sending StopTest signal to perf.control");
            if let Err(e) = message_bus
                .publish("perf.control", Arc::new(stop_msg))
                .await
            {
                warn!("Failed to send StopTest: {}", e);
                return;
            }
            info!("StopTest signal sent successfully");

            sleep(Duration::from_secs(scenario_config.cooldown_secs)).await;

            // Wait for metrics to arrive with timeout and polling
            info!("Waiting for metrics from subscribers...");
            let mut attempts = 0;
            let max_attempts = 30; // 30 seconds total
            loop {
                sleep(Duration::from_secs(1)).await;
                attempts += 1;

                let count = collected_metrics.lock().await.len();
                if count > 0 {
                    info!("Metrics received after {} seconds", attempts);
                    break;
                }

                if attempts >= max_attempts {
                    warn!("Timeout waiting for metrics after {} seconds", max_attempts);
                    break;
                }

                if attempts % 5 == 0 {
                    info!(
                        "Still waiting for metrics... ({}/{}s)",
                        attempts, max_attempts
                    );
                    info!("re-Sending StopTest signal to perf.control");
                    let stop_msg = PerfMessage::stop_test(scenario_id.clone());
                    if let Err(e) = message_bus
                        .publish("perf.control", Arc::new(stop_msg))
                        .await
                    {
                        warn!("Failed to send StopTest: {}", e);
                        return;
                    }
                    info!("re-send of StopTest signal sent successfully");
                }
            }

            // Aggregate and report metrics
            info!("Aggregating metrics...");
            let metrics_vec = collected_metrics.lock().await;

            if metrics_vec.is_empty() {
                error!(
                    "No metrics collected! This may indicate a race condition or subscriber issue."
                );
                std::process::exit(1);
            }

            info!("Collected metrics from {} subscriber(s)", metrics_vec.len());

            // For now, use the first subscriber's metrics
            // TODO: Properly aggregate from multiple subscribers
            let final_metrics = if metrics_vec.len() == 1 {
                metrics_vec[0].clone()
            } else {
                // Simple aggregation: sum messages and average latencies
                aggregate_metrics(&metrics_vec)
            };

            // Generate reports
            info!("Generating reports...");
            for format in &coordinator_config.output_formats {
                let reporter = create_reporter(*format, &coordinator_config.output_dir);
                if let Err(e) = reporter.report(&final_metrics, &scenario_name) {
                    warn!("Failed to generate {:?} report: {}", format, e);
                }
            }

            info!("Performance test complete!");

            // Force immediate exit - don't rely on graceful shutdown
            // as it can hang with context.run() tasks
            std::process::exit(0);
        });

        Ok(())
    }
}

/// Aggregate metrics from multiple subscribers
fn aggregate_metrics(metrics_vec: &[PerformanceMetrics]) -> PerformanceMetrics {
    if metrics_vec.is_empty() {
        return PerformanceMetrics::default();
    }

    if metrics_vec.len() == 1 {
        return metrics_vec[0].clone();
    }

    // Sum total messages
    let total_messages: u64 = metrics_vec.iter().map(|m| m.total_messages).sum();

    // Use the longest duration
    let duration_secs = metrics_vec
        .iter()
        .map(|m| m.duration_secs)
        .fold(0.0f64, f64::max);

    // Calculate aggregate throughput
    let throughput_msg_sec = if duration_secs > 0.0 {
        total_messages as f64 / duration_secs
    } else {
        0.0
    };

    // For latency and queue depth, use the first subscriber's stats
    // TODO: Properly merge histograms
    let latency_nanos = metrics_vec[0].latency_nanos.clone();
    let queue_depth = metrics_vec[0].queue_depth.clone();

    // Average resource usage
    let cpu_percent =
        metrics_vec.iter().map(|m| m.cpu_percent).sum::<f64>() / metrics_vec.len() as f64;
    let memory_usage_mb =
        metrics_vec.iter().map(|m| m.memory_usage_mb).sum::<f64>() / metrics_vec.len() as f64;

    PerformanceMetrics {
        total_messages,
        duration_secs,
        throughput_msg_sec,
        latency_nanos,
        queue_depth,
        memory_usage_mb,
        cpu_percent,
    }
}
