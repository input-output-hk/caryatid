//! Caraytid performance test - subscriber side
use crate::message::Message;
use anyhow::Result;
use caryatid_sdk::{module, Context, Module};
use config::Config;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::Instant;
use tracing::info;

/// Performance test subscriber
#[module(
    message_type(Message),
    name = "subscriber",
    description = "Performance test subscriber"
)]
pub struct Subscriber;

struct Stats {
    first_message_time: Instant,
    last_message_time: Instant,
    count: u64,
}

impl Subscriber {
    async fn init(&self, context: Arc<Context<Message>>, config: Arc<Config>) -> Result<()> {
        // Get configuration
        let topic = config.get_string("topic").unwrap_or("test".to_string());
        info!("Creating simple subscriber on '{}'", topic);

        let stats = Arc::new(Mutex::new(Stats {
            first_message_time: Instant::now(),
            last_message_time: Instant::now(),
            count: 0,
        }));

        // Register a subscriber
        let mut subscription = context.subscribe(&topic).await?;
        context.run(async move {
            loop {
                let Ok((_, message)) = subscription.read().await else {
                    return;
                };
                match message.as_ref() {
                    Message::Stop(_) => {
                        let stats = stats.lock().await;
                        let elapsed = stats
                            .last_message_time
                            .duration_since(stats.first_message_time)
                            .as_secs_f64();

                        info!("Elapsed time: {:.2}s", elapsed);
                        info!("Count: {}", stats.count);
                        let fcount = stats.count as f64;
                        info!("Average time: {:.2}us", elapsed / fcount * 1e6);
                        if elapsed > 0.0 {
                            info!("Rate: {}/sec", (fcount / elapsed).round() as u64);
                        }
                    }
                    _ => {
                        let now = Instant::now();
                        let mut stats = stats.lock().await;
                        stats.last_message_time = now;
                        if stats.count == 0 {
                            stats.first_message_time = now;
                        }
                        stats.count += 1;
                    }
                }
            }
        });

        Ok(())
    }
}
