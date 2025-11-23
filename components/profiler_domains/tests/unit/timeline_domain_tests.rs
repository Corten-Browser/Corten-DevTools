//! Unit tests for TimelineDomain (FEAT-034)

use profiler_domains::{
    FrameTiming, TimelineConfig, TimelineDomain, TimelineEvent, TimelineEventCategory,
    TimelineMemorySnapshot, TimelineRecording,
};
use protocol_handler::DomainHandler;
use serde_json::json;

// ============================================================================
// TimelineDomain Basic Tests
// ============================================================================

#[test]
fn test_timeline_domain_creation() {
    let domain = TimelineDomain::new();
    assert_eq!(domain.name(), "Timeline");
    assert!(!domain.is_recording());
    assert!(!domain.is_enabled());
}

#[test]
fn test_timeline_domain_default() {
    let domain = TimelineDomain::default();
    assert_eq!(domain.name(), "Timeline");
}

// ============================================================================
// Enable/Disable Tests
// ============================================================================

#[tokio::test]
async fn test_enable() {
    let domain = TimelineDomain::new();

    let result = domain.handle_method("enable", None).await;
    assert!(result.is_ok());
    assert!(domain.is_enabled());
}

#[tokio::test]
async fn test_disable() {
    let domain = TimelineDomain::new();
    domain.handle_method("enable", None).await.unwrap();

    let result = domain.handle_method("disable", None).await;
    assert!(result.is_ok());
    assert!(!domain.is_enabled());
}

#[tokio::test]
async fn test_disable_stops_recording() {
    let domain = TimelineDomain::new();
    domain.handle_method("enable", None).await.unwrap();
    domain.handle_method("start", None).await.unwrap();
    assert!(domain.is_recording());

    domain.handle_method("disable", None).await.unwrap();
    assert!(!domain.is_recording());
}

// ============================================================================
// Start/Stop Recording Tests
// ============================================================================

#[tokio::test]
async fn test_start_recording() {
    let domain = TimelineDomain::new();
    domain.handle_method("enable", None).await.unwrap();

    let result = domain.handle_method("start", None).await;
    assert!(result.is_ok());
    assert!(domain.is_recording());
}

#[tokio::test]
async fn test_start_without_enable_fails() {
    let domain = TimelineDomain::new();

    let result = domain.handle_method("start", None).await;
    assert!(result.is_err());
    assert!(!domain.is_recording());
}

