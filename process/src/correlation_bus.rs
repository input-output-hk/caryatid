//! Message bus wrapper which turns requests into individual publish/subscribes and
//! correlates the results
use tokio::sync::{Mutex, oneshot};
use std::sync::Arc;
use tokio::sync::oneshot::Sender;
use anyhow::{Result, anyhow};
use config::Config;
use futures::future::{BoxFuture, ready};
use caryatid_sdk::message_bus::{MessageBus, Subscriber, MessageBounds};
use tracing::{debug, info, error};
use std::collections::{HashSet, HashMap};
use rand::Rng;
use tokio::time::{timeout, Duration};

const DEFAULT_TIMEOUT: u64 = 5;

/// Wrapper for a message with a oneshot to send back a result
struct Request<M: MessageBounds> {
    notify: Sender<Result<Arc<M>>>,
}

/// Correlation bus
pub struct CorrelationBus<M: MessageBounds> {

    /// Wrapped bus
    bus: Arc<dyn MessageBus<M>>,

    /// Record of response subscriptions, by response topic
    response_subscribed: Arc<Mutex<HashSet<String>>>,

    /// Active requests, by ID
    requests: Arc<Mutex<HashMap<String, Request<M>>>>,

    /// Timeout
    timeout: Duration,
}

impl<M: MessageBounds> CorrelationBus<M> {

    /// Construct with config, wrapping the given bus
    pub fn new(config: &Config, bus: Arc<dyn MessageBus<M>>) -> Self {

        info!("Creating correlation bus");

        let timeout = config.get::<u64>("timeout").unwrap_or(DEFAULT_TIMEOUT);
        let timeout = Duration::from_secs(timeout);

        Self {
            bus,
            response_subscribed: Arc::new(Mutex::new(HashSet::new())),
            requests: Arc::new(Mutex::new(HashMap::new())),
            timeout
        }
    }
}

impl<M> MessageBus<M> for CorrelationBus<M>
  where M: MessageBounds {

    /// Publish a message on a given topic
    fn publish(&self, topic: &str, message: Arc<M>) -> BoxFuture<'static, Result<()>> {
        // Pass straight through
        self.bus.publish(topic, message)
    }

    /// Request a response on a given topic
    fn request(&self, topic: &str, message: Arc<M>)-> BoxFuture<'static, Result<Arc<M>>> {

        let response_subscribed = self.response_subscribed.clone();
        let requests = self.requests.clone();
        let bus = self.bus.clone();
        let topic = topic.to_string();
        let req_timeout = self.timeout;

        // Generate a 64-bit request ID
        let mut rng = rand::thread_rng();
        let random_bytes: [u8; 8] = rng.gen();
        let request_id = hex::encode(random_bytes);

        Box::pin(async move {

            let request_topic = format!("{topic}.{request_id}");
            let response_pattern = format!("{topic}.*.response");

            // Have we already subscribed?
            let mut response_subscribed = response_subscribed.lock().await;
            if !response_subscribed.contains(&topic) {

                // Remember we've done it
                response_subscribed.insert(topic.clone());

                let requests = requests.clone();

                // Subscribe to all responses matching the response_topic
                let _ = bus.register_subscriber(
                    &response_pattern,
                    Arc::new(move |response_topic: &str, response_message: Arc<M>| {

                        debug!("Correlator received response on {response_topic}");
                        let response_topic = response_topic.to_owned();

                        // Check it matches the request topic
                        if response_topic.starts_with(&topic) {
                            let suffix = &response_topic[topic.len()..];
                            if suffix.starts_with('.') && suffix.ends_with(".response") {
                                let response_id = &suffix[1..suffix.len()-9];
                                let requests = requests.clone();
                                let response_id = response_id.to_owned();

                                tokio::spawn(async move {
                                    let mut requests = requests.lock().await;
                                    if let Some(request) = requests.remove(&response_id) {
                                        let _ = request.notify.send(Ok(response_message.clone()));
                                    } else {
                                        error!("Unrecognised response ID in {response_topic}");
                                    }
                                });
                            }
                            else {
                                error!("No response ID found in {response_topic}");
                            }
                        } else {
                            error!("Response topic {response_topic} doesn't match topic {topic}");
                        }

                        Box::pin(ready(()))
                    })
                ).await;
            }

            // Record in-flight requests with a OneShot to recover the result
            let (notify_sender, notify_receiver) = oneshot::channel();
            let request = Request { notify: notify_sender };

            { // Hold lock only for insert, otherwise deadlock!
                let mut requests = requests.lock().await;
                requests.insert(request_id, request);
            }

            // Just publish the message
            let _ = bus.publish(&request_topic, message).await;

            // Get the result back
            match timeout(req_timeout, notify_receiver).await {
                Ok(result) => match result {
                    Ok(res) => res,
                    Err(e) => Err(anyhow!("Notify receive failed: {e}"))
                }
                Err(_) => Err(anyhow!("Request timed out"))
            }
        })
    }

    // Subscribe for a message with an subscriber function
    fn register_subscriber(&self, topic: &str, subscriber: Arc<Subscriber<M>>)
                                                      -> BoxFuture<'static, Result<()>> {
        self.bus.register_subscriber(topic, subscriber)
    }

    /// Shut down, shutting down all the buses
    fn shutdown(&self) -> BoxFuture<'static, Result<()>> {
        self.bus.shutdown()
    }
}

