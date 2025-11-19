//! Caryatid REST server module
//! Provides a REST endpoint which integrates with the message bus

use anyhow::Result;
use caryatid_sdk::{module, Context, MessageBounds};
use config::Config;
use std::{collections::HashMap, sync::Arc};
use tracing::{error, info};

use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::Response,
    Router,
};
use hyper::body;

use std::convert::Infallible;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

pub mod messages;
use messages::{GetRESTResponse, RESTRequest, RESTResponse};

/// Default IP address and port to listen on
const DEFAULT_IP: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
const DEFAULT_PORT: u16 = 4340;

/// Maximum length of body to log
const MAX_LOG: usize = 40;

/// REST module
/// Parameterised by the outer message enum used on the bus
#[module(message_type(M), name = "rest-server", description = "REST server")]
pub struct RESTServer<M: From<RESTRequest> + GetRESTResponse + MessageBounds>;

impl<M: From<RESTRequest> + GetRESTResponse + MessageBounds> RESTServer<M> {
    async fn init(&self, context: Arc<Context<M>>, config: Arc<Config>) -> Result<()> {
        let message_bus = context.message_bus.clone();

        // Get topic prefix from config
        let topic_prefix = config.get_string("topic").unwrap_or("rest".to_string());

        // Generic request handler
        let handle_request = |req: Request<Body>| async move {
            info!(
                "Received REST request {} {}{}",
                req.method().as_str(),
                req.uri().path(),
                req.uri()
                    .query()
                    .map(|q| format!("?{}", q))
                    .unwrap_or_default(),
            );

            let method = req.method().as_str().to_string();
            let path = req.uri().path().to_string();
            let query_parameters: HashMap<String, String> = req
                .uri()
                .query()
                .unwrap_or("")
                .split('&')
                .filter(|s| !s.is_empty())
                .filter_map(|pair| {
                    let mut parts = pair.splitn(2, '=');
                    let key = parts.next()?;
                    let value = parts.next().unwrap_or("");
                    Some((key.to_string(), value.to_string()))
                })
                .collect();

            let bytes = match body::to_bytes(req.into_body()).await {
                Ok(b) => b,
                Err(e) => {
                    return Ok(Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(e.to_string())
                        .unwrap())
                }
            };

            let body = match String::from_utf8(bytes.to_vec()) {
                Ok(b) => b,
                Err(e) => {
                    return Ok(Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(e.to_string())
                        .unwrap())
                }
            };

            // Construct topic, turning / to . and remove leading and trailing /
            let method_lower = method.to_lowercase();
            let dot_path = path.strip_prefix("/").unwrap_or(&path);
            let dot_path = dot_path.strip_suffix("/").unwrap_or(&dot_path);
            let dot_path = dot_path.replace('/', ".");
            let topic = format!("{topic_prefix}.{method_lower}.{dot_path}");
            info!("Sending to topic {}", topic);

            let path_elements = dot_path.split('.').map(String::from).collect();

            // Construct message
            let message = RESTRequest {
                method,
                path,
                body,
                path_elements,
                query_parameters,
            };

            let response = match message_bus.request(&topic, Arc::new(message.into())).await {
                Ok(response) => match response.get_rest_response() {
                    Some(RESTResponse {
                        code,
                        body,
                        content_type,
                    }) => {
                        info!(
                            "Got response: {code} {}{}",
                            &body[..std::cmp::min(body.len(), MAX_LOG)],
                            if body.len() > MAX_LOG { "..." } else { "" }
                        );

                        Response::builder()
                            .status(
                                StatusCode::from_u16(code)
                                    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                            )
                            .header("Content-Type", content_type)
                            .body(body)
                            .unwrap()
                    }
                    _ => {
                        error!("Response isn't RESTResponse");
                        Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .body("".to_string())
                            .unwrap()
                    }
                },
                Err(_) => {
                    error!("No handler for {topic}");
                    Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .body("".to_string())
                        .unwrap()
                }
            };

            Ok::<_, Infallible>(response)
        };

        context.run(async move {
            // Define the address to bind the server to
            let ip = config.get::<IpAddr>("address").unwrap_or(DEFAULT_IP);
            let port: u16 = config.get::<u16>("port").unwrap_or(DEFAULT_PORT);
            let addr = SocketAddr::from((ip, port));
            info!("REST server listening on http://{}", addr);

            // Create an 'app' - actually we handle all the routing, we just use axum to
            // sugar over hyper
            let app = Router::new().fallback(handle_request);

            // Run it
            axum::Server::bind(&addr)
                .serve(app.into_make_service())
                .await
                .unwrap();
        });

        Ok(())
    }
}

