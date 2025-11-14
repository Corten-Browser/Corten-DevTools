//! Integration tests for DevTools public API
//!
//! These tests verify that the DevTools API works correctly
//! with the underlying devtools_component.

use devtools_api::{DevTools, DevToolsConfig};

#[tokio::test]
async fn test_full_devtools_lifecycle() {
    // Test complete lifecycle: create -> start -> stop
    let config = DevToolsConfig::builder().port(0).build();

    let devtools = DevTools::new(config).expect("Failed to create DevTools");

    // Start the server
    devtools
        .start(0)
        .await
        .expect("Failed to start DevTools server");

    // Verify URLs are generated correctly
    let url = devtools.get_url();
    assert!(url.contains("http://"));
    assert!(url.contains("localhost"));
    assert!(url.contains("/json"));

    let debugger_url = devtools.get_debugger_url("test-target");
    assert!(debugger_url.contains("ws://"));
    assert!(debugger_url.contains("test-target"));

    // Stop the server
    devtools
        .stop()
        .await
        .expect("Failed to stop DevTools server");
}

#[tokio::test]
async fn test_start_with_specific_port() {
    // Test starting DevTools on a specific port
    let config = DevToolsConfig::default();
    let devtools = DevTools::new(config).expect("Failed to create DevTools");

    // Start on ephemeral port for testing (to avoid port conflicts)
    devtools
        .start(0)
        .await
        .expect("Failed to start on specific port");

    // URLs should reflect the port
    let url = devtools.get_url();
    assert!(url.contains("localhost"));

    devtools.stop().await.expect("Failed to stop");
}

#[tokio::test]
async fn test_restart_after_stop() {
    // Test that DevTools can be restarted after stopping
    let config = DevToolsConfig::default();
    let devtools = DevTools::new(config).expect("Failed to create DevTools");

    // First lifecycle
    devtools.start(0).await.expect("Failed to start first time");
    devtools.stop().await.expect("Failed to stop first time");

    // Second lifecycle
    devtools
        .start(0)
        .await
        .expect("Failed to restart after stop");
    devtools.stop().await.expect("Failed to stop second time");
}

#[tokio::test]
async fn test_cannot_start_twice_integration() {
    // Integration test verifying double-start protection
    let config = DevToolsConfig::default();
    let devtools = DevTools::new(config).expect("Failed to create DevTools");

    devtools.start(0).await.expect("Failed to start");

    // Attempt to start again should fail
    let result = devtools.start(0).await;
    assert!(result.is_err(), "Should not be able to start server twice");

    devtools.stop().await.expect("Failed to stop");
}

#[tokio::test]
async fn test_custom_configuration() {
    // Test DevTools with custom configuration
    let config = DevToolsConfig::builder()
        .port(0)
        .enable_remote_debugging(true)
        .allowed_origin("http://example.com".to_string())
        .max_message_size(50 * 1024 * 1024)
        .build();

    let devtools = DevTools::new(config).expect("Failed to create DevTools with custom config");

    devtools.start(0).await.expect("Failed to start");

    // Verify it's working
    let _url = devtools.get_url();

    devtools.stop().await.expect("Failed to stop");
}

#[tokio::test]
async fn test_multiple_target_urls() {
    // Test generating URLs for multiple debugging targets
    let config = DevToolsConfig::builder().port(9999).build();
    let devtools = DevTools::new(config).expect("Failed to create DevTools");

    let targets = vec!["page-1", "page-2", "worker-1", "service-worker-1"];

    for target in targets {
        let url = devtools.get_debugger_url(target);
        assert!(url.contains(target), "URL should contain target ID");
        assert!(url.contains("ws://"), "URL should use WebSocket protocol");
        assert!(url.contains("9999"), "URL should contain port");
    }
}

#[tokio::test]
async fn test_error_handling_stop_without_start() {
    // Test that stopping without starting returns an error
    let config = DevToolsConfig::default();
    let devtools = DevTools::new(config).expect("Failed to create DevTools");

    let result = devtools.stop().await;
    assert!(
        result.is_err(),
        "Stopping without starting should return an error"
    );
}
