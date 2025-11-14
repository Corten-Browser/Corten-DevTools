//! Integration tests for NetworkDomain with ProtocolHandler
//!
//! Tests that NetworkDomain integrates correctly with the protocol_handler component

use network_domain::NetworkDomain;
use protocol_handler::ProtocolHandler;
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn test_network_domain_registration() {
    // Test registering NetworkDomain with ProtocolHandler
    let handler = ProtocolHandler::new();
    let network_domain = Arc::new(NetworkDomain::new());

    handler.register_domain(network_domain);

    // Verify domain is registered by sending a message
    let message = json!({
        "id": 1,
        "method": "Network.enable"
    });

    let response = handler.handle_message(&message.to_string()).await;
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    assert_eq!(response_json["id"], 1);
    assert!(response_json["result"].is_object());
    assert!(response_json["error"].is_null());
}

#[tokio::test]
async fn test_network_enable_through_protocol_handler() {
    // Test Network.enable through ProtocolHandler
    let handler = ProtocolHandler::new();
    let network_domain = Arc::new(NetworkDomain::new());

    handler.register_domain(network_domain);

    let message = json!({
        "id": 1,
        "method": "Network.enable",
        "params": {
            "maxTotalBufferSize": 10485760
        }
    });

    let response = handler.handle_message(&message.to_string()).await;
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    assert_eq!(response_json["id"], 1);
    assert!(response_json["error"].is_null());
}

#[tokio::test]
async fn test_network_disable_through_protocol_handler() {
    // Test Network.disable through ProtocolHandler
    let handler = ProtocolHandler::new();
    let network_domain = Arc::new(NetworkDomain::new());

    handler.register_domain(network_domain);

    let message = json!({
        "id": 2,
        "method": "Network.disable"
    });

    let response = handler.handle_message(&message.to_string()).await;
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    assert_eq!(response_json["id"], 2);
    assert!(response_json["error"].is_null());
}

#[tokio::test]
async fn test_network_get_response_body_through_protocol_handler() {
    // Test Network.getResponseBody through ProtocolHandler
    let handler = ProtocolHandler::new();
    let network_domain = Arc::new(NetworkDomain::new());

    // Track a request and store response
    network_domain.track_request(
        "test-123".to_string(),
        "https://example.com".to_string(),
        "GET".to_string(),
    );
    network_domain.store_response_body("test-123".to_string(), "Response Body".to_string(), false);

    handler.register_domain(network_domain);

    let message = json!({
        "id": 3,
        "method": "Network.getResponseBody",
        "params": {
            "requestId": "test-123"
        }
    });

    let response = handler.handle_message(&message.to_string()).await;
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    assert_eq!(response_json["id"], 3);
    assert!(response_json["error"].is_null());
    assert_eq!(response_json["result"]["body"], "Response Body");
    assert_eq!(response_json["result"]["base64Encoded"], false);
}

#[tokio::test]
async fn test_network_set_request_interception_through_protocol_handler() {
    // Test Network.setRequestInterception through ProtocolHandler
    let handler = ProtocolHandler::new();
    let network_domain = Arc::new(NetworkDomain::new());

    handler.register_domain(network_domain);

    let message = json!({
        "id": 4,
        "method": "Network.setRequestInterception",
        "params": {
            "patterns": [
                {
                    "urlPattern": "*.example.com/*",
                    "resourceType": "Document"
                }
            ]
        }
    });

    let response = handler.handle_message(&message.to_string()).await;
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    assert_eq!(response_json["id"], 4);
    assert!(response_json["error"].is_null());
}

#[tokio::test]
async fn test_network_unknown_method() {
    // Test that unknown Network methods return error
    let handler = ProtocolHandler::new();
    let network_domain = Arc::new(NetworkDomain::new());

    handler.register_domain(network_domain);

    let message = json!({
        "id": 5,
        "method": "Network.unknownMethod"
    });

    let response = handler.handle_message(&message.to_string()).await;
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    assert_eq!(response_json["id"], 5);
    assert!(response_json["error"].is_object());
    assert_eq!(response_json["error"]["code"], -32601); // Method not found
}

#[tokio::test]
async fn test_network_missing_request_id() {
    // Test that getResponseBody returns error when requestId is missing
    let handler = ProtocolHandler::new();
    let network_domain = Arc::new(NetworkDomain::new());

    handler.register_domain(network_domain);

    let message = json!({
        "id": 6,
        "method": "Network.getResponseBody",
        "params": {}
    });

    let response = handler.handle_message(&message.to_string()).await;
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    assert_eq!(response_json["id"], 6);
    assert!(response_json["error"].is_object());
}

#[tokio::test]
async fn test_network_request_not_found() {
    // Test that getResponseBody returns error when request doesn't exist
    let handler = ProtocolHandler::new();
    let network_domain = Arc::new(NetworkDomain::new());

    handler.register_domain(network_domain);

    let message = json!({
        "id": 7,
        "method": "Network.getResponseBody",
        "params": {
            "requestId": "non-existent"
        }
    });

    let response = handler.handle_message(&message.to_string()).await;
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    assert_eq!(response_json["id"], 7);
    assert!(response_json["error"].is_object());
}

#[tokio::test]
async fn test_multiple_network_operations() {
    // Test sequence of Network operations
    let handler = ProtocolHandler::new();
    let network_domain = Arc::new(NetworkDomain::new());

    handler.register_domain(network_domain);

    // 1. Enable network monitoring
    let enable_msg = json!({
        "id": 1,
        "method": "Network.enable"
    });
    let response = handler.handle_message(&enable_msg.to_string()).await;
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();
    assert_eq!(response_json["id"], 1);
    assert!(response_json["error"].is_null());

    // 2. Set request interception
    let intercept_msg = json!({
        "id": 2,
        "method": "Network.setRequestInterception",
        "params": {
            "patterns": [{"urlPattern": "*"}]
        }
    });
    let response = handler.handle_message(&intercept_msg.to_string()).await;
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();
    assert_eq!(response_json["id"], 2);
    assert!(response_json["error"].is_null());

    // 3. Disable network monitoring
    let disable_msg = json!({
        "id": 3,
        "method": "Network.disable"
    });
    let response = handler.handle_message(&disable_msg.to_string()).await;
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();
    assert_eq!(response_json["id"], 3);
    assert!(response_json["error"].is_null());
}
