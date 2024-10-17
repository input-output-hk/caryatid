// In-memory pub-sub bus with multi-threaded async workers
use tokio::sync::{mpsc, Mutex};
use std::collections::HashMap;
use std::sync::Arc;
use anyhow::Result;
use config::Config;
use tracing::error;
use futures::future::BoxFuture;
use crate::message_bus::{MessageBus, BoxedSubscriber, MessageBounds};
use tracing::info;

const DEFAULT_WORKERS: i64 = 4;

pub struct InMemoryBus<M: MessageBounds> {

    // Map of subscriber functions by topic
    subscribers: Arc<Mutex<HashMap<String, Vec<Arc<BoxedSubscriber<M>>>>>>,

    // Sender for received messages
    sender: mpsc::Sender<(String, Arc<M>)>,
}

impl<M: MessageBounds> InMemoryBus<M> {
    pub fn new(config: &Config) -> Self {
        let num_workers = config.get_int("workers")
            .unwrap_or(DEFAULT_WORKERS) as usize;

        let (sender, mut receiver) = mpsc::channel::<(String, Arc<M>)>(100);

        info!("Creating in-memory message bus with {} workers", num_workers);

        let subscribers: Arc<Mutex<HashMap<String,
                                         Vec<Arc<BoxedSubscriber<M>>>>>> =
            Arc::new(Mutex::new(HashMap::new()));

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
                if let Some(subscriber_list) = subscribers.get(&topic) {
                    // For each subscriber, dispatch the task to a worker
                    for subscriber in subscriber_list {
                        let worker_tx =
                            &worker_txs[round_robin_index % num_workers];

                        // Send the subscriber and the message to a worker
                        if let Err(e) = worker_tx.send((subscriber.clone(),
                                                        message.clone())).await {
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
            sender.send((topic, message)).await?;
            Ok(())
        })
    }

    // Subscribe for a message with an subscriber function
    fn register_subscriber(&self, topic: &str, subscriber: BoxedSubscriber<M>)
                         -> Result<()> {
        tokio::task::block_in_place(|| {
            let mut subscribers = self.subscribers.blocking_lock();
            subscribers.entry(topic.to_string())
                .or_insert(Vec::new())
                .push(Arc::new(subscriber));
            Ok(())
        })
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

