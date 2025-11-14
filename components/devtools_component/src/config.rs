//! Configuration for DevTools component

use serde::{Deserialize, Serialize};

/// Configuration for DevTools component
///
/// This struct holds all configuration options for the DevTools server,
/// including port, security settings, and protocol version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevToolsConfig {
    /// Port to bind the WebSocket server to
    port: u16,

    /// Whether to enable remote debugging (allows connections from other machines)
    enable_remote_debugging: bool,

    /// List of allowed origins for CORS
    allowed_origins: Vec<String>,

    /// Maximum message size in bytes
    max_message_size: usize,

    /// Chrome DevTools Protocol version
    protocol_version: String,
}

impl DevToolsConfig {
    /// Create a new builder for DevToolsConfig
    ///
    /// # Example
    ///
    /// ```
    /// use devtools_component::DevToolsConfig;
    ///
    /// let config = DevToolsConfig::builder()
    ///     .port(9222)
    ///     .enable_remote_debugging(false)
    ///     .build();
    /// ```
    pub fn builder() -> DevToolsConfigBuilder {
        DevToolsConfigBuilder::default()
    }

    /// Get the configured port
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Get whether remote debugging is enabled
    pub fn enable_remote_debugging(&self) -> bool {
        self.enable_remote_debugging
    }

    /// Get the list of allowed origins
    pub fn allowed_origins(&self) -> &[String] {
        &self.allowed_origins
    }

    /// Get the maximum message size
    pub fn max_message_size(&self) -> usize {
        self.max_message_size
    }

    /// Get the protocol version
    pub fn protocol_version(&self) -> &str {
        &self.protocol_version
    }
}

impl Default for DevToolsConfig {
    /// Create a default configuration
    ///
    /// Default values:
    /// - port: 9222
    /// - enable_remote_debugging: false
    /// - allowed_origins: ["http://localhost:*"]
    /// - max_message_size: 100 MB
    /// - protocol_version: "1.3"
    fn default() -> Self {
        Self {
            port: 9222,
            enable_remote_debugging: false,
            allowed_origins: vec!["http://localhost:*".to_string()],
            max_message_size: 100 * 1024 * 1024, // 100 MB
            protocol_version: "1.3".to_string(),
        }
    }
}

/// Builder for DevToolsConfig
///
/// Provides a fluent interface for constructing DevToolsConfig instances.
#[derive(Debug, Clone, Default)]
pub struct DevToolsConfigBuilder {
    port: Option<u16>,
    enable_remote_debugging: Option<bool>,
    allowed_origins: Vec<String>,
    max_message_size: Option<usize>,
    protocol_version: Option<String>,
}

impl DevToolsConfigBuilder {
    /// Set the port
    ///
    /// # Arguments
    ///
    /// * `port` - Port number to bind to (0 for ephemeral port)
    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    /// Enable or disable remote debugging
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to allow remote connections
    pub fn enable_remote_debugging(mut self, enabled: bool) -> Self {
        self.enable_remote_debugging = Some(enabled);
        self
    }

    /// Add an allowed origin for CORS
    ///
    /// # Arguments
    ///
    /// * `origin` - Origin to allow (e.g., "http://localhost:3000")
    pub fn allowed_origin(mut self, origin: String) -> Self {
        self.allowed_origins.push(origin);
        self
    }

    /// Set maximum message size
    ///
    /// # Arguments
    ///
    /// * `size` - Maximum message size in bytes
    pub fn max_message_size(mut self, size: usize) -> Self {
        self.max_message_size = Some(size);
        self
    }

    /// Set protocol version
    ///
    /// # Arguments
    ///
    /// * `version` - CDP protocol version
    pub fn protocol_version(mut self, version: String) -> Self {
        self.protocol_version = Some(version);
        self
    }

    /// Build the DevToolsConfig
    ///
    /// Uses default values for any options not explicitly set.
    pub fn build(self) -> DevToolsConfig {
        let default = DevToolsConfig::default();

        let mut allowed_origins = default.allowed_origins;
        if !self.allowed_origins.is_empty() {
            allowed_origins = self.allowed_origins;
        }

        DevToolsConfig {
            port: self.port.unwrap_or(default.port),
            enable_remote_debugging: self
                .enable_remote_debugging
                .unwrap_or(default.enable_remote_debugging),
            allowed_origins,
            max_message_size: self.max_message_size.unwrap_or(default.max_message_size),
            protocol_version: self.protocol_version.unwrap_or(default.protocol_version),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DevToolsConfig::default();

        assert_eq!(config.port(), 9222);
        assert!(!config.enable_remote_debugging());
        assert_eq!(config.allowed_origins(), &["http://localhost:*"]);
        assert_eq!(config.max_message_size(), 100 * 1024 * 1024);
        assert_eq!(config.protocol_version(), "1.3");
    }

    #[test]
    fn test_builder_all_options() {
        let config = DevToolsConfig::builder()
            .port(8080)
            .enable_remote_debugging(true)
            .allowed_origin("http://example.com".to_string())
            .allowed_origin("http://test.com".to_string())
            .max_message_size(50 * 1024 * 1024)
            .protocol_version("1.4".to_string())
            .build();

        assert_eq!(config.port(), 8080);
        assert!(config.enable_remote_debugging());
        assert_eq!(config.allowed_origins().len(), 2);
        assert!(config
            .allowed_origins()
            .contains(&"http://example.com".to_string()));
        assert!(config
            .allowed_origins()
            .contains(&"http://test.com".to_string()));
        assert_eq!(config.max_message_size(), 50 * 1024 * 1024);
        assert_eq!(config.protocol_version(), "1.4");
    }

    #[test]
    fn test_builder_partial_options() {
        let config = DevToolsConfig::builder().port(7777).build();

        assert_eq!(config.port(), 7777);
        // Other values should be defaults
        assert!(!config.enable_remote_debugging());
        assert_eq!(config.protocol_version(), "1.3");
    }

    #[test]
    fn test_builder_no_options() {
        let config = DevToolsConfig::builder().build();

        // Should be equivalent to default
        let default = DevToolsConfig::default();

        assert_eq!(config.port(), default.port());
        assert_eq!(
            config.enable_remote_debugging(),
            default.enable_remote_debugging()
        );
        assert_eq!(config.max_message_size(), default.max_message_size());
        assert_eq!(config.protocol_version(), default.protocol_version());
    }
}
