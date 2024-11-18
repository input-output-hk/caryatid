//! Message super-bus which routes to other MessageBuses
use tokio::sync::Mutex;
use std::sync::Arc;
use anyhow::Result;
use config::Config;
use futures::future::BoxFuture;
use caryatid_sdk::message_bus::{MessageBus, Subscriber, MessageBounds};
use tracing::{info, error};
use crate::match_topic::match_topic;
use caryatid_sdk::config::config_from_value;
use std::collections::BTreeMap;

struct Route<M: MessageBounds> {
    pattern: String,
    buses: Vec<Arc<dyn MessageBus<M>>>,
}

/// Message bus with ID
pub struct BusInfo<M: MessageBounds> {
    pub id: String,
    pub bus: Arc<dyn MessageBus<M>>,
}

/// Routing super-bus
pub struct RoutingBus<M: MessageBounds> {

    /// Buses
    buses: Arc<Mutex<BTreeMap<String, Arc<dyn MessageBus<M>>>>>,

    /// Routes
    routes: Arc<Mutex<Vec<Route<M>>>>,
}

impl<M: MessageBounds> RoutingBus<M> {
    pub fn new(config: &Config,
               bus_infos: Arc<Vec<Arc<BusInfo<M>>>>) -> Self {

        info!("Creating routing bus:");

        // Create buses map
        let mut buses: BTreeMap<String, Arc<dyn MessageBus<M>>> = BTreeMap::new();
        for bus_info in bus_infos.iter() {
            info!(" - Bus {}", bus_info.id);
            buses.insert(bus_info.id.clone(), bus_info.bus.clone());
        }

        // Create routes
        let mut routes: Vec<Route<M>> = Vec::new();
        if let Ok(rconfs) = config.get_array("route") {
            for rconf in rconfs {
                if let Ok(rt) = rconf.into_table() {
                    let rtc = config_from_value(rt);
                    if let Ok(pattern) = rtc.get_string("pattern") {

                        info!(" - Route {pattern} to: ");
                        let mut route = Route { pattern, buses: Vec::new() };

                        if let Ok(bus_id) = rtc.get_string("bus") {

                            // Single bus
                            if let Some(bus) = buses.get(&bus_id) {
                                info!("   - {bus_id}");
                                route.buses.push(bus.clone());
                            } else {
                                error!("No such bus id {bus_id}");
                            }

                        } else if let Ok(bus_id_vs) = rtc.get_array("bus") {

                            // Multiple buses
                            for bus_id_v in bus_id_vs {
                                if let Ok(bus_id) = bus_id_v.into_string() {
                                    if let Some(bus) = buses.get(&bus_id) {
                                        info!("   - {bus_id}");
                                        route.buses.push(bus.clone());
                                    } else {
                                        error!("No such bus id {bus_id}");
                                    }
                                }
                            }
                        }

                        routes.push(route);
                    }
                }
            }
        }

        Self {
            buses: Arc::new(Mutex::new(buses)),
            routes: Arc::new(Mutex::new(routes))
        }
    }
}

impl<M> MessageBus<M> for RoutingBus<M>
where M: MessageBounds + serde::Serialize + serde::de::DeserializeOwned {

    /// Publish a message on a given topic
    fn publish(&self, topic: &str, message: Arc<M>) -> BoxFuture<'static, Result<()>> {

        let routes = self.routes.clone();
        let topic = topic.to_string();

        Box::pin(async move {

            let routes = routes.lock().await;
            let topic = topic.clone();

            for route in routes.iter() {
                // Check for topic match
                if match_topic(&route.pattern, &topic) {
                    for bus in route.buses.iter() {
                        let _ = bus.publish(&topic, message.clone()).await;
                    }
                    break;  // Stop after match
                }
            }
            Ok(())
        })
    }

    // Subscribe for a message with an subscriber function
    fn register_subscriber(&self, topic: &str, subscriber: Arc<Subscriber<M>>) -> Result<()> {
        let topic = topic.to_string();
        let routes = self.routes.clone();

        tokio::spawn(async move {
            let routes = routes.lock().await;
            let topic = topic.clone();
            let subscriber = subscriber.clone();

            for route in routes.iter() {
                // Check for topic match
                if match_topic(&route.pattern, &topic) {
                    for bus in route.buses.iter() {
                        let _ = bus.register_subscriber(&topic,
                                                        subscriber.clone());
                    }
                    break;  // Stop after match
                }
            }
        });

        Ok(())
    }

    /// Shut down, shutting down all the buses
    fn shutdown(&self) -> BoxFuture<'static, Result<()>> {
        let buses = self.buses.clone();

        Box::pin(async move {
            let buses = buses.lock().await;
            for (_, bus) in buses.iter() {
                let _ = bus.shutdown().await;
            }

            Ok(())
        })
    }
}

