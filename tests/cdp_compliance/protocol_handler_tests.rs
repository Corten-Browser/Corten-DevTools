//! Protocol Handler Compliance Tests (FEAT-043)
//!
//! Tests the protocol handler routing and domain registration functionality.

use profiler_domains::{HeapProfilerDomain, ProfilerDomain, TimelineDomain};
use protocol_handler::{DomainHandler, ProtocolHandler};
use serde_json::{json, Value};
use std::sync::Arc;

// ============================================================================
// Domain Registration Tests
// ============================================================================

#[tokio::test]
async fn test_register_profiler_domain() {
    let handler = ProtocolHandler::new();
    let domain = Arc::new(ProfilerDomain::new());

    handler.register_domain(domain);

    let request = json!({
        "id": 1,
        "method": "Profiler.enable"
    });

    let response = handler.handle_message(&request.to_string()).await;
    let response_json: Value = serde_json::from_str(&response).unwrap();

    assert!(response_json.get("error").is_none());
}

#[tokio::test]
async fn test_register_heap_profiler_domain() {
    let handler = ProtocolHandler::new();
    let domain = Arc::new(HeapProfilerDomain::new());

    handler.register_domain(domain);

    let request = json!({
        "id": 1,
        "method": "HeapProfiler.enable"
    });

    let response = handler.handle_message(&request.to_string()).await;
    let response_json: Value = serde_json::from_str(&response).unwrap();

    assert!(response_json.get("error").is_none());
}

#[tokio::test]
async fn test_register_timeline_domain() {
    let handler = ProtocolHandler::new();
    let domain = Arc::new(TimelineDomain::new());

    handler.register_domain(domain);

    let request = json!({
        "id": 1,
        "method": "Timeline.enable"
    });

    let response = handler.handle_message(&request.to_string()).await;
    let response_json: Value = serde_json::from_str(&response).unwrap();

    assert!(response_json.get("error").is_none());
}

#[tokio::test]
async fn test_register_multiple_domains() {
    let handler = ProtocolHandler::new();

    handler.register_domain(Arc::new(ProfilerDomain::new()));
    handler.register_domain(Arc::new(HeapProfilerDomain::new()));
    handler.register_domain(Arc::new(TimelineDomain::new()));

    // Test Profiler
    let request1 = json!({
        "id": 1,
        "method": "Profiler.enable"
    });
    let response1 = handler.handle_message(&request1.to_string()).await;
    let json1: Value = serde_json::from_str(&response1).unwrap();
    assert!(json1.get("error").is_none());

    // Test HeapProfiler
    let request2 = json!({
        "id": 2,
        "method": "HeapProfiler.enable"
    });
    let response2 = handler.handle_message(&request2.to_string()).await;
    let json2: Value = serde_json::from_str(&response2).unwrap();
    assert!(json2.get("error").is_none());

    // Test Timeline
    let request3 = json!({
        "id": 3,
        "method": "Timeline.enable"
    });
    let response3 = handler.handle_message(&request3.to_string()).await;
    let json3: Value = serde_json::from_str(&response3).unwrap();
    assert!(json3.get("error").is_none());
}

#[tokio::test]
async fn test_unregister_domain() {
    let handler = ProtocolHandler::new();
    handler.register_domain(Arc::new(ProfilerDomain::new()));

    // Should work before unregistration
    let request1 = json!({
        "id": 1,
        "method": "Profiler.enable"
    });
    let response1 = handler.handle_message(&request1.to_string()).await;
    let json1: Value = serde_json::from_str(&response1).unwrap();
    assert!(json1.get("error").is_none());

    // Unregister
    let removed = handler.unregister_domain("Profiler");
    assert!(removed.is_some());

    // Should fail after unregistration
    let request2 = json!({
        "id": 2,
        "method": "Profiler.enable"
    });
    let response2 = handler.handle_message(&request2.to_string()).await;
    let json2: Value = serde_json::from_str(&response2).unwrap();
    assert!(json2.get("error").is_some());
}

// ============================================================================
// Request ID Tests
// ============================================================================

#[tokio::test]
async fn test_response_includes_request_id() {
    let handler = ProtocolHandler::new();
    handler.register_domain(Arc::new(ProfilerDomain::new()));

    for id in [1u64, 100, 999999] {
        let request = json!({
            "id": id,
            "method": "Profiler.enable"
        });

        let response = handler.handle_message(&request.to_string()).await;
        let response_json: Value = serde_json::from_str(&response).unwrap();

        assert_eq!(response_json["id"], id);
    }
}

#[tokio::test]
async fn test_error_response_includes_request_id() {
    let handler = ProtocolHandler::new();

    let request = json!({
        "id": 42,
        "method": "Unknown.method"
    });

    let response = handler.handle_message(&request.to_string()).await;
    let response_json: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(response_json["id"], 42);
}

// ============================================================================
// Method Routing Tests
// ============================================================================

