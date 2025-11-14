// Integration tests for protocol_handler component
// These tests define the expected behavior before implementation (TDD RED phase)

use async_trait::async_trait;
use protocol_handler::{DomainHandler, ProtocolHandler};
use serde_json::json;

// Mock domain handler for testing
struct MockDomainHandler {
    name: String,
}

impl MockDomainHandler {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

#[async_trait]
impl DomainHandler for MockDomainHandler {
    fn name(&self) -> &str {
        &self.name
    }

    async fn handle_method(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, cdp_types::CdpError> {
        match method {
            "echo" => Ok(params.unwrap_or(json!(null))),
            "getValue" => Ok(json!({"value": 42})),
            "error" => Err(cdp_types::CdpError::internal_error("Test error")),
            _ => Err(cdp_types::CdpError::method_not_found(format!(
                "{}.{}",
                self.name, method
            ))),
        }
    }
}

#[tokio::test]
async fn test_protocol_handler_creation() {
    let _handler = ProtocolHandler::new();
    assert!(true, "ProtocolHandler should be created successfully");
}

#[tokio::test]
async fn test_register_domain() {
    let handler = ProtocolHandler::new();
    let mock_domain = std::sync::Arc::new(MockDomainHandler::new("TestDomain"));

    handler.register_domain(mock_domain);
    // Registration should succeed without panic
}

#[tokio::test]
async fn test_handle_valid_message() {
    let handler = ProtocolHandler::new();
    let mock_domain = std::sync::Arc::new(MockDomainHandler::new("TestDomain"));
    handler.register_domain(mock_domain);

    let request = json!({
        "id": 1,
        "method": "TestDomain.echo",
        "params": {"test": "data"}
    });

    let response = handler.handle_message(&request.to_string()).await;
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    assert_eq!(response_json["id"], 1);
    assert!(response_json["result"].is_object());
    assert_eq!(response_json["result"]["test"], "data");
}

#[tokio::test]
async fn test_handle_message_no_params() {
    let handler = ProtocolHandler::new();
    let mock_domain = std::sync::Arc::new(MockDomainHandler::new("TestDomain"));
    handler.register_domain(mock_domain);

    let request = json!({
        "id": 2,
        "method": "TestDomain.getValue"
    });

    let response = handler.handle_message(&request.to_string()).await;
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    assert_eq!(response_json["id"], 2);
    assert_eq!(response_json["result"]["value"], 42);
}

#[tokio::test]
async fn test_handle_unknown_domain() {
    let handler = ProtocolHandler::new();

    let request = json!({
        "id": 3,
        "method": "UnknownDomain.method"
    });

    let response = handler.handle_message(&request.to_string()).await;
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    assert_eq!(response_json["id"], 3);
    assert!(response_json["error"].is_object());
    assert_eq!(response_json["error"]["code"], -32601); // Method not found
}

#[tokio::test]
async fn test_handle_unknown_method() {
    let handler = ProtocolHandler::new();
    let mock_domain = std::sync::Arc::new(MockDomainHandler::new("TestDomain"));
    handler.register_domain(mock_domain);

    let request = json!({
        "id": 4,
        "method": "TestDomain.unknownMethod"
    });

    let response = handler.handle_message(&request.to_string()).await;
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    assert_eq!(response_json["id"], 4);
    assert!(response_json["error"].is_object());
    assert_eq!(response_json["error"]["code"], -32601);
}

#[tokio::test]
async fn test_handle_domain_error() {
    let handler = ProtocolHandler::new();
    let mock_domain = std::sync::Arc::new(MockDomainHandler::new("TestDomain"));
    handler.register_domain(mock_domain);

    let request = json!({
        "id": 5,
        "method": "TestDomain.error"
    });

    let response = handler.handle_message(&request.to_string()).await;
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    assert_eq!(response_json["id"], 5);
    assert!(response_json["error"].is_object());
    assert_eq!(response_json["error"]["code"], -32603); // Internal error
}

#[tokio::test]
async fn test_invalid_json() {
    let handler = ProtocolHandler::new();

    let response = handler.handle_message("invalid json {{{").await;
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    assert!(response_json["error"].is_object());
    assert_eq!(response_json["error"]["code"], -32700); // Parse error
}

#[tokio::test]
async fn test_missing_method_field() {
    let handler = ProtocolHandler::new();

    let request = json!({
        "id": 6
        // Missing "method" field
    });

    let response = handler.handle_message(&request.to_string()).await;
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    assert!(response_json["error"].is_object());
    assert_eq!(response_json["error"]["code"], -32600); // Invalid request
}

#[tokio::test]
async fn test_invalid_method_format() {
    let handler = ProtocolHandler::new();

    let request = json!({
        "id": 7,
        "method": "InvalidMethodFormat" // Should be "Domain.method"
    });

    let response = handler.handle_message(&request.to_string()).await;
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    assert_eq!(response_json["id"], 7);
    assert!(response_json["error"].is_object());
    assert_eq!(response_json["error"]["code"], -32600); // Invalid request
}

#[tokio::test]
async fn test_concurrent_message_handling() {
    let handler = std::sync::Arc::new(ProtocolHandler::new());
    let mock_domain = std::sync::Arc::new(MockDomainHandler::new("TestDomain"));
    handler.register_domain(mock_domain);

    let mut handles = vec![];

    for i in 1..=10 {
        let handler_clone = handler.clone();
        let handle = tokio::spawn(async move {
            let request = json!({
                "id": i,
                "method": "TestDomain.getValue"
            });

            let response = handler_clone.handle_message(&request.to_string()).await;
            let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

            assert_eq!(response_json["id"], i);
            assert_eq!(response_json["result"]["value"], 42);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }
}

#[tokio::test]
async fn test_multiple_domains() {
    let handler = ProtocolHandler::new();
    let domain1 = std::sync::Arc::new(MockDomainHandler::new("Domain1"));
    let domain2 = std::sync::Arc::new(MockDomainHandler::new("Domain2"));

    handler.register_domain(domain1);
    handler.register_domain(domain2);

    // Test Domain1
    let request1 = json!({
        "id": 1,
        "method": "Domain1.getValue"
    });
    let response1 = handler.handle_message(&request1.to_string()).await;
    let response1_json: serde_json::Value = serde_json::from_str(&response1).unwrap();
    assert_eq!(response1_json["result"]["value"], 42);

    // Test Domain2
    let request2 = json!({
        "id": 2,
        "method": "Domain2.getValue"
    });
    let response2 = handler.handle_message(&request2.to_string()).await;
    let response2_json: serde_json::Value = serde_json::from_str(&response2).unwrap();
    assert_eq!(response2_json["result"]["value"], 42);
}
