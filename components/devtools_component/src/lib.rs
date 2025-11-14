//! Main DevTools orchestration and integration
//!
//! This module provides the DevToolsComponent that integrates all CDP domain handlers
//! with the WebSocket server to provide a complete Chrome DevTools Protocol implementation.
//!
//! # Example
//!
//! ```no_run
//! use devtools_component::{DevToolsComponent, DevToolsConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = DevToolsConfig::builder()
//!         .port(9222)
//!         .enable_remote_debugging(false)
//!         .build();
//!
//!     let devtools = DevToolsComponent::new(config)?;
//!     devtools.start().await?;
//!     Ok(())
//! }
//! ```

mod component;
mod config;
mod error;

pub use component::DevToolsComponent;
pub use config::{DevToolsConfig, DevToolsConfigBuilder};
pub use error::{DevToolsError, Result};

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // TDD Phase: RED - Write failing tests first
    // ============================================================================

    #[test]
    fn test_config_default() {
        // Test that default config has expected values
        let config = DevToolsConfig::default();

        assert_eq!(config.port(), 9222);
        assert!(!config.enable_remote_debugging());
        assert_eq!(config.max_message_size(), 100 * 1024 * 1024); // 100MB
        assert_eq!(config.protocol_version(), "1.3");
    }

    #[test]
    fn test_config_builder() {
        // Test builder pattern for custom config
        let config = DevToolsConfig::builder()
            .port(8080)
            .enable_remote_debugging(true)
            .max_message_size(50 * 1024 * 1024)
            .allowed_origin("http://localhost:3000".to_string())
            .build();

        assert_eq!(config.port(), 8080);
        assert!(config.enable_remote_debugging());
        assert_eq!(config.max_message_size(), 50 * 1024 * 1024);
        assert!(config
            .allowed_origins()
            .contains(&"http://localhost:3000".to_string()));
    }

    #[test]
    fn test_devtools_component_new() {
        // Test creating a new DevToolsComponent
        let config = DevToolsConfig::default();
        let result = DevToolsComponent::new(config);

        assert!(result.is_ok());
    }

    #[test]
    fn test_devtools_component_config() {
        // Test that component stores config correctly
        let config = DevToolsConfig::builder().port(9999).build();

        let devtools = DevToolsComponent::new(config).unwrap();

        assert_eq!(devtools.config().port(), 9999);
    }

    #[tokio::test]
    async fn test_domain_registration() {
        // Test that all 13 domains are registered
        let config = DevToolsConfig::default();
        let devtools = DevToolsComponent::new(config).unwrap();

        // Verify all domains are registered
        let domains = devtools.registered_domains();

        assert!(domains.contains(&"Browser"));
        assert!(domains.contains(&"Page"));
        assert!(domains.contains(&"Security"));
        assert!(domains.contains(&"Emulation"));
        assert!(domains.contains(&"DOM"));
        assert!(domains.contains(&"CSS"));
        assert!(domains.contains(&"Network"));
        assert!(domains.contains(&"Runtime"));
        assert!(domains.contains(&"Debugger"));
        assert!(domains.contains(&"Profiler"));
        assert!(domains.contains(&"HeapProfiler"));
        assert!(domains.contains(&"Console"));
        assert!(domains.contains(&"Storage"));

        // Should have exactly 13 domains
        assert_eq!(domains.len(), 13);
    }

    #[tokio::test]
    async fn test_start_server() {
        // Test starting the DevTools server
        let config = DevToolsConfig::builder()
            .port(0) // Use ephemeral port for testing
            .build();

        let devtools = DevToolsComponent::new(config).unwrap();

        // Start should succeed
        let result = devtools.start().await;
        assert!(result.is_ok());

        // Should be able to get actual port
        assert!(devtools.actual_port().is_some());
    }

    #[tokio::test]
    async fn test_stop_server() {
        // Test stopping the DevTools server
        let config = DevToolsConfig::builder().port(0).build();

        let devtools = DevToolsComponent::new(config).unwrap();

        devtools.start().await.unwrap();
        assert!(devtools.is_running());

        // Stop should succeed
        let result = devtools.stop().await;
        assert!(result.is_ok());
        assert!(!devtools.is_running());
    }

    #[tokio::test]
    async fn test_server_lifecycle() {
        // Test full server lifecycle
        let config = DevToolsConfig::builder().port(0).build();

        let devtools = DevToolsComponent::new(config).unwrap();

        // Initially not running
        assert!(!devtools.is_running());

        // Start server
        devtools.start().await.unwrap();
        assert!(devtools.is_running());

        // Stop server
        devtools.stop().await.unwrap();
        assert!(!devtools.is_running());
    }

    #[tokio::test]
    async fn test_cannot_start_twice() {
        // Test that starting an already running server returns error
        let config = DevToolsConfig::builder().port(0).build();

        let devtools = DevToolsComponent::new(config).unwrap();

        devtools.start().await.unwrap();

        // Starting again should fail
        let result = devtools.start().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_can_restart_after_stop() {
        // Test that server can be restarted after stopping
        let config = DevToolsConfig::builder().port(0).build();

        let devtools = DevToolsComponent::new(config).unwrap();

        // Start, stop, start again
        devtools.start().await.unwrap();
        devtools.stop().await.unwrap();
        let result = devtools.start().await;

        assert!(result.is_ok());
        assert!(devtools.is_running());
    }

    #[tokio::test]
    async fn test_get_debugger_url() {
        // Test getting the debugger WebSocket URL
        let config = DevToolsConfig::builder().port(9222).build();

        let devtools = DevToolsComponent::new(config).unwrap();

        let url = devtools.get_debugger_url("page-123");

        assert_eq!(url, "ws://localhost:9222/devtools/page/page-123");
    }

    #[tokio::test]
    async fn test_get_json_url() {
        // Test getting the JSON endpoint URL
        let config = DevToolsConfig::builder().port(9222).build();

        let devtools = DevToolsComponent::new(config).unwrap();

        let url = devtools.get_json_url();

        assert_eq!(url, "http://localhost:9222/json");
    }
}
