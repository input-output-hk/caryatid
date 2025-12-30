//! Performance test message types for Caryatid benchmarking

use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Performance metrics collected during a test run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Total number of messages processed
    pub total_messages: u64,
    /// Test duration in seconds
    pub duration_secs: f64,
    /// Throughput in messages per second
    pub throughput_msg_sec: f64,
    /// Latency statistics in nanoseconds
    pub latency_nanos: LatencyStats,
    /// Queue depth statistics
    pub queue_depth: QueueDepthStats,
    /// Memory usage in MB
    pub memory_usage_mb: f64,
    /// CPU usage percentage
    pub cpu_percent: f64,
}

/// Latency statistics with percentile distribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyStats {
    pub min: u64,
    pub p50: u64,
    pub p90: u64,
    pub p95: u64,
    pub p99: u64,
    pub p999: u64,
    pub max: u64,
    pub mean: f64,
    pub stddev: f64,
}

/// Queue depth statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueDepthStats {
    pub min: u64,
    pub p50: u64,
    pub p90: u64,
    pub p95: u64,
    pub p99: u64,
    pub p999: u64,
    pub max: u64,
    pub mean: f64,
    pub stddev: f64,
}

/// Performance test messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PerfMessage {
    /// Signal from subscriber that it's ready to receive messages
    Ready {
        module_id: String,
        module_type: String, // "subscriber" or "publisher"
    },

    /// Signal to start a test scenario
    StartTest {
        scenario_id: String,
        #[serde(skip)]
        timestamp: Option<Instant>,
    },

    /// Signal to stop a test scenario
    StopTest { scenario_id: String },

    /// Performance test data message
    PerfData {
        sequence: u64,
        /// Nanoseconds since epoch
        timestamp_nanos: i64,
        payload: Vec<u8>,
        scenario_id: String,
    },

    /// Metrics results from a subscriber
    Metrics {
        scenario_id: String,
        subscriber_id: String,
        metrics: PerformanceMetrics,
    },
}

impl Default for PerfMessage {
    fn default() -> Self {
        Self::StartTest {
            scenario_id: String::new(),
            timestamp: None,
        }
    }
}

impl PerfMessage {
    /// Create a Ready signal
    pub fn ready(module_id: String, module_type: String) -> Self {
        Self::Ready {
            module_id,
            module_type,
        }
    }

    /// Create a new StartTest message
    pub fn start_test(scenario_id: String) -> Self {
        Self::StartTest {
            scenario_id,
            timestamp: Some(Instant::now()),
        }
    }

    /// Create a new StopTest message
    pub fn stop_test(scenario_id: String) -> Self {
        Self::StopTest { scenario_id }
    }

    /// Create a new PerfData message
    pub fn perf_data(sequence: u64, payload: Vec<u8>, scenario_id: String) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};

        let timestamp_nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_nanos() as i64;

        Self::PerfData {
            sequence,
            timestamp_nanos,
            payload,
            scenario_id,
        }
    }

    /// Create a new Metrics message
    pub fn metrics(
        scenario_id: String,
        subscriber_id: String,
        metrics: PerformanceMetrics,
    ) -> Self {
        Self::Metrics {
            scenario_id,
            subscriber_id,
            metrics,
        }
    }
}

impl PerformanceMetrics {
    /// Create a new PerformanceMetrics with default values
    pub fn new() -> Self {
        Self {
            total_messages: 0,
            duration_secs: 0.0,
            throughput_msg_sec: 0.0,
            latency_nanos: LatencyStats::default(),
            queue_depth: QueueDepthStats::default(),
            memory_usage_mb: 0.0,
            cpu_percent: 0.0,
        }
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for LatencyStats {
    fn default() -> Self {
        Self {
            min: 0,
            p50: 0,
            p90: 0,
            p95: 0,
            p99: 0,
            p999: 0,
            max: 0,
            mean: 0.0,
            stddev: 0.0,
        }
    }
}

impl Default for QueueDepthStats {
    fn default() -> Self {
        Self {
            min: 0,
            p50: 0,
            p90: 0,
            p95: 0,
            p99: 0,
            p999: 0,
            max: 0,
            mean: 0.0,
            stddev: 0.0,
        }
    }
}
