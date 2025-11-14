//! Message transport layer for CDP protocol

use crate::error::{CdpServerError, Result};
use cdp_types::CdpMessage;

/// Parse a CDP message from JSON string
pub fn parse_cdp_message(json: &str) -> Result<CdpMessage> {
    serde_json::from_str(json)
        .map_err(|e| CdpServerError::InvalidMessage(format!("Failed to parse CDP message: {}", e)))
}

/// Serialize a CDP message to JSON string
pub fn serialize_cdp_message(message: &CdpMessage) -> Result<String> {
    serde_json::to_string(message).map_err(CdpServerError::from)
}

/// Validate message size
pub fn validate_message_size(message: &str, max_size: usize) -> Result<()> {
    let size = message.len();
    if size > max_size {
        Err(CdpServerError::MessageTooLarge(size, max_size))
    } else {
        Ok(())
    }
}

/// Validate origin header against allowed origins
pub fn validate_origin(origin: &str, allowed_origins: &[String]) -> bool {
    allowed_origins.iter().any(|allowed| {
        if allowed.ends_with('*') {
            // Wildcard match
            let prefix = &allowed[..allowed.len() - 1];
            origin.starts_with(prefix)
        } else {
            // Exact match
            origin == allowed
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_request() {
        let json = r#"{"id": 1, "method": "Runtime.evaluate"}"#;
        let result = parse_cdp_message(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_invalid() {
        let json = "not valid json";
        let result = parse_cdp_message(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_serialize() {
        let response = cdp_types::CdpResponse {
            id: 1,
            result: Some(json!({"value": 42})),
            error: None,
        };
        let result = serialize_cdp_message(&CdpMessage::Response(response));
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_size_ok() {
        let msg = "test";
        assert!(validate_message_size(msg, 1024).is_ok());
    }

    #[test]
    fn test_validate_size_too_large() {
        let msg = "a".repeat(2000);
        assert!(validate_message_size(&msg, 1024).is_err());
    }

    #[test]
    fn test_validate_origin_exact() {
        let origins = vec!["http://localhost:3000".to_string()];
        assert!(validate_origin("http://localhost:3000", &origins));
        assert!(!validate_origin("http://localhost:4000", &origins));
    }

    #[test]
    fn test_validate_origin_wildcard() {
        let origins = vec!["http://localhost:*".to_string()];
        assert!(validate_origin("http://localhost:3000", &origins));
        assert!(validate_origin("http://localhost:9222", &origins));
        assert!(!validate_origin("http://example.com:3000", &origins));
    }
}
