# Caryatid module SDK

This is an SDK for building modules in Caryatid. It provides a `#[module]` macro which does
most of the hard work - all you need to do is implement an `init` function, which gets passed
a `context` which contains the message bus and global configuration, and a `config` which
is the subset of configuration for your particular module.

You need to choose a message type that this module will handle. This can be a generic format
like JSON, or a specific Message enum for all the messages in your application. The message type
has to be common across all modules built into a single process.

### Template module

So a module looks like this:

```rust
use caryatid_sdk::{Context, MessageBusExt, Module, module};
use std::sync::Arc;
use anyhow::Result;
use config::Config;

// Define a message type
type MType = serde_json::Value;

// Define it as a module, with a name and description
#[module(
    message_type(MType),
    name = "my-module",
    description = "An example module"
)]
pub struct MyModule;

impl MyModule {
    // Implement the single initialisation function, with application
    // Context and this module's Config
    fn init(&self, context: Arc<Context<MType>>, config: Arc<Config>) -> Result<()> {
        // ... Read configuration from config
        // ... Register message subscribers and/or request handlers on context.message_bus
        // ... Start other async processes
    }
}
```

## Message bus

The SDK provides a `MessageBus` trait which is implemented by a collection of units in
[`process`](../process). As far as a module is concerned it doesn't need to know what kind
of message bus it is talking to, whether messages are routed internal or externally, or both.
It just publishes and subscribes on the bus given in `context.message_bus`.

### Simple pub-sub

For pure publish-subscribe, where messages are events describing some change in the system, and
don't require a response, you can call `context.message_bus.publish()` and `context.message_bus.subscribe()`:

#### Publishing

Assuming the module is created with message_type `serde_json::Value` as above:

```rust
    let topic = "test.simple";
    let test_message = Arc::new(json!({
        "message": "Hello, world!",
    }));

    context.message_bus.publish(topic, test_message)
      .await
      .expect("Failed to publish message");
```

Note that the message must be wrapped in an `Arc` to be passed around. You can see that publish
is async, so this needs to be called in an async context.

#### Subscribing

```rust
    let topic = "test.simple";

    context.message_bus.subscribe(&topic,
                                  |message: Arc<serde_json::Value>| {
       info!("Received: {:?}", message);
    })?;
```

The subscriber is a closure (lambda) which takes an Arc containing the message type.

A full version of this can be found in [examples/simple](../examples/simple).

### Request-response

The SDK also offers a request-response model, layered on top of the basic pub-sub in an
implementation-independent way. In this case you call `context.message_bus.request()` and
`context.message_bus.handle()`:

#### Requesting

```rust
    let topic = "test.simple";
    let test_message = Arc::new(json!({
        "message": "Hello, world!",
    }));

    match context.message_bus.request(topic, test_message).await {
      Ok(response) => { info!("Got response: {:?}", response); },
      Err(e) => { error!("Got error: {e}"); }
    }
```

`request()` returns a Result, which either contains an `Arc` of the response message, or an error
if nothing responded (after a timeout). Note that application-level errors need to be encoded into
the message type - in this case, in the JSON object.

#### Responding

Requests are responded to by handlers, which are usually async functions taking and returning
an `Arc` with the message type.

```rust
    async fn handler(message: Arc<Value>) -> Arc<Value> {
        let response = json!({
          "message": "Pleased to meet you!",
        });

        Arc::new(response)
    }

    let topic = "test.simple";
    context.message_bus.handle(topic, handler)?;
```

Note that as above the handler must return a valid message - it cannot return an Error.

A full version of request/response can be found in [examples/request](../examples/request).

## Typed Messages

It is perfectly possible to communicate with JSON objects, or even strings, and some
applications may wish to do so. However, there is a cost both in execution time and
code complexity in packing and unpacking JSON for every message.

The Caryatid SDK therefore provides a way to specify messages as native Rust types. Since in any
real-world system there will be more than one type, this is done by defining a system-wide
`enum` of all the messages - e.g.

```rust
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct Test {
    pub data: String,
    pub number: i64
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Message {
    None(()),                 // Just so we have a simple default
    Test(Test),               // A custom struct
    String(String),           // Simple string
    JSON(serde_json::Value),  // Any JSON value
}

impl Default for Message {
    fn default() -> Self {
        Message::None(())
    }
}

```

The `enum Message` has a custom struct `Test` but also general options `String` and `JSON` which
can be used for extensibility.

### Typed subscription

A subscriber module using this `Message` type would now look like this:

```rust
// ... other imports ...
use crate::message::Message;

#[module(
    message_type(Message),
    name = "typed-subscriber",
    description = "Typed subscriber module"
)]
pub struct TypedSubscriber;

impl TypedSubscriber {
    fn init(&self, context: Arc<Context<Message>>, config: Arc<Config>) -> Result<()> {
        let topic = "test.simple";
        context.message_bus.subscribe(topic, |message: Arc<Message>| {
            match message.as_ref()
            {
                Message::None(_) => error!("Received empty message!"),
                Message::Test(test) => info!("Received test: {} {}", test.data, test.number),
                Message::String(s) => info!("Received string {s}"),
                Message::JSON(json) => info!("Received JSON {:?}", json),
                _ => error!("Unexpected message type")
            }
        })?;
    }
```

This example matches every possible option - normally a subscriber would just test for one,
and error if it received anything else.

### Typed publication

The publisher using just one of the options of Message, would look like this:

```rust
use crate::message::{Test, Message};

#[module(
    message_type(Message),
    name = "typed-publisher",
    description = "Typed publisher module"
)]
pub struct TypedPublisher;

impl TypedPublisher {
    // Context and this module's Config
    fn init(&self, context: Arc<Context<Message>>, config: Arc<Config>) -> Result<()> {
        let message_bus = context.message_bus.clone();
        let topic = "test.simple";

        tokio::spawn(async move {
            let test_message = Message::Test(Test {
                data: "Hello, world!".to_string(),
                number: 42
            });
            message_bus.publish(topic, Arc::new(test_message))
                .await
                .expect("Failed to publish message");
        });
    }
}
```

The full versions of `TypedSubscriber` and `TypedPublisher` can be found in
[examples/typed](../examples/typed).

---
## Module configuration

The module's `init` function is passed a [`config::Config`](https://docs.rs/config/) that merges the `[global]` section
with the `[module.x]` module-specific configuration. This allows modules to access shared configuration values defined in
`[global.*]` while still having their own overrides. Module-specific values take precedence over global values when keys
collide.

For example, if your module was called `my-module` and the global configuration file contained:

```toml
[global.startup]
method = "default"
topic = "app.startup"

[module.my-module]
topic = "test.simple"
count = 10
```

Your module config would contain:

- `startup.method` → `"default"` (from global)
- `startup.topic` → `"app.startup"` (from global)
- `topic` → `"test.simple"` (module-specific)
- `count` → `10` (module-specific)

You can access these values like any other config:

```rust
// ... other imports
use config::Config;

let method = config.get_string("startup.method")?;
let topic = config.get_string("topic")?;
let count = config.get::<u32>("count")?;
```

(Better error handling or defaulting is left as an exercise for the reader!)