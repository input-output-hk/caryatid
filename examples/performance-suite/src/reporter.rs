//! Results reporting in multiple formats

use crate::config::OutputFormat;
use crate::message::PerformanceMetrics;
use anyhow::Result;
use chrono::Utc;
use prettytable::{Cell, Row, Table};
use serde_json::json;
use std::fs;
use std::path::Path;

/// Trait for performance test reporters
pub trait Reporter {
    fn report(&self, metrics: &PerformanceMetrics, scenario_name: &str) -> Result<()>;
}

/// Console reporter with formatted output
pub struct ConsoleReporter;

impl ConsoleReporter {
    pub fn new() -> Self {
        Self
    }

    fn format_nanos_as_micros(nanos: u64) -> String {
        format!("{:.2}", nanos as f64 / 1_000.0)
    }
}

impl Reporter for ConsoleReporter {
    fn report(&self, metrics: &PerformanceMetrics, scenario_name: &str) -> Result<()> {
        println!("\n{}", "=".repeat(70));
        println!("Performance Test Results");
        println!("{}", "=".repeat(70));
        println!("Scenario: {}", scenario_name);
        println!("Duration: {:.2}s", metrics.duration_secs);
        println!("Messages: {}", format_with_commas(metrics.total_messages));
        println!(
            "Throughput: {} msg/sec",
            format_with_commas(metrics.throughput_msg_sec as u64)
        );
        println!();

        // Latency table
        let mut latency_table = Table::new();
        latency_table.add_row(Row::new(vec![
            Cell::new("Latency (μs)"),
            Cell::new("Value"),
        ]));
        latency_table.add_row(Row::new(vec![
            Cell::new("Min"),
            Cell::new(&Self::format_nanos_as_micros(metrics.latency_nanos.min)),
        ]));
        latency_table.add_row(Row::new(vec![
            Cell::new("p50"),
            Cell::new(&Self::format_nanos_as_micros(metrics.latency_nanos.p50)),
        ]));
        latency_table.add_row(Row::new(vec![
            Cell::new("p90"),
            Cell::new(&Self::format_nanos_as_micros(metrics.latency_nanos.p90)),
        ]));
        latency_table.add_row(Row::new(vec![
            Cell::new("p95"),
            Cell::new(&Self::format_nanos_as_micros(metrics.latency_nanos.p95)),
        ]));
        latency_table.add_row(Row::new(vec![
            Cell::new("p99"),
            Cell::new(&Self::format_nanos_as_micros(metrics.latency_nanos.p99)),
        ]));
        latency_table.add_row(Row::new(vec![
            Cell::new("p99.9"),
            Cell::new(&Self::format_nanos_as_micros(metrics.latency_nanos.p999)),
        ]));
        latency_table.add_row(Row::new(vec![
            Cell::new("Max"),
            Cell::new(&Self::format_nanos_as_micros(metrics.latency_nanos.max)),
        ]));
        latency_table.add_row(Row::new(vec![
            Cell::new("Mean"),
            Cell::new(&Self::format_nanos_as_micros(
                metrics.latency_nanos.mean as u64,
            )),
        ]));

        latency_table.printstd();
        println!();

        // Queue depth table
        if metrics.queue_depth.max > 0 {
            let mut queue_table = Table::new();
            queue_table.add_row(Row::new(vec![Cell::new("Queue Depth"), Cell::new("Value")]));
            queue_table.add_row(Row::new(vec![
                Cell::new("Min"),
                Cell::new(&metrics.queue_depth.min.to_string()),
            ]));
            queue_table.add_row(Row::new(vec![
                Cell::new("p50"),
                Cell::new(&metrics.queue_depth.p50.to_string()),
            ]));
            queue_table.add_row(Row::new(vec![
                Cell::new("p95"),
                Cell::new(&metrics.queue_depth.p95.to_string()),
            ]));
            queue_table.add_row(Row::new(vec![
                Cell::new("p99"),
                Cell::new(&metrics.queue_depth.p99.to_string()),
            ]));
            queue_table.add_row(Row::new(vec![
                Cell::new("Max"),
                Cell::new(&metrics.queue_depth.max.to_string()),
            ]));

            queue_table.printstd();
            println!();
        }

        // Resources
        println!("Resources:");
        println!("  CPU: {:.1}% avg", metrics.cpu_percent);
        println!("  Memory: {:.1} MB", metrics.memory_usage_mb);
        println!("{}", "=".repeat(70));
        println!();

        Ok(())
    }
}

/// JSON reporter for structured output
pub struct JsonReporter {
    output_dir: String,
}

impl JsonReporter {
    pub fn new(output_dir: String) -> Self {
        Self { output_dir }
    }
}

