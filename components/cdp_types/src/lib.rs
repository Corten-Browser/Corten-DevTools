// CDP protocol types, events, and error definitions
//
// This module is part of the CortenBrowser DevTools implementation.

pub mod domains;
pub mod errors;

// Re-export commonly used types
pub use errors::CdpError;

use serde::{Deserialize, Serialize};

/// CDP Request message
/// Represents a request from the client to the CDP server
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CdpRequest {
    /// Unique identifier for this request
    pub id: u64,
    /// Method name in format "Domain.method"
    pub method: String,
    /// Optional parameters for the method
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

/// CDP Response message
/// Represents a response from the CDP server to a request
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CdpResponse {
    /// Request ID this response corresponds to
    pub id: u64,
    /// Result of the method call (if successful)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Error information (if method failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<errors::CdpError>,
}

/// CDP Event message
/// Represents an unsolicited event from the CDP server
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CdpEvent {
    /// Event name in format "Domain.event"
    pub method: String,
    /// Event parameters
    pub params: serde_json::Value,
}

/// Generic CDP Message that can be request, response, or event
/// Useful for parsing incoming messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CdpMessage {
    /// A request message (has id and method)
    Request(CdpRequest),
    /// A response message (has id and result/error)
    Response(CdpResponse),
    /// An event message (has method but no id)
    Event(CdpEvent),
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_request_basic() {
        let request = CdpRequest {
            id: 1,
            method: "Runtime.evaluate".to_string(),
            params: None,
        };

        assert_eq!(request.id, 1);
        assert_eq!(request.method, "Runtime.evaluate");
    }

    #[test]
    fn test_response_basic() {
        let response = CdpResponse {
            id: 1,
            result: Some(json!({"value": 42})),
            error: None,
        };

        assert_eq!(response.id, 1);
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_event_basic() {
        let event = CdpEvent {
            method: "Network.requestWillBeSent".to_string(),
            params: json!({"requestId": "123"}),
        };

        assert_eq!(event.method, "Network.requestWillBeSent");
    }
}
