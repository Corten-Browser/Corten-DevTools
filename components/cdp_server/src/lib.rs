//! WebSocket server and session management
//!
//! This module provides the CDP (Chrome DevTools Protocol) WebSocket server
//! that accepts client connections and manages sessions.
//!
//! # Example
//!
//! ```no_run
//! use cdp_server::{CdpWebSocketServer, ServerConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = ServerConfig::default();
//!     let server = CdpWebSocketServer::new(config)?;
//!     server.start().await?;
//!     Ok(())
//! }
//! ```

// Public modules
pub mod config;
pub mod error;
pub mod server;
pub mod session;
pub mod transport;
pub mod validation;

// Re-export main types
pub use config::ServerConfig;
pub use error::{CdpServerError, Result};
pub use server::CdpWebSocketServer;
pub use session::{Session, SessionId, SessionState};
pub use transport::{
    parse_cdp_message, serialize_cdp_message, validate_message_size, validate_origin,
};
pub use validation::{
    validate_origin_detailed, OriginValidationResult, OriginValidator, OriginValidatorConfig,
};
