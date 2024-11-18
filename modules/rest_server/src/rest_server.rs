//! Caryatid REST server module
//! Provides a REST endpoint which integrates with the message bus

use caryatid_sdk::{Context, Module, module, MessageBounds};
use caryatid_sdk::messages::{RESTRequest, RESTResponse, GetRESTResponse};
use std::sync::Arc;
use anyhow::Result;
use config::Config;
use tracing::{info, error};

use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::Response,
    Router,
};
use hyper::body;

use std::net::{SocketAddr, IpAddr, Ipv4Addr};
use std::convert::Infallible;

/// Default IP address and port to listen on
const DEFAULT_IP: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
const DEFAULT_PORT: u16 = 4340;

/// Maximum length of body to log
const MAX_LOG: usize = 40;

/// REST module
/// Parameterised by the outer message enum used on the bus
#[module(
    message_type(M),
    name = "rest-server",
    description = "REST server"
)]
pub struct RESTServer<M: From<RESTRequest> + GetRESTResponse + MessageBounds>;

impl<M: From<RESTRequest> + GetRESTResponse + MessageBounds> RESTServer<M>
{
    fn init(&self, context: Arc<Context<M>>, config: Arc<Config>) -> Result<()> {
        let message_bus = context.message_bus.clone();

        // Get topic prefix from config
        let topic_prefix = config.get_string("topic").unwrap_or("rest".to_string());

        // Generic request handler
        let handle_request = |req: Request<Body>| async move {
            info!("Received REST request {} {}",
                  req.method().as_str(), req.uri().path());

            let method = req.method().as_str().to_string();
            let path = req.uri().path().to_string();

            let bytes = match body::to_bytes(req.into_body()).await {
                Ok(b) => b,
                Err(e) => return Ok(Response::builder()
                                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                                    .body(e.to_string())
                                    .unwrap())
            };

            let body = match String::from_utf8(bytes.to_vec()) {
                Ok(b) => b,
                Err(e) => return Ok(Response::builder()
                                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                                    .body(e.to_string())
                                    .unwrap())
            };

            // Construct topic, turning / to . and remove leading /
            let method_lower = method.to_lowercase();
            let dot_path = path.strip_prefix("/")
                .unwrap_or(&path)
                .replace('/', ".");
            let topic = format!("{topic_prefix}.{method_lower}.{dot_path}");
            info!("Sending to topic {}", topic);

            // Construct message
            let message = RESTRequest { method, path, body };

            let response = match message_bus.request(&topic, Arc::new(message.into())).await {
                Ok(response) => match response.get_rest_response() {
                    Some(RESTResponse { code, body }) => {

                        info!("Got response: {code} {}{}",
                              &body[..std::cmp::min(body.len(), MAX_LOG)],
                              if body.len()>MAX_LOG {"..."} else {""});

                        Response::builder()
                            .status(StatusCode::from_u16(code)
                                    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR))
                            .body(body)
                            .unwrap()
                    },
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

        tokio::spawn(async move {

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

