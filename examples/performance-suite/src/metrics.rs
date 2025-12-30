//! Metrics collection and statistical analysis

use crate::message::{LatencyStats, PerformanceMetrics, QueueDepthStats};
use hdrhistogram::Histogram;
use std::time::Instant;
use sysinfo::System;

/// Collects performance metrics during test execution
pub struct MetricsCollector {
    /// Histogram for latency measurements (nanoseconds)
    latency_histogram: Histogram<u64>,

    /// Histogram for queue depth measurements
    queue_depth_histogram: Histogram<u64>,

    /// Test start time
    start_time: Instant,

    /// Test end time
    end_time: Option<Instant>,

    /// Total message count
    message_count: u64,

    /// System monitor for resource tracking
    system: System,

    /// CPU samples for averaging
    cpu_samples: Vec<f32>,

    /// Memory samples for averaging
    memory_samples: Vec<u64>,
}

impl MetricsCollector {
    /// Create a new MetricsCollector
    pub fn new() -> Self {
        // Configure histogram for latencies up to 60 seconds with 3 significant digits
        let latency_histogram = Histogram::<u64>::new_with_bounds(1, 60_000_000_000, 3)
            .expect("Failed to create latency histogram");

        // Configure histogram for queue depths up to 1 million with 3 significant digits
        let queue_depth_histogram = Histogram::<u64>::new_with_bounds(1, 1_000_000, 3)
            .expect("Failed to create queue depth histogram");

        Self {
            latency_histogram,
            queue_depth_histogram,
            start_time: Instant::now(),
            end_time: None,
            message_count: 0,
            system: System::new_all(),
            cpu_samples: Vec::new(),
            memory_samples: Vec::new(),
        }
    }

    /// Record a latency measurement in nanoseconds
    pub fn record_latency(&mut self, nanos: u64) {
        // Clamp to valid range to avoid histogram errors
        let clamped = nanos.clamp(1, 60_000_000_000);
        self.latency_histogram.record(clamped).ok();
    }

    /// Record a queue depth measurement
    pub fn record_queue_depth(&mut self, depth: u64) {
        // Clamp to valid range
        let clamped = depth.clamp(0, 1_000_000);
        if clamped > 0 {
            self.queue_depth_histogram.record(clamped).ok();
        }
    }

    /// Increment message counter
    pub fn increment_message_count(&mut self) {
        self.message_count += 1;
    }

    /// Sample system resources
    pub fn sample_resources(&mut self) {
        self.system.refresh_all();

        // Get current process
        let pid = sysinfo::get_current_pid();

        if let Ok(pid) = pid {
            if let Some(process) = self.system.process(pid) {
                // Record CPU usage
                self.cpu_samples.push(process.cpu_usage());

                // Record memory usage (in bytes)
                self.memory_samples.push(process.memory());
            }
        }
    }

    /// Get percentile value from latency histogram
    pub fn latency_percentile(&self, percentile: f64) -> u64 {
        if self.latency_histogram.len() == 0 {
            return 0;
        }
        self.latency_histogram.value_at_quantile(percentile)
    }

    /// Get percentile value from queue depth histogram
    pub fn queue_depth_percentile(&self, percentile: f64) -> u64 {
        if self.queue_depth_histogram.len() == 0 {
            return 0;
        }
        self.queue_depth_histogram.value_at_quantile(percentile)
    }

    /// Finalize metrics collection and generate PerformanceMetrics
    pub fn finalize(&mut self) -> PerformanceMetrics {
        self.end_time = Some(Instant::now());

        let duration = self.end_time.unwrap().duration_since(self.start_time);
        let duration_secs = duration.as_secs_f64();

        let throughput_msg_sec = if duration_secs > 0.0 {
            self.message_count as f64 / duration_secs
        } else {
            0.0
        };

        // Calculate latency statistics
        let latency_nanos = if self.latency_histogram.len() > 0 {
            LatencyStats {
                min: self.latency_histogram.min(),
                p50: self.latency_percentile(0.50),
                p90: self.latency_percentile(0.90),
                p95: self.latency_percentile(0.95),
                p99: self.latency_percentile(0.99),
                p999: self.latency_percentile(0.999),
                max: self.latency_histogram.max(),
                mean: self.latency_histogram.mean(),
                stddev: self.latency_histogram.stdev(),
            }
        } else {
            LatencyStats::default()
        };

        // Calculate queue depth statistics
        let queue_depth = if self.queue_depth_histogram.len() > 0 {
            QueueDepthStats {
                min: self.queue_depth_histogram.min(),
                p50: self.queue_depth_percentile(0.50),
                p90: self.queue_depth_percentile(0.90),
                p95: self.queue_depth_percentile(0.95),
                p99: self.queue_depth_percentile(0.99),
                p999: self.queue_depth_percentile(0.999),
                max: self.queue_depth_histogram.max(),
                mean: self.queue_depth_histogram.mean(),
                stddev: self.queue_depth_histogram.stdev(),
            }
        } else {
            QueueDepthStats::default()
        };

        // Calculate average resource usage
        let cpu_percent = if !self.cpu_samples.is_empty() {
            self.cpu_samples.iter().sum::<f32>() / self.cpu_samples.len() as f32
        } else {
            0.0
        } as f64;

        let memory_usage_mb = if !self.memory_samples.is_empty() {
            let avg_bytes =
                self.memory_samples.iter().sum::<u64>() / self.memory_samples.len() as u64;
            avg_bytes as f64 / 1_048_576.0 // Convert to MB
        } else {
            0.0
        };

        PerformanceMetrics {
            total_messages: self.message_count,
            duration_secs,
            throughput_msg_sec,
            latency_nanos,
            queue_depth,
            memory_usage_mb,
            cpu_percent,
        }
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_latency_recording() {
        let mut collector = MetricsCollector::new();

        collector.record_latency(1000); // 1 microsecond
        collector.record_latency(50000); // 50 microseconds
        collector.record_latency(100000); // 100 microseconds

        assert_eq!(collector.latency_histogram.len(), 3);
        assert!(collector.latency_percentile(0.50) > 0);
    }

    #[test]
    fn test_queue_depth_recording() {
        let mut collector = MetricsCollector::new();

        collector.record_queue_depth(1);
        collector.record_queue_depth(5);
        collector.record_queue_depth(10);

        assert_eq!(collector.queue_depth_histogram.len(), 3);
        assert!(collector.queue_depth_percentile(0.50) > 0);
    }

    #[test]
    fn test_finalize() {
        let mut collector = MetricsCollector::new();

        for _ in 0..100 {
            collector.record_latency(1000);
            collector.increment_message_count();
        }

        let metrics = collector.finalize();

        assert_eq!(metrics.total_messages, 100);
        assert!(metrics.duration_secs > 0.0);
        assert!(metrics.throughput_msg_sec > 0.0);
    }
}
