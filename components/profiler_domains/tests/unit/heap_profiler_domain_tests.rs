//! Unit tests for HeapProfilerDomain
//!
//! These tests verify the HeapProfilerDomain implementation following TDD principles.

use profiler_domains::{HeapProfilerDomain, SamplingHeapProfile, SamplingHeapProfileNode};
use protocol_handler::DomainHandler;
use serde_json::json;

#[tokio::test]
async fn test_heap_profiler_domain_name() {
    let heap_profiler = HeapProfilerDomain::new();
    assert_eq!(heap_profiler.name(), "HeapProfiler");
}

#[tokio::test]
async fn test_heap_profiler_enable() {
    let heap_profiler = HeapProfilerDomain::new();
    let result = heap_profiler.handle_method("enable", None).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), json!({}));
}

#[tokio::test]
async fn test_heap_profiler_disable() {
    let heap_profiler = HeapProfilerDomain::new();

    // Enable first
    let _ = heap_profiler.handle_method("enable", None).await;

    // Then disable
    let result = heap_profiler.handle_method("disable", None).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), json!({}));
}

#[tokio::test]
async fn test_heap_profiler_start_sampling() {
    let heap_profiler = HeapProfilerDomain::new();

    // Enable heap profiler first
    let _ = heap_profiler.handle_method("enable", None).await;

    // Start sampling with optional params
    let params = json!({
        "samplingInterval": 32768
    });

    let result = heap_profiler
        .handle_method("startSampling", Some(params))
        .await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), json!({}));

    // Verify sampling is active
    assert!(heap_profiler.is_sampling());
}

#[tokio::test]
async fn test_heap_profiler_start_sampling_no_params() {
    let heap_profiler = HeapProfilerDomain::new();

    // Enable heap profiler
    let _ = heap_profiler.handle_method("enable", None).await;

    // Start sampling without params (should use default)
    let result = heap_profiler.handle_method("startSampling", None).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), json!({}));

    // Verify sampling is active
    assert!(heap_profiler.is_sampling());
}

#[tokio::test]
async fn test_heap_profiler_stop_sampling() {
    let heap_profiler = HeapProfilerDomain::new();

    // Enable and start sampling
    let _ = heap_profiler.handle_method("enable", None).await;
    let _ = heap_profiler.handle_method("startSampling", None).await;

    // Stop sampling
    let result = heap_profiler.handle_method("stopSampling", None).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response["profile"].is_object());
    assert!(response["profile"]["head"].is_object());
    assert!(response["profile"]["samples"].is_array());

    // Verify sampling is inactive
    assert!(!heap_profiler.is_sampling());
}

#[tokio::test]
async fn test_heap_profiler_collect_garbage() {
    let heap_profiler = HeapProfilerDomain::new();

    // Enable heap profiler
    let _ = heap_profiler.handle_method("enable", None).await;

    // Trigger garbage collection
    let result = heap_profiler.handle_method("collectGarbage", None).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), json!({}));
}

#[tokio::test]
async fn test_heap_profiler_take_heap_snapshot() {
    let heap_profiler = HeapProfilerDomain::new();

    // Enable heap profiler
    let _ = heap_profiler.handle_method("enable", None).await;

    // Take heap snapshot with optional params
    let params = json!({
        "reportProgress": true,
        "treatGlobalObjectsAsRoots": true
    });

    let result = heap_profiler
        .handle_method("takeHeapSnapshot", Some(params))
        .await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), json!({}));
}

#[tokio::test]
async fn test_heap_profiler_get_heap_object_id() {
    let heap_profiler = HeapProfilerDomain::new();

    // Enable heap profiler
    let _ = heap_profiler.handle_method("enable", None).await;

    // Get heap object ID
    let params = json!({
        "objectId": "obj-123"
    });

    let result = heap_profiler
        .handle_method("getHeapObjectId", Some(params))
        .await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response["heapSnapshotObjectId"].is_string());
}

#[tokio::test]
async fn test_heap_profiler_get_object_by_heap_object_id() {
    let heap_profiler = HeapProfilerDomain::new();

    // Enable heap profiler
    let _ = heap_profiler.handle_method("enable", None).await;

    // Get object by heap object ID
    let params = json!({
        "objectId": "123",
        "objectGroup": "test-group"
    });

    let result = heap_profiler
        .handle_method("getObjectByHeapObjectId", Some(params))
        .await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response["result"].is_object());
}

#[tokio::test]
async fn test_heap_profiler_start_tracking_heap_objects() {
    let heap_profiler = HeapProfilerDomain::new();

    // Enable heap profiler
    let _ = heap_profiler.handle_method("enable", None).await;

    // Start tracking heap objects
    let params = json!({
        "trackAllocations": true
    });

    let result = heap_profiler
        .handle_method("startTrackingHeapObjects", Some(params))
        .await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), json!({}));
}

#[tokio::test]
async fn test_heap_profiler_stop_tracking_heap_objects() {
    let heap_profiler = HeapProfilerDomain::new();

    // Enable and start tracking
    let _ = heap_profiler.handle_method("enable", None).await;
    let _ = heap_profiler
        .handle_method("startTrackingHeapObjects", None)
        .await;

    // Stop tracking with optional params
    let params = json!({
        "reportProgress": true,
        "treatGlobalObjectsAsRoots": false
    });

    let result = heap_profiler
        .handle_method("stopTrackingHeapObjects", Some(params))
        .await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), json!({}));
}

#[tokio::test]
async fn test_heap_profiler_unknown_method() {
    let heap_profiler = HeapProfilerDomain::new();

    let result = heap_profiler.handle_method("unknownMethod", None).await;
    assert!(result.is_err());
}

#[test]
fn test_sampling_heap_profile_node_creation() {
    let node = SamplingHeapProfileNode {
        call_frame: json!({
            "functionName": "main",
            "scriptId": "1",
            "url": "http://example.com",
            "lineNumber": 10,
            "columnNumber": 5
        }),
        self_size: 1024,
        id: 1,
        children: vec![],
    };

    assert_eq!(node.self_size, 1024);
    assert_eq!(node.id, 1);
    assert_eq!(node.children.len(), 0);
}

#[test]
fn test_sampling_heap_profile_creation() {
    let profile = SamplingHeapProfile {
        head: SamplingHeapProfileNode {
            call_frame: json!({}),
            self_size: 0,
            id: 0,
            children: vec![],
        },
        samples: vec![],
    };

    assert_eq!(profile.head.id, 0);
    assert_eq!(profile.samples.len(), 0);
}

#[test]
fn test_sampling_heap_profile_serialization() {
    let profile = SamplingHeapProfile {
        head: SamplingHeapProfileNode {
            call_frame: json!({
                "functionName": "root",
                "scriptId": "0",
                "url": "",
                "lineNumber": 0,
                "columnNumber": 0
            }),
            self_size: 0,
            id: 0,
            children: vec![SamplingHeapProfileNode {
                call_frame: json!({
                    "functionName": "child",
                    "scriptId": "1",
                    "url": "http://example.com",
                    "lineNumber": 1,
                    "columnNumber": 1
                }),
                self_size: 512,
                id: 1,
                children: vec![],
            }],
        },
        samples: vec![],
    };

    let json = serde_json::to_value(&profile).unwrap();
    assert!(json["head"].is_object());
    assert!(json["head"]["children"].is_array());
    assert_eq!(json["head"]["children"][0]["selfSize"], 512);
}
