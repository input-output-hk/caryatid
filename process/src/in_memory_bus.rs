// In-memory pub-sub bus with multi-threaded async workers
use tokio::sync::{mpsc, Mutex};
use std::collections::HashMap;
use std::sync::Arc;
use anyhow::Result;
use config::Config;
use tracing::error;
use futures::future::BoxFuture;
use crate::message_bus::{MessageBus, BoxedObserverFn, MessageBounds};
use tracing::info;

const DEFAULT_WORKERS: i64 = 4;

pub struct InMemoryBus<M: MessageBounds> {

    // Map of observer functions by topic
    observers: Arc<Mutex<HashMap<String, Vec<Arc<BoxedObserverFn<M>>>>>>,

    // Sender for received messages
    sender: mpsc::Sender<(String, Arc<M>)>,
}

impl<M: MessageBounds> InMemoryBus<M> {
    pub fn new(config: &Config) -> Self {
        let num_workers = config.get_int("workers")
            .unwrap_or(DEFAULT_WORKERS) as usize;

        let (sender, mut receiver) = mpsc::channel::<(String, Arc<M>)>(100);

        info!("Creating in-memory message bus with {} workers", num_workers);

        let observers: Arc<Mutex<HashMap<String,
                                         Vec<Arc<BoxedObserverFn<M>>>>>> =
            Arc::new(Mutex::new(HashMap::new()));

        // Create a task queue channel for each worker
        let mut worker_txs = Vec::new();
        for _ in 0..num_workers {
            let (worker_tx, mut worker_rx) =
                mpsc::channel::<(Arc<BoxedObserverFn<M>>, Arc<M>)>(100);
            worker_txs.push(worker_tx);

            // Spawn worker tasks that handle individual observer invocations
            tokio::spawn(async move {
                while let Some((observer, message)) = worker_rx.recv().await {
                    observer(message).await;
                }
            });
        }

        // Single receiver task to handle incoming messages
        let obs_clone = observers.clone(); // Clone the Arc for the receiver task
        tokio::spawn(async move {
            let mut round_robin_index = 0;

            while let Some((topic, message)) = receiver.recv().await {
                // Lock the observers for the topic
                let observers = obs_clone.lock().await;
                if let Some(observer_list) = observers.get(&topic) {
                    // For each observer, dispatch the task to a worker
                    for observer in observer_list {
                        let worker_tx =
                            &worker_txs[round_robin_index % num_workers];

                        // Send the observer and the message to a worker
                        if let Err(e) = worker_tx.send((observer.clone(),
                                                        message.clone())).await {
                            error!("Failed to send message to worker: {}", e);
                        }

                        round_robin_index += 1;
                    }
                }
            }
        });

        InMemoryBus { observers, sender }
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

    // Subscribe for a message with an observer function
    fn register_observer(&self, topic: &str, observer: BoxedObserverFn<M>)
                         -> Result<()> {
        tokio::task::block_in_place(|| {
            let mut observers = self.observers.blocking_lock();
            observers.entry(topic.to_string())
                .or_insert(Vec::new())
                .push(Arc::new(observer));
            Ok(())
        })
    }

    /// Shut down, clearing all observers
    fn shutdown(&self) -> BoxFuture<'static, Result<()>> {
        let observers = self.observers.clone();

        Box::pin(async move {
            let mut observers = observers.lock().await;
            observers.clear();
            Ok(())
        })
    }
}

