// CDP Error types
// Implements JSON-RPC 2.0 error codes and CDP-specific errors

use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

/// CDP Error following JSON-RPC 2.0 error specification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CdpError {
    /// Error code (JSON-RPC standard codes)
    pub code: i32,
    /// Human-readable error message
    pub message: String,
    /// Additional error data (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl fmt::Display for CdpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CDP Error {}: {}", self.code, self.message)
    }
}

impl std::error::Error for CdpError {}

impl CdpError {
    /// Create a new CDP error
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// Create error with additional data
    pub fn with_data(code: i32, message: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            code,
            message: message.into(),
            data: Some(data),
        }
    }

    /// Parse error (-32700)
    /// Invalid JSON was received by the server
    pub fn parse_error() -> Self {
        Self::new(-32700, "Parse error")
    }

    /// Invalid request (-32600)
    /// The JSON sent is not a valid Request object
    pub fn invalid_request() -> Self {
        Self::new(-32600, "Invalid Request")
    }

    /// Method not found (-32601)
    /// The method does not exist / is not available
    pub fn method_not_found(method: impl Into<String>) -> Self {
        let method = method.into();
        Self::with_data(
            -32601,
            "Method not found",
            serde_json::json!({ "method": method }),
        )
    }

    /// Invalid params (-32602)
    /// Invalid method parameter(s)
    pub fn invalid_params(details: impl Into<String>) -> Self {
        let details = details.into();
        Self::with_data(
            -32602,
            "Invalid params",
            serde_json::json!({ "details": details }),
        )
    }

    /// Internal error (-32603)
    /// Internal JSON-RPC error
    pub fn internal_error(details: impl Into<String>) -> Self {
        let details = details.into();
        Self::with_data(
            -32603,
            "Internal error",
            serde_json::json!({ "details": details }),
        )
    }

    /// Server error (-32000 to -32099)
    /// Reserved for implementation-defined server-errors
    pub fn server_error(code: i32, message: impl Into<String>) -> Self {
        assert!(
            (-32099..=-32000).contains(&code),
            "Server error codes must be between -32099 and -32000"
        );
        Self::new(code, message)
    }
}

/// CDP-specific error type using thiserror
#[derive(Error, Debug)]
pub enum CdpProtocolError {
    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// Protocol violation
    #[error("Protocol violation: {0}")]
    ProtocolViolation(String),

    /// Invalid domain
    #[error("Invalid domain: {0}")]
    InvalidDomain(String),

    /// Invalid method
    #[error("Invalid method: {0}")]
    InvalidMethod(String),

    /// Node not found
    #[error("Node not found: {0}")]
    NodeNotFound(u32),

    /// Object not found
    #[error("Object not found: {0}")]
    ObjectNotFound(String),

    /// Generic CDP error
    #[error("CDP error: {0}")]
    CdpError(#[from] CdpError),
}

impl From<CdpProtocolError> for CdpError {
    fn from(error: CdpProtocolError) -> Self {
        match error {
            CdpProtocolError::SerializationError(e) => CdpError::with_data(
                -32700,
                "Parse error",
                serde_json::json!({ "error": e.to_string() }),
            ),
            CdpProtocolError::ProtocolViolation(msg) => CdpError::server_error(-32000, msg),
            CdpProtocolError::InvalidDomain(domain) => {
                CdpError::method_not_found(format!("Invalid domain: {}", domain))
            }
            CdpProtocolError::InvalidMethod(method) => CdpError::method_not_found(method),
            CdpProtocolError::NodeNotFound(node_id) => {
                CdpError::server_error(-32000, format!("Node not found: {}", node_id))
            }
            CdpProtocolError::ObjectNotFound(object_id) => {
                CdpError::server_error(-32000, format!("Object not found: {}", object_id))
            }
            CdpProtocolError::CdpError(e) => e,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        assert_eq!(CdpError::parse_error().code, -32700);
        assert_eq!(CdpError::invalid_request().code, -32600);
        assert_eq!(CdpError::method_not_found("test").code, -32601);
        assert_eq!(CdpError::invalid_params("bad").code, -32602);
        assert_eq!(CdpError::internal_error("error").code, -32603);
    }

    #[test]
    fn test_server_error() {
        let error = CdpError::server_error(-32000, "Custom error");
        assert_eq!(error.code, -32000);
        assert_eq!(error.message, "Custom error");
    }

    #[test]
    fn test_error_serialization() {
        let error = CdpError::new(-32601, "Method not found");
        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("-32601"));
        assert!(json.contains("Method not found"));
    }
}
