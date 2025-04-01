//! Definition of core framework messages

// We don't use these messages in the SDK itself
#![allow(dead_code)]

use chrono::{DateTime, Utc};

/// Clock tick message
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClockTickMessage {
    /// Time of tick, UTC
    pub time: DateTime<Utc>,

    /// Tick number
    pub number: u64
}

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
    pub body: String
}

/// REST response message
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct RESTResponse {

    /// HTTP response code
    pub code: u16,

    /// Response body (if any)
    pub body: String,

    /// Response content-type (if any, server defaults to application/json)
    pub content_type: Option<String>,
}

pub trait GetRESTResponse {
    fn get_rest_response(&self) -> Option<RESTResponse>;
}
