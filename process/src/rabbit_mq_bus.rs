//! MessageBus implementation for RabbitMQ
use lapin::{
    options::{BasicConsumeOptions, BasicPublishOptions, ExchangeDeclareOptions,
              QueueBindOptions, QueueDeclareOptions},
    types::FieldTable,
    BasicProperties, Channel, Connection, ConnectionProperties,
};
use futures::StreamExt;
use anyhow::{Result, Context};
use config::Config;
use std::sync::Arc;
use tokio::sync::Mutex;
use futures::future::BoxFuture;
use caryatid_sdk::message_bus::{MessageBus, Subscriber, MessageBounds};
use std::marker::PhantomData;
use tracing::{info, error};

/// RabbitMQ message bus implementation
pub struct RabbitMQBus<M: MessageBounds> {
    connection: Arc<Mutex<Connection>>,  // RabbitMQ connection
    channel: Arc<Mutex<Channel>>,        // RabbitMQ outgoing channel
    exchange: String,                    // Exchange name
    _phantom: PhantomData<M>,            // Required to associate with <M> (eww)
}

impl<M: MessageBounds> RabbitMQBus<M> {

    // New
    pub async fn new(config: &Config) -> Result<Self> {
        // Connect to RabbitMQ server
        let url = config.get_string("url")
            .unwrap_or("amqp://127.0.0.1:5672/%2f".to_string());
        info!("Connecting to RabbitMQ at {}", url);

        let connection = Connection::connect(&url, ConnectionProperties::default())
            .await
            .with_context(|| "Can't create RabbitMQ connection")?;

        info!("RabbitMQ connected");

        // Get exchange name
        let exchange_name = config.get_string("exchange")
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

        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
            channel: Arc::new(Mutex::new(channel)),
            exchange: exchange_name,
            _phantom: PhantomData,
        })
    }
}

/// Implement MessageBus trait
impl<M: MessageBounds + serde::Serialize + serde::de::DeserializeOwned>
    MessageBus<M> for RabbitMQBus<M>
{
    /// Publish a message on a topic
    fn publish(&self, topic: &str, message: Arc<M>)
               -> BoxFuture<'static, Result<()>> {
        let channel = self.channel.clone();
        let message = Arc::clone(&message);
        let topic = topic.to_string();
        let exchange = self.exchange.clone();

        Box::pin(async move {
            let channel = channel.lock().await;

            // Serialise the message
            let payload = serde_cbor::ser::to_vec(&*message)?;

            // Publish the message to the queue
            channel
                .basic_publish(
                    &exchange,
                    &topic,
                    BasicPublishOptions::default(),
                    &payload,
                    BasicProperties::default(),
                )
                .await?
                .await?;

            Ok(())
        })
    }

    // Subscribe to a topic
    fn register_subscriber(
        &self,
        topic: &str,
        subscriber: Arc<Subscriber<M>>,
    ) -> Result<()> {

        // Clone over async boundary
        let connection = self.connection.clone();  // Clone the connection
        let topic = topic.to_string();
        let exchange = self.exchange.clone();

        tokio::spawn(async move {
            // Create a new channel for this subscriber
            let channel = connection.lock()
                .await
                .create_channel()
                .await.with_context(|| "Failed to create channel")?;

            // Declare the queue
            let queue = channel
                .queue_declare(&topic, QueueDeclareOptions::default(),
                               FieldTable::default())
                .await.with_context(|| "Failed to declare queue")?;

            // Bind the queue to the exchange with the specified pattern
            channel
                .queue_bind(
                    queue.name().as_str(),
                    &exchange,
                    &topic,
                    QueueBindOptions::default(),
                    FieldTable::default(),
                )
                .await.with_context(|| "Failed to bind queue")?;

            // Start consuming messages from the queue
            let mut consumer = channel
                .basic_consume(
                    queue.name().as_str(),
                    "",
                    BasicConsumeOptions::default(),
                    FieldTable::default(),
                )
                .await.with_context(|| "Failed to start consumer")?;

            // Process each message received
            while let Some(delivery) = consumer.next().await {
                let delivery = delivery.with_context(|| "Error in consumer")?;

                // Decode it
                match serde_cbor::de::from_slice::<M>(&delivery.data) {
                    Ok(message) => {
                        // Call the subscriber function with the message
                        subscriber(delivery.routing_key.as_str(), Arc::new(message)).await;
                    },
                    Err(e) => error!("Invalid CBOR message received: {}", e)
                }

                // Acknowledge the message anyway, otherwise it stays around
                // forever
                delivery
                    .ack(lapin::options::BasicAckOptions::default())
                    .await.with_context(|| "Failed to acknowledge message")?;
            }

            Ok::<(), anyhow::Error>(())  // Inform Rust what ? should return
        });

        Ok(())
    }

    /// Shut down the bus connection
    fn shutdown(&self) -> BoxFuture<'static, Result<()>> {
        info!("Shutting down RabbitMQ interface");
        let connection = self.connection.clone();

        Box::pin(async move {
            // Close the connection
            let connection = connection.lock().await;
            connection.close(200, "Goodbye").await?;
            Ok(())
        })
    }
}
