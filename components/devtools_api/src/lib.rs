//! Public API and configuration for CortenBrowser DevTools
//!
//! This module provides a simple, ergonomic API for integrating DevTools
//! into CortenBrowser. It wraps the lower-level `devtools_component` with
//! a clean public interface.
//!
//! # Example
//!
//! ```no_run
//! use devtools_api::{DevTools, DevToolsConfig};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = DevToolsConfig::default();
//!     let devtools = DevTools::new(config)?;
//!
//!     devtools.start(9222).await?;
//!     println!("DevTools URL: {}", devtools.get_url());
//!
//!     // ... browser runs ...
//!
//!     devtools.stop().await?;
//!     Ok(())
//! }
//! ```

#![warn(missing_docs)]

use std::sync::Arc;
use tokio::sync::RwLock;

// Re-export public types from devtools_component
pub use devtools_component::{DevToolsConfig, DevToolsError, Result};

use devtools_component::DevToolsComponent;

/// Main DevTools public API
///
/// This is the primary interface for working with CortenBrowser DevTools.
/// It provides a simplified wrapper around the underlying DevToolsComponent.
pub struct DevTools {
    component: Arc<RwLock<Option<DevToolsComponent>>>,
    base_config: DevToolsConfig,
    actual_port: Arc<RwLock<Option<u16>>>,
}

impl DevTools {
    /// Create a new DevTools instance with the given configuration
    ///
    /// # Arguments
    ///
    /// * `config` - DevTools configuration
    ///
    /// # Returns
    ///
    /// Returns `Ok(DevTools)` on success, or an error if initialization fails.
    ///
    /// # Example
    ///
    /// ```
    /// use devtools_api::{DevTools, DevToolsConfig};
    ///
    /// let config = DevToolsConfig::default();
    /// let devtools = DevTools::new(config).unwrap();
    /// ```
    pub fn new(config: DevToolsConfig) -> Result<Self> {
        Ok(Self {
            component: Arc::new(RwLock::new(None)),
            base_config: config,
            actual_port: Arc::new(RwLock::new(None)),
        })
    }

    /// Start the DevTools server on the specified port
    ///
    /// # Arguments
    ///
    /// * `port` - Port number to bind the DevTools server to
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if the server cannot be started.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use devtools_api::{DevTools, DevToolsConfig};
    /// # #[tokio::main]
    /// # async fn main() -> anyhow::Result<()> {
    /// let devtools = DevTools::new(DevToolsConfig::default())?;
    /// devtools.start(9222).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn start(&self, port: u16) -> Result<()> {
        let mut component_lock = self.component.write().await;

        // Check if already started
        if component_lock.is_some() {
            return Err(DevToolsError::ServerAlreadyRunning);
        }

        // Create a new config with the specified port
        let mut config_builder = DevToolsConfig::builder();

        // Copy settings from base config
        if self.base_config.enable_remote_debugging() {
            config_builder = config_builder.enable_remote_debugging(true);
        }

        for origin in self.base_config.allowed_origins() {
            config_builder = config_builder.allowed_origin(origin.clone());
        }

        config_builder = config_builder
            .port(port)
            .max_message_size(self.base_config.max_message_size());

        let config = config_builder.build();

        // Create and start the component
        let component = DevToolsComponent::new(config)?;
        component.start().await?;

        // Store the actual port (might be different if port was 0)
        let actual_port = component.actual_port().unwrap_or(port);
        *self.actual_port.write().await = Some(actual_port);

        // Store the component
        *component_lock = Some(component);

