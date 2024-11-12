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
    routing::any,
};
use hyper::body;

use std::net::SocketAddr;
use std::convert::Infallible;

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
        let topic = "rest.get.hello".to_string(); // !!! Construct from request !!!

        // Generic request handler
        let handle_request = |req: Request<Body>| async move {
            info!("Received REST request {} {}",
                  req.method().as_str(), req.uri().path());

            let method = req.method().as_str().to_string();
            let path = req.uri().path().to_string();
            let bytes = body::to_bytes(req.into_body()).await.unwrap(); // !!! Handle
            let body = String::from_utf8(bytes.to_vec())
                .expect("request body should be valid UTF-8"); // !!! Handle 

            let message = RESTRequest { method, path, body };
            info!("Sending {:?}", message);

            let response = match message_bus.request(&topic, Arc::new(message.into())).await {
                Ok(result) => {
                    match result.as_ref() {
                        Ok(response) => match response.get_rest_response() {
                            Some(RESTResponse { code, status, body }) => {
                                info!("Got response: {code} {status}");
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
                                    .body("Internal server error".to_string())
                                    .unwrap()
                            }
                        },

                        Err(e) => {
                            error!("Request failed: {e}");
                            Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .body(e.to_string())
                                .unwrap()
                        }
                    }
                },
                Err(e) => {
                    error!("Request failed: {e}");
                    Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .body(e.to_string())
                        .unwrap()
                }
            };

            Ok::<_, Infallible>(response)
        };

        tokio::spawn(async move {

            // Define the address to bind the server to !!! config
            let addr = SocketAddr::from(([127, 0, 0, 1], 3120));
            info!("REST server listening on http://{}", addr);

            // Create an 'app' - actually we handle all the routing, we just use axum to
            // sugar over hyper
            let app = Router::new().route("/", any(handle_request));

            // Run it
            axum::Server::bind(&addr)
                .serve(app.into_make_service())
                .await
                .unwrap();
        });

        Ok(())
    }
}

