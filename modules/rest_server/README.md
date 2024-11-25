# Standard REST server module for Caryatid

The `RESTServer` module provides a REST endpoint which turns HTTP requests into message bus requests.

For example, an HTTP request `GET /foo/bar` is turned into a message bus request on topic `rest.get.foo.bar`.  Notice
how the method (GET, POST etc.) is lower-cased, and the path is turned from slashes into dots so that the topic wildcards
work.  You could for example subscribe to `rest.get.foo.#` to get all GET requests on `foo`.

## Configuration

The REST server just needs configuration for what address and port to listen on:

```toml
[module.rest_server]
address = "0.0.0.0"
port = 4340
topic = "rest"
```

For safety, by default the server only listens on `127.0.0.1` (localhost).  Address `0.0.0.0` as above listens
on all interfaces.

The prefix added to the topic can be changed by setting `topic`.  It defaults to `rest` and can therefore usually be left out.

## Messages

The RESTServer module sends a `RESTRequest` message, and expects a `RESTResponse` both of which are defined in the common
[messages](../../sdk/src/messages.rs) in the SDK:

```rust
/// REST request message
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct RESTRequest {

    // HTTP method: GET, POST etc.
    pub method: String,

    // URL path: /foo
    pub path: String,

    // Request body (if any)
    pub body: String
}

/// REST response message
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct RESTResponse {

    // HTTP response code
    pub code: u16,

    // Response body (if any)
    pub body: String
}

pub trait GetRESTResponse {
    fn get_rest_response(&self) -> Option<RESTResponse>;
}
```

It is up to the receiver to handle errors by returning a suitable code (e.g. 40x, 500).  The body should be set to `""` if no response
body is required.

The `GetRESTResponse` trait is used to check if a message is a `RESTResponse` and return it if so.

## Registration

The RESTServer module needs to be parameterised with the type of an outer message `enum` which contains `RESTRequest` and `RESTResponse` 
variants, provides a `From` implementation to promote them, as well as a `GetRESTResponse` implementation to extract a response
from the outer message.  For example, your system-wide message enum might be:

```rust
use caryatid_sdk::messages::{RESTRequest, RESTResponse, GetRESTResponse};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Message {
    None(()),
    // ... other messages ...
    RESTRequest(RESTRequest),   // REST request
    RESTResponse(RESTResponse), // REST response
}

// Casts to platform-wide messages
impl From<RESTRequest> for Message {
    fn from(msg: RESTRequest) -> Self {
        Message::RESTRequest(msg)
    }
}

impl From<RESTResponse> for Message {
    fn from(msg: RESTResponse) -> Self {
        Message::RESTResponse(msg)
    }
}

// Casts from platform-wide messages
impl GetRESTResponse for Message {
    fn get_rest_response(&self) -> Option<RESTResponse> {
        if let Message::RESTResponse(result) = self {
            Some(result.clone())
        } else {
            None
        }
    }
}
```

Then within your `main.rs` you would register the RESTServer module into the [process](../../process) like this:

```rust
    RESTServer::<Message>::register(&mut process);
```

See the [REST example](../../examples/rest) to see this in action.

## A simple REST handler

You can see a simple 'Hello, world!' handler in [rest_hello_world.rs](src/rest_hello_world.rs).  The core of it is this:

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
