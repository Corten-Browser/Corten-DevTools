//! Unit tests for NetworkDomain
//!
//! Following TDD methodology: RED → GREEN → REFACTOR
//! These tests are written FIRST (RED phase) and will fail until implementation is complete

use network_domain::*;
use protocol_handler::DomainHandler;
use serde_json::json;

#[tokio::test]
async fn test_network_domain_new() {
    // Test that we can create a new NetworkDomain instance
    let domain = NetworkDomain::new();
    assert_eq!(domain.name(), "Network");
}

#[tokio::test]
async fn test_network_domain_enable() {
    // Test enable method with no parameters
    let domain = NetworkDomain::new();
    let result = domain.enable(None).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_network_domain_enable_with_params() {
    // Test enable method with max_total_buffer_size parameter
    let domain = NetworkDomain::new();
    let params = json!({
        "maxTotalBufferSize": 10485760,  // 10MB
        "maxResourceBufferSize": 5242880  // 5MB
    });
    let result = domain.enable(Some(params)).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_network_domain_disable() {
    // Test disable method
    let domain = NetworkDomain::new();

    // Enable first
    domain.enable(None).await.unwrap();

    // Then disable
    let result = domain.disable().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_get_response_body_not_found() {
    // Test get_response_body when request doesn't exist
    let domain = NetworkDomain::new();
    let params = json!({
        "requestId": "non-existent-request"
    });

    let result = domain.get_response_body(Some(params)).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_response_body_success() {
    // Test get_response_body for a tracked request
    let domain = NetworkDomain::new();

    // First, register a mock request
    let request_id = "test-request-123";
    domain.track_request(
        request_id.to_string(),
        "https://example.com".to_string(),
        "GET".to_string(),
    );

    // Store response body
    domain.store_response_body(
        request_id.to_string(),
        "Hello, World!".to_string(),
        false, // not base64 encoded
    );

    // Get the response body
    let params = json!({
        "requestId": request_id
    });

    let result = domain.get_response_body(Some(params)).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response["body"], "Hello, World!");
    assert_eq!(response["base64Encoded"], false);
}

#[tokio::test]
async fn test_get_response_body_base64() {
    // Test get_response_body for binary content (base64 encoded)
    let domain = NetworkDomain::new();

    let request_id = "binary-request-456";
    domain.track_request(
        request_id.to_string(),
        "https://example.com/image.png".to_string(),
        "GET".to_string(),
    );

    // Store binary response (base64 encoded)
    domain.store_response_body(
        request_id.to_string(),
        "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==".to_string(),
        true, // base64 encoded
    );

    let params = json!({
        "requestId": request_id
    });

    let result = domain.get_response_body(Some(params)).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response["base64Encoded"].as_bool().unwrap());
}

#[tokio::test]
async fn test_set_request_interception_enable() {
    // Test enabling request interception with patterns
    let domain = NetworkDomain::new();

    let params = json!({
        "patterns": [
            {
                "urlPattern": "*.example.com/*",
                "resourceType": "Document",
                "interceptionStage": "Request"
            },
            {
                "urlPattern": "*.googleapis.com/*",
                "interceptionStage": "HeadersReceived"
            }
        ]
    });

    let result = domain.set_request_interception(Some(params)).await;
    assert!(result.is_ok());

    // Verify interception is enabled
    assert!(domain.is_interception_enabled());
}

#[tokio::test]
async fn test_set_request_interception_disable() {
    // Test disabling request interception
    let domain = NetworkDomain::new();

    // First enable it
    let enable_params = json!({
        "patterns": [{"urlPattern": "*"}]
    });
    domain
        .set_request_interception(Some(enable_params))
        .await
        .unwrap();

    // Then disable it with empty patterns
    let disable_params = json!({
        "patterns": []
    });

    let result = domain.set_request_interception(Some(disable_params)).await;
    assert!(result.is_ok());

    // Verify interception is disabled
    assert!(!domain.is_interception_enabled());
}

#[tokio::test]
async fn test_track_request() {
    // Test request tracking functionality
    let domain = NetworkDomain::new();

    let request_id = "track-test-789";
    domain.track_request(
        request_id.to_string(),
        "https://api.example.com/users".to_string(),
        "POST".to_string(),
    );

    // Verify request is tracked
    assert!(domain.has_request(request_id));
}

#[tokio::test]
async fn test_untrack_request() {
    // Test removing tracked request
    let domain = NetworkDomain::new();

    let request_id = "untrack-test-999";
    domain.track_request(
        request_id.to_string(),
        "https://example.com".to_string(),
        "GET".to_string(),
    );

    assert!(domain.has_request(request_id));

    domain.untrack_request(request_id);

    assert!(!domain.has_request(request_id));
}

#[tokio::test]
async fn test_domain_handler_trait() {
    // Test that NetworkDomain implements DomainHandler correctly
    use protocol_handler::DomainHandler;

    let domain = NetworkDomain::new();

    // Test name
    assert_eq!(domain.name(), "Network");

    // Test handle_method for enable
    let result = domain.handle_method("enable", None).await;
    assert!(result.is_ok());

    // Test handle_method for unknown method
    let result = domain.handle_method("unknownMethod", None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_handle_method_enable() {
    // Test handle_method routing for enable
    use protocol_handler::DomainHandler;

    let domain = NetworkDomain::new();
    let result = domain.handle_method("enable", None).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), json!({}));
}

#[tokio::test]
async fn test_handle_method_disable() {
    // Test handle_method routing for disable
    use protocol_handler::DomainHandler;

    let domain = NetworkDomain::new();
    let result = domain.handle_method("disable", None).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), json!({}));
}

