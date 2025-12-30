//! Configuration structures for performance testing

use serde::{Deserialize, Serialize};

/// Output format for test results
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Console,
    Json,
    Csv,
    Markdown,
}

/// Coordinator configuration
#[derive(Debug, Clone, Deserialize)]
pub struct CoordinatorConfig {
    /// Output directory for results
    #[serde(default = "default_output_dir")]
    pub output_dir: String,

    /// Output formats to generate
    #[serde(default = "default_output_formats")]
    pub output_formats: Vec<OutputFormat>,
}

fn default_output_dir() -> String {
    "./results".to_string()
}

fn default_output_formats() -> Vec<OutputFormat> {
    vec![OutputFormat::Console, OutputFormat::Json]
}

/// Test scenario configuration
#[derive(Debug, Clone, Deserialize)]
pub struct ScenarioConfig {
    /// Scenario name
    pub name: String,

    /// Scenario description
    #[serde(default)]
    pub description: String,

    /// Warmup period in seconds
    #[serde(default = "default_warmup_secs")]
    pub warmup_secs: u64,

    /// Test duration in seconds
    #[serde(default = "default_test_duration_secs")]
    pub test_duration_secs: u64,

    /// Cooldown period in seconds
    #[serde(default = "default_cooldown_secs")]
    pub cooldown_secs: u64,
}

fn default_warmup_secs() -> u64 {
    5
}

fn default_test_duration_secs() -> u64 {
    60
}

fn default_cooldown_secs() -> u64 {
    2
}

/// Publisher module configuration
#[derive(Debug, Clone, Deserialize)]
pub struct PublisherConfig {
    /// Topic to publish to
    pub topic: String,

    /// Total number of messages to send
    #[serde(default = "default_message_count")]
    pub message_count: u64,

    /// Size of each message payload in bytes
    #[serde(default = "default_message_size")]
    pub message_size: usize,

    /// Number of concurrent publishing tasks
    #[serde(default = "default_concurrent_tasks")]
    pub concurrent_tasks: usize,

    /// Optional rate limit in messages per second (None = unlimited)
    pub rate_limit: Option<f64>,
}

fn default_message_count() -> u64 {
    1_000_000
}

fn default_message_size() -> usize {
    100
}

fn default_concurrent_tasks() -> usize {
    1
}

/// Subscriber module configuration
#[derive(Debug, Clone, Deserialize)]
pub struct SubscriberConfig {
    /// Topic to subscribe to
    pub topic: String,

    /// Number of concurrent subscribing tasks
    #[serde(default = "default_concurrent_tasks")]
    pub concurrent_tasks: usize,

    /// Optional rate limit for slow consumer simulation (messages per second)
    pub rate_limit: Option<f64>,
}

impl Default for CoordinatorConfig {
    fn default() -> Self {
        Self {
            output_dir: default_output_dir(),
            output_formats: default_output_formats(),
        }
    }
}

impl Default for ScenarioConfig {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            description: String::new(),
            warmup_secs: default_warmup_secs(),
            test_duration_secs: default_test_duration_secs(),
            cooldown_secs: default_cooldown_secs(),
        }
    }
}

impl PublisherConfig {
    /// Validate configuration
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.message_count == 0 {
            anyhow::bail!("message_count must be greater than 0");
        }
        if self.message_size == 0 {
            anyhow::bail!("message_size must be greater than 0");
        }
        if self.concurrent_tasks == 0 {
            anyhow::bail!("concurrent_tasks must be greater than 0");
        }
        if let Some(rate) = self.rate_limit {
            if rate <= 0.0 {
                anyhow::bail!("rate_limit must be greater than 0");
            }
        }
        Ok(())
    }
}

impl SubscriberConfig {
    /// Validate configuration
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.concurrent_tasks == 0 {
            anyhow::bail!("concurrent_tasks must be greater than 0");
        }
        if let Some(rate) = self.rate_limit {
            if rate <= 0.0 {
                anyhow::bail!("rate_limit must be greater than 0");
            }
        }
        Ok(())
    }
}
