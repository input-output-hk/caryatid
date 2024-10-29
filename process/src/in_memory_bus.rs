// In-memory pub-sub bus with multi-threaded async workers
use tokio::sync::{mpsc, Mutex, oneshot};
use tokio::sync::oneshot::Sender;
use std::sync::Arc;
use anyhow::{Result, anyhow};
use config::Config;
use tracing::error;
use futures::future::BoxFuture;
use caryatid_sdk::message_bus::{MessageBus, Subscriber, MessageBounds};
use tracing::info;
use crate::match_topic::match_topic;

const DEFAULT_WORKERS: i64 = 4;

/// Subscriber on a particular topic pattern
struct PatternSubscriber<M: MessageBounds> {
    pattern: String,
    subscriber: Arc<Subscriber<M>>,
}

/// Wrapper for a message with a oneshot to send back a result
struct Envelope<M: MessageBounds> {
    message: Arc<M>,
    notify: Option<Sender<Arc<Result<M>>>>,
}

/// In-memory, zero-copy pub-sub bus
pub struct InMemoryBus<M: MessageBounds> {

    /// Subscribers
    subscribers: Arc<Mutex<Vec<PatternSubscriber<M>>>>,

    /// Sender for received messages
    sender: mpsc::Sender<(String, Arc<Mutex<Envelope<M>>>)>,
}

impl<M: MessageBounds> InMemoryBus<M> {
    pub fn new(config: &Config) -> Self {
        let num_workers = config.get_int("workers")
            .unwrap_or(DEFAULT_WORKERS) as usize;

        let (sender, mut receiver) =
            mpsc::channel::<(String, Arc<Mutex<Envelope<M>>>)>(100);

        info!("Creating in-memory message bus with {} workers", num_workers);

        let subscribers: Arc<Mutex<Vec<PatternSubscriber<M>>>> =
            Arc::new(Mutex::new(Vec::new()));

        // Create a task queue channel for each worker
        let mut worker_txs = Vec::new();
        for _ in 0..num_workers {
            let (worker_tx, mut worker_rx) =
                mpsc::channel::<(Arc<Subscriber<M>>,
                                 Arc<Mutex<Envelope<M>>>)>(100);
            worker_txs.push(worker_tx);

            // Spawn worker tasks that handle individual subscriber invocations
            tokio::spawn(async move {
                while let Some((subscriber, envelope)) = worker_rx.recv().await {
                    let result =
                        subscriber(envelope.lock().await.message.clone()).await;
                    let mut envelope = envelope.lock().await;
                    if let Some(notify) = envelope.notify.take() {
                        let _ = notify.send(result);
                    }
                }
            });
        }

        // Single receiver task to handle incoming messages
        let subs_clone = subscribers.clone();
        tokio::spawn(async move {
            let mut round_robin_index = 0;

            while let Some((topic, envelope)) = receiver.recv().await {

                // Lock the subscribers for the topic
                let subscribers = subs_clone.lock().await;
                for patsub in subscribers.iter() {
                    if match_topic(&patsub.pattern, &topic) {
                        // For each subscriber, dispatch the task to a worker
                        let worker_tx =
                            &worker_txs[round_robin_index % num_workers];

                        // Send the subscriber and the message to a worker
                        if let Err(e) = worker_tx.send(
                            (patsub.subscriber.clone(), envelope.clone())
                        ).await {
                            error!("Failed to send message to worker: {}", e);
                        }

                        round_robin_index += 1;
                    }
                }
            }
        });

        InMemoryBus { subscribers, sender }
    }
}

impl<M: MessageBounds> MessageBus<M> for InMemoryBus<M> {

    /// Publish a message on a given topic
    fn publish(&self, topic: &str, message: Arc<M>) ->
        BoxFuture<'static, Result<()>> {
        let topic = topic.to_string();
        let sender = self.sender.clone();
        let message = message.clone();

        Box::pin(async move {
            let envelope = Envelope { message, notify: None };
            sender.send((topic, Arc::new(Mutex::new(envelope)))).await?;
            Ok(())
        })
    }

    /// Request/response on a given topic
    fn request(&self, topic: &str, message: Arc<M>) ->
        BoxFuture<'static, Result<Arc<Result<M>>>> {
        let topic = topic.to_string();
        let sender = self.sender.clone();
        let message = message.clone();

        Box::pin(async move {
            let (notify_sender, notify_receiver) = oneshot::channel();
            let envelope = Envelope { message, notify: Some(notify_sender) };
            sender.send((topic, Arc::new(Mutex::new(envelope)))).await?;
            match notify_receiver.await {
                Ok(result) => Ok(result),
                Err(e) => Err(anyhow!("Notify receive failed: {e}"))
            }
        })
    }

    /// Subscribe for a message with an subscriber function
    fn register_subscriber(&self, topic: &str, subscriber: Arc<Subscriber<M>>)
                         -> Result<()> {
        let subscribers = self.subscribers.clone();
        let topic = topic.to_string();
        tokio::spawn(async move {
            let mut subscribers = subscribers.lock().await;
            subscribers.push(PatternSubscriber {
                pattern: topic,
                subscriber: subscriber
            });
        });

        Ok(())
    }

    /// Shut down, clearing all subscribers
    fn shutdown(&self) -> BoxFuture<'static, Result<()>> {
        let subscribers = self.subscribers.clone();

        Box::pin(async move {
            let mut subscribers = subscribers.lock().await;
            subscribers.clear();
            Ok(())
        })
    }
}

