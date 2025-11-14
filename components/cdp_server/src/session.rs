//! Session management for CDP connections

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::error::{CdpServerError, Result};

/// Unique identifier for a CDP session
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(Uuid);

impl SessionId {
    /// Generate a new unique session ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create session ID from string
    pub fn from_string(s: &str) -> Result<Self> {
        Uuid::parse_str(s)
            .map(Self)
            .map_err(|_| CdpServerError::InvalidSessionId(s.to_string()))
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// State of a CDP session
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Session is active and processing messages
    Active,
    /// Session is paused
    Paused,
    /// Session is closed
    Closed,
}

/// A CDP session representing a connected client
pub struct Session {
    /// Unique session ID
    id: SessionId,

    /// Current state
    state: Arc<RwLock<SessionState>>,

    /// Creation timestamp
    created_at: SystemTime,

    /// Message queue for outgoing messages
    message_tx: mpsc::UnboundedSender<String>,

    /// Message receiver
    message_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<String>>>>,
}

impl Session {
    /// Create a new session
    pub fn new(id: SessionId) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        Self {
            id,
            state: Arc::new(RwLock::new(SessionState::Active)),
            created_at: SystemTime::now(),
            message_tx: tx,
            message_rx: Arc::new(RwLock::new(Some(rx))),
        }
    }

    /// Get session ID
    pub fn id(&self) -> SessionId {
        self.id
    }

    /// Get current state
    pub fn state(&self) -> SessionState {
        *self.state.read()
    }

    /// Get creation time
    pub fn created_at(&self) -> SystemTime {
        self.created_at
    }

    /// Pause the session
    pub fn pause(&mut self) {
        let mut state = self.state.write();
        if *state == SessionState::Active {
            *state = SessionState::Paused;
        }
    }

    /// Resume the session
    pub fn resume(&mut self) {
        let mut state = self.state.write();
        if *state == SessionState::Paused {
            *state = SessionState::Active;
        }
    }

    /// Close the session
    pub fn close(&mut self) {
        *self.state.write() = SessionState::Closed;
    }

    /// Queue a message for sending
    pub async fn queue_message(&mut self, message: String) {
        // Ignore send errors (channel may be closed)
        let _ = self.message_tx.send(message);
    }

    /// Dequeue a message
    pub async fn dequeue_message(&mut self) -> Option<String> {
        // Get mutable receiver reference without holding the lock across await
        let rx_opt = {
            let mut rx_guard = self.message_rx.write();
            rx_guard.take()
        };

        if let Some(mut rx) = rx_opt {
            let result = rx.recv().await;
            // Put the receiver back
            *self.message_rx.write() = Some(rx);
            result
        } else {
            None
        }
    }

    /// Get count of pending messages
    pub fn pending_messages_count(&self) -> usize {
        // This is an approximation since mpsc doesn't provide len()
        // In practice, we'd track this separately
        0 // Simplified for now
    }

    /// Clear all pending messages
    pub async fn clear_messages(&mut self) {
        // Drain the channel
        let mut rx_guard = self.message_rx.write();
        if let Some(rx) = rx_guard.as_mut() {
            while rx.try_recv().is_ok() {
                // Drain
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_id_new() {
        let id1 = SessionId::new();
        let id2 = SessionId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_session_id_from_string() {
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        let id = SessionId::from_string(uuid_str).unwrap();
        assert_eq!(format!("{}", id), uuid_str);
    }

    #[test]
    fn test_session_id_invalid() {
        let result = SessionId::from_string("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_session_new() {
        let session = Session::new(SessionId::new());
        assert_eq!(session.state(), SessionState::Active);
    }

    #[test]
    fn test_session_state_transitions() {
        let mut session = Session::new(SessionId::new());

        assert_eq!(session.state(), SessionState::Active);

        session.pause();
        assert_eq!(session.state(), SessionState::Paused);

        session.resume();
        assert_eq!(session.state(), SessionState::Active);

        session.close();
        assert_eq!(session.state(), SessionState::Closed);
    }

    #[test]
    fn test_session_cannot_resume_closed() {
        let mut session = Session::new(SessionId::new());

        session.close();
        session.resume();

        assert_eq!(session.state(), SessionState::Closed);
    }

    #[tokio::test]
    async fn test_message_queue() {
        let mut session = Session::new(SessionId::new());

        session.queue_message("test1".to_string()).await;
        session.queue_message("test2".to_string()).await;

        let msg1 = session.dequeue_message().await;
        assert_eq!(msg1, Some("test1".to_string()));

        let msg2 = session.dequeue_message().await;
        assert_eq!(msg2, Some("test2".to_string()));
    }
}
