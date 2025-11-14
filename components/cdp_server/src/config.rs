//! Server configuration

use serde::{Deserialize, Serialize};

/// Configuration for the CDP WebSocket server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Port to bind to
    pub port: u16,

    /// Maximum message size in bytes (default 100MB)
    pub max_message_size: usize,

    /// Allowed origins for CORS
    pub allowed_origins: Vec<String>,

    /// Bind address (default 127.0.0.1 for localhost only)
    pub bind_address: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 9222,
            max_message_size: 100 * 1024 * 1024, // 100MB
            allowed_origins: vec!["http://localhost:*".to_string()],
            bind_address: "127.0.0.1".to_string(),
        }
    }
}

impl ServerConfig {
    /// Create a new server configuration
    pub fn new(port: u16) -> Self {
        Self {
            port,
            ..Default::default()
        }
    }

    /// Set maximum message size
    pub fn with_max_message_size(mut self, size: usize) -> Self {
        self.max_message_size = size;
        self
    }

    /// Set allowed origins
    pub fn with_allowed_origins(mut self, origins: Vec<String>) -> Self {
        self.allowed_origins = origins;
        self
    }

    /// Set bind address
    pub fn with_bind_address(mut self, address: String) -> Self {
        self.bind_address = address;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ServerConfig::default();
        assert_eq!(config.port, 9222);
        assert_eq!(config.max_message_size, 100 * 1024 * 1024);
        assert_eq!(config.bind_address, "127.0.0.1");
    }

    #[test]
    fn test_builder_pattern() {
        let config = ServerConfig::new(8080)
            .with_max_message_size(1024)
            .with_allowed_origins(vec!["https://example.com".to_string()])
            .with_bind_address("0.0.0.0".to_string());

        assert_eq!(config.port, 8080);
        assert_eq!(config.max_message_size, 1024);
        assert_eq!(config.allowed_origins[0], "https://example.com");
        assert_eq!(config.bind_address, "0.0.0.0");
    }
}
