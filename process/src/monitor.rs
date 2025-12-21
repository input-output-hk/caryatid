use std::{
    collections::BTreeMap,
    future::Future,
    path::PathBuf,
    sync::Arc,
    task::Poll,
    time::{Duration, Instant},
};

use anyhow::Result;
use async_trait::async_trait;
use caryatid_sdk::{MessageBounds, MessageBus, Subscription, SubscriptionBounds};
use dashmap::DashMap;
use futures::{future::BoxFuture, FutureExt};
use serde::{Deserialize, Serialize};
use tokio::{fs, time};
use tracing::warn;

#[derive(Default, Clone)]
struct ReadStreamState {
    read: u64,
    pending_since: Option<Instant>,
}

#[derive(Default, Clone)]
struct WriteStreamState {
    written: u64,
    pending_since: Option<Instant>,
}

#[derive(Default, Clone)]
struct ModuleState {
    reads: DashMap<String, ReadStreamState>,
    writes: DashMap<String, WriteStreamState>,
}

/// Serialized state for a read stream.
/// This type is shared with caryatid-doctor for parsing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedReadStreamState {
    pub read: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unread: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending_for: Option<String>,
}

/// Serialized state for a write stream.
/// This type is shared with caryatid-doctor for parsing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedWriteStreamState {
    pub written: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending_for: Option<String>,
}

/// Serialized state for a module.
/// This type is shared with caryatid-doctor for parsing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedModuleState {
    pub reads: BTreeMap<String, SerializedReadStreamState>,
    pub writes: BTreeMap<String, SerializedWriteStreamState>,
}

/// A complete monitor snapshot.
/// This is the top-level type published to the message bus or written to file.
pub type MonitorSnapshot = BTreeMap<String, SerializedModuleState>;

const fn default_frequency() -> f64 {
    5.0
}

/// Configuration for the Monitor.
///
/// At least one of `output` or `topic` must be specified.
///
/// # Examples
///
/// File output only (original behavior):
/// ```toml
/// [monitor]
/// output = "monitor.json"
/// frequency_secs = 5.0
/// ```
///
/// Message bus output only:
/// ```toml
/// [monitor]
/// topic = "caryatid.monitor.snapshot"
/// frequency_secs = 5.0
/// ```
///
/// Both file and message bus:
/// ```toml
/// [monitor]
/// output = "monitor.json"
/// topic = "caryatid.monitor.snapshot"
/// frequency_secs = 5.0
/// ```
#[derive(Deserialize)]
pub struct MonitorConfig {
    /// File path to write JSON snapshots (optional).
    #[serde(default)]
    pub output: Option<PathBuf>,

    /// Topic to publish snapshots to via message bus (optional).
    #[serde(default)]
    pub topic: Option<String>,

    /// How often to emit snapshots, in seconds.
    #[serde(default = "default_frequency")]
    pub frequency_secs: f64,
}

/// Type alias for the publisher callback.
/// Takes a JSON-serialized snapshot and publishes it.
pub type SnapshotPublisher = Box<dyn Fn(Vec<u8>) + Send + Sync>;

pub struct Monitor {
    modules: BTreeMap<String, Arc<ModuleState>>,
    stream_writes: Arc<DashMap<String, u64>>,
    output_path: Option<PathBuf>,
    topic: Option<String>,
    write_frequency: Duration,
}

impl Monitor {
    pub fn new(config: MonitorConfig) -> Self {
        Self {
            modules: BTreeMap::new(),
            stream_writes: Arc::new(DashMap::new()),
            output_path: config.output,
            topic: config.topic,
            write_frequency: Duration::from_secs_f64(config.frequency_secs),
        }
    }

    /// Returns the topic to publish to, if configured.
    pub fn topic(&self) -> Option<&str> {
        self.topic.as_deref()
    }

    pub fn spy_on_bus<M: MessageBounds>(
        &mut self,
        module_name: &str,
        message_bus: Arc<dyn MessageBus<M>>,
    ) -> Arc<dyn MessageBus<M>> {
        let state = Arc::new(ModuleState::default());
        self.modules.insert(module_name.to_string(), state.clone());

        Arc::new(MonitorBus {
            inner: message_bus,
            stream_writes: self.stream_writes.clone(),
            state,
        })
    }

    /// Collect the current state into a snapshot.
    fn collect_snapshot(&self) -> MonitorSnapshot {
        let now = Instant::now();
        self.modules
            .iter()
            .map(|(name, state)| {
                let reads = state
                    .reads
                    .iter()
                    .map(|kvp| {
                        let (topic, data) = kvp.pair();
                        let read = data.read;
                        let unread = self
                            .stream_writes
                            .get(topic)
                            .and_then(|w| w.checked_sub(read))
                            .filter(|u| *u > 0);
                        let pending_for = data
                            .pending_since
                            .map(|d| format!("{:?}", now.duration_since(d)));
                        let state = SerializedReadStreamState {
                            read,
                            unread,
                            pending_for,
                        };
                        (topic.clone(), state)
                    })
                    .collect();

                let writes = state
                    .writes
                    .iter()
                    .map(|kvp| {
                        let (topic, data) = kvp.pair();
                        let written = data.written;
                        let pending_for = data
                            .pending_since
                            .map(|d| format!("{:?}", now.duration_since(d)));
                        let state = SerializedWriteStreamState {
                            written,
                            pending_for,
                        };
                        (topic.clone(), state)
                    })
                    .collect();

                (name.clone(), SerializedModuleState { reads, writes })
            })
            .collect()
    }

