//! Message super-bus which routes to other MessageBuses
use tokio::sync::Mutex;
use std::sync::Arc;
use anyhow::Result;
use config::Config;
use futures::future::BoxFuture;
use tracing::{info, error};
use caryatid_sdk::message_bus::{MessageBus, Subscriber, MessageBounds};
use caryatid_sdk::match_topic::match_topic;
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
    routes: Arc<Mutex<Vec<Arc<Route<M>>>>>,
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
        let mut routes: Vec<Arc<Route<M>>> = Vec::new();
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

                        routes.push(Arc::new(route));
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

            // Get matching routes, limiting lock duration
            let matching: Vec<_> = {
                let routes = routes.lock().await;
                routes.iter()
                    .filter(|route| match_topic(&route.pattern, &topic))
                    .map(Arc::clone)
                    .collect()
            };
            
            for route in matching {
                for bus in route.buses.iter() {
                    let _ = bus.publish(&topic, message.clone()).await;
                }
                break;  // Stop after match
            }
            Ok(())
        })
    }

    // Subscribe for a message with an subscriber function
    fn register_subscriber(&self, topic: &str, subscriber: Arc<Subscriber<M>>)
                           -> BoxFuture<'static, Result<()>> {
        let routes = self.routes.clone();
        let topic = topic.to_string();

        Box::pin(async move {

            // Get matching routes, limiting lock duration
            let matching: Vec<_> = {
                let routes = routes.lock().await;
                routes.iter()
                    .filter(|route| match_topic(&route.pattern, &topic))
                    .map(Arc::clone)
                    .collect()
            };

            for route in matching {
                for bus in route.buses.iter() {
                    let _ = bus.register_subscriber(&topic,
                                                    subscriber.clone()).await;
                }
                break;  // Stop after match
            }

            Ok(())
        })
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

// -- Tests --
#[cfg(test)]
mod tests {
    use super::*;
    use caryatid_sdk::mock_bus::MockBus;
    use config::{Config, FileFormat};
    use futures::future::ready;
    use tracing::Level;
    use tracing_subscriber;

    // Helper to set up a routing bus with 2 mock sub-buses, from given config string
    struct TestSetup<M: MessageBounds> {
        mock_foo: Arc<MockBus<M>>,
        mock_bar: Arc<MockBus<M>>,
        bus: RoutingBus<M>
    }

    impl<M: MessageBounds> TestSetup<M> {

        fn new(config_str: &str) -> Self {

            // Set up tracing
            let _ = tracing_subscriber::fmt()
                .with_max_level(Level::DEBUG)
                .with_test_writer()
                .try_init();

            // Create mock buses
            let mock_foo = Arc::new(MockBus::<M>::new());
            let mock_bar = Arc::new(MockBus::<M>::new());

            // BusInfo to pass to routing
            let mut buses: Vec<Arc<BusInfo<M>>> = Vec::new();
            buses.push(Arc::new(BusInfo {
                id: "foo".to_string(),
                bus: mock_foo.clone()
            }));

            buses.push(Arc::new(BusInfo {
                id: "bar".to_string(),
                bus: mock_bar.clone()
            }));

            // Parse config
            let config = Config::builder()
                .add_source(config::File::from_str(config_str, FileFormat::Toml))
                .build()
                .unwrap();

            // Create the bus
            let bus = RoutingBus::<M>::new(&config, Arc::new(buses));

            Self { mock_foo, mock_bar, bus }
        }
    }

    #[tokio::test]
    async fn subscribe_with_double_route_subscribes_to_both() {

        let config = r###"
[[route]]
pattern = "#"
bus = ["foo", "bar"]
"###;

        let setup = TestSetup::<String>::new(config);

        // Subscribe
        assert!(setup.bus.register_subscriber("test",
                                              Arc::new(|_topic: &str, _message: Arc<String>| {
                                                  Box::pin(ready(()))
                                              }))
                .await
                .is_ok());

        // Check foo got it
        let foo_subscribes = setup.mock_foo.subscribes.lock().await;
        assert_eq!(foo_subscribes.len(), 1);
        let foo_0 = &foo_subscribes[0];
        assert_eq!(foo_0.topic, "test");

        // Check bar got it
        let bar_subscribes = setup.mock_bar.subscribes.lock().await;
        assert_eq!(bar_subscribes.len(), 1);
        let bar_0 = &bar_subscribes[0];
        assert_eq!(bar_0.topic, "test");
    }

    #[tokio::test]
    async fn subscribe_with_single_route_subscribes_to_only_one() {

        let config = r###"
[[route]]
pattern = "#"
bus = "foo"
"###;

        let setup = TestSetup::<String>::new(config);

        // Subscribe
        assert!(setup.bus.register_subscriber("test",
                                              Arc::new(|_topic: &str, _message: Arc<String>| {
                                                  Box::pin(ready(()))
                                              }))
                .await
                .is_ok());

        // Check foo got it
        let foo_subscribes = setup.mock_foo.subscribes.lock().await;
        assert_eq!(foo_subscribes.len(), 1);
        let foo_0 = &foo_subscribes[0];
        assert_eq!(foo_0.topic, "test");

        // Check bar didn't get it
        let bar_subscribes = setup.mock_bar.subscribes.lock().await;
        assert_eq!(bar_subscribes.len(), 0);
    }

    #[tokio::test]
    async fn publish_with_double_route_sends_to_both() {

        let config = r###"
[[route]]
pattern = "#"
bus = ["foo", "bar"]
"###;

        let setup = TestSetup::<String>::new(config);

        // Send a message
        assert!(setup.bus.publish("test", Arc::new("Hello, world!".to_string())).await.is_ok());

        // Check foo got it
        let foo_publishes = setup.mock_foo.publishes.lock().await;
        assert_eq!(foo_publishes.len(), 1);
        let foo_0 = &foo_publishes[0];
        assert_eq!(foo_0.topic, "test");
        assert_eq!(foo_0.message.as_ref(), "Hello, world!");

        // Check bar got it
        let bar_publishes = setup.mock_bar.publishes.lock().await;
        assert_eq!(bar_publishes.len(), 1);
        let bar_0 = &bar_publishes[0];
        assert_eq!(bar_0.topic, "test");
        assert_eq!(bar_0.message.as_ref(), "Hello, world!");
    }

    #[tokio::test]
    async fn publish_with_single_route_sends_to_only_one() {

        let config = r###"
[[route]]
pattern = "test"
bus = "foo"

[[route]]
pattern = "#"
bus = ["foo", "bar"]
"###;

        let setup = TestSetup::<String>::new(config);

        // Send a message
        assert!(setup.bus.publish("test", Arc::new("Hello, world!".to_string())).await.is_ok());

        // Check foo got it
        let foo_publishes = setup.mock_foo.publishes.lock().await;
        assert_eq!(foo_publishes.len(), 1);
        let foo_0 = &foo_publishes[0];
        assert_eq!(foo_0.topic, "test");
        assert_eq!(foo_0.message.as_ref(), "Hello, world!");

        // Check bar didn't get it
        let bar_publishes = setup.mock_bar.publishes.lock().await;
        assert_eq!(bar_publishes.len(), 0);
    }

    #[tokio::test]
    async fn shutdown_is_passed_to_both_sub_buses() {
        let setup = TestSetup::<String>::new("");

        // Shut it down
        assert!(setup.bus.shutdown().await.is_ok());

        // Check foo got it
        assert_eq!(*setup.mock_foo.shutdowns.lock().await, 1);

        // Check bar got it
        assert_eq!(*setup.mock_bar.shutdowns.lock().await, 1);
    }
}
