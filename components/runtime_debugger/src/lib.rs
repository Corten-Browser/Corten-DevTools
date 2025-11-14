//! Runtime and Debugger domain handlers for Chrome DevTools Protocol
//!
//! This module provides implementations for the Runtime and Debugger domains,
//! enabling JavaScript execution and debugging capabilities.

pub mod debugger;
pub mod runtime;

pub use debugger::DebuggerDomain;
pub use runtime::RuntimeDomain;

use thiserror::Error;

/// Errors that can occur in runtime_debugger operations
#[derive(Error, Debug)]
pub enum RuntimeDebuggerError {
    /// Object not found with given ID
    #[error("Object not found: {0}")]
    ObjectNotFound(String),

    /// Breakpoint not found with given ID
    #[error("Breakpoint not found: {0}")]
    BreakpointNotFound(String),

    /// Call frame not found with given ID
    #[error("Call frame not found: {0}")]
    CallFrameNotFound(String),

    /// JavaScript evaluation error
    #[error("Evaluation error: {0}")]
    EvaluationError(String),

    /// Invalid parameters provided
    #[error("Invalid parameters: {0}")]
    InvalidParams(String),

    /// Debugger not enabled
    #[error("Debugger not enabled")]
    DebuggerNotEnabled,

    /// Debugger not paused
    #[error("Debugger not paused")]
    DebuggerNotPaused,

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, RuntimeDebuggerError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = RuntimeDebuggerError::ObjectNotFound("obj-123".to_string());
        assert_eq!(err.to_string(), "Object not found: obj-123");

        let err = RuntimeDebuggerError::DebuggerNotEnabled;
        assert_eq!(err.to_string(), "Debugger not enabled");
    }
}
