//! Unit tests for CpuProfiler
//!
//! These tests verify the enhanced CPU profiler implementation.

use profiler_domains::{CallFrame, CpuProfiler, ProfileStats};

#[test]
fn test_cpu_profiler_creation() {
    let profiler = CpuProfiler::new();
    assert!(!profiler.is_profiling());
    assert_eq!(profiler.get_sampling_interval(), 100);
}

#[test]
fn test_set_sampling_interval() {
    let profiler = CpuProfiler::new();
    profiler.set_sampling_interval(500);
    assert_eq!(profiler.get_sampling_interval(), 500);
}

#[test]
fn test_start_stop_profiling() {
    let profiler = CpuProfiler::new();

    assert!(profiler.start().is_ok());
    assert!(profiler.is_profiling());

    let profile = profiler.stop();
    assert!(profile.is_ok());
    assert!(!profiler.is_profiling());
}

#[test]
fn test_cannot_start_twice() {
    let profiler = CpuProfiler::new();

    assert!(profiler.start().is_ok());
    assert!(profiler.start().is_err());

    profiler.stop().unwrap();
}

#[test]
fn test_cannot_stop_when_not_profiling() {
    let profiler = CpuProfiler::new();
    assert!(profiler.stop().is_err());
}

#[test]
fn test_add_sample() {
    let profiler = CpuProfiler::new();
    profiler.start().unwrap();

    let call_stack = vec![
        CallFrame {
            function_name: "main".to_string(),
            script_id: "1".to_string(),
            url: "http://example.com/script.js".to_string(),
            line_number: 10,
            column_number: 5,
        },
        CallFrame {
            function_name: "helper".to_string(),
            script_id: "1".to_string(),
            url: "http://example.com/script.js".to_string(),
            line_number: 50,
            column_number: 10,
        },
    ];

    assert!(profiler.add_sample(call_stack).is_ok());

    let profile = profiler.stop().unwrap();
    assert!(profile.nodes.len() >= 2); // Root + at least one other node
}

#[test]
fn test_cannot_add_sample_when_not_profiling() {
    let profiler = CpuProfiler::new();

    let call_stack = vec![CallFrame {
        function_name: "test".to_string(),
        script_id: "1".to_string(),
        url: "test.js".to_string(),
        line_number: 1,
        column_number: 0,
    }];

    assert!(profiler.add_sample(call_stack).is_err());
}

#[test]
fn test_generate_call_tree() {
    let profiler = CpuProfiler::new();
    profiler.start().unwrap();

    // Add multiple samples
    for _ in 0..5 {
        let call_stack = vec![
            CallFrame {
                function_name: "main".to_string(),
                script_id: "1".to_string(),
                url: "test.js".to_string(),
                line_number: 1,
                column_number: 0,
            },
            CallFrame {
                function_name: "child".to_string(),
                script_id: "1".to_string(),
                url: "test.js".to_string(),
                line_number: 10,
                column_number: 0,
            },
        ];
        profiler.add_sample(call_stack).unwrap();
    }

    profiler.stop().unwrap();

    let tree = profiler.generate_call_tree();
    assert_eq!(tree.function_name, "(root)");
    assert!(tree.total_time > 0.0);
}

#[test]
fn test_export_profile_requires_stopped() {
    let profiler = CpuProfiler::new();
    profiler.start().unwrap();

    let result = profiler.export_profile();
    assert!(result.is_err());

    profiler.stop().unwrap();
}

#[test]
fn test_export_profile_after_stop() {
    let profiler = CpuProfiler::new();
    profiler.start().unwrap();

    let call_stack = vec![CallFrame {
        function_name: "test".to_string(),
        script_id: "1".to_string(),
        url: "test.js".to_string(),
        line_number: 1,
        column_number: 0,
    }];
    profiler.add_sample(call_stack).unwrap();
    profiler.stop().unwrap();

    let json = profiler.export_profile();
    assert!(json.is_ok());
    let json_str = json.unwrap();
    assert!(json_str.contains("nodes"));
    assert!(json_str.contains("startTime"));
}

#[test]
fn test_profile_stats() {
    let profiler = CpuProfiler::new();
    profiler.start().unwrap();

    for i in 0..10 {
        let call_stack = vec![CallFrame {
            function_name: format!("func_{}", i % 3),
            script_id: "1".to_string(),
            url: "test.js".to_string(),
            line_number: i,
            column_number: 0,
        }];
        profiler.add_sample(call_stack).unwrap();
    }

    profiler.stop().unwrap();

    let stats = profiler.get_stats();
    assert_eq!(stats.total_samples, 10);
    assert!(stats.total_nodes > 1);
    assert!(!stats.hot_functions.is_empty());
}

#[test]
fn test_set_title() {
    let profiler = CpuProfiler::new();
    profiler.set_title("Test Profile".to_string());
    profiler.start().unwrap();

    let call_stack = vec![CallFrame {
        function_name: "test".to_string(),
        script_id: "1".to_string(),
        url: "test.js".to_string(),
        line_number: 1,
        column_number: 0,
    }];
    profiler.add_sample(call_stack).unwrap();

    let profile = profiler.stop().unwrap();
    assert_eq!(profile.title, Some("Test Profile".to_string()));
}

#[test]
fn test_multiple_samples_same_function() {
    let profiler = CpuProfiler::new();
    profiler.start().unwrap();

    let call_stack = vec![CallFrame {
        function_name: "repeat".to_string(),
        script_id: "1".to_string(),
        url: "test.js".to_string(),
        line_number: 5,
        column_number: 0,
    }];

    // Add same call stack multiple times
    for _ in 0..10 {
        profiler.add_sample(call_stack.clone()).unwrap();
    }

    let profile = profiler.stop().unwrap();

    // Should have root + one function node
    assert_eq!(profile.nodes.len(), 2);

    // The function should have hit_count of 10
    let func_node = profile.nodes.iter().find(|n| n.call_frame.function_name == "repeat");
    assert!(func_node.is_some());
    assert_eq!(func_node.unwrap().hit_count, Some(10));
}

#[test]
fn test_position_ticks() {
    let profiler = CpuProfiler::new();
    profiler.start().unwrap();

    // Add samples at same line
    for _ in 0..5 {
        let call_stack = vec![CallFrame {
            function_name: "func".to_string(),
            script_id: "1".to_string(),
            url: "test.js".to_string(),
            line_number: 42,
            column_number: 0,
        }];
        profiler.add_sample(call_stack).unwrap();
    }

    let profile = profiler.stop().unwrap();

    let func_node = profile.nodes.iter().find(|n| n.call_frame.function_name == "func");
    assert!(func_node.is_some());

    let node = func_node.unwrap();
    assert!(node.position_ticks.is_some());
    let ticks = node.position_ticks.as_ref().unwrap();
    assert!(!ticks.is_empty());
    assert_eq!(ticks[0].line, 42);
    assert_eq!(ticks[0].ticks, 5);
}
