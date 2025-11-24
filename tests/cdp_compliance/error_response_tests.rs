//! Error Response Tests (FEAT-043)
//!
//! Tests that CDP domains return proper error responses for various error conditions.

use profiler_domains::{HeapProfilerDomain, ProfilerDomain, TimelineDomain};
use protocol_handler::{DomainHandler, ProtocolHandler};
use serde_json::{json, Value};
use std::sync::Arc;

// ============================================================================
// Unknown Method Error Tests
// ============================================================================

#[tokio::test]
async fn test_profiler_unknown_method() {
    let domain = ProfilerDomain::new();

    let result = domain.handle_method("unknownMethod", None).await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    // CDP error code for "Method not found" is -32601
    assert_eq!(err.code, -32601);
}

#[tokio::test]
async fn test_heap_profiler_unknown_method() {
    let domain = HeapProfilerDomain::new();

    let result = domain.handle_method("unknownMethod", None).await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert_eq!(err.code, -32601);
}

#[tokio::test]
async fn test_timeline_unknown_method() {
    let domain = TimelineDomain::new();

    let result = domain.handle_method("unknownMethod", None).await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert_eq!(err.code, -32601);
}

// ============================================================================
// Invalid State Error Tests
// ============================================================================

#[tokio::test]
async fn test_profiler_start_not_enabled_error() {
    let domain = ProfilerDomain::new();

    let result = domain.handle_method("start", None).await;
    assert!(result.is_err());

    // Invalid request error
    let err = result.unwrap_err();
    assert_eq!(err.code, -32600);
}

#[tokio::test]
async fn test_profiler_stop_not_started_error() {
    let domain = ProfilerDomain::new();
    domain.handle_method("enable", None).await.unwrap();

    let result = domain.handle_method("stop", None).await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert_eq!(err.code, -32600);
}

#[tokio::test]
async fn test_heap_profiler_sample_not_enabled_error() {
    let domain = HeapProfilerDomain::new();

    let result = domain.handle_method("startSampling", None).await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert_eq!(err.code, -32600);
}

#[tokio::test]
async fn test_heap_profiler_stop_sample_not_started_error() {
    let domain = HeapProfilerDomain::new();
    domain.handle_method("enable", None).await.unwrap();

    let result = domain.handle_method("stopSampling", None).await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert_eq!(err.code, -32600);
}

#[tokio::test]
async fn test_timeline_start_not_enabled_error() {
    let domain = TimelineDomain::new();

    let result = domain.handle_method("start", None).await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert_eq!(err.code, -32600);
}

#[tokio::test]
async fn test_timeline_stop_not_started_error() {
    let domain = TimelineDomain::new();
    domain.handle_method("enable", None).await.unwrap();

    let result = domain.handle_method("stop", None).await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert_eq!(err.code, -32600);
}

// ============================================================================
// Invalid Parameter Error Tests
// ============================================================================

#[tokio::test]
async fn test_profiler_set_interval_missing_param_error() {
    let domain = ProfilerDomain::new();

    let result = domain
        .handle_method("setSamplingInterval", Some(json!({})))
        .await;
    assert!(result.is_err());

    // Invalid params error
    let err = result.unwrap_err();
    assert_eq!(err.code, -32602);
}

#[tokio::test]
async fn test_timeline_record_event_missing_type_error() {
    let domain = TimelineDomain::new();
    domain.handle_method("enable", None).await.unwrap();
    domain.handle_method("start", None).await.unwrap();

    let result = domain
        .handle_method("recordEvent", Some(json!({ "category": "scripting" })))
        .await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert_eq!(err.code, -32602);
}

#[tokio::test]
async fn test_timeline_record_event_null_params_error() {
    let domain = TimelineDomain::new();
    domain.handle_method("enable", None).await.unwrap();
    domain.handle_method("start", None).await.unwrap();

    let result = domain.handle_method("recordEvent", None).await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert_eq!(err.code, -32602);
}

#[tokio::test]
async fn test_timeline_record_frame_null_params_error() {
    let domain = TimelineDomain::new();
    domain.handle_method("enable", None).await.unwrap();
    domain.handle_method("start", None).await.unwrap();

    let result = domain.handle_method("recordFrame", None).await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert_eq!(err.code, -32602);
}

// ============================================================================
// Protocol Handler Error Tests
// ============================================================================

#[tokio::test]
async fn test_protocol_handler_unknown_domain() {
    let handler = ProtocolHandler::new();

    let request = json!({
        "id": 1,
        "method": "UnknownDomain.method"
    });

    let response = handler.handle_message(&request.to_string()).await;
    let response_json: Value = serde_json::from_str(&response).unwrap();

    assert!(response_json.get("error").is_some());
    assert_eq!(response_json["error"]["code"], -32601);
}

#[tokio::test]
async fn test_protocol_handler_invalid_method_format() {
    let handler = ProtocolHandler::new();

    let request = json!({
        "id": 1,
        "method": "InvalidMethodFormat"
    });

    let response = handler.handle_message(&request.to_string()).await;
    let response_json: Value = serde_json::from_str(&response).unwrap();

    assert!(response_json.get("error").is_some());
    assert_eq!(response_json["error"]["code"], -32600);
}