#[tokio::test]
async fn test_double_start_fails() {
    let domain = TimelineDomain::new();
    domain.handle_method("enable", None).await.unwrap();
    domain.handle_method("start", None).await.unwrap();

    let result = domain.handle_method("start", None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_stop_recording() {
    let domain = TimelineDomain::new();
    domain.handle_method("enable", None).await.unwrap();
    domain.handle_method("start", None).await.unwrap();

    let result = domain.handle_method("stop", None).await;
    assert!(result.is_ok());
    assert!(!domain.is_recording());
}

#[tokio::test]
async fn test_stop_without_start_fails() {
    let domain = TimelineDomain::new();
    domain.handle_method("enable", None).await.unwrap();

    let result = domain.handle_method("stop", None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_stop_returns_timeline_data() {
    let domain = TimelineDomain::new();
    domain.handle_method("enable", None).await.unwrap();
    domain.handle_method("start", None).await.unwrap();

    // Record some events
    domain
        .handle_method(
            "recordEvent",
            Some(json!({
                "type": "TestEvent",
                "category": "scripting",
                "duration": 100.0
            })),
        )
        .await
        .unwrap();

    let result = domain.handle_method("stop", None).await.unwrap();
    assert!(result.get("timeline").is_some());
}

// ============================================================================
// Record Event Tests
// ============================================================================

#[tokio::test]
async fn test_record_event() {
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
    assert_eq!(domain.event_count(), 1);
}

#[tokio::test]
async fn test_record_event_without_recording_fails() {
    let domain = TimelineDomain::new();
    domain.handle_method("enable", None).await.unwrap();

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
async fn test_record_event_all_categories() {
    let domain = TimelineDomain::new();
    domain.handle_method("enable", None).await.unwrap();
    domain.handle_method("start", None).await.unwrap();

    let categories = vec!["scripting", "rendering", "painting", "loading", "other"];

    for category in categories {
        domain
            .handle_method(
                "recordEvent",
                Some(json!({
                    "type": format!("{}_event", category),
                    "category": category
                })),
            )
            .await
            .unwrap();
    }

    assert_eq!(domain.event_count(), 5);
}

#[tokio::test]
async fn test_record_event_with_all_fields() {
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
                "data": {"key": "value"}
            })),
        )
        .await;

    assert!(result.is_ok());
    assert_eq!(domain.event_count(), 1);
}

// ============================================================================
// Memory Snapshot Tests
// ============================================================================

#[tokio::test]
async fn test_take_memory_snapshot() {
    let domain = TimelineDomain::new();
    domain.handle_method("enable", None).await.unwrap();
    domain.handle_method("start", None).await.unwrap();

    let result = domain.handle_method("takeMemorySnapshot", None).await;
    assert!(result.is_ok());
    assert_eq!(domain.memory_snapshot_count(), 1);

    let value = result.unwrap();
    assert!(value.get("snapshot").is_some());
}

#[tokio::test]
async fn test_memory_snapshot_without_recording_fails() {
    let domain = TimelineDomain::new();
    domain.handle_method("enable", None).await.unwrap();

    let result = domain.handle_method("takeMemorySnapshot", None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_memory_snapshots() {
    let domain = TimelineDomain::new();
    domain.handle_method("enable", None).await.unwrap();
    domain.handle_method("start", None).await.unwrap();

    // Take multiple snapshots
    domain.handle_method("takeMemorySnapshot", None).await.unwrap();
    domain.handle_method("takeMemorySnapshot", None).await.unwrap();

    let result = domain.handle_method("getMemorySnapshots", None).await;
    assert!(result.is_ok());
}

// ============================================================================
// Frame Recording Tests
// ============================================================================

#[tokio::test]
async fn test_record_frame() {
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
    assert_eq!(domain.frame_count(), 1);

    let value = result.unwrap();
    assert!(value.get("frame").is_some());
}

#[tokio::test]
async fn test_record_frame_without_recording_fails() {
    let domain = TimelineDomain::new();
    domain.handle_method("enable", None).await.unwrap();

    let result = domain
        .handle_method(
            "recordFrame",
            Some(json!({
                "frameId": "frame-1"
            })),
        )
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_frames() {
    let domain = TimelineDomain::new();
    domain.handle_method("enable", None).await.unwrap();
    domain.handle_method("start", None).await.unwrap();

    domain
        .handle_method("recordFrame", Some(json!({"frameId": "frame-1"})))
        .await
        .unwrap();

    let result = domain.handle_method("getFrames", None).await;
    assert!(result.is_ok());
}

// ============================================================================
// Get Events Tests
// ============================================================================

#[tokio::test]
async fn test_get_events() {
    let domain = TimelineDomain::new();
    domain.handle_method("enable", None).await.unwrap();
    domain.handle_method("start", None).await.unwrap();

    domain
        .handle_method(
            "recordEvent",
            Some(json!({
                "type": "Event1",
                "category": "scripting"
            })),
        )
        .await
        .unwrap();

    let result = domain.handle_method("getEvents", None).await;
    assert!(result.is_ok());

    let value = result.unwrap();
    assert!(value.get("events").is_some());
}

// ============================================================================
// Helper Method Tests
// ============================================================================

#[tokio::test]
async fn test_record_scripting_event_helper() {
    let domain = TimelineDomain::new();
    domain.handle_method("enable", None).await.unwrap();
    domain.handle_method("start", None).await.unwrap();

    domain.record_scripting_event("FunctionCall", 100.0, None);
    assert_eq!(domain.event_count(), 1);
}

#[tokio::test]
async fn test_record_rendering_event_helper() {
    let domain = TimelineDomain::new();
    domain.handle_method("enable", None).await.unwrap();
    domain.handle_method("start", None).await.unwrap();

    domain.record_rendering_event("Layout", 50.0, None);
    assert_eq!(domain.event_count(), 1);
}

#[tokio::test]
async fn test_record_painting_event_helper() {
    let domain = TimelineDomain::new();
    domain.handle_method("enable", None).await.unwrap();
    domain.handle_method("start", None).await.unwrap();

    domain.record_painting_event("Paint", 30.0, None);
    assert_eq!(domain.event_count(), 1);
}

#[tokio::test]
async fn test_record_loading_event_helper() {
    let domain = TimelineDomain::new();
    domain.handle_method("enable", None).await.unwrap();
    domain.handle_method("start", None).await.unwrap();

    domain.record_loading_event("ResourceLoad", 200.0, Some("http://example.com".to_string()));
    assert_eq!(domain.event_count(), 1);
}

#[tokio::test]
async fn test_helper_methods_do_nothing_when_not_recording() {
    let domain = TimelineDomain::new();
    domain.handle_method("enable", None).await.unwrap();
    // Not started!

    domain.record_scripting_event("FunctionCall", 100.0, None);
    domain.record_rendering_event("Layout", 50.0, None);
    domain.record_painting_event("Paint", 30.0, None);
    domain.record_loading_event("ResourceLoad", 200.0, None);

    assert_eq!(domain.event_count(), 0);
}

// ============================================================================
// Unknown Method Test
// ============================================================================

#[tokio::test]
async fn test_unknown_method() {
    let domain = TimelineDomain::new();
    let result = domain.handle_method("unknownMethod", None).await;
    assert!(result.is_err());
}

// ============================================================================
// Type Tests
// ============================================================================

#[test]
fn test_timeline_event_category_default() {
    let category: TimelineEventCategory = Default::default();
    assert_eq!(category, TimelineEventCategory::Other);
}

#[test]
fn test_timeline_event_category_display() {
    assert_eq!(TimelineEventCategory::Scripting.to_string(), "scripting");
    assert_eq!(TimelineEventCategory::Rendering.to_string(), "rendering");
    assert_eq!(TimelineEventCategory::Painting.to_string(), "painting");
    assert_eq!(TimelineEventCategory::Loading.to_string(), "loading");
    assert_eq!(TimelineEventCategory::Other.to_string(), "other");
}

#[test]
fn test_timeline_event_builder() {
    let event = TimelineEvent::new(
        "TestEvent".to_string(),
        TimelineEventCategory::Scripting,
        1000.0,
    )
    .with_duration(500.0)
    .with_thread_id(1)
    .with_frame_id("frame-1".to_string())
    .with_data(json!({"key": "value"}));

    assert_eq!(event.event_type, "TestEvent");
    assert_eq!(event.category, TimelineEventCategory::Scripting);
    assert_eq!(event.start_time, 1000.0);
    assert_eq!(event.duration, 500.0);
    assert_eq!(event.thread_id, Some(1));
    assert_eq!(event.frame_id, Some("frame-1".to_string()));
    assert!(event.data.is_some());
}

#[test]
fn test_frame_timing_new() {
    let frame = FrameTiming::new("frame-1".to_string(), 1000.0);
    assert_eq!(frame.frame_id, "frame-1");
    assert_eq!(frame.start_time, 1000.0);
    assert_eq!(frame.end_time, 1000.0);
    assert_eq!(frame.duration, 0.0);
    assert!(!frame.dropped);
}

#[test]
fn test_frame_timing_complete() {
    let mut frame = FrameTiming::new("frame-1".to_string(), 1000.0);
    frame.complete(1016.67, 10.0, false);

    assert_eq!(frame.end_time, 1016.67);
    assert!((frame.duration - 16.67).abs() < 0.01);
    assert_eq!(frame.cpu_time, 10.0);
    assert!(!frame.dropped);
}

#[test]
fn test_frame_timing_dropped() {
    let mut frame = FrameTiming::new("frame-1".to_string(), 1000.0);
    frame.complete(1033.0, 25.0, true);

    assert!(frame.dropped);
}

#[test]
fn test_timeline_memory_snapshot_default() {
    let snapshot: TimelineMemorySnapshot = Default::default();
    assert_eq!(snapshot.timestamp, 0.0);
    assert_eq!(snapshot.js_heap_size_used, 0);
    assert_eq!(snapshot.js_heap_size_total, 0);
    assert_eq!(snapshot.documents, 0);
    assert_eq!(snapshot.nodes, 0);
    assert_eq!(snapshot.listeners, 0);
}

#[test]
fn test_timeline_config_default() {
    let config: TimelineConfig = Default::default();
    assert_eq!(config.max_events, 10000);
    assert!(config.capture_memory);
    assert!(config.capture_stacks);
    assert_eq!(config.max_stack_depth, 16);
    assert!(config.categories.is_empty());
}

#[test]
fn test_get_config() {
    let domain = TimelineDomain::new();
    let config = domain.get_config();
    assert_eq!(config.max_events, 10000);
}

// ============================================================================
// Serialization Tests
// ============================================================================

#[test]
fn test_timeline_event_serialization() {
    let event = TimelineEvent::new(
        "TestEvent".to_string(),
        TimelineEventCategory::Scripting,
        1000.0,
    );

    let serialized = serde_json::to_string(&event).unwrap();
    let deserialized: TimelineEvent = serde_json::from_str(&serialized).unwrap();

    assert_eq!(deserialized.event_type, "TestEvent");
    assert_eq!(deserialized.category, TimelineEventCategory::Scripting);
}

#[test]
fn test_frame_timing_serialization() {
    let mut frame = FrameTiming::new("frame-1".to_string(), 1000.0);
    frame.complete(1016.67, 10.0, false);

    let serialized = serde_json::to_string(&frame).unwrap();
    let deserialized: FrameTiming = serde_json::from_str(&serialized).unwrap();

    assert_eq!(deserialized.frame_id, "frame-1");
    assert_eq!(deserialized.duration, frame.duration);
}

#[test]
fn test_timeline_memory_snapshot_serialization() {
    let snapshot = TimelineMemorySnapshot {
        timestamp: 1000.0,
        js_heap_size_used: 50_000_000,
        js_heap_size_total: 100_000_000,
        documents: 1,
        nodes: 500,
        listeners: 100,
    };

    let serialized = serde_json::to_string(&snapshot).unwrap();
    let deserialized: TimelineMemorySnapshot = serde_json::from_str(&serialized).unwrap();

    assert_eq!(deserialized.timestamp, 1000.0);
    assert_eq!(deserialized.js_heap_size_used, 50_000_000);
}