// -- Tests --
#[cfg(test)]
mod tests {
    use super::*;
    use caryatid_sdk::mock_bus::MockBus;
    use caryatid_sdk::Module;
    use config::{Config, FileFormat};
    use futures::future;
    use hyper::Client;
    use std::net::TcpListener;
    use tokio::sync::{watch::Sender, Notify};
    use tokio::time::{timeout, Duration};
    use tracing::{debug, Level};
    use tracing_subscriber;

    // Message type which includes a ClockTickMessage
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub enum Message {
        None(()),
        RESTRequest(RESTRequest),   // REST request
        RESTResponse(RESTResponse), // REST response
    }

    impl Default for Message {
        fn default() -> Self {
            Message::None(())
        }
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

    // Helper to create a clock talking to a mock message bus
    struct TestSetup {
        module: Arc<dyn Module<Message>>,
        context: Arc<Context<Message>>,
        startup_watch: Sender<bool>,
    }

    impl TestSetup {
        async fn new(config_str: &str) -> Self {
            // Set up tracing
            let _ = tracing_subscriber::fmt()
                .with_max_level(Level::DEBUG)
                .with_test_writer()
                .try_init();

            // Parse config
            let config = Arc::new(
                Config::builder()
                    .add_source(config::File::from_str(config_str, FileFormat::Toml))
                    .build()
                    .unwrap(),
            );

            // Create mock bus
            let mock_bus = Arc::new(MockBus::<Message>::new(&config));

            // Create a context
            let startup_watch = Sender::new(false);
            let context = Arc::new(Context::new(
                config.clone(),
                mock_bus,
                startup_watch.subscribe(),
            ));

            // Create the server
            let rest_server = RESTServer::<Message> {
                _marker: std::marker::PhantomData,
            };
            assert!(rest_server.init(context.clone(), config).await.is_ok());

            Self {
                module: Arc::new(rest_server),
                context,
                startup_watch,
            }
        }

        fn start(&self) {
            let _ = self.startup_watch.send(true);
        }
    }

    #[tokio::test]
    async fn construct_a_rest_server() {
        let setup = TestSetup::new("").await;
        assert_eq!(setup.module.get_name(), "rest-server");
        assert_eq!(setup.module.get_description(), "REST server");
    }

    #[tokio::test]
    async fn rest_server_generates_request_and_returns_response() {
        // Find a free port, then discard the listener
        let port: u16;
        {
            let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind to address");
            port = listener.local_addr().unwrap().port()
        }

        assert!(port > 0);

        let setup = TestSetup::new(&format!("port = {port}")).await;
        let notify = Arc::new(Notify::new());

        // Register for rest.get.test
        let notify_clone = notify.clone();
        setup
            .context
            .handle("rest.get.test", move |message: Arc<Message>| {
                let response = match message.as_ref() {
                    Message::RESTRequest(request) => {
                        info!(
                            "REST hello world received {} {}",
                            request.method, request.path
                        );
                        RESTResponse::with_text(200, "Hello, world!")
                    }
                    _ => {
                        error!("Unexpected message type {:?}", message);
                        RESTResponse::with_text(500, "Unexpected message in REST request")
                    }
                };

                notify_clone.notify_one();
                future::ready(Arc::new(Message::RESTResponse(response)))
            });

        setup.start();

        // Let it get set up
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Request it
        let client = Client::new();
        let uri = format!("http://127.0.0.1:{}/test", port).parse().unwrap();
        match timeout(Duration::from_secs(1), client.get(uri)).await {
            Ok(Ok(response)) => {
                debug!("HTTP response: {:?}", response);
                assert_eq!(response.status(), 200);
            }
            Ok(Err(e)) => panic!("HTTP request failed: {e}"),
            Err(e) => panic!("HTTP request timed out: {e}"),
        }

        // Wait for it to be received, or timeout
        assert!(
            timeout(Duration::from_secs(1), notify.notified())
                .await
                .is_ok(),
            "Didn't receive a rest.get.test message"
        );
    }

    #[tokio::test]
    async fn rest_server_with_no_handler_generates_404() {
        // Find a free port, then discard the listener
        let port: u16;
        {
            let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind to address");
            port = listener.local_addr().unwrap().port()
        }

        assert!(port > 0);

        let setup = TestSetup::new(&format!("port = {port}\nrequest-timeout = 1")).await;
        setup.start();

        // Let it get set up
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Request it
        let client = Client::new();
        let uri = format!("http://127.0.0.1:{}/test", port).parse().unwrap();
        // Note long enough timeout for bus to timeout above
        match timeout(Duration::from_secs(2), client.get(uri)).await {
            Ok(Ok(response)) => {
                debug!("HTTP response: {:?}", response);
                assert_eq!(response.status(), 404);
            }
            Ok(Err(e)) => panic!("HTTP request failed: {e}"),
            Err(e) => panic!("HTTP request timed out: {e}"),
        }
    }
}
