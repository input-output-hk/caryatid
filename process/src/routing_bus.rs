//! Message super-bus which routes to other MessageBuses
use tokio::sync::Mutex;
use std::sync::Arc;
use anyhow::Result;
use config::Config;
use futures::future::{BoxFuture, ready};
use caryatid_sdk::message_bus::{MessageBus, Subscriber, MessageBounds};
use tracing::info;
use crate::InMemoryBus;
use crate::RabbitMQBus;
use crate::match_topic::match_topic;

struct Route<M: MessageBounds> {
    pattern: String,
    in_memory_bus: Option<Arc<InMemoryBus<M>>>,
    rabbit_mq_bus: Option<Arc<RabbitMQBus<M>>>
}

/// Routing super-bus
pub struct RoutingBus<M: MessageBounds> {

    /// Routes
    routes: Arc<Mutex<Vec<Route<M>>>>,
}

impl<M: MessageBounds> RoutingBus<M> {
    pub fn new(config: &Config,
               in_memory_bus: Arc<InMemoryBus<M>>,
               rabbit_mq_bus: Arc<RabbitMQBus<M>>) -> Self {

        info!("Creating routing bus:");

        let mut routes: Vec<Route<M>> = Vec::new();

        if let Ok(rconfs) = config.get_array("route") {
            for rconf in rconfs {
                if let Ok(rt) = rconf.into_table() {
                    if let Some(pattern) = rt.get("pattern")
                        .and_then(|v| v.clone().into_string().ok()) {
                        let mut route = Route {
                            pattern: pattern.clone(),
                            in_memory_bus: None,
                            rabbit_mq_bus: None
                        };

                        info!(" - Route {pattern} to:");
                        if let Some(in_memory) = rt.get("in-memory")
                            .and_then(|v| v.clone().into_bool().ok()) {
                                if in_memory {
                                    route.in_memory_bus =
                                        Some(in_memory_bus.clone());
                                    info!("   - in-memory");
                                }
                            }
                        if let Some(rabbit_mq) = rt.get("rabbit-mq")
                            .and_then(|v| v.clone().into_bool().ok()) {
                                if rabbit_mq {
                                    route.rabbit_mq_bus =
                                        Some(rabbit_mq_bus.clone());
                                    info!("   - rabbit-mq");
                                }
                            }

                        routes.push(route);
                    }
                }
            }
        }

        RoutingBus { routes: Arc::new(Mutex::new(routes)) }
    }

}

impl<M> MessageBus<M> for RoutingBus<M>
where M: MessageBounds + serde::Serialize + serde::de::DeserializeOwned {

    /// Publish a message on a given topic
    fn publish(&self, topic: &str, message: Arc<M>)
               -> BoxFuture<'static, Result<()>> {

        let routes = self.routes.clone();
        let topic = topic.to_string();

        Box::pin(async move {

            let routes = routes.lock().await;
            let topic = topic.clone();

            for route in routes.iter() {
                // Check for topic match
                if match_topic(&route.pattern, &topic) {
                    if let Some(in_memory_bus) = &route.in_memory_bus {
                        let _ = in_memory_bus.publish(&topic, message.clone())
                            .await;
                    }
                    if let Some(rabbit_mq_bus) = &route.rabbit_mq_bus {
                        let _ = rabbit_mq_bus.publish(&topic, message.clone())
                            .await;
                    }

                    break;  // Stop after match
                }
            }
            Ok(())
        })
    }

    // Subscribe for a message with an subscriber function
    fn register_subscriber(&self, topic: &str, subscriber: Arc<Subscriber<M>>)
                         -> Result<()> {
        let topic = topic.to_string();
        let routes = self.routes.clone();

        tokio::spawn(async move {
            let routes = routes.lock().await;
            let topic = topic.clone();
            let subscriber = subscriber.clone();

            for route in routes.iter() {
                // Check for topic match
                if match_topic(&route.pattern, &topic) {
                    if let Some(in_memory_bus) = &route.in_memory_bus {
                        let _ = in_memory_bus.register_subscriber(&topic,
                                                         subscriber.clone());
                    }
                    if let Some(rabbit_mq_bus) = &route.rabbit_mq_bus {
                        let _ = rabbit_mq_bus.register_subscriber(&topic,
                                                         subscriber.clone());
                    }

                    break;  // Stop after match
                }
            }
        });

        Ok(())
    }

    /// Shut down, clearing all subscribers
    fn shutdown(&self) -> BoxFuture<'static, Result<()>> {
        Box::pin(ready(Ok(())))
    }
}

