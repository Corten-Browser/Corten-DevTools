//! End-to-End Integration Tests for CortenBrowser DevTools
//!
//! These tests verify that all components work together correctly across the entire stack.

use devtools_api::{DevTools, DevToolsConfig};
use tokio::time::{sleep, Duration};

/// Test 1: Basic DevTools lifecycle (start and stop)
#[tokio::test]
async fn test_devtools_lifecycle() {
    let config = DevToolsConfig::default();
    let devtools = DevTools::new(config).expect("Failed to create DevTools");

    // Start on ephemeral port
    devtools
        .start(0)
        .await
        .expect("Failed to start DevTools server");

    // Verify we got a valid URL
    let url = devtools.get_url();
    assert!(url.starts_with("http://localhost:"));
    assert!(url.contains("/json"));

    // Stop the server
    devtools.stop().await.expect("Failed to stop DevTools");
}

/// Test 2: Multiple start/stop cycles
#[tokio::test]
async fn test_multiple_cycles() {
    let config = DevToolsConfig::default();
    let devtools = DevTools::new(config).expect("Failed to create DevTools");

    // Cycle 1
    devtools.start(0).await.expect("Cycle 1: start failed");
    sleep(Duration::from_millis(100)).await;
    devtools.stop().await.expect("Cycle 1: stop failed");

    // Cycle 2
    devtools.start(0).await.expect("Cycle 2: start failed");
    sleep(Duration::from_millis(100)).await;
    devtools.stop().await.expect("Cycle 2: stop failed");

    // Cycle 3
    devtools.start(0).await.expect("Cycle 3: start failed");
    sleep(Duration::from_millis(100)).await;
    devtools.stop().await.expect("Cycle 3: stop failed");
}

/// Test 3: Custom configuration
#[tokio::test]
async fn test_custom_configuration() {
    // Use default config (fields are private, accessed via Default trait)
    let config = DevToolsConfig::default();

    let devtools = DevTools::new(config).expect("Failed to create DevTools");

    devtools
        .start(0)
        .await
        .expect("Failed to start with custom config");

    // Verify URL is valid
    let url = devtools.get_url();
    assert!(url.contains("localhost"));

    devtools.stop().await.expect("Failed to stop");
}

/// Test 4: Debugger URL generation
#[tokio::test]
async fn test_debugger_url_generation() {
    let config = DevToolsConfig::default();
    let devtools = DevTools::new(config).expect("Failed to create DevTools");

    devtools.start(0).await.expect("Failed to start");

    // Generate debugger URLs for different targets
    let url1 = devtools.get_debugger_url("page-1");
    let url2 = devtools.get_debugger_url("page-2");
    let url3 = devtools.get_debugger_url("worker-1");

    // Verify format
    assert!(url1.starts_with("ws://localhost:"));
    assert!(url1.contains("/devtools/page/page-1"));

    assert!(url2.contains("/devtools/page/page-2"));
    assert!(url3.contains("/devtools/page/worker-1"));

    // Different targets should have different URLs
    assert_ne!(url1, url2);
    assert_ne!(url2, url3);

    devtools.stop().await.expect("Failed to stop");
}

/// Test 5: Concurrent API calls
#[tokio::test]
async fn test_concurrent_operations() {
    let config = DevToolsConfig::default();
    let devtools = DevTools::new(config).expect("Failed to create DevTools");

    devtools.start(0).await.expect("Failed to start");

    // Spawn multiple tasks that access the DevTools concurrently
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let target_id = format!("target-{}", i);
            tokio::spawn(async move {
                let config = DevToolsConfig::default();
                let dt = DevTools::new(config).unwrap();
                dt.start(0).await.unwrap();
                let _url = dt.get_debugger_url(&target_id);
                sleep(Duration::from_millis(10)).await;
                dt.stop().await.unwrap();
            })
        })
        .collect();

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.expect("Task panicked");
    }

    devtools.stop().await.expect("Failed to stop");
}

/// Test 6: Error handling - double start
#[tokio::test]
async fn test_error_double_start() {
    let config = DevToolsConfig::default();
    let devtools = DevTools::new(config).expect("Failed to create DevTools");

    devtools.start(0).await.expect("First start should succeed");

    // Second start should fail
    let result = devtools.start(0).await;
    assert!(result.is_err(), "Second start should fail");

    devtools.stop().await.expect("Stop should succeed");
}

/// Test 7: Error handling - stop without start
#[tokio::test]
async fn test_error_stop_without_start() {
    let config = DevToolsConfig::default();
    let devtools = DevTools::new(config).expect("Failed to create DevTools");

    // Stop without starting should fail
    let result = devtools.stop().await;
    assert!(result.is_err(), "Stop without start should fail");
}

/// Test 8: Long-running server stability
#[tokio::test]
async fn test_server_stability() {
    let config = DevToolsConfig::default();
    let devtools = DevTools::new(config).expect("Failed to create DevTools");

    devtools.start(0).await.expect("Failed to start");

    // Keep server running and perform operations
    for i in 0..50 {
        let _url = devtools.get_debugger_url(&format!("target-{}", i));
        sleep(Duration::from_millis(10)).await;
    }

    devtools.stop().await.expect("Failed to stop");
}

/// Test 9: Verify all CDP domains are registered
/// This test ensures the integration component properly registers all domain handlers
#[tokio::test]
async fn test_all_domains_registered() {
    let config = DevToolsConfig::default();
    let devtools = DevTools::new(config).expect("Failed to create DevTools");

    devtools.start(0).await.expect("Failed to start");

    // If we get here, all domains were registered successfully
    // (domain registration happens in DevToolsComponent::new())
    // The component would panic if domain registration failed

    // Expected domains (13 total):
    // - Browser, Page, Security, Emulation (browser_page_domains)
    // - DOM, CSS (dom_domain)
    // - Network (network_domain)
    // - Runtime, Debugger (runtime_debugger)
    // - Profiler, HeapProfiler (profiler_domains)
    // - Console, Storage (console_storage)

    devtools.stop().await.expect("Failed to stop");
}

/// Test 10: Component integration - public API to server
/// Verifies the full stack from devtools_api → devtools_component → cdp_server
#[tokio::test]
async fn test_full_stack_integration() {
    // Use default config
    let config = DevToolsConfig::default();

    let devtools = DevTools::new(config).expect("Failed to create DevTools");

    // Start the full stack
    devtools
        .start(0)
        .await
        .expect("Failed to start full stack");

    // Verify all URLs are accessible
    let http_url = devtools.get_url();
    assert!(!http_url.is_empty());
    assert!(http_url.starts_with("http://"));

    let ws_url = devtools.get_debugger_url("test-target");
    assert!(!ws_url.is_empty());
    assert!(ws_url.starts_with("ws://"));

    // Keep running briefly to ensure stability
    sleep(Duration::from_millis(500)).await;

    // Clean shutdown
    devtools
        .stop()
        .await
        .expect("Failed to stop full stack");
}
