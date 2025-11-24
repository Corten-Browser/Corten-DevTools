//! Method Parameter Validation Tests (FEAT-043)
//!
//! Tests that CDP methods properly validate their parameters
//! and return appropriate errors for invalid inputs.

use profiler_domains::{HeapProfilerDomain, ProfilerDomain, TimelineDomain};
use protocol_handler::DomainHandler;
use serde_json::json;

// ============================================================================
// Profiler Domain Parameter Tests
// ============================================================================

#[tokio::test]
async fn test_profiler_set_sampling_interval_valid() {
    let domain = ProfilerDomain::new();

    let result = domain
        .handle_method("setSamplingInterval", Some(json!({ "interval": 100 })))
        .await;
    assert!(result.is_ok());
    assert_eq!(domain.get_sampling_interval(), 100);
}

#[tokio::test]
async fn test_profiler_set_sampling_interval_missing_param() {
    let domain = ProfilerDomain::new();

    let result = domain
        .handle_method("setSamplingInterval", Some(json!({})))
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_profiler_set_sampling_interval_null_param() {
    let domain = ProfilerDomain::new();

    let result = domain.handle_method("setSamplingInterval", None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_profiler_set_sampling_interval_invalid_type() {
    let domain = ProfilerDomain::new();

    let result = domain
        .handle_method(
            "setSamplingInterval",
            Some(json!({ "interval": "not_a_number" })),
        )
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_profiler_start_requires_enable() {
    let domain = ProfilerDomain::new();

    // Start without enable should fail
    let result = domain.handle_method("start", None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_profiler_stop_requires_start() {
    let domain = ProfilerDomain::new();
    domain.handle_method("enable", None).await.unwrap();

    // Stop without start should fail
    let result = domain.handle_method("stop", None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_profiler_start_precise_coverage_requires_enable() {
    let domain = ProfilerDomain::new();

    let result = domain.handle_method("startPreciseCoverage", None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_profiler_take_precise_coverage_requires_start() {
    let domain = ProfilerDomain::new();
    domain.handle_method("enable", None).await.unwrap();

    // Take coverage without starting should fail
    let result = domain.handle_method("takePreciseCoverage", None).await;
    assert!(result.is_err());
}

// ============================================================================
// HeapProfiler Domain Parameter Tests
// ============================================================================

#[tokio::test]
async fn test_heap_profiler_start_sampling_valid() {
    let domain = HeapProfilerDomain::new();
    domain.handle_method("enable", None).await.unwrap();

    let result = domain
        .handle_method("startSampling", Some(json!({ "samplingInterval": 16384 })))
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_heap_profiler_start_sampling_default_interval() {
    let domain = HeapProfilerDomain::new();
    domain.handle_method("enable", None).await.unwrap();

    // No params should use default
    let result = domain.handle_method("startSampling", None).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_heap_profiler_start_sampling_requires_enable() {
    let domain = HeapProfilerDomain::new();

    let result = domain.handle_method("startSampling", None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_heap_profiler_stop_sampling_requires_start() {
    let domain = HeapProfilerDomain::new();
    domain.handle_method("enable", None).await.unwrap();

    let result = domain.handle_method("stopSampling", None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_heap_profiler_take_heap_snapshot_requires_enable() {
    let domain = HeapProfilerDomain::new();

    let result = domain.handle_method("takeHeapSnapshot", None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_heap_profiler_start_tracking_requires_enable() {
    let domain = HeapProfilerDomain::new();

    let result = domain.handle_method("startTrackingHeapObjects", None).await;
    assert!(result.is_err());
}

// ============================================================================
// Timeline Domain Parameter Tests
// ============================================================================

#[tokio::test]
async fn test_timeline_start_requires_enable() {
    let domain = TimelineDomain::new();

    let result = domain.handle_method("start", None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_timeline_stop_requires_start() {
    let domain = TimelineDomain::new();
    domain.handle_method("enable", None).await.unwrap();

    let result = domain.handle_method("stop", None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_timeline_double_start_fails() {
    let domain = TimelineDomain::new();
    domain.handle_method("enable", None).await.unwrap();
    domain.handle_method("start", None).await.unwrap();

    // Second start should fail
    let result = domain.handle_method("start", None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_timeline_record_event_requires_recording() {
    let domain = TimelineDomain::new();
    domain.handle_method("enable", None).await.unwrap();
    // Not started!

    let result = domain
        .handle_method(
            "recordEvent",
            Some(json!({
                "type": "TestEvent",
                "category": "scripting"
            })),
        )
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_timeline_record_event_missing_type() {
    let domain = TimelineDomain::new();
    domain.handle_method("enable", None).await.unwrap();
    domain.handle_method("start", None).await.unwrap();

    // Missing required "type" param
    let result = domain
        .handle_method("recordEvent", Some(json!({ "category": "scripting" })))
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_timeline_record_event_null_params() {
    let domain = TimelineDomain::new();
    domain.handle_method("enable", None).await.unwrap();
    domain.handle_method("start", None).await.unwrap();

    let result = domain.handle_method("recordEvent", None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_timeline_record_event_valid() {
    let domain = TimelineDomain::new();
    domain.handle_method("enable", None).await.unwrap();
    domain.handle_method("start", None).await.unwrap();

    let result = domain
        .handle_method(
            "recordEvent",
            Some(json!({
                "type": "FunctionCall",
                "category": "scripting",
                "duration": 1000.0
            })),
        )
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_timeline_record_frame_requires_recording() {
    let domain = TimelineDomain::new();
    domain.handle_method("enable", None).await.unwrap();

    let result = domain
        .handle_method("recordFrame", Some(json!({ "frameId": "frame-1" })))
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_timeline_record_frame_null_params() {
    let domain = TimelineDomain::new();
    domain.handle_method("enable", None).await.unwrap();
    domain.handle_method("start", None).await.unwrap();

    let result = domain.handle_method("recordFrame", None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_timeline_memory_snapshot_requires_recording() {
    let domain = TimelineDomain::new();
    domain.handle_method("enable", None).await.unwrap();

    let result = domain.handle_method("takeMemorySnapshot", None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_timeline_start_with_config() {
    let domain = TimelineDomain::new();
    domain.handle_method("enable", None).await.unwrap();

    let result = domain
        .handle_method(
            "start",
            Some(json!({
                "maxCallStackDepth": 32,
                "includeCounters": true
            })),
        )
        .await;
    assert!(result.is_ok());
}

// ============================================================================
// Parameter Type Validation Tests
// ============================================================================

#[tokio::test]
async fn test_profiler_sampling_interval_large_value() {
    let domain = ProfilerDomain::new();

    let result = domain
        .handle_method(
            "setSamplingInterval",
            Some(json!({ "interval": 1_000_000 })),
        )
        .await;
    assert!(result.is_ok());
    assert_eq!(domain.get_sampling_interval(), 1_000_000);
}

#[tokio::test]
async fn test_profiler_sampling_interval_zero() {
    let domain = ProfilerDomain::new();

    let result = domain
        .handle_method("setSamplingInterval", Some(json!({ "interval": 0 })))
        .await;
    assert!(result.is_ok());
    assert_eq!(domain.get_sampling_interval(), 0);
}

#[tokio::test]
async fn test_timeline_record_event_all_categories() {
    let domain = TimelineDomain::new();
    domain.handle_method("enable", None).await.unwrap();
    domain.handle_method("start", None).await.unwrap();

    let categories = vec!["scripting", "rendering", "painting", "loading", "other", "unknown"];

    for cat in categories {
        let result = domain
            .handle_method(
                "recordEvent",
                Some(json!({
                    "type": format!("{}_event", cat),
                    "category": cat
                })),
            )
            .await;
        assert!(result.is_ok(), "Failed for category: {}", cat);
    }
}

#[tokio::test]
async fn test_timeline_record_event_with_all_optional_fields() {
    let domain = TimelineDomain::new();
    domain.handle_method("enable", None).await.unwrap();
    domain.handle_method("start", None).await.unwrap();

    let result = domain
        .handle_method(
            "recordEvent",
            Some(json!({
                "type": "CompleteEvent",
                "category": "scripting",
                "startTime": 1000.0,
                "duration": 500.0,
                "threadId": 1,
                "frameId": "frame-1",
                "data": {
                    "functionName": "testFunction",
                    "scriptId": "1"
                }
            })),
        )
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_timeline_record_frame_with_all_fields() {
    let domain = TimelineDomain::new();
    domain.handle_method("enable", None).await.unwrap();
    domain.handle_method("start", None).await.unwrap();

    let result = domain
        .handle_method(
            "recordFrame",
            Some(json!({
                "frameId": "frame-1",
                "startTime": 1000.0,
                "endTime": 1016.67,
                "cpuTime": 10.0,
                "dropped": false
            })),
        )
        .await;
    assert!(result.is_ok());

    let value = result.unwrap();
    let frame = value.get("frame").unwrap();
    assert_eq!(frame.get("frameId").unwrap().as_str().unwrap(), "frame-1");
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[tokio::test]
async fn test_profiler_full_workflow() {
    let domain = ProfilerDomain::new();

    // Enable -> Set interval -> Start -> Stop
    domain.handle_method("enable", None).await.unwrap();
    domain
        .handle_method("setSamplingInterval", Some(json!({ "interval": 100 })))
        .await
        .unwrap();
    domain.handle_method("start", None).await.unwrap();
    let result = domain.handle_method("stop", None).await;
    assert!(result.is_ok());

    let value = result.unwrap();
    assert!(value.get("profile").is_some());
}

#[tokio::test]
async fn test_heap_profiler_full_workflow() {
    let domain = HeapProfilerDomain::new();

    domain.handle_method("enable", None).await.unwrap();
    domain.handle_method("startSampling", None).await.unwrap();
    let result = domain.handle_method("stopSampling", None).await;
    assert!(result.is_ok());

    let value = result.unwrap();
    assert!(value.get("profile").is_some());
}

#[tokio::test]
async fn test_timeline_full_workflow() {
    let domain = TimelineDomain::new();

    domain.handle_method("enable", None).await.unwrap();
    domain.handle_method("start", None).await.unwrap();

    // Record some events
    domain
        .handle_method(
            "recordEvent",
            Some(json!({ "type": "Event1", "category": "scripting" })),
        )
        .await
        .unwrap();

    domain
        .handle_method(
            "recordFrame",
            Some(json!({ "frameId": "frame-1" })),
        )
        .await
        .unwrap();

    domain.handle_method("takeMemorySnapshot", None).await.unwrap();

    let result = domain.handle_method("stop", None).await;
    assert!(result.is_ok());

    let value = result.unwrap();
    assert!(value.get("timeline").is_some());
}
