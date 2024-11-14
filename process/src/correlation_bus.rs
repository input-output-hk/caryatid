//! Message bus wrapper which turns requests into individual publish/subscribes and
//! sorrelates the results
use tokio::sync::Mutex;
use std::sync::Arc;
use anyhow::{Result, anyhow};
use config::Config;
use futures::future::BoxFuture;
use caryatid_sdk::message_bus::{MessageBus, Subscriber, MessageBounds};
use tracing::{info, error};
use crate::match_topic::match_topic;
use caryatid_sdk::config::config_from_value;
use std::collections::BTreeMap;

/// Correlation bus
pub struct CorrelationBus<M: MessageBounds> {

    /// Wrapped bus
    bus: Arc<dyn MessageBus<M>>,
}

impl<M: MessageBounds> CorrelationBus<M> {

    /// Construct with config, wrapping the given bus
    pub fn new(config: &Config, bus: Arc<dyn MessageBus<M>>) -> Self {

        info!("Creating correlation bus");

        Self { bus }
    }
}

impl<M> MessageBus<M> for CorrelationBus<M>
  where M: MessageBounds {

    /// Publish a message on a given topic
    fn publish(&self, topic: &str, message: Arc<M>) -> BoxFuture<'static, Result<()>> {
        self.bus.publish(topic, message)
    }

    /// Request a response on a given topic
    fn request(&self, topic: &str, message: Arc<M>)-> BoxFuture<'static, Result<Arc<Result<M>>>> {
        self.bus.request(topic, message)
    }

    // Subscribe for a message with an subscriber function
    fn register_subscriber(&self, topic: &str, subscriber: Arc<Subscriber<M>>) -> Result<()> {
        self.bus.register_subscriber(topic, subscriber)
    }

    /// Shut down, shutting down all the buses
    fn shutdown(&self) -> BoxFuture<'static, Result<()>> {
        self.bus.shutdown()
    }
}