#[tokio::test]
async fn test_method_routing_profiler_enable() {
    let handler = ProtocolHandler::new();
    handler.register_domain(Arc::new(ProfilerDomain::new()));

    let request = json!({
        "id": 1,
        "method": "Profiler.enable"
    });

    let response = handler.handle_message(&request.to_string()).await;
    let response_json: Value = serde_json::from_str(&response).unwrap();

    assert!(response_json.get("result").is_some());
    assert_eq!(response_json["result"], json!({}));
}

#[tokio::test]
async fn test_method_routing_profiler_start_stop() {
    let handler = ProtocolHandler::new();
    handler.register_domain(Arc::new(ProfilerDomain::new()));

    // Enable
    let enable_req = json!({ "id": 1, "method": "Profiler.enable" });
    handler.handle_message(&enable_req.to_string()).await;

    // Start
    let start_req = json!({ "id": 2, "method": "Profiler.start" });
    let start_response = handler.handle_message(&start_req.to_string()).await;
    let start_json: Value = serde_json::from_str(&start_response).unwrap();
    assert!(start_json.get("error").is_none());

    // Stop
    let stop_req = json!({ "id": 3, "method": "Profiler.stop" });
    let stop_response = handler.handle_message(&stop_req.to_string()).await;
    let stop_json: Value = serde_json::from_str(&stop_response).unwrap();
    assert!(stop_json.get("error").is_none());
    assert!(stop_json.get("result").unwrap().get("profile").is_some());
}

#[tokio::test]
async fn test_method_routing_heap_profiler_workflow() {
    let handler = ProtocolHandler::new();
    handler.register_domain(Arc::new(HeapProfilerDomain::new()));

    // Enable
    let enable_req = json!({ "id": 1, "method": "HeapProfiler.enable" });
    handler.handle_message(&enable_req.to_string()).await;

    // Start sampling
    let start_req = json!({ "id": 2, "method": "HeapProfiler.startSampling" });
    let start_response = handler.handle_message(&start_req.to_string()).await;
    let start_json: Value = serde_json::from_str(&start_response).unwrap();
    assert!(start_json.get("error").is_none());

    // Stop sampling
    let stop_req = json!({ "id": 3, "method": "HeapProfiler.stopSampling" });
    let stop_response = handler.handle_message(&stop_req.to_string()).await;
    let stop_json: Value = serde_json::from_str(&stop_response).unwrap();
    assert!(stop_json.get("error").is_none());
}

#[tokio::test]
async fn test_method_routing_timeline_workflow() {
    let handler = ProtocolHandler::new();
    handler.register_domain(Arc::new(TimelineDomain::new()));

    // Enable
    let enable_req = json!({ "id": 1, "method": "Timeline.enable" });
    handler.handle_message(&enable_req.to_string()).await;

    // Start
    let start_req = json!({ "id": 2, "method": "Timeline.start" });
    let start_response = handler.handle_message(&start_req.to_string()).await;
    let start_json: Value = serde_json::from_str(&start_response).unwrap();
    assert!(start_json.get("error").is_none());

    // Record event
    let event_req = json!({
        "id": 3,
        "method": "Timeline.recordEvent",
        "params": {
            "type": "TestEvent",
            "category": "scripting"
        }
    });
    let event_response = handler.handle_message(&event_req.to_string()).await;
    let event_json: Value = serde_json::from_str(&event_response).unwrap();
    assert!(event_json.get("error").is_none());

    // Stop
    let stop_req = json!({ "id": 4, "method": "Timeline.stop" });
    let stop_response = handler.handle_message(&stop_req.to_string()).await;
    let stop_json: Value = serde_json::from_str(&stop_response).unwrap();
    assert!(stop_json.get("error").is_none());
    assert!(stop_json.get("result").unwrap().get("timeline").is_some());
}

// ============================================================================
// Params Passing Tests
// ============================================================================

#[tokio::test]
async fn test_params_passed_correctly() {
    let handler = ProtocolHandler::new();
    handler.register_domain(Arc::new(ProfilerDomain::new()));

    let request = json!({
        "id": 1,
        "method": "Profiler.setSamplingInterval",
        "params": {
            "interval": 500
        }
    });

    let response = handler.handle_message(&request.to_string()).await;
    let response_json: Value = serde_json::from_str(&response).unwrap();

    assert!(response_json.get("error").is_none());
}

#[tokio::test]
async fn test_params_empty_object() {
    let handler = ProtocolHandler::new();
    handler.register_domain(Arc::new(ProfilerDomain::new()));

    let request = json!({
        "id": 1,
        "method": "Profiler.enable",
        "params": {}
    });

    let response = handler.handle_message(&request.to_string()).await;
    let response_json: Value = serde_json::from_str(&response).unwrap();

    assert!(response_json.get("error").is_none());
}

