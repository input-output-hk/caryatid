// In-memory pub-sub bus with multi-threaded async workers
use tokio::sync::{mpsc, Mutex};
use std::sync::Arc;
use anyhow::Result;
use config::Config;
use tracing::error;
use futures::future::BoxFuture;
use caryatid_sdk::message_bus::{MessageBus, BoxedSubscriber, MessageBounds};
use tracing::info;

const DEFAULT_WORKERS: i64 = 4;

struct PatternSubscriber<M: MessageBounds> {
    pattern: String,
    subscriber: Arc<BoxedSubscriber<M>>
}

/// In-memory, zero-copy pub-sub bus
pub struct InMemoryBus<M: MessageBounds> {

    /// Subscribers
    subscribers: Arc<Mutex<Vec<PatternSubscriber<M>>>>,

    /// Sender for received messages
    sender: mpsc::Sender<(String, Arc<M>)>,
}

impl<M: MessageBounds> InMemoryBus<M> {
    pub fn new(config: &Config) -> Self {
        let num_workers = config.get_int("workers")
            .unwrap_or(DEFAULT_WORKERS) as usize;

        let (sender, mut receiver) = mpsc::channel::<(String, Arc<M>)>(100);

        info!("Creating in-memory message bus with {} workers", num_workers);

        let subscribers: Arc<Mutex<Vec<PatternSubscriber<M>>>> =
            Arc::new(Mutex::new(Vec::new()));

        // Create a task queue channel for each worker
        let mut worker_txs = Vec::new();
        for _ in 0..num_workers {
            let (worker_tx, mut worker_rx) =
                mpsc::channel::<(Arc<BoxedSubscriber<M>>, Arc<M>)>(100);
            worker_txs.push(worker_tx);

            // Spawn worker tasks that handle individual subscriber invocations
            tokio::spawn(async move {
                while let Some((subscriber, message)) = worker_rx.recv().await {
                    subscriber(message).await;
                }
            });
        }

        // Single receiver task to handle incoming messages
        let subs_clone = subscribers.clone();
        tokio::spawn(async move {
            let mut round_robin_index = 0;

            while let Some((topic, message)) = receiver.recv().await {
                // Lock the subscribers for the topic
                let subscribers = subs_clone.lock().await;
                for patsub in subscribers.iter() {
                    if Self::match_topic(&patsub.pattern, &topic) {
                        // For each subscriber, dispatch the task to a worker
                        let worker_tx =
                            &worker_txs[round_robin_index % num_workers];

                        // Send the subscriber and the message to a worker
                        if let Err(e) = worker_tx.send(
                            (patsub.subscriber.clone(), message.clone())
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

    /// Match a dotted topic against a pattern - implements as RabbitMQ:
    ///   * - match one word
    ///   # - match zero or more words
    fn match_topic(pattern: &str, topic: &str) -> bool {

        let pattern_parts: Vec<&str> = pattern.split('.').collect();
        let topic_parts: Vec<&str> = topic.split('.').collect();

        let mut i = 0;
        let mut j = 0;

        while i < pattern_parts.len() && j < topic_parts.len() {

            match pattern_parts[i] {
                // * matches exactly one word
                "*" => {
                    i += 1;
                    j += 1;
                }

                // # matches zero or more words
                "#" => {
                    if i == pattern_parts.len() - 1 {
                        // All the rest
                        return true;
                    }

                    // Try to match the next part of the pattern to any
                    // subsequent part of the topic
                    while j < topic_parts.len() {
                        if Self::match_topic(&pattern_parts[i + 1..].join("."),
                                             &topic_parts[j..].join(".")) {
                            return true;
                        }
                        j += 1;
                    }

                    // No match found for the rest of the topic after #
                    return false;
                }

                // Direct match for a part?
                part if part == topic_parts[j] => {
                    i += 1;
                    j += 1;
                }

                // No match
                _ => return false
            }
        }

        // If we reached the end of both the pattern and topic, it's a match
        i == pattern_parts.len() && j == topic_parts.len()
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
            sender.send((topic, message)).await?;
            Ok(())
        })
    }

    // Subscribe for a message with an subscriber function
    fn register_subscriber(&self, topic: &str, subscriber: BoxedSubscriber<M>)
                         -> Result<()> {
        let subscribers = self.subscribers.clone();
        let topic = topic.to_string();
        tokio::spawn(async move {
            let mut subscribers = subscribers.lock().await;
            subscribers.push(PatternSubscriber {
                pattern: topic,
                subscriber: subscriber.into()
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

