//! Messages for REST server module

use std::collections::HashMap;

/// REST request message
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct RESTRequest {
    /// HTTP method: GET, POST etc.
    pub method: String,

    /// URL path: /foo
    pub path: String,

    /// URL path elements (split on /)
    pub path_elements: Vec<String>,

    /// Request body (if any)
    pub body: String,

    /// Request query parameters
    pub query_parameters: HashMap<String, String>,
}

/// REST response message
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct RESTResponse {
    /// HTTP response code
    pub code: u16,

    /// Response body (if any)
    pub body: String,

    /// Response content-type
    pub content_type: String,
}

impl RESTResponse {
    /// Construct with any content-type
    pub fn new(code: u16, body: &str, content_type: &str) -> Self {
        Self {
            code,
            body: body.to_string(),
            content_type: content_type.to_string(),
        }
    }

    /// Construct for text/plain
    pub fn with_text(code: u16, body: &str) -> Self {
        Self::new(code, body, "text/plain")
    }

    /// Construct for application/json
    pub fn with_json(code: u16, body: &str) -> Self {
        Self::new(code, body, "application/json")
    }
}

pub trait GetRESTResponse {
    fn get_rest_response(&self) -> Option<RESTResponse>;
}
