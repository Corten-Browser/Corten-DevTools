//! Unit tests for MemoryProfiler
//!
//! These tests verify the memory profiler implementation with allocation tracking
//! and leak detection capabilities.

use profiler_domains::{CallFrame, MemoryProfiler, MemoryProfilerConfig, MemoryStats};

#[test]
fn test_memory_profiler_creation() {
    let profiler = MemoryProfiler::new();
    assert!(!profiler.is_tracking());
}

#[test]
fn test_start_stop_tracking() {
    let profiler = MemoryProfiler::new();

    assert!(profiler.start_tracking().is_ok());
    assert!(profiler.is_tracking());

    let snapshot = profiler.stop_tracking();
    assert!(snapshot.is_ok());
    assert!(!profiler.is_tracking());
}

#[test]
fn test_cannot_start_twice() {
    let profiler = MemoryProfiler::new();

    assert!(profiler.start_tracking().is_ok());
    assert!(profiler.start_tracking().is_err());

    profiler.stop_tracking().unwrap();
}

#[test]
fn test_cannot_stop_when_not_tracking() {
    let profiler = MemoryProfiler::new();
    assert!(profiler.stop_tracking().is_err());
}

#[test]
fn test_record_allocation() {
    let profiler = MemoryProfiler::new();
    profiler.start_tracking().unwrap();

    let stack_trace = vec![CallFrame {
        function_name: "allocate".to_string(),
        script_id: "1".to_string(),
        url: "test.js".to_string(),
        line_number: 10,
        column_number: 5,
    }];

    let id = profiler.record_allocation(2048, stack_trace).unwrap();
    assert!(id > 0);

    let stats = profiler.get_stats();
    assert_eq!(stats.live_allocations, 1);
    assert_eq!(stats.current_heap_bytes, 2048);
}

#[test]
fn test_cannot_allocate_when_not_tracking() {
    let profiler = MemoryProfiler::new();
    assert!(profiler.record_allocation(1024, vec![]).is_err());
}

#[test]
fn test_record_deallocation() {
    let profiler = MemoryProfiler::new();
    profiler.start_tracking().unwrap();

    let id = profiler.record_allocation(2048, vec![]).unwrap();
    assert!(profiler.record_deallocation(id).is_ok());

    let stats = profiler.get_stats();
    assert_eq!(stats.live_allocations, 0);
    assert_eq!(stats.freed_allocations, 1);
    assert_eq!(stats.total_freed_bytes, 2048);
}

#[test]
fn test_cannot_deallocate_when_not_tracking() {
    let profiler = MemoryProfiler::new();
    assert!(profiler.record_deallocation(123).is_err());
}

#[test]
fn test_double_free_detection() {
    let profiler = MemoryProfiler::new();
    profiler.start_tracking().unwrap();

    let id = profiler.record_allocation(2048, vec![]).unwrap();
    assert!(profiler.record_deallocation(id).is_ok());
    assert!(profiler.record_deallocation(id).is_err());
}

#[test]
fn test_unknown_allocation_deallocation() {
    let profiler = MemoryProfiler::new();
    profiler.start_tracking().unwrap();

    assert!(profiler.record_deallocation(999999).is_err());
}

#[test]
fn test_take_snapshot() {
    let profiler = MemoryProfiler::new();
    profiler.start_tracking().unwrap();

    profiler.record_allocation(1024, vec![]).unwrap();
    profiler.record_allocation(2048, vec![]).unwrap();

    let snapshot = profiler.take_snapshot().unwrap();
    assert_eq!(snapshot.used_heap_size, 3072);
    assert!(!snapshot.allocation_summaries.is_empty());
}

#[test]
fn test_allocation_summaries_by_call_site() {
    let profiler = MemoryProfiler::new();
    profiler.start_tracking().unwrap();

    let stack_trace = vec![CallFrame {
        function_name: "createBuffer".to_string(),
        script_id: "1".to_string(),
        url: "test.js".to_string(),
        line_number: 42,
        column_number: 0,
    }];

    // Multiple allocations from same call site
    for _ in 0..5 {
        profiler.record_allocation(1024, stack_trace.clone()).unwrap();
    }

    let snapshot = profiler.take_snapshot().unwrap();
    let summary = snapshot
        .allocation_summaries
        .iter()
        .find(|s| s.call_site.contains("createBuffer"))
        .unwrap();

    assert_eq!(summary.allocation_count, 5);
    assert_eq!(summary.total_bytes, 5 * 1024);
    assert_eq!(summary.live_bytes, 5 * 1024);
}

