//! Unit tests for ProfilerDomain
//!
//! These tests verify the ProfilerDomain implementation following TDD principles.

use profiler_domains::{CoverageRange, FunctionCoverage, ProfilerDomain, ScriptCoverage};
use protocol_handler::DomainHandler;
use serde_json::json;

#[tokio::test]
async fn test_profiler_domain_name() {
    let profiler = ProfilerDomain::new();
    assert_eq!(profiler.name(), "Profiler");
}

#[tokio::test]
async fn test_profiler_enable() {
    let profiler = ProfilerDomain::new();
    let result = profiler.handle_method("enable", None).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), json!({}));
}

#[tokio::test]
async fn test_profiler_disable() {
    let profiler = ProfilerDomain::new();

    // Enable first
    let _ = profiler.handle_method("enable", None).await;

    // Then disable
    let result = profiler.handle_method("disable", None).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), json!({}));
}

#[tokio::test]
async fn test_profiler_start() {
    let profiler = ProfilerDomain::new();

    // Enable profiler first
    let _ = profiler.handle_method("enable", None).await;

    // Start profiling
    let result = profiler.handle_method("start", None).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), json!({}));

    // Verify profiling is active
    assert!(profiler.is_profiling());
}

#[tokio::test]
async fn test_profiler_stop() {
    let profiler = ProfilerDomain::new();

    // Enable and start profiling
    let _ = profiler.handle_method("enable", None).await;
    let _ = profiler.handle_method("start", None).await;

    // Stop profiling
    let result = profiler.handle_method("stop", None).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response["profile"].is_object());
    assert!(response["profile"]["nodes"].is_array());

    // Verify profiling is inactive
    assert!(!profiler.is_profiling());
}

#[tokio::test]
async fn test_profiler_start_precise_coverage() {
    let profiler = ProfilerDomain::new();

    // Enable profiler first
    let _ = profiler.handle_method("enable", None).await;

    // Start precise coverage with params
    let params = json!({
        "callCount": true,
        "detailed": true
    });

    let result = profiler
        .handle_method("startPreciseCoverage", Some(params))
        .await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), json!({"timestamp": 0.0}));

    // Verify coverage is active
    assert!(profiler.is_coverage_active());
}

#[tokio::test]
async fn test_profiler_take_precise_coverage() {
    let profiler = ProfilerDomain::new();

    // Enable and start coverage
    let _ = profiler.handle_method("enable", None).await;
    let _ = profiler.handle_method("startPreciseCoverage", None).await;

    // Take coverage snapshot
    let result = profiler.handle_method("takePreciseCoverage", None).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response["result"].is_array());
    assert!(response["timestamp"].is_number());
}

#[tokio::test]
async fn test_profiler_stop_precise_coverage() {
    let profiler = ProfilerDomain::new();

    // Enable and start coverage
    let _ = profiler.handle_method("enable", None).await;
    let _ = profiler.handle_method("startPreciseCoverage", None).await;

    // Stop coverage
    let result = profiler.handle_method("stopPreciseCoverage", None).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), json!({}));

    // Verify coverage is inactive
    assert!(!profiler.is_coverage_active());
}

#[tokio::test]
async fn test_profiler_get_best_effort_coverage() {
    let profiler = ProfilerDomain::new();

    // Enable profiler
    let _ = profiler.handle_method("enable", None).await;

    // Get best effort coverage (doesn't require startPreciseCoverage)
    let result = profiler.handle_method("getBestEffortCoverage", None).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response["result"].is_array());
}

#[tokio::test]
async fn test_profiler_unknown_method() {
    let profiler = ProfilerDomain::new();

    let result = profiler.handle_method("unknownMethod", None).await;
    assert!(result.is_err());
}

#[test]
fn test_script_coverage_serialization() {
    let coverage = ScriptCoverage {
        script_id: "123".to_string(),
        url: "http://example.com/script.js".to_string(),
        functions: vec![FunctionCoverage {
            function_name: "main".to_string(),
            ranges: vec![CoverageRange {
                start_offset: 0,
                end_offset: 100,
                count: 5,
            }],
            is_block_coverage: true,
        }],
    };

    let json = serde_json::to_value(&coverage).unwrap();
    assert_eq!(json["scriptId"], "123");
    assert_eq!(json["url"], "http://example.com/script.js");
    assert!(json["functions"].is_array());
}

#[test]
fn test_coverage_range_creation() {
    let range = CoverageRange {
        start_offset: 10,
        end_offset: 50,
        count: 3,
    };

    assert_eq!(range.start_offset, 10);
    assert_eq!(range.end_offset, 50);
    assert_eq!(range.count, 3);
}

#[test]
fn test_function_coverage_creation() {
    let func_coverage = FunctionCoverage {
        function_name: "testFunc".to_string(),
        ranges: vec![],
        is_block_coverage: false,
    };

    assert_eq!(func_coverage.function_name, "testFunc");
    assert_eq!(func_coverage.ranges.len(), 0);
    assert!(!func_coverage.is_block_coverage);
}
