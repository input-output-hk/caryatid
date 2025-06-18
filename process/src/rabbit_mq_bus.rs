//! MessageBus implementation for RabbitMQ
use anyhow::{Context, Result};
use async_trait::async_trait;
use caryatid_sdk::message_bus::{MessageBounds, MessageBus, Subscription, SubscriptionBounds};
use config::Config;
use futures::future::BoxFuture;
use futures::StreamExt;
use lapin::{
    options::{
        BasicConsumeOptions, BasicPublishOptions, ExchangeDeclareOptions, QueueBindOptions,
        QueueDeclareOptions,
    },
    types::FieldTable,
    BasicProperties, Channel, Connection, ConnectionProperties, Consumer,
};
use std::marker::PhantomData;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::Duration;
use tracing::{error, info};

const DEFAULT_REQUEST_TIMEOUT: u64 = 5;

struct RabbitMQSubscription<M> {
    consumer: Consumer,
    _phantom: PhantomData<M>,
}

impl<M: MessageBounds> SubscriptionBounds for RabbitMQSubscription<M> {}

impl<M: MessageBounds> Subscription<M> for RabbitMQSubscription<M> {
    fn read(&mut self) -> BoxFuture<anyhow::Result<(String, Arc<M>)>> {
        Box::pin(async move {
            loop {
                if let Some(delivery) = self.consumer.next().await {
                    let delivery = delivery.with_context(|| "Error in consumer")?;

                    // Acknowledge the message anyway, otherwise it stays around
                    // forever
                    delivery
                        .ack(lapin::options::BasicAckOptions::default())
                        .await
                        .with_context(|| "Failed to acknowledge message")?;

                    // Decode it
                    match serde_cbor::de::from_slice::<M>(&delivery.data) {
                        Ok(message) => {
                            // Call the subscriber function with the message
                            return Ok((delivery.routing_key.to_string(), Arc::new(message)));
                        }
                        Err(e) => error!("Invalid CBOR message received: {}", e),
                    }
                }
            }
        })
    }
}

/// RabbitMQ message bus implementation
pub struct RabbitMQBus<M: MessageBounds> {
    connection: Arc<Mutex<Connection>>, // RabbitMQ connection
    channel: Arc<Mutex<Channel>>,       // RabbitMQ outgoing channel
    exchange: String,                   // Exchange name
    _phantom: PhantomData<M>,           // Required to associate with <M> (eww)
    request_timeout: Duration,
}

impl<M: MessageBounds> RabbitMQBus<M> {
    // New
    pub async fn new(config: &Config) -> Result<Self> {
        // Connect to RabbitMQ server
        let url = config
            .get_string("url")
            .unwrap_or("amqp://127.0.0.1:5672/%2f".to_string());
        info!("Connecting to RabbitMQ at {}", url);

        let props =
            ConnectionProperties::default().with_executor(tokio_executor_trait::Tokio::current());
        let connection = Connection::connect(&url, props)
            .await
            .with_context(|| "Can't create RabbitMQ connection")?;

        info!("RabbitMQ connected");

        // Get exchange name
        let exchange_name = config
            .get_string("exchange")
            .unwrap_or("caryatid".to_string());

        // Create a channel for outgoing messages
        let channel = connection
            .create_channel()
            .await
            .with_context(|| "Can't create outgoing channel")?;

        // Declare the topic exchange
        channel
            .exchange_declare(
                &exchange_name,
                lapin::ExchangeKind::Topic,
                ExchangeDeclareOptions::default(),
                FieldTable::default(),
            )
            .await
            .with_context(|| "Failed to declare exchange")?;

        let timeout = config
            .get::<u64>("request-timeout")
            .unwrap_or(DEFAULT_REQUEST_TIMEOUT);
        let timeout = Duration::from_secs(timeout);

        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
            channel: Arc::new(Mutex::new(channel)),
            exchange: exchange_name,
            request_timeout: timeout,
            _phantom: PhantomData,
        })
    }
}

/// Implement MessageBus trait
#[async_trait]
impl<M: MessageBounds + serde::Serialize + serde::de::DeserializeOwned> MessageBus<M>
    for RabbitMQBus<M>
{
    /// Publish a message on a topic
    async fn publish(&self, topic: &str, message: Arc<M>) -> Result<()> {
        let topic = topic.to_string();
        let channel = self.channel.lock().await;

        // Serialise the message
        let payload = serde_cbor::ser::to_vec(&message)?;

        // Publish the message to the queue
        channel
            .basic_publish(
                &self.exchange,
                &topic,
                BasicPublishOptions::default(),
                &payload,
                BasicProperties::default(),
            )
            .await?
            .await?;

        Ok(())
    }

    // Request timeout
    fn request_timeout(&self) -> Duration {
        self.request_timeout
    }

    // Subscribe to a topic
    async fn subscribe(&self, topic: &str) -> Result<Box<dyn Subscription<M>>> {
        // Clone over async boundary
        let connection = self.connection.clone();
        let topic = topic.to_string();
        let exchange = self.exchange.clone();

        // Create a new channel for this subscriber
        let channel = connection
            .lock()
            .await
            .create_channel()
            .await
            .with_context(|| "Failed to create channel")?;

        // Declare the queue
        let queue = channel
            .queue_declare(
                &topic,
                QueueDeclareOptions::default(),
                FieldTable::default(),
            )
            .await
            .with_context(|| "Failed to declare queue")?;

        // Bind the queue to the exchange with the specified pattern
        channel
            .queue_bind(
                queue.name().as_str(),
                &exchange,
                &topic,
                QueueBindOptions::default(),
                FieldTable::default(),
            )
            .await
            .with_context(|| "Failed to bind queue")?;

        // Start consuming messages from the queue
        let consumer = channel
            .basic_consume(
                queue.name().as_str(),
                "",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await
            .with_context(|| "Failed to start consumer")?;

        Ok(Box::new(RabbitMQSubscription {
            consumer,
            _phantom: PhantomData,
        }) as Box<dyn Subscription<M>>)
    }

    /// Shut down the bus connection
    async fn shutdown(&self) -> Result<()> {
        info!("Shutting down RabbitMQ interface");

        // Close the connection
        let connection = self.connection.lock().await;
        connection.close(200, "Goodbye").await?;
        Ok(())
    }
}
