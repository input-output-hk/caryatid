# Caryatid basic request-response example

This example demonstrates a request-response model running on top of the pub-sub message bus.  The
request is still routed in the same way as any other message, but the response is correlated with it
to provide a asynchronous RPC-style model as you might use in REST or gRPC.  For simplicity,
the messages are just JSON objects.

## How to run it

```shell
$ cd examples/request
$ cargo run
```

## Points of interest

The [`Requester`](src/requester.rs) uses [`message_bus.request()`](src/requester.rs#L37) to send a `Hello, world!` message
and awaits a response, which it then unpacks and logs:

```rest
            match message_bus.request(&topic, test_message.clone()).await {
                Ok(response) => { info!("Got response: {:?}", response); },
                Err(e) => { error!("Got error: {e}"); }
            }
```

It also sends a request on a [bogus topic](src/requester.rs#L44) to demonstrate the timeout if there is no response.  To make
the test run quicker, this is set in the [configuration](request.toml#L20) to 1 second - by default it is 5:

```toml
[message-correlator]
timeout = 1
```

The [`Responder`](src/responder.rs) defines an async [handler function](src/responder.rs#L22) which adds a response property
to the message and returns it:

```rust
    async fn handler(message: Arc<Value>) -> Arc<Value> {
        info!("Handler received {:?}", message);

        let mut message = (*message).clone();

        if let Some(obj) = message.as_object_mut() {
            obj.insert("response".to_string(),
                       Value::String("Loud and clear".to_string()));
        }

        info!("Responding with {:?}", message);
        Arc::new(message)
    }
```

The handler function is registered on the topic with [`message_bus.handle()`](src/responder.rs#L40):

```rust
        context.message_bus.handle(&topic, Self::handler)?;
```
