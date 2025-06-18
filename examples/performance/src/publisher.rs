//! Caryatid performance test - publisher side
use crate::message::Message;
use anyhow::Result;
use arcstr::ArcStr;
use caryatid_sdk::{module, Context, Module};
use config::Config;
use futures::future::join_all;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::info;

/// Performance publisher module
#[module(
    message_type(Message),
    name = "publisher",
    description = "Performance test publisher module"
)]
pub struct Publisher;

const DEFAULT_COUNT: i64 = 1000_000;
const DEFAULT_THREADS: i64 = 1;
const DEFAULT_LENGTH: i64 = 100;

impl Publisher {
    async fn init(&self, context: Arc<Context<Message>>, config: Arc<Config>) -> Result<()> {
        // Get configuration
        let topic = config.get_string("topic").unwrap_or("test".to_string());
        let threads = config.get_int("threads").unwrap_or(DEFAULT_THREADS);
        let count = config.get_int("count").unwrap_or(DEFAULT_COUNT);
        let length = config.get_int("length").unwrap_or(DEFAULT_LENGTH);

        info!("Creating performance-testing publisher on '{topic}'");
        info!(" - with {threads} threads");
        info!(" - publishing {count} messages with {length} bytes of data");

        let data = "*".repeat(length.try_into().unwrap());

        let message_bus = context.message_bus.clone();
        tokio::spawn(async move {
            let mut handles = Vec::new();

            for thread in 1..=threads {
                let message_bus = message_bus.clone();
                let topic = topic.clone();
                let arc_str = ArcStr::from(data.clone());
                info!("Starting thread {thread}");

                handles.push(context.run(async move {
                    for _ in 1..=count {
                        // Avoid cloning the string and skewing the result
                        let message = Arc::new(Message::Data(arc_str.clone()));
                        message_bus
                            .publish(&topic, message)
                            .await
                            .expect("Failed to publish message");
                    }
                }));
            }

            // Send the stop signal after they've all finished
            join_all(handles).await;

            // Let the subscriber catch up
            sleep(Duration::from_secs(1)).await;

            // Send a stop
            let message = Arc::new(Message::Stop(()));
            message_bus
                .publish(&topic, message)
                .await
                .expect("Failed to publish message");
        });

        Ok(())
    }
}
