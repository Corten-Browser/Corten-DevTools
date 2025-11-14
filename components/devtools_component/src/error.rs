//! Error types for DevTools component

use thiserror::Error;

/// Errors that can occur in DevTools component operations
#[derive(Error, Debug)]
pub enum DevToolsError {
    /// Server is already running
    #[error("Server is already running")]
    ServerAlreadyRunning,

    /// Server is not running
    #[error("Server is not running")]
    ServerNotRunning,

    /// Failed to start server
    #[error("Failed to start server: {0}")]
    ServerStartFailed(String),

    /// Failed to stop server
    #[error("Failed to stop server: {0}")]
    ServerStopFailed(String),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    /// CDP server error
    #[error("CDP server error: {0}")]
    CdpServerError(#[from] cdp_server::CdpServerError),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Other errors
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Result type for DevTools operations
pub type Result<T> = std::result::Result<T, DevToolsError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = DevToolsError::ServerAlreadyRunning;
        assert_eq!(err.to_string(), "Server is already running");

        let err = DevToolsError::ServerNotRunning;
        assert_eq!(err.to_string(), "Server is not running");

        let err = DevToolsError::InvalidConfiguration("test".to_string());
        assert_eq!(err.to_string(), "Invalid configuration: test");
    }
}