#[tokio::test]
async fn test_handle_method_get_response_body() {
    // Test handle_method routing for getResponseBody
    use protocol_handler::DomainHandler;

    let domain = NetworkDomain::new();

    // Track a request first
    let request_id = "method-test-111";
    domain.track_request(
        request_id.to_string(),
        "https://example.com".to_string(),
        "GET".to_string(),
    );

    domain.store_response_body(request_id.to_string(), "Test Body".to_string(), false);

    let params = json!({
        "requestId": request_id
    });

    let result = domain.handle_method("getResponseBody", Some(params)).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response["body"], "Test Body");
}

#[tokio::test]
async fn test_handle_method_set_request_interception() {
    // Test handle_method routing for setRequestInterception
    use protocol_handler::DomainHandler;

    let domain = NetworkDomain::new();

    let params = json!({
        "patterns": [
            {"urlPattern": "*.example.com/*"}
        ]
    });

    let result = domain
        .handle_method("setRequestInterception", Some(params))
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_concurrent_request_tracking() {
    // Test thread-safe concurrent request tracking
    use std::sync::Arc;

    let domain = Arc::new(NetworkDomain::new());
    let mut handles = vec![];

    // Spawn 10 concurrent tasks that track requests
    for i in 0..10 {
        let domain_clone = Arc::clone(&domain);
        let handle = tokio::spawn(async move {
            let request_id = format!("concurrent-{}", i);
            domain_clone.track_request(
                request_id.clone(),
                format!("https://example.com/{}", i),
                "GET".to_string(),
            );

            // Verify it was tracked
            assert!(domain_clone.has_request(&request_id));
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }

    // Verify all requests are tracked
    for i in 0..10 {
        let request_id = format!("concurrent-{}", i);
        assert!(domain.has_request(&request_id));
    }
}

#[tokio::test]
async fn test_missing_params_error() {
    // Test that methods requiring params return error when params are None
    use protocol_handler::DomainHandler;

    let domain = NetworkDomain::new();

    // getResponseBody requires requestId param
    let result = domain.handle_method("getResponseBody", None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_invalid_params_error() {
    // Test that methods return error when params are invalid
    use protocol_handler::DomainHandler;

    let domain = NetworkDomain::new();

    // getResponseBody with invalid params structure
    let params = json!({
        "wrongField": "value"
    });

    let result = domain.handle_method("getResponseBody", Some(params)).await;
    assert!(result.is_err());
}