        Ok(())
    }

    /// Stop the DevTools server
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if the server cannot be stopped.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use devtools_api::{DevTools, DevToolsConfig};
    /// # #[tokio::main]
    /// # async fn main() -> anyhow::Result<()> {
    /// # let devtools = DevTools::new(DevToolsConfig::default())?;
    /// # devtools.start(9222).await?;
    /// devtools.stop().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn stop(&self) -> Result<()> {
        let mut component_lock = self.component.write().await;

        if let Some(component) = component_lock.as_ref() {
            component.stop().await?;
            *component_lock = None;
            *self.actual_port.write().await = None;
            Ok(())
        } else {
            Err(DevToolsError::ServerNotRunning)
        }
    }

    /// Get the DevTools server URL
    ///
    /// Returns the HTTP endpoint URL for the DevTools JSON API.
    ///
    /// # Returns
    ///
    /// The URL as a String (e.g., "http://localhost:9222/json")
    ///
    /// # Example
    ///
    /// ```
    /// # use devtools_api::{DevTools, DevToolsConfig};
    /// let devtools = DevTools::new(DevToolsConfig::default()).unwrap();
    /// let url = devtools.get_url();
    /// assert!(url.contains("/json"));
    /// ```
    pub fn get_url(&self) -> String {
        // Use actual port if started, otherwise use base config port
        let port = match self.actual_port.try_read() {
            Ok(lock) => lock.unwrap_or(self.base_config.port()),
            Err(_) => self.base_config.port(),
        };
        format!("http://localhost:{}/json", port)
    }

    /// Get the WebSocket debugger URL for a specific target
    ///
    /// # Arguments
    ///
    /// * `target_id` - The ID of the debugging target (e.g., page ID)
    ///
    /// # Returns
    ///
    /// The WebSocket URL as a String
    ///
    /// # Example
    ///
    /// ```
    /// # use devtools_api::{DevTools, DevToolsConfig};
    /// let devtools = DevTools::new(DevToolsConfig::default()).unwrap();
    /// let url = devtools.get_debugger_url("page-123");
    /// assert!(url.contains("ws://"));
    /// assert!(url.contains("page-123"));
    /// ```
    pub fn get_debugger_url(&self, target_id: &str) -> String {
        // Use actual port if started, otherwise use base config port
        let port = match self.actual_port.try_read() {
            Ok(lock) => lock.unwrap_or(self.base_config.port()),
            Err(_) => self.base_config.port(),
        };
        format!("ws://localhost:{}/devtools/page/{}", port, target_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // TDD Phase: RED - Write failing tests first
    // ============================================================================

    #[test]
    fn test_devtools_new_with_default_config() {
        // Test creating DevTools with default configuration
        let config = DevToolsConfig::default();
        let result = DevTools::new(config);

        assert!(result.is_ok(), "Should successfully create DevTools");
    }

    #[test]
    fn test_devtools_new_with_custom_config() {
        // Test creating DevTools with custom configuration
        let config = DevToolsConfig::builder()
            .port(8080)
            .enable_remote_debugging(true)
            .build();

        let result = DevTools::new(config);

        assert!(
            result.is_ok(),
            "Should successfully create DevTools with custom config"
        );
    }

    #[tokio::test]
    async fn test_start_devtools_server() {
        // Test starting the DevTools server on a specific port
        let config = DevToolsConfig::default();
        let devtools = DevTools::new(config).unwrap();

        let result = devtools.start(9222).await;

        assert!(result.is_ok(), "Should successfully start DevTools server");
    }

    #[tokio::test]
    async fn test_start_with_ephemeral_port() {
        // Test starting with port 0 (ephemeral port)
        let config = DevToolsConfig::default();
        let devtools = DevTools::new(config).unwrap();

        let result = devtools.start(0).await;

        assert!(result.is_ok(), "Should start with ephemeral port");
    }

    #[tokio::test]
    async fn test_stop_devtools_server() {
        // Test stopping the DevTools server
        let config = DevToolsConfig::default();
        let devtools = DevTools::new(config).unwrap();

        devtools.start(0).await.unwrap();
        let result = devtools.stop().await;

        assert!(result.is_ok(), "Should successfully stop DevTools server");
    }

    #[tokio::test]
    async fn test_server_lifecycle() {
        // Test complete server lifecycle: start -> stop -> start again
        let config = DevToolsConfig::default();
        let devtools = DevTools::new(config).unwrap();

        // Start server
        devtools.start(0).await.unwrap();

        // Stop server
        devtools.stop().await.unwrap();

        // Should be able to start again
        let result = devtools.start(0).await;
        assert!(result.is_ok(), "Should be able to restart after stop");
    }

    #[test]
    fn test_get_url_with_default_port() {
        // Test getting the DevTools URL
        let config = DevToolsConfig::builder().port(9222).build();
        let devtools = DevTools::new(config).unwrap();

        let url = devtools.get_url();

        assert!(url.contains("http://"), "URL should use HTTP protocol");
        assert!(url.contains("localhost"), "URL should use localhost");
        assert!(url.contains("/json"), "URL should have /json endpoint");
    }

    #[test]
    fn test_get_url_reflects_configured_port() {
        // Test that get_url reflects the configured port
        let config = DevToolsConfig::builder().port(8080).build();
        let devtools = DevTools::new(config).unwrap();

        let url = devtools.get_url();

        assert!(url.contains("8080"), "URL should contain configured port");
        assert_eq!(url, "http://localhost:8080/json");
    }

    #[test]
    fn test_get_debugger_url() {
        // Test getting the WebSocket debugger URL
        let config = DevToolsConfig::builder().port(9222).build();
        let devtools = DevTools::new(config).unwrap();

        let url = devtools.get_debugger_url("page-123");

        assert!(url.contains("ws://"), "URL should use WebSocket protocol");
        assert!(url.contains("localhost"), "URL should use localhost");
        assert!(url.contains("9222"), "URL should contain port");
        assert!(url.contains("page-123"), "URL should contain target ID");
        assert_eq!(url, "ws://localhost:9222/devtools/page/page-123");
    }

    #[test]
    fn test_get_debugger_url_with_different_target() {
        // Test debugger URL with different target IDs
        let config = DevToolsConfig::default();
        let devtools = DevTools::new(config).unwrap();

        let url1 = devtools.get_debugger_url("target-1");
        let url2 = devtools.get_debugger_url("target-2");

        assert_ne!(url1, url2, "Different targets should have different URLs");
        assert!(url1.contains("target-1"));
        assert!(url2.contains("target-2"));
    }

    #[tokio::test]
    async fn test_cannot_start_twice() {
        // Test that starting an already running server returns an error
        let config = DevToolsConfig::default();
        let devtools = DevTools::new(config).unwrap();

        devtools.start(0).await.unwrap();

        // Attempt to start again should fail
        let result = devtools.start(0).await;
        assert!(result.is_err(), "Should not be able to start server twice");
    }

    #[test]
    fn test_config_reexport() {
        // Verify that DevToolsConfig is properly re-exported
        let _config: DevToolsConfig = DevToolsConfig::default();
        // If this compiles, the re-export is working
    }

    #[test]
    fn test_error_reexport() {
        // Verify that Result and DevToolsError are properly re-exported
        let _result: Result<()> = Ok(());
        // If this compiles, the re-export is working
    }
}