#[test]
fn test_min_allocation_size_filter() {
    let mut config = MemoryProfilerConfig::default();
    config.min_allocation_size = 2048;

    let profiler = MemoryProfiler::with_config(config);
    profiler.start_tracking().unwrap();

    // This should be skipped (too small)
    let id1 = profiler.record_allocation(1024, vec![]).unwrap();
    assert_eq!(id1, 0);

    // This should be tracked
    let id2 = profiler.record_allocation(4096, vec![]).unwrap();
    assert!(id2 > 0);

    let stats = profiler.get_stats();
    assert_eq!(stats.live_allocations, 1);
}

#[test]
fn test_potential_leak_detection() {
    let mut config = MemoryProfilerConfig::default();
    config.leak_threshold_age = 1.0; // Very short for testing

    let profiler = MemoryProfiler::with_config(config);
    profiler.start_tracking().unwrap();

    // Large allocation
    profiler.record_allocation(2_000_000, vec![]).unwrap();

    // Wait a tiny bit
    std::thread::sleep(std::time::Duration::from_micros(10));

    let snapshot = profiler.take_snapshot().unwrap();
    assert!(!snapshot.potential_leaks.is_empty());

    let leak = &snapshot.potential_leaks[0];
    assert!(leak.leak_score > 0);
    assert!(leak.size == 2_000_000);
}

#[test]
fn test_memory_timeline() {
    let mut config = MemoryProfilerConfig::default();
    config.timeline_interval = 1.0; // 1 microsecond

    let profiler = MemoryProfiler::with_config(config);
    profiler.start_tracking().unwrap();

    for _ in 0..5 {
        profiler.record_allocation(1024, vec![]).unwrap();
        std::thread::sleep(std::time::Duration::from_micros(5));
    }

    let timeline = profiler.get_timeline();
    assert!(!timeline.is_empty());
}

#[test]
fn test_get_live_allocations() {
    let profiler = MemoryProfiler::new();
    profiler.start_tracking().unwrap();

    let id1 = profiler.record_allocation(1024, vec![]).unwrap();
    let id2 = profiler.record_allocation(2048, vec![]).unwrap();
    profiler.record_deallocation(id1).unwrap();

    let live = profiler.get_live_allocations();
    assert_eq!(live.len(), 1);
    assert_eq!(live[0].id, id2);
    assert_eq!(live[0].size, 2048);
}

#[test]
fn test_force_gc() {
    let profiler = MemoryProfiler::new();
    profiler.start_tracking().unwrap();

    // Should not panic
    profiler.force_gc();

    let timeline = profiler.get_timeline();
    assert!(!timeline.is_empty());
}

#[test]
fn test_config_modification() {
    let profiler = MemoryProfiler::new();

    let mut new_config = MemoryProfilerConfig::default();
    new_config.max_stack_depth = 8;
    new_config.min_allocation_size = 4096;

    profiler.set_config(new_config.clone());

    let config = profiler.get_config();
    assert_eq!(config.max_stack_depth, 8);
    assert_eq!(config.min_allocation_size, 4096);
}

#[test]
fn test_stack_trace_truncation() {
    let mut config = MemoryProfilerConfig::default();
    config.max_stack_depth = 2;

    let profiler = MemoryProfiler::with_config(config);
    profiler.start_tracking().unwrap();

    // Create a deep stack trace
    let deep_trace: Vec<CallFrame> = (0..10)
        .map(|i| CallFrame {
            function_name: format!("func_{}", i),
            script_id: "1".to_string(),
            url: "test.js".to_string(),
            line_number: i,
            column_number: 0,
        })
        .collect();

    profiler.record_allocation(1024, deep_trace).unwrap();

    let live = profiler.get_live_allocations();
    assert_eq!(live[0].stack_trace.len(), 2); // Should be truncated
}

#[test]
fn test_disable_stack_traces() {
    let mut config = MemoryProfilerConfig::default();
    config.capture_stack_traces = false;

    let profiler = MemoryProfiler::with_config(config);
    profiler.start_tracking().unwrap();

    let stack_trace = vec![CallFrame {
        function_name: "test".to_string(),
        script_id: "1".to_string(),
        url: "test.js".to_string(),
        line_number: 1,
        column_number: 0,
    }];

    profiler.record_allocation(1024, stack_trace).unwrap();

    let live = profiler.get_live_allocations();
    assert!(live[0].stack_trace.is_empty());
}

#[test]
fn test_memory_stats() {
    let profiler = MemoryProfiler::new();
    profiler.start_tracking().unwrap();

    let id1 = profiler.record_allocation(1024, vec![]).unwrap();
    let id2 = profiler.record_allocation(2048, vec![]).unwrap();
    profiler.record_deallocation(id1).unwrap();

    let stats = profiler.get_stats();
    assert_eq!(stats.total_allocations, 2);
    assert_eq!(stats.live_allocations, 1);
    assert_eq!(stats.freed_allocations, 1);
    assert_eq!(stats.total_allocated_bytes, 3072);
    assert_eq!(stats.total_freed_bytes, 1024);
    assert_eq!(stats.current_heap_bytes, 2048);
}
