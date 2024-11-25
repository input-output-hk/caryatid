# Caryatid REST server example

This example demonstrates adding a REST server to a Caryatid framework, and using the message
subscription system to implement request routing as you might do explicitly in Axum or Express.

The [example configuration](rest.toml) sets up two REST server endpoints, on ports 4340 and 4341.
It also creates two versions of a 'Hello world!' responder, which register on the `rest.get.hello`
and `rest2.get.hello` topics:

```toml
[module.rest-server]
address = "127.0.0.1"
port = 4340

[module.rest-server-2]
class = "rest-server"
topic = "rest2"
port = 4341

[module.rest-hello-world]
topic = "rest.get.hello"

[module.rest-hello-world-2]
class = "rest-hello-world"
topic = "rest2.get.hello"
```

## How to run it

```shell
$ cd examples/rest
$ cargo run
```

Then go to [localhost:4340/hello](http://localhost:4340/hello) in your browser.  Try [other URL paths](http://localhost:4340/bogus) (which will fail
after a 5 second timeout) and also [port 4341](http://localhost:4341/hello).

## Points of interest

The process uses typed messages (as explained in the [typed example](../typed)), and defines its `Message` enum in [message.rs](src/message.rs#L6):

```rust
pub enum Message {
    None(()),                   // Just so we have a simple default
    RESTRequest(RESTRequest),   // REST request
    RESTResponse(RESTResponse), // REST response
}
```

It then implements `From` and `GetRESTResponse` to create and unpack this `Message` from/to the `RESTRequest` and `RESTResponse`.

The process is defined in [main.rs](src/main.rs#L39) using the `Message` type:

```rust
    let mut process = Process::<Message>::create(config).await;
```

and likewise the [`RESTServer`](src/main.rs#L43) - as a generic module which can be used by any application, it needs to know the top-level message
type in use:

```rust
    RESTServer::<Message>::register(&mut process);
```

See the [RESTServer documentation](../../modules/rest_server/) for more details.

To respond to the REST request, we define a [`RESTHelloWorld`](src/rest_hello_world.rs) responder.  This registers a [lambda](src/rest_hello_world.rs#L26) as the handler
for the configured topic:

```rust
        context.message_bus.handle(&topic, |message: Arc<Message>| {
            let response = match message.as_ref() {
                Message::RESTRequest(request) => {
                    info!("REST hello world received {} {}", request.method, request.path);
                    RESTResponse {
                        code: 200,
                        body: "Hello, world!".to_string()
                    }
                },
                _ => {
                    error!("Unexpected message type {:?}", message);
                    RESTResponse {
                        code: 500,
                        body: "Unexpected message in REST request".to_string() }
                }
            };

            future::ready(Arc::new(Message::RESTResponse(response)))
        })?;
```

Note that because we're using a synchronous lambda we need to manually return the `future` for the result.
