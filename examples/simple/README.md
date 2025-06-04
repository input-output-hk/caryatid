# Caryatid simple publish-subscribe example

This example is the simplest possible pub-sub model. There are two modules, a [`Publisher`](src/publisher.rs) and a
[`Subscriber`](src/subscriber.rs). The publisher publishes a message on topic `sample.test` and the subscriber subscribes
to `sample.#`.  The messages are JSON objects.

There are two message buses, the internal in-memory one and an external RabbitMQ one.  The routing is set up
so all messages go to both buses, so the subscriber gets two copies of the message.

## How to run it

```shell
$ cd examples/simple
$ cargo run
```

## Points of interest

Take a look at the definition of `MType` in [main.rs](src/main.rs#L19)

```rust
type MType = serde_json::Value;
```

and how it is used to define the type of the [`Process`](src/main.rs#L38):

```rust
    let mut process = Process::<MType>::create(config).await;
```

In the the [subscriber](src/subscriber.rs) you can see how you can subscribe with
a [lambda](src/subscriber.rs#L32) which receives the message wrapped in an `Arc`:

```rust
        context.message_bus.subscribe(&topic,
                                      |message: Arc<serde_json::Value>| {
           info!("Received: {:?}", message);
        })?;
```

Then in the [publisher](src/publisher.rs) you can see it asynchronously publishing
a message:

```rust
        context.run(async move {

            let test_message = Arc::new(json!({
                "message": "Hello, world!",
            }));

            info!("Sending {:?}", test_message);

            message_bus.publish(&topic, test_message)
                .await.expect("Failed to publish message");
        });
```



