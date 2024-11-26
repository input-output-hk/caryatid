# Caryatid typed message example

This example demonstrates using typed messages in the Caryatid message bus framework.  Rather than using a generic,
open data format such as JSON (as we do in the [simple example](../simple)), using typed messages allows using
real Rust types to communicate.  This has two benefits:

* Less code to pack and unpack messages and handle errors
* No performance loss in creating and unpacking JSON objects

The set of possible messages are defined in an `enum`.  The same enum must be used to define the [process](../../process)
and be used in all the modules within it.  Usually, the message enum definition will be system wide.

When passed between modules in the same process, the messages are just passed in `Arc` references.  When passed
to an external message bus, they are automatically serialised and deserialised to [CBOR](https://cbor.io) 
(a packed, binary form of JSON).

The example creates a typed [`Publisher`](src/publisher.rs) and [`Subscriber`](src/subscriber.rs) which exchange a range of
different message types, and also uses the standard [`Clock`](../../modules/clock) module to produce a 'speaking clock' output.

As with the [simple example](../simple), the messages are routed both internally and externally, so the subscriber receives two
copies of each.  One has been passed with no copying internally, the other through CBOR serialisation and deserialisation.  The
important thing to notice is that neither the publisher nor subscriber need to know the difference.

## How to run it

```shell
$ cd examples/typed
$ cargo run
```

## Points of interest

The messages are defined in the `Message` enum in [message.rs](src/message.rs#L12).  Notice how we provide a `None` option and a variety of other
message types, and we derive serialisation and deserialisation implementations:

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Message {
    None(()),                 // Just so we have a simple default
    Test(Test),               // A custom struct
    String(String),           // Simple string
    Clock(ClockTickMessage),  // Clock tick
    JSON(serde_json::Value),  // Get out of jail free card
}
```

So that the `Clock` can generate the correct message variant, we need to provide a `From` implementation from its message type:

```rust
impl From<ClockTickMessage> for Message {
    fn from(msg: ClockTickMessage) -> Self {
        Message::Clock(msg)
    }
}
```

When we create the `Process` in [main.rs](src/main.rs#L42) we have to provide the message type:

```rust
    let mut process = Process::<Message>::create(config).await;
```

and also when we register the [Clock](src/main.rs#L47):

```rust
    Clock::<Message>::register(&mut process);
```

The [publisher](src/publisher.rs) publishes a variety of different message types wrapped in the overall `Message` enum - for example:

```rust
            let test_message_1 = Message::Test(Test {
                data: "Hello, world!".to_string(),
                number: 42
            });
            info!("Sending {:?}", test_message_1);
            message_bus.publish(&format!("{topic}.test"), Arc::new(test_message_1))
                .await.expect("Failed to publish message");
```

The [subscriber](src/subscriber.rs) registers a [lambda](src/subscriber.rs#L30) as usual, but then then matches on the message to handle different cases:

```rust
        context.message_bus.subscribe(&topic, |message: Arc<Message>| {
            match message.as_ref()
            {
                Message::None(_) => error!("Received empty message!"),
                Message::Test(test) => info!("Received test: {} {}", test.data, test.number),
                Message::String(s) => info!("Received string {s}"),
                Message::JSON(json) => info!("Received JSON {:?}", json),
                _ => error!("Unexpected message type")
            }
        })?;
```

Often there will only be two cases - handle the message you want, and everything else is an error.


