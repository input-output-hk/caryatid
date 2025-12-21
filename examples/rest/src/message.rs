//! Message type definitions for REST process

use caryatid_module_rest_server::messages::{GetRESTResponse, RESTRequest, RESTResponse};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Message {
    None(()),                   // Just so we have a simple default
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
