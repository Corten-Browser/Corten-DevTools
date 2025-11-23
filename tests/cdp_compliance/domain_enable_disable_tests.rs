//! Domain Enable/Disable Compliance Tests (FEAT-043)
//!
//! Tests that all CDP domains properly implement enable/disable functionality
//! according to the CDP specification.

use profiler_domains::{HeapProfilerDomain, ProfilerDomain, TimelineDomain};
use protocol_handler::DomainHandler;
use serde_json::json;

// ============================================================================
// Profiler Domain Enable/Disable Tests
// ============================================================================

#[tokio::test]
async fn test_profiler_enable() {
    let domain = ProfilerDomain::new();
    assert_eq!(domain.name(), "Profiler");

    let result = domain.handle_method("enable", None).await;
    assert!(result.is_ok());

    let value = result.unwrap();
    assert_eq!(value, json!({}));
}

#[tokio::test]
async fn test_profiler_disable() {
    let domain = ProfilerDomain::new();

    // Enable first
    domain.handle_method("enable", None).await.unwrap();

    // Then disable
    let result = domain.handle_method("disable", None).await;
    assert!(result.is_ok());

    let value = result.unwrap();
    assert_eq!(value, json!({}));
}

#[tokio::test]
async fn test_profiler_double_enable() {
    let domain = ProfilerDomain::new();

    // Enable twice should work
    domain.handle_method("enable", None).await.unwrap();
    let result = domain.handle_method("enable", None).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_profiler_disable_without_enable() {
    let domain = ProfilerDomain::new();

    // Disable without enable should work (idempotent)
    let result = domain.handle_method("disable", None).await;
    assert!(result.is_ok());
}

// ============================================================================
// HeapProfiler Domain Enable/Disable Tests
// ============================================================================

#[tokio::test]
async fn test_heap_profiler_enable() {
    let domain = HeapProfilerDomain::new();
    assert_eq!(domain.name(), "HeapProfiler");

    let result = domain.handle_method("enable", None).await;
    assert!(result.is_ok());

    let value = result.unwrap();
    assert_eq!(value, json!({}));
}

#[tokio::test]
async fn test_heap_profiler_disable() {
    let domain = HeapProfilerDomain::new();

    domain.handle_method("enable", None).await.unwrap();

    let result = domain.handle_method("disable", None).await;
    assert!(result.is_ok());

    let value = result.unwrap();
    assert_eq!(value, json!({}));
}

#[tokio::test]
async fn test_heap_profiler_double_enable() {
    let domain = HeapProfilerDomain::new();

    domain.handle_method("enable", None).await.unwrap();
    let result = domain.handle_method("enable", None).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_heap_profiler_disable_without_enable() {
    let domain = HeapProfilerDomain::new();

    let result = domain.handle_method("disable", None).await;
    assert!(result.is_ok());
}

// ============================================================================
// Timeline Domain Enable/Disable Tests
// ============================================================================

#[tokio::test]
async fn test_timeline_enable() {
    let domain = TimelineDomain::new();
    assert_eq!(domain.name(), "Timeline");

    let result = domain.handle_method("enable", None).await;
    assert!(result.is_ok());

    let value = result.unwrap();
    assert_eq!(value, json!({}));
}

#[tokio::test]
async fn test_timeline_disable() {
    let domain = TimelineDomain::new();

    domain.handle_method("enable", None).await.unwrap();

    let result = domain.handle_method("disable", None).await;
    assert!(result.is_ok());

    let value = result.unwrap();
    assert_eq!(value, json!({}));
}

#[tokio::test]
async fn test_timeline_disable_stops_recording() {
    let domain = TimelineDomain::new();

    domain.handle_method("enable", None).await.unwrap();
    domain.handle_method("start", None).await.unwrap();

    assert!(domain.is_recording());

    domain.handle_method("disable", None).await.unwrap();

    assert!(!domain.is_recording());
}

// ============================================================================
// Domain State Consistency Tests
// ============================================================================

#[tokio::test]
async fn test_profiler_enable_state_consistency() {
    let domain = ProfilerDomain::new();

    // Initially not profiling
    assert!(!domain.is_profiling());

    // Enable and start
    domain.handle_method("enable", None).await.unwrap();
    domain.handle_method("start", None).await.unwrap();

    assert!(domain.is_profiling());

    // Disable should stop profiling
    domain.handle_method("disable", None).await.unwrap();

    assert!(!domain.is_profiling());
}

#[tokio::test]
async fn test_profiler_coverage_state_consistency() {
    let domain = ProfilerDomain::new();

    assert!(!domain.is_coverage_active());

    domain.handle_method("enable", None).await.unwrap();
    domain
        .handle_method("startPreciseCoverage", None)
        .await
        .unwrap();

    assert!(domain.is_coverage_active());

    domain.handle_method("disable", None).await.unwrap();

    assert!(!domain.is_coverage_active());
}

#[tokio::test]
async fn test_timeline_state_consistency() {
    let domain = TimelineDomain::new();

    assert!(!domain.is_enabled());
    assert!(!domain.is_recording());

    domain.handle_method("enable", None).await.unwrap();

    assert!(domain.is_enabled());
    assert!(!domain.is_recording());

    domain.handle_method("start", None).await.unwrap();

    assert!(domain.is_enabled());
    assert!(domain.is_recording());

    domain.handle_method("stop", None).await.unwrap();

    assert!(domain.is_enabled());
    assert!(!domain.is_recording());

    domain.handle_method("disable", None).await.unwrap();

    assert!(!domain.is_enabled());
}

// ============================================================================
// Domain Name Tests
// ============================================================================

#[test]
fn test_all_domain_names() {
    let profiler = ProfilerDomain::new();
    let heap_profiler = HeapProfilerDomain::new();
    let timeline = TimelineDomain::new();

    assert_eq!(profiler.name(), "Profiler");
    assert_eq!(heap_profiler.name(), "HeapProfiler");
    assert_eq!(timeline.name(), "Timeline");
}

// ============================================================================
// Enable/Disable Idempotency Tests
// ============================================================================

#[tokio::test]
async fn test_profiler_enable_idempotent() {
    let domain = ProfilerDomain::new();

    // Multiple enables should all succeed
    for _ in 0..5 {
        let result = domain.handle_method("enable", None).await;
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_profiler_disable_idempotent() {
    let domain = ProfilerDomain::new();

    domain.handle_method("enable", None).await.unwrap();

    // Multiple disables should all succeed
    for _ in 0..5 {
        let result = domain.handle_method("disable", None).await;
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_heap_profiler_enable_idempotent() {
    let domain = HeapProfilerDomain::new();

    for _ in 0..5 {
        let result = domain.handle_method("enable", None).await;
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_timeline_enable_idempotent() {
    let domain = TimelineDomain::new();

    for _ in 0..5 {
        let result = domain.handle_method("enable", None).await;
        assert!(result.is_ok());
    }
}
