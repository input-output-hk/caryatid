//! MessageBus implementation for RabbitMQ
use lapin::{
    options::{BasicConsumeOptions, BasicPublishOptions, ExchangeDeclareOptions,
              QueueBindOptions, QueueDeclareOptions},
    types::FieldTable,
    BasicProperties, Channel, Connection, ConnectionProperties,
};
use futures::StreamExt;
use anyhow::{Result, anyhow, Context};
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
            .await?;

        info!("RabbitMQ connected");

        // Get exchange name
        let exchange_name = config.get_string("exchange")
            .unwrap_or("caryatid".to_string());

        // Create a channel for outgoing messages
        let channel = connection.create_channel().await?;

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

    /// Publish a message on a topic
    fn request(&self, topic: &str, message: Arc<M>)
               -> BoxFuture<'static, Result<Arc<Result<M>>>> {
        let channel = self.channel.clone();
        let message = Arc::clone(&message);
        let topic = topic.to_string();
        let exchange = self.exchange.clone();

        Box::pin(async move {
            let channel = channel.lock().await;

            // Serialise the message
            let payload = serde_cbor::ser::to_vec(&*message)?;

            // Create a temporary response queue
            // !todo: Make this persistent
            let response_queue = channel
                .queue_declare(
                    "",
                    QueueDeclareOptions { exclusive: true,
                                          ..Default::default() },
                    FieldTable::default())
                .await.with_context(|| "Can't create response queue")?;

            // !todo Create correlation ID
            let correlation_id = "foo";

            // Publish the message to the queue
            channel
                .basic_publish(
                    &exchange,
                    &topic,
                    BasicPublishOptions::default(),
                    &payload,
                    BasicProperties::default()
                        .with_reply_to(response_queue.name().clone())
                        .with_correlation_id(correlation_id.into())
                )
                .await?
                .await?;

            // Set up consumer on the response queue
            let mut consumer = channel
                .basic_consume(
                    response_queue.name().as_str(),
                    "",
                    BasicConsumeOptions::default(),
                    FieldTable::default(),
                )
                .await.with_context(|| "Failed to start consumer")?;

            // Wait for the response
            match consumer.next().await {
                Some(delivery) => {
                    let delivery = delivery.unwrap();
                    let response_corr_id = delivery.properties.correlation_id()
                        .as_ref().map(|id| id.as_str());

                    // Acknowledge the response
                    delivery
                        .ack(lapin::options::BasicAckOptions::default())
                        .await.with_context(|| "Failed to acknowledge response")?;

                    // Match the correlation_id
                    if response_corr_id == Some(correlation_id) {
                        match serde_cbor::de::from_slice::<M>(&delivery.data) {

                            Ok(message) => Ok(Arc::new(Ok(message))),

                            Err(e) => {
                                error!("Invalid CBOR message received: {}", e);
                                Err(anyhow!("Invalid CBOR received"))
                            }
                        }
                    } else {
                        error!("Wrong correlation ID received");
                        Err(anyhow!("Wrong correlation ID"))
                    }
                },
                _ => {
                    error!("Nothing returned from consumer");
                    Err(anyhow!("No response"))
                }
            }
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

            // Declare the topic exchange
            // !todo - make singular for connection
            channel
                .exchange_declare(
                    &exchange,
                    lapin::ExchangeKind::Topic,
                    ExchangeDeclareOptions::default(),
                    FieldTable::default(),
                )
                .await.with_context(|| "Failed to declare exchange")?;

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
                        let result = subscriber(Arc::new(message)).await;

                        // Response required?
                        if let Some(reply_to) = delivery.properties.reply_to().as_ref() {

                            // Send it back
                            if let Ok(message) = result.as_ref() {
                                if let Some(corr_id) = delivery.properties.correlation_id() {

                                    // CBOR encode it
                                    match serde_cbor::ser::to_vec(message) {
                                        Ok(cbor) => {

                                            // Publish it on reply channel
                                            channel
                                                .basic_publish(
                                                    "",
                                                    reply_to.as_str(),
                                                    BasicPublishOptions::default(),
                                                    &cbor,
                                                    BasicProperties::default()
                                                        .with_correlation_id(corr_id.clone())
                                                )
                                                .await.with_context(|| "Can't send response")?;
                                        },
                                        Err(e) => {
                                            error!("Can't encode response message: {e}");
                                        }
                                    }
                                } else {
                                    error!("No correlation ID supplied for reply");
                                }
                            } else {
                                error!("Reply requested to {topic} but none returned");
                            }
                        }
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
