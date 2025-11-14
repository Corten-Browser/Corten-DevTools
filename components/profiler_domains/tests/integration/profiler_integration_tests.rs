//! Integration tests for profiler_domains
//!
//! These tests verify that ProfilerDomain and HeapProfilerDomain integrate correctly
//! with the protocol_handler system.

use profiler_domains::{HeapProfilerDomain, ProfilerDomain};
use protocol_handler::ProtocolHandler;
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn test_profiler_domain_registration() {
    let handler = ProtocolHandler::new();
    let profiler = Arc::new(ProfilerDomain::new());

    handler.register_domain(profiler);

    // Verify domain is registered by sending a request
    let request = json!({
        "id": 1,
        "method": "Profiler.enable"
    });

    let response = handler.handle_message(&request.to_string()).await;
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    assert_eq!(response_json["id"], 1);
    assert!(response_json["result"].is_object());
    assert!(response_json["error"].is_null());
}

#[tokio::test]
async fn test_heap_profiler_domain_registration() {
    let handler = ProtocolHandler::new();
    let heap_profiler = Arc::new(HeapProfilerDomain::new());

    handler.register_domain(heap_profiler);

    // Verify domain is registered by sending a request
    let request = json!({
        "id": 2,
        "method": "HeapProfiler.enable"
    });

    let response = handler.handle_message(&request.to_string()).await;
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    assert_eq!(response_json["id"], 2);
    assert!(response_json["result"].is_object());
    assert!(response_json["error"].is_null());
}

#[tokio::test]
async fn test_both_domains_registered() {
    let handler = ProtocolHandler::new();
    let profiler = Arc::new(ProfilerDomain::new());
    let heap_profiler = Arc::new(HeapProfilerDomain::new());

    handler.register_domain(profiler);
    handler.register_domain(heap_profiler);

    // Test Profiler domain
    let request1 = json!({
        "id": 1,
        "method": "Profiler.enable"
    });

    let response1 = handler.handle_message(&request1.to_string()).await;
    let response_json1: serde_json::Value = serde_json::from_str(&response1).unwrap();
    assert_eq!(response_json1["id"], 1);
    assert!(response_json1["error"].is_null());

    // Test HeapProfiler domain
    let request2 = json!({
        "id": 2,
        "method": "HeapProfiler.enable"
    });

    let response2 = handler.handle_message(&request2.to_string()).await;
    let response_json2: serde_json::Value = serde_json::from_str(&response2).unwrap();
    assert_eq!(response_json2["id"], 2);
    assert!(response_json2["error"].is_null());
}

#[tokio::test]
async fn test_profiler_start_stop_workflow() {
    let handler = ProtocolHandler::new();
    let profiler = Arc::new(ProfilerDomain::new());
    handler.register_domain(profiler);

    // Enable
    let enable_request = json!({
        "id": 1,
        "method": "Profiler.enable"
    });
    let _ = handler.handle_message(&enable_request.to_string()).await;

    // Start profiling
    let start_request = json!({
        "id": 2,
        "method": "Profiler.start"
    });
    let start_response = handler.handle_message(&start_request.to_string()).await;
    let start_json: serde_json::Value = serde_json::from_str(&start_response).unwrap();
    assert_eq!(start_json["id"], 2);
    assert!(start_json["error"].is_null());

    // Stop profiling
    let stop_request = json!({
        "id": 3,
        "method": "Profiler.stop"
    });
    let stop_response = handler.handle_message(&stop_request.to_string()).await;
    let stop_json: serde_json::Value = serde_json::from_str(&stop_response).unwrap();
    assert_eq!(stop_json["id"], 3);
    assert!(stop_json["result"]["profile"].is_object());
}

#[tokio::test]
async fn test_heap_profiler_sampling_workflow() {
    let handler = ProtocolHandler::new();
    let heap_profiler = Arc::new(HeapProfilerDomain::new());
    handler.register_domain(heap_profiler);

    // Enable
    let enable_request = json!({
        "id": 1,
        "method": "HeapProfiler.enable"
    });
    let _ = handler.handle_message(&enable_request.to_string()).await;

    // Start sampling
    let start_request = json!({
        "id": 2,
        "method": "HeapProfiler.startSampling",
        "params": {
            "samplingInterval": 16384
        }
    });
    let start_response = handler.handle_message(&start_request.to_string()).await;
    let start_json: serde_json::Value = serde_json::from_str(&start_response).unwrap();
    assert_eq!(start_json["id"], 2);
    assert!(start_json["error"].is_null());

    // Stop sampling
    let stop_request = json!({
        "id": 3,
        "method": "HeapProfiler.stopSampling"
    });
    let stop_response = handler.handle_message(&stop_request.to_string()).await;
    let stop_json: serde_json::Value = serde_json::from_str(&stop_response).unwrap();
    assert_eq!(stop_json["id"], 3);
    assert!(stop_json["result"]["profile"].is_object());
}

#[tokio::test]
async fn test_profiler_coverage_workflow() {
    let handler = ProtocolHandler::new();
    let profiler = Arc::new(ProfilerDomain::new());
    handler.register_domain(profiler);

    // Enable
    let enable_request = json!({
        "id": 1,
        "method": "Profiler.enable"
    });
    let _ = handler.handle_message(&enable_request.to_string()).await;

    // Start precise coverage
    let start_coverage_request = json!({
        "id": 2,
        "method": "Profiler.startPreciseCoverage",
        "params": {
            "callCount": true,
            "detailed": true
        }
    });
    let start_coverage_response = handler
        .handle_message(&start_coverage_request.to_string())
        .await;
    let start_coverage_json: serde_json::Value =
        serde_json::from_str(&start_coverage_response).unwrap();
    assert_eq!(start_coverage_json["id"], 2);
    assert!(start_coverage_json["error"].is_null());

    // Take coverage
    let take_coverage_request = json!({
        "id": 3,
        "method": "Profiler.takePreciseCoverage"
    });
    let take_coverage_response = handler
        .handle_message(&take_coverage_request.to_string())
        .await;
    let take_coverage_json: serde_json::Value =
        serde_json::from_str(&take_coverage_response).unwrap();
    assert_eq!(take_coverage_json["id"], 3);
    assert!(take_coverage_json["result"]["result"].is_array());

    // Stop coverage
    let stop_coverage_request = json!({
        "id": 4,
        "method": "Profiler.stopPreciseCoverage"
    });
    let stop_coverage_response = handler
        .handle_message(&stop_coverage_request.to_string())
        .await;
    let stop_coverage_json: serde_json::Value =
        serde_json::from_str(&stop_coverage_response).unwrap();
    assert_eq!(stop_coverage_json["id"], 4);
    assert!(stop_coverage_json["error"].is_null());
}