impl Reporter for JsonReporter {
    fn report(&self, metrics: &PerformanceMetrics, scenario_name: &str) -> Result<()> {
        // Ensure output directory exists
        fs::create_dir_all(&self.output_dir)?;

        let timestamp = Utc::now();
        let output = json!({
            "scenario": scenario_name,
            "timestamp": timestamp.to_rfc3339(),
            "duration_secs": metrics.duration_secs,
            "messages": metrics.total_messages,
            "throughput": metrics.throughput_msg_sec,
            "latency_nanos": {
                "min": metrics.latency_nanos.min,
                "p50": metrics.latency_nanos.p50,
                "p90": metrics.latency_nanos.p90,
                "p95": metrics.latency_nanos.p95,
                "p99": metrics.latency_nanos.p99,
                "p999": metrics.latency_nanos.p999,
                "max": metrics.latency_nanos.max,
                "mean": metrics.latency_nanos.mean,
                "stddev": metrics.latency_nanos.stddev,
            },
            "latency_micros": {
                "min": metrics.latency_nanos.min as f64 / 1_000.0,
                "p50": metrics.latency_nanos.p50 as f64 / 1_000.0,
                "p90": metrics.latency_nanos.p90 as f64 / 1_000.0,
                "p95": metrics.latency_nanos.p95 as f64 / 1_000.0,
                "p99": metrics.latency_nanos.p99 as f64 / 1_000.0,
                "p999": metrics.latency_nanos.p999 as f64 / 1_000.0,
                "max": metrics.latency_nanos.max as f64 / 1_000.0,
                "mean": metrics.latency_nanos.mean / 1_000.0,
                "stddev": metrics.latency_nanos.stddev / 1_000.0,
            },
            "queue_depth": {
                "min": metrics.queue_depth.min,
                "p50": metrics.queue_depth.p50,
                "p90": metrics.queue_depth.p90,
                "p95": metrics.queue_depth.p95,
                "p99": metrics.queue_depth.p99,
                "p999": metrics.queue_depth.p999,
                "max": metrics.queue_depth.max,
                "mean": metrics.queue_depth.mean,
                "stddev": metrics.queue_depth.stddev,
            },
            "resources": {
                "cpu_percent": metrics.cpu_percent,
                "memory_mb": metrics.memory_usage_mb,
            }
        });

        let filename = format!("{}/{}_results.json", self.output_dir, scenario_name);
        let json_str = serde_json::to_string_pretty(&output)?;
        fs::write(&filename, json_str)?;

        println!("JSON results written to: {}", filename);

        Ok(())
    }
}

/// CSV reporter for spreadsheet import
pub struct CsvReporter {
    output_dir: String,
}

impl CsvReporter {
    pub fn new(output_dir: String) -> Self {
        Self { output_dir }
    }
}

impl Reporter for CsvReporter {
    fn report(&self, metrics: &PerformanceMetrics, scenario_name: &str) -> Result<()> {
        fs::create_dir_all(&self.output_dir)?;

        let filename = format!("{}/results.csv", self.output_dir);
        let file_exists = Path::new(&filename).exists();

        let mut csv_content = String::new();

        // Write header if file doesn't exist
        if !file_exists {
            csv_content.push_str("scenario,timestamp,duration_secs,messages,throughput,");
            csv_content.push_str(
                "latency_min_us,latency_p50_us,latency_p95_us,latency_p99_us,latency_max_us,",
            );
            csv_content
                .push_str("queue_depth_p50,queue_depth_p95,queue_depth_p99,queue_depth_max,");
            csv_content.push_str("cpu_percent,memory_mb\n");
        }

        // Write data row
        let timestamp = Utc::now().to_rfc3339();
        csv_content.push_str(&format!(
            "{},{},{},{},{},",
            scenario_name,
            timestamp,
            metrics.duration_secs,
            metrics.total_messages,
            metrics.throughput_msg_sec
        ));
        csv_content.push_str(&format!(
            "{},{},{},{},{},",
            metrics.latency_nanos.min as f64 / 1_000.0,
            metrics.latency_nanos.p50 as f64 / 1_000.0,
            metrics.latency_nanos.p95 as f64 / 1_000.0,
            metrics.latency_nanos.p99 as f64 / 1_000.0,
            metrics.latency_nanos.max as f64 / 1_000.0,
        ));
        csv_content.push_str(&format!(
            "{},{},{},{},",
            metrics.queue_depth.p50,
            metrics.queue_depth.p95,
            metrics.queue_depth.p99,
            metrics.queue_depth.max,
        ));
        csv_content.push_str(&format!(
            "{},{}\n",
            metrics.cpu_percent, metrics.memory_usage_mb
        ));

        // Append to file
        use std::fs::OpenOptions;
        use std::io::Write;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&filename)?;

        file.write_all(csv_content.as_bytes())?;

        println!("CSV results appended to: {}", filename);

        Ok(())
    }
}

/// Create reporter based on output format
pub fn create_reporter(format: OutputFormat, output_dir: &str) -> Box<dyn Reporter> {
    match format {
        OutputFormat::Console => Box::new(ConsoleReporter::new()),
        OutputFormat::Json => Box::new(JsonReporter::new(output_dir.to_string())),
        OutputFormat::Csv => Box::new(CsvReporter::new(output_dir.to_string())),
        OutputFormat::Markdown => {
            // For now, markdown uses JSON reporter
            // TODO: Implement proper markdown reporter
            Box::new(JsonReporter::new(output_dir.to_string()))
        }
    }
}

/// Format number with thousands separators
fn format_with_commas(n: u64) -> String {
    n.to_string()
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(std::str::from_utf8)
        .collect::<Result<Vec<&str>, _>>()
        .unwrap()
        .join(",")
}
