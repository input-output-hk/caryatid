//! Mock message bus for tests
use crate::match_topic::match_topic;
use crate::message_bus::{MessageBounds, MessageBus, Subscription, SubscriptionBounds};
use anyhow::Result;
use futures::future::BoxFuture;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};
use tracing::debug;

pub struct PublishRecord<M: MessageBounds> {
    pub topic: String,
    pub message: Arc<M>,
}

#[derive(Clone)]
pub struct MockSubscription<M> {
    messages: Arc<Mutex<VecDeque<(String, Arc<M>)>>>,
    notify: Arc<Notify>,
}

pub struct SubscriptionRecord<M> {
    pub topic: String,
    pub subscription: MockSubscription<M>,
}

impl<M: MessageBounds> SubscriptionBounds for MockSubscription<M> {}

impl<M> MockSubscription<M> {
    pub fn new() -> Self {
        Self {
            messages: Arc::new(Mutex::new(VecDeque::new())),
            notify: Arc::new(Notify::new()),
        }
    }

    pub async fn push(&self, topic: &str, message: Arc<M>) {
        self.messages
            .lock()
            .await
            .push_back((topic.to_string(), message));
        self.notify.notify_one();
    }
}

impl<M: MessageBounds> Subscription<M> for MockSubscription<M> {
    fn read(&mut self) -> BoxFuture<anyhow::Result<(String, Arc<M>)>> {
        Box::pin(async move {
            loop {
                self.notify.notified().await;
                if let Some(entry) = self.messages.lock().await.pop_front() {
                    return Ok(entry);
                }
            }
        })
    }
}

pub struct MockBus<M: MessageBounds> {
    pub publishes: Arc<Mutex<Vec<PublishRecord<M>>>>,
    pub subscriptions: Arc<Mutex<Vec<SubscriptionRecord<M>>>>,
    pub shutdowns: Arc<Mutex<u16>>, // just count them
}

impl<M: MessageBounds> MockBus<M> {
    pub fn new() -> Self {
        Self {
            publishes: Arc::new(Mutex::new(Vec::new())),
            subscriptions: Arc::new(Mutex::new(Vec::new())),
            shutdowns: Arc::new(Mutex::new(0)),
        }
    }
}

impl<M> MessageBus<M> for MockBus<M>
where
    M: MessageBounds + serde::Serialize + serde::de::DeserializeOwned,
{
    fn publish(&self, topic: &str, message: Arc<M>) -> BoxFuture<'static, Result<()>> {
        debug!("Mock publish on {topic}");
        let publishes = self.publishes.clone();
        let subscriptions = self.subscriptions.clone();
        let topic = topic.to_string();

        Box::pin(async move {
            // Limit lock because we can be re-entrant
            {
                let mut publishes = publishes.lock().await;
                publishes.push(PublishRecord {
                    topic: topic.clone(),
                    message: message.clone(),
                });
            }

            // Send to all subscriptions if topic matches
            // Copy relevant subscriptions before calling them to avoid deadlock if a subscription
            // does another publish (as handle() does)
            let mut relevant_subscriptions: Vec<MockSubscription<M>> = Vec::new();
            {
                let subscriptions = subscriptions.lock().await;
                for sub in subscriptions.iter() {
                    if match_topic(&sub.topic, &topic) {
                        relevant_subscriptions.push(sub.subscription.clone())
                    }
                }
            }

            for sub in relevant_subscriptions.iter() {
                sub.push(&topic, message.clone()).await;
            }

            Ok(())
        })
    }

    fn register(&self, topic: &str) -> BoxFuture<Result<Box<dyn Subscription<M>>>> {
        let topic = topic.to_string();
        Box::pin(async move {
            debug!("Mock subscribe on {topic}");
            let mut subscriptions = self.subscriptions.lock().await;
            let subscription = MockSubscription::new();
            subscriptions.push(SubscriptionRecord {
                topic: topic.to_string(),
                subscription: subscription.clone(),
            });
            Ok(Box::new(subscription) as Box<dyn Subscription<M>>)
        })
    }

    fn shutdown(&self) -> BoxFuture<'static, Result<()>> {
        let shutdowns = self.shutdowns.clone();
        Box::pin(async move {
            let mut shutdowns = shutdowns.lock().await;
            *shutdowns += 1;
            Ok(())
        })
    }
}
