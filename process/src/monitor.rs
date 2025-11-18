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

#[derive(Serialize)]
struct SerializedReadStreamState {
    read: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    unread: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pending_for: Option<String>,
}

#[derive(Serialize)]
struct SerializedWriteStreamState {
    written: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pending_for: Option<String>,
}

#[derive(Serialize)]
struct SerializedModuleState {
    reads: BTreeMap<String, SerializedReadStreamState>,
    writes: BTreeMap<String, SerializedWriteStreamState>,
}

const fn default_frequency() -> f64 {
    5.0
}

#[derive(Deserialize)]
pub struct MonitorConfig {
    output: PathBuf,
    #[serde(default = "default_frequency")]
    frequency_secs: f64,
}

pub struct Monitor {
    modules: BTreeMap<String, Arc<ModuleState>>,
    stream_writes: Arc<DashMap<String, u64>>,
    output_path: PathBuf,
    write_frequency: Duration,
}
impl Monitor {
    pub fn new(config: MonitorConfig) -> Self {
        Self {
            modules: BTreeMap::new(),
            stream_writes: Arc::new(DashMap::new()),
            output_path: config.output,
            write_frequency: Duration::from_secs_f64(config.frequency_secs),
        }
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

    pub async fn monitor(self) {
        loop {
            time::sleep(self.write_frequency).await;
            let now = Instant::now();
            let state = self
                .modules
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
                .collect::<BTreeMap<_, _>>();
            let serialized = serde_json::to_vec_pretty(&state).expect("could not serialize state");
            fs::write(&self.output_path, serialized)
                .await
                .expect("could not write file");
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
        writes.written += 1;
        writes.pending_since = None;
        *self.stream_writes.entry(topic.to_string()).or_default() += 1;
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