#[tokio::test]
async fn test_protocol_handler_parse_error() {
    let handler = ProtocolHandler::new();

    let response = handler.handle_message("invalid json {{{").await;
    let response_json: Value = serde_json::from_str(&response).unwrap();

    assert!(response_json.get("error").is_some());
    // Parse error code
    assert_eq!(response_json["error"]["code"], -32700);
}

#[tokio::test]
async fn test_protocol_handler_missing_method() {
    let handler = ProtocolHandler::new();

    let request = json!({
        "id": 1
    });

    let response = handler.handle_message(&request.to_string()).await;
    let response_json: Value = serde_json::from_str(&response).unwrap();

    assert!(response_json.get("error").is_some());
}

#[tokio::test]
async fn test_protocol_handler_empty_method() {
    let handler = ProtocolHandler::new();

    let request = json!({
        "id": 1,
        "method": ""
    });

    let response = handler.handle_message(&request.to_string()).await;
    let response_json: Value = serde_json::from_str(&response).unwrap();

    assert!(response_json.get("error").is_some());
    assert_eq!(response_json["error"]["code"], -32600);
}

// ============================================================================
// Error Message Content Tests
// ============================================================================

#[tokio::test]
async fn test_profiler_unknown_method_error_message() {
    let domain = ProfilerDomain::new();

    let result = domain.handle_method("nonExistent", None).await;
    let err = result.unwrap_err();

    // Error should have a message and the method info in data field
    assert!(!err.message.is_empty());
    assert_eq!(err.code, -32601); // Method not found

    // Method name is in the data field
    if let Some(data) = &err.data {
        let method_str = data.get("method").and_then(|m| m.as_str()).unwrap_or("");
        assert!(method_str.contains("nonExistent") || method_str.contains("Profiler"));
    }
}

#[tokio::test]
async fn test_heap_profiler_unknown_method_error_message() {
    let domain = HeapProfilerDomain::new();

    let result = domain.handle_method("nonExistent", None).await;
    let err = result.unwrap_err();

    // Error should have a message and the method info in data field
    assert!(!err.message.is_empty());
    assert_eq!(err.code, -32601); // Method not found

    // Method name is in the data field
    if let Some(data) = &err.data {
        let method_str = data.get("method").and_then(|m| m.as_str()).unwrap_or("");
        assert!(method_str.contains("nonExistent") || method_str.contains("HeapProfiler"));
    }
}

#[tokio::test]
async fn test_timeline_unknown_method_error_message() {
    let domain = TimelineDomain::new();

    let result = domain.handle_method("nonExistent", None).await;
    let err = result.unwrap_err();

    // Error should have a message and the method info in data field
    assert!(!err.message.is_empty());
    assert_eq!(err.code, -32601); // Method not found

    // Method name is in the data field
    if let Some(data) = &err.data {
        let method_str = data.get("method").and_then(|m| m.as_str()).unwrap_or("");
        assert!(method_str.contains("nonExistent") || method_str.contains("Timeline"));
    }
}

// ============================================================================
// Protocol Handler with Registered Domain Tests
// ============================================================================

#[tokio::test]
async fn test_protocol_handler_success_response() {
    let handler = ProtocolHandler::new();
    let domain = Arc::new(ProfilerDomain::new());
    handler.register_domain(domain);

    let request = json!({
        "id": 1,
        "method": "Profiler.enable"
    });

    let response = handler.handle_message(&request.to_string()).await;
    let response_json: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(response_json["id"], 1);
    assert!(response_json.get("result").is_some());
    assert!(response_json.get("error").is_none());
}

#[tokio::test]
async fn test_protocol_handler_error_response_format() {
    let handler = ProtocolHandler::new();
    let domain = Arc::new(ProfilerDomain::new());
    handler.register_domain(domain);

    let request = json!({
        "id": 1,
        "method": "Profiler.unknownMethod"
    });

    let response = handler.handle_message(&request.to_string()).await;
    let response_json: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(response_json["id"], 1);
    assert!(response_json.get("error").is_some());

    let error = &response_json["error"];
    assert!(error.get("code").is_some());
    assert!(error.get("message").is_some());
}

// ============================================================================
// Multiple Domain Error Tests
// ============================================================================

#[tokio::test]
async fn test_all_domains_unknown_method_error_code() {
    let profiler = ProfilerDomain::new();
    let heap_profiler = HeapProfilerDomain::new();
    let timeline = TimelineDomain::new();

    let err1 = profiler.handle_method("xxx", None).await.unwrap_err();
    let err2 = heap_profiler.handle_method("xxx", None).await.unwrap_err();
    let err3 = timeline.handle_method("xxx", None).await.unwrap_err();

    // All should return the same error code for unknown method
    assert_eq!(err1.code, -32601);
    assert_eq!(err2.code, -32601);
    assert_eq!(err3.code, -32601);
}

#[tokio::test]
async fn test_all_domains_invalid_state_error_code() {
    let profiler = ProfilerDomain::new();
    let heap_profiler = HeapProfilerDomain::new();
    let timeline = TimelineDomain::new();

    // All should fail when not enabled
    let err1 = profiler.handle_method("start", None).await.unwrap_err();
    let err2 = heap_profiler.handle_method("startSampling", None).await.unwrap_err();
    let err3 = timeline.handle_method("start", None).await.unwrap_err();

    // All should return the same error code for invalid state
    assert_eq!(err1.code, -32600);
    assert_eq!(err2.code, -32600);
    assert_eq!(err3.code, -32600);
}