    /// Run the monitor loop, writing to file only.
    /// This maintains backward compatibility with existing usage.
    pub async fn monitor(self) {
        self.monitor_with_publisher(None).await
    }

    /// Run the monitor loop with an optional publisher callback.
    ///
    /// The publisher is called with the JSON-serialized snapshot bytes
    /// each time a snapshot is collected. This allows the caller to
    /// publish to a message bus or any other destination.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let bus = message_bus.clone();
    /// let topic = "caryatid.monitor.snapshot".to_string();
    /// let publisher = Box::new(move |json: Vec<u8>| {
    ///     let bus = bus.clone();
    ///     let topic = topic.clone();
    ///     tokio::spawn(async move {
    ///         let msg = MyMessage::MonitorSnapshot(json);
    ///         let _ = bus.publish(&topic, Arc::new(msg)).await;
    ///     });
    /// });
    /// monitor.monitor_with_publisher(Some(publisher)).await;
    /// ```
    pub async fn monitor_with_publisher(self, publisher: Option<SnapshotPublisher>) {
        loop {
            time::sleep(self.write_frequency).await;
            let snapshot = self.collect_snapshot();

            // Serialize once for both outputs
            let serialized =
                serde_json::to_vec_pretty(&snapshot).expect("could not serialize state");

            // Write to file if configured
            if let Some(ref path) = self.output_path {
                if let Err(e) = fs::write(path, &serialized).await {
                    warn!("Failed to write monitor file: {}", e);
                }
            }

            // Call publisher if provided
            if let Some(ref publish) = publisher {
                publish(serialized);
            }
        }
    }
}

pub struct MonitorBus<M: MessageBounds> {
    inner: Arc<dyn MessageBus<M>>,
    stream_writes: Arc<DashMap<String, u64>>,
    state: Arc<ModuleState>,
}

#[async_trait]
impl<M: MessageBounds> MessageBus<M> for MonitorBus<M> {
    async fn publish(&self, topic: &str, message: Arc<M>) -> Result<()> {
        self.state
            .writes
            .entry(topic.to_string())
            .or_default()
            .pending_since = Some(Instant::now());
        let res = self.inner.publish(topic, message).await;
        let mut writes = self.state.writes.entry(topic.to_string()).or_default();
        writes.pending_since = None;
        if res.is_ok() {
            writes.written += 1;
            *self.stream_writes.entry(topic.to_string()).or_default() += 1;
        }
        res
    }

    fn request_timeout(&self) -> std::time::Duration {
        self.inner.request_timeout()
    }

    async fn subscribe(&self, topic: &str) -> Result<Box<dyn Subscription<M>>> {
        self.state.reads.entry(topic.to_string()).or_default();
        Ok(Box::new(MonitorSubscription {
            inner: self.inner.subscribe(topic).await?,
            state: self.state.clone(),
            topic: topic.to_string(),
        }))
    }

    async fn shutdown(&self) -> Result<()> {
        self.inner.shutdown().await
    }
}

struct MonitorSubscription<M: MessageBounds> {
    inner: Box<dyn Subscription<M>>,
    state: Arc<ModuleState>,
    topic: String,
}
impl<M: MessageBounds> SubscriptionBounds for MonitorSubscription<M> {}

impl<M: MessageBounds> Subscription<M> for MonitorSubscription<M> {
    fn read(&mut self) -> BoxFuture<'_, Result<(String, Arc<M>)>> {
        Box::pin(
            MonitorReadFuture {
                inner: self.inner.read(),
                state: &self.state,
                topic: &self.topic,
            }
            .fuse(),
        )
    }
}

struct MonitorReadFuture<'a, M: MessageBounds> {
    inner: BoxFuture<'a, Result<(String, Arc<M>)>>,
    state: &'a ModuleState,
    topic: &'a str,
}
impl<'a, M: MessageBounds> Future for MonitorReadFuture<'a, M> {
    type Output = Result<(String, Arc<M>)>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let res = self.inner.poll_unpin(cx);
        let mut entry = self.state.reads.entry(self.topic.to_string()).or_default();
        match &res {
            Poll::Pending => {
                if entry.pending_since.is_none() {
                    entry.pending_since = Some(Instant::now());
                }
            }
            Poll::Ready(r) => {
                entry.pending_since = None;
                if r.is_ok() {
                    entry.read += 1;
                }
            }
        }
        res
    }
}
impl<'a, M: MessageBounds> Drop for MonitorReadFuture<'a, M> {
    fn drop(&mut self) {
        let mut entry = self.state.reads.entry(self.topic.to_string()).or_default();
        entry.pending_since = None;
    }
}