#[tokio::test]
async fn test_params_not_provided() {
    let handler = ProtocolHandler::new();
    handler.register_domain(Arc::new(ProfilerDomain::new()));

    let request = json!({
        "id": 1,
        "method": "Profiler.enable"
    });

    let response = handler.handle_message(&request.to_string()).await;
    let response_json: Value = serde_json::from_str(&response).unwrap();

    assert!(response_json.get("error").is_none());
}

// ============================================================================
// Concurrent Request Tests
// ============================================================================

#[tokio::test]
async fn test_concurrent_requests_different_domains() {
    let handler = Arc::new(ProtocolHandler::new());
    handler.register_domain(Arc::new(ProfilerDomain::new()));
    handler.register_domain(Arc::new(HeapProfilerDomain::new()));
    handler.register_domain(Arc::new(TimelineDomain::new()));

    let handler1 = handler.clone();
    let handler2 = handler.clone();
    let handler3 = handler.clone();

    let task1 = tokio::spawn(async move {
        let req = json!({ "id": 1, "method": "Profiler.enable" });
        handler1.handle_message(&req.to_string()).await
    });

    let task2 = tokio::spawn(async move {
        let req = json!({ "id": 2, "method": "HeapProfiler.enable" });
        handler2.handle_message(&req.to_string()).await
    });

    let task3 = tokio::spawn(async move {
        let req = json!({ "id": 3, "method": "Timeline.enable" });
        handler3.handle_message(&req.to_string()).await
    });

    let (r1, r2, r3) = tokio::try_join!(task1, task2, task3).unwrap();

    let json1: Value = serde_json::from_str(&r1).unwrap();
    let json2: Value = serde_json::from_str(&r2).unwrap();
    let json3: Value = serde_json::from_str(&r3).unwrap();

    assert!(json1.get("error").is_none());
    assert!(json2.get("error").is_none());
    assert!(json3.get("error").is_none());
}

// ============================================================================
// Domain State Isolation Tests
// ============================================================================

#[tokio::test]
async fn test_domain_state_isolation() {
    let handler = ProtocolHandler::new();
    let profiler1 = Arc::new(ProfilerDomain::new());
    let profiler_clone = profiler1.clone();

    handler.register_domain(profiler1);

    // Enable via handler
    let enable_req = json!({ "id": 1, "method": "Profiler.enable" });
    handler.handle_message(&enable_req.to_string()).await;

    // Start via handler
    let start_req = json!({ "id": 2, "method": "Profiler.start" });
    let start_response = handler.handle_message(&start_req.to_string()).await;
    let start_json: Value = serde_json::from_str(&start_response).unwrap();
    assert!(start_json.get("error").is_none());

    // Verify state via clone
    assert!(profiler_clone.is_profiling());
}

// ============================================================================
// Response Format Tests
// ============================================================================

#[tokio::test]
async fn test_success_response_format() {
    let handler = ProtocolHandler::new();
    handler.register_domain(Arc::new(ProfilerDomain::new()));

    let request = json!({ "id": 1, "method": "Profiler.enable" });
    let response = handler.handle_message(&request.to_string()).await;
    let response_json: Value = serde_json::from_str(&response).unwrap();

    // Must have id
    assert!(response_json.get("id").is_some());

    // Must have result
    assert!(response_json.get("result").is_some());

    // Must NOT have error
    assert!(response_json.get("error").is_none());
}

#[tokio::test]
async fn test_error_response_format() {
    let handler = ProtocolHandler::new();

    let request = json!({ "id": 1, "method": "Unknown.method" });
    let response = handler.handle_message(&request.to_string()).await;
    let response_json: Value = serde_json::from_str(&response).unwrap();

    // Must have id
    assert!(response_json.get("id").is_some());

    // Must have error
    assert!(response_json.get("error").is_some());

    // Error must have code and message
    let error = &response_json["error"];
    assert!(error.get("code").is_some());
    assert!(error.get("message").is_some());
}

// ============================================================================
// Method Case Sensitivity Tests
// ============================================================================

#[tokio::test]
async fn test_method_case_sensitive() {
    let handler = ProtocolHandler::new();
    handler.register_domain(Arc::new(ProfilerDomain::new()));

    // Correct case should work
    let req1 = json!({ "id": 1, "method": "Profiler.enable" });
    let resp1 = handler.handle_message(&req1.to_string()).await;
    let json1: Value = serde_json::from_str(&resp1).unwrap();
    assert!(json1.get("error").is_none());

    // Wrong case should fail (domain name)
    let req2 = json!({ "id": 2, "method": "profiler.enable" });
    let resp2 = handler.handle_message(&req2.to_string()).await;
    let json2: Value = serde_json::from_str(&resp2).unwrap();
    assert!(json2.get("error").is_some());

    // Wrong case should fail (method name)
    let req3 = json!({ "id": 3, "method": "Profiler.Enable" });
    let resp3 = handler.handle_message(&req3.to_string()).await;
    let json3: Value = serde_json::from_str(&resp3).unwrap();
    assert!(json3.get("error").is_some());
}
