// Shared context passed to each module

use std::sync::Arc;
use anyhow::Result;
use config::Config;
use crate::message_bus::{MessageBounds, MessageBus};
use std::fmt;
use std::future::Future;
use tokio::sync::watch::Sender;
use tokio::task;
use tracing::error;
use crate::Subscription;

pub struct Context<M: MessageBounds> {
    pub config: Arc<Config>,
    pub message_bus: Arc<dyn MessageBus<M>>,
    pub startup_watch: Sender<bool>,
}

impl<M: MessageBounds> Context<M> {
    pub fn new(
        config: Arc<Config>,
        message_bus: Arc<dyn MessageBus<M>>,
        startup_watch: Sender<bool>,
    ) -> Self {
        Self { config, message_bus, startup_watch }
    }

    pub fn run<T, F>(&self, func: F) -> task::JoinHandle<T>
    where
        T: Send + 'static,
        F: Future<Output = T> + Send + 'static,
    {
        let mut signal = self.startup_watch.subscribe();
        tokio::spawn(async move {
            loop {
                let _ = signal.changed().await;
                if *signal.borrow_and_update() {
                    break;
                }
            }
            func.await
        })
    }

    pub async fn publish(&self, topic: &str, message: Arc<M>) -> Result<()> {
        self.message_bus.publish(topic, message).await
    }

    pub async fn request(&self, topic: &str, message: Arc<M>) -> Result<Arc<M>> {
        self.message_bus.request(topic, message).await
    }

    pub async fn subscribe(&self, topic: &str) -> Result<Box<dyn Subscription<M>>> {
        self.message_bus.subscribe(topic).await
    }

    pub fn handle<F, Fut>(&self, topic: &str, handler: F) -> task::JoinHandle<()>
    where
        F: Fn(Arc<M>) -> Fut + Send + 'static,
        Fut: Future<Output = Arc<M>> + Send + 'static,
    {
        let topic = topic.to_string();
        let message_bus = self.message_bus.clone();
        self.run(async move {
            let request_topic = format!("{topic}.#.request");
            let Ok(mut subscription) = message_bus.subscribe(&request_topic).await else { return; };
            loop {
                let Ok((topic, message)) = subscription.read().await else { return; };
                let message = handler(message).await;
                match topic.strip_suffix(".request") {
                    Some(topic) => {
                        let response_topic = format!("{topic}.response");
                        let _ = message_bus.publish(&response_topic, message).await;
                    },
                    None => {
                        error!("Badly formed response topic {}", topic);
                    },
                }
            }
        })
    }
}

/// Minimal implementation of Debug for tracing
impl<M: MessageBounds> fmt::Debug for Context<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Context")
            .finish()
    }
}

// -- Tests --
#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock_bus::MockBus;
    use config::{Config, FileFormat};
    use futures::future::ready;
    use tracing::{info, Level};
    use tracing_subscriber;

    // Helper to set up a correlation bus with a mock sub-bus, from given config string
    struct TestSetup<M: MessageBounds> {
        context: Arc<Context<M>>,
    }

    impl<M: MessageBounds> TestSetup<M> {

        fn new(config_str: &str) -> Self {

            // Set up tracing
            let _ = tracing_subscriber::fmt()
                .with_max_level(Level::DEBUG)
                .with_test_writer()
                .try_init();

            // Create context
            let config = Config::builder()
                .add_source(config::File::from_str(config_str, FileFormat::Toml))
                .build()
                .unwrap();

            // Create mock bus
            let mock = Arc::new(MockBus::<M>::new(&config));

            // Create context
            let context = Arc::new(Context::<M>::new(Arc::new(config), mock, Sender::new(false)));

            Self { context }
        }
    }

    #[tokio::test]
    async fn request_succeeds_with_response() {
        let setup = TestSetup::<String>::new("request-timeout = 1");

        // Register a handler on test
        setup.context.handle("test", |message: Arc<String>| {
            info!("Subscriber got request {message}");
            ready(Arc::new("Nice to meet you".to_string()))
        });

        let _ = setup.context.startup_watch.send(true);

        // Wait for responder to be ready
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Request with no response should timeout
        assert!(setup.context.request("test", Arc::new("Hello, world!".to_string()))
                .await
                .is_ok());
    }
}
