//! MessageBus implementation for RabbitMQ
use lapin::{
    options::{BasicConsumeOptions, BasicPublishOptions, QueueDeclareOptions},
    types::FieldTable,
    BasicProperties, Channel, Connection, ConnectionProperties,
};
use futures::StreamExt;
use anyhow::Result;
use config::Config;
use std::sync::Arc;
use tokio::sync::Mutex;
use futures::future::BoxFuture;
use crate::message_bus::{MessageBus, BoxedObserverFn, MessageBounds};
use std::marker::PhantomData;
use tracing::info;

/// RabbitMQ message bus implementation
pub struct RabbitMQBus<M: MessageBounds> {
    connection: Arc<Mutex<Connection>>,  // RabbitMQ connection
    channel: Arc<Mutex<Channel>>,        // RabbitMQ outgoing channel
    _phantom: PhantomData<M>,      // Required to associate with <M> (eww)
}

impl<M: MessageBounds> RabbitMQBus<M> {

    // New
    pub async fn new(_config: &Config) -> Result<Self> {
        // Connect to RabbitMQ server
        let addr = std::env::var("AMQP_ADDR")
            .unwrap_or_else(|_| "amqp://127.0.0.1:5672/%2f".into());

        info!("Connecting to RabbitMQ at {}", addr);

        let connection =
            Connection::connect(&addr, ConnectionProperties::default())
            .await
            .expect("Failed to connect to RabbitMQ");

        info!("RabbitMQ connected");

        // Create a channel for outgoing messages
        let channel = connection.create_channel().await?;

        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
            channel: Arc::new(Mutex::new(channel)),
            _phantom: PhantomData
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

        Box::pin(async move {
            let channel = channel.lock().await;

            // Declare the queue first if it doesn't exist
            channel
                .queue_declare(&topic, QueueDeclareOptions::default(),
                               FieldTable::default())
                .await?;

            // Serialise the message
            let payload = serde_json::to_vec(&*message)?;

            // Publish the message to the queue
            channel
                .basic_publish(
                    "",
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
    fn register_observer(
        &self,
        topic: &str,
        observer: BoxedObserverFn<M>,
    ) -> Result<()> {
        let connection = self.connection.clone();  // Clone the connection
        let observer = Arc::new(observer); // Shared observer function
        let topic = topic.to_string();

        tokio::spawn(async move {

            // Create a new channel for this observer
            let channel = connection.lock()
                .await
                .create_channel()
                .await
                .expect("Failed to create channel");

            // Declare the queue
            channel
                .queue_declare(&topic, QueueDeclareOptions::default(),
                               FieldTable::default())
                .await
                .expect("Failed to declare queue");

            // Start consuming messages from the queue
            let mut consumer = channel
                .basic_consume(
                    &topic,
                    "",
                    BasicConsumeOptions::default(),
                    FieldTable::default(),
                )
                .await
                .expect("Failed to start consumer");

            // Process each message received
            while let Some(delivery) = consumer.next().await {
                let delivery = delivery.expect("Error in consumer");
                let message: M = serde_json::from_slice(&delivery.data)
                    .expect("Invalid message format");

                // Call the observer function with the message
                observer(Arc::new(message)).await;

                // Acknowledge the message
                delivery
                    .ack(lapin::options::BasicAckOptions::default())
                    .await
                    .expect("Failed to acknowledge message");
            }
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
