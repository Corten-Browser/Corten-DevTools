//! Error types for CDP server

use thiserror::Error;

/// Errors that can occur in the CDP server
#[derive(Error, Debug)]
pub enum CdpServerError {
    /// WebSocket error (boxed to reduce size)
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] Box<tungstenite::Error>),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Invalid origin
    #[error("Invalid origin: {0}")]
    InvalidOrigin(String),

    /// Message too large
    #[error("Message size {0} exceeds limit {1}")]
    MessageTooLarge(usize, usize),

    /// Invalid message format
    #[error("Invalid message format: {0}")]
    InvalidMessage(String),

    /// Session not found
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    /// Session closed
    #[error("Session is closed")]
    SessionClosed,

    /// Invalid session ID
    #[error("Invalid session ID: {0}")]
    InvalidSessionId(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Other errors
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Result type for CDP server operations
pub type Result<T> = std::result::Result<T, CdpServerError>;
