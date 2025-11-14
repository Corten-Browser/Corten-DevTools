//! Integration tests for DevTools component
//!
//! These tests verify end-to-end functionality including server lifecycle,
//! domain registration, and configuration.

use devtools_component::{DevToolsComponent, DevToolsConfig};

#[tokio::test]
async fn test_full_lifecycle() {
    // Test complete lifecycle: create, start, use, stop
    let config = DevToolsConfig::builder()
        .port(0) // Ephemeral port
        .build();

    let devtools = DevToolsComponent::new(config).expect("Failed to create component");

    // Verify initial state
    assert!(!devtools.is_running());
    assert!(devtools.actual_port().is_none());

    // Start server
    devtools.start().await.expect("Failed to start server");
    assert!(devtools.is_running());

    // Verify we got a port
    let port = devtools.actual_port().expect("No port assigned");
    assert!(port > 0);

    // Verify we can get URLs
    let ws_url = devtools.get_debugger_url("test-page");
    assert!(ws_url.contains(&port.to_string()));
    assert!(ws_url.starts_with("ws://localhost:"));

    let json_url = devtools.get_json_url();
    assert!(json_url.contains(&port.to_string()));
    assert!(json_url.starts_with("http://localhost:"));

    // Stop server
    devtools.stop().await.expect("Failed to stop server");
    assert!(!devtools.is_running());
    assert!(devtools.actual_port().is_none());
}

#[tokio::test]
async fn test_multiple_start_stop_cycles() {
    // Test that we can start and stop multiple times
    let config = DevToolsConfig::builder().port(0).build();
    let devtools = DevToolsComponent::new(config).expect("Failed to create component");

    // Cycle 1
    devtools.start().await.expect("Failed to start (cycle 1)");
    let port1 = devtools.actual_port();
    devtools.stop().await.expect("Failed to stop (cycle 1)");

    // Cycle 2
    devtools.start().await.expect("Failed to start (cycle 2)");
    let port2 = devtools.actual_port();
    devtools.stop().await.expect("Failed to stop (cycle 2)");

    // Cycle 3
    devtools.start().await.expect("Failed to start (cycle 3)");
    let port3 = devtools.actual_port();
    devtools.stop().await.expect("Failed to stop (cycle 3)");

    // All cycles should have gotten valid ports
    assert!(port1.is_some());
    assert!(port2.is_some());
    assert!(port3.is_some());

    // Final state should be stopped
    assert!(!devtools.is_running());
}

#[tokio::test]
async fn test_domain_registration_count() {
    // Verify all 13 domains are registered
    let config = DevToolsConfig::default();
    let devtools = DevToolsComponent::new(config).expect("Failed to create component");

    let domains = devtools.registered_domains();

    // Should have exactly 13 domains
    assert_eq!(
        domains.len(),
        13,
        "Expected 13 domains, got {}",
        domains.len()
    );

    // Verify each expected domain
    let expected_domains = vec![
        "Browser",
        "Page",
        "Security",
        "Emulation",
        "DOM",
        "CSS",
        "Network",
        "Runtime",
        "Debugger",
        "Profiler",
        "HeapProfiler",
        "Console",
        "Storage",
    ];

    for expected in expected_domains {
        assert!(
            domains.contains(&expected),
            "Missing domain: {}",
            expected
        );
    }
}

#[tokio::test]
async fn test_config_applied_correctly() {
    // Test that custom configuration is properly applied
    let config = DevToolsConfig::builder()
        .port(0)
        .enable_remote_debugging(true)
        .max_message_size(50 * 1024 * 1024) // 50MB
        .allowed_origin("https://example.com".to_string())
        .protocol_version("1.4".to_string())
        .build();

    let devtools = DevToolsComponent::new(config).expect("Failed to create component");

    // Verify config is stored correctly
    assert!(devtools.config().enable_remote_debugging());
    assert_eq!(devtools.config().max_message_size(), 50 * 1024 * 1024);
    assert_eq!(devtools.config().protocol_version(), "1.4");
    assert!(devtools
        .config()
        .allowed_origins()
        .contains(&"https://example.com".to_string()));
}

#[tokio::test]
async fn test_error_start_when_already_running() {
    // Verify proper error when trying to start an already running server
    let config = DevToolsConfig::builder().port(0).build();
    let devtools = DevToolsComponent::new(config).expect("Failed to create component");

    // Start once
    devtools.start().await.expect("First start failed");

    // Try to start again - should error
    let result = devtools.start().await;
    assert!(
        result.is_err(),
        "Expected error when starting already running server"
    );

    // Clean up
    devtools.stop().await.expect("Failed to stop");
}

#[tokio::test]
async fn test_error_stop_when_not_running() {
    // Verify proper error when trying to stop a non-running server
    let config = DevToolsConfig::default();
    let devtools = DevToolsComponent::new(config).expect("Failed to create component");

    // Try to stop without starting - should error
    let result = devtools.stop().await;
    assert!(
        result.is_err(),
        "Expected error when stopping non-running server"
    );
}

#[tokio::test]
async fn test_ephemeral_port_allocation() {
    // Test that port 0 gives us a valid ephemeral port
    let config = DevToolsConfig::builder().port(0).build();
    let devtools = DevToolsComponent::new(config).expect("Failed to create component");

    devtools.start().await.expect("Failed to start");

    let port = devtools.actual_port().expect("No port allocated");

    // Ephemeral ports are typically > 1024
    assert!(port > 1024, "Expected ephemeral port, got {}", port);

    devtools.stop().await.expect("Failed to stop");
}

#[tokio::test]
async fn test_url_generation() {
    // Test URL generation with different target IDs
    let config = DevToolsConfig::builder().port(9222).build();
    let devtools = DevToolsComponent::new(config).expect("Failed to create component");

    // Test debugger URLs with different target IDs
    assert_eq!(
        devtools.get_debugger_url("page-1"),
        "ws://localhost:9222/devtools/page/page-1"
    );
    assert_eq!(
        devtools.get_debugger_url("page-2"),
        "ws://localhost:9222/devtools/page/page-2"
    );
    assert_eq!(
        devtools.get_debugger_url("worker-123"),
        "ws://localhost:9222/devtools/page/worker-123"
    );

    // Test JSON URL
    assert_eq!(devtools.get_json_url(), "http://localhost:9222/json");
}

#[tokio::test]
async fn test_concurrent_operations() {
    // Test that component handles concurrent operations safely
    let config = DevToolsConfig::builder().port(0).build();
    let devtools = std::sync::Arc::new(
        DevToolsComponent::new(config).expect("Failed to create component"),
    );

    // Start the server
    devtools.start().await.expect("Failed to start");

    // Spawn multiple tasks that query component state
    let mut handles = vec![];

    for i in 0..10 {
        let devtools_clone = std::sync::Arc::clone(&devtools);
        let handle = tokio::spawn(async move {
            // Each task checks if running and gets URLs
            assert!(devtools_clone.is_running());
            let _url = devtools_clone.get_debugger_url(&format!("task-{}", i));
            let _json = devtools_clone.get_json_url();
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.expect("Task panicked");
    }

    // Stop the server
    devtools.stop().await.expect("Failed to stop");
}
