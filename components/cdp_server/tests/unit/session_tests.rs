//! Unit tests for Session management

use cdp_server::*;

#[test]
fn test_session_state_initial() {
    let session = Session::new(SessionId::new());

    assert_eq!(session.state(), SessionState::Active);
    assert!(session.created_at() <= std::time::SystemTime::now());
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

    // Once closed, session stays closed
    assert_eq!(session.state(), SessionState::Closed);
}

#[tokio::test]
async fn test_session_message_queue() {
    let mut session = Session::new(SessionId::new());

    // Queue some messages
    session.queue_message("message1".to_string()).await;
    session.queue_message("message2".to_string()).await;
    session.queue_message("message3".to_string()).await;

    assert_eq!(session.pending_messages_count(), 3);
}

#[tokio::test]
async fn test_session_message_dequeue() {
    let mut session = Session::new(SessionId::new());

    session.queue_message("test message".to_string()).await;

    let msg = session.dequeue_message().await;
    assert!(msg.is_some());
    assert_eq!(msg.unwrap(), "test message");

    // Queue should be empty now
    assert_eq!(session.pending_messages_count(), 0);
}

#[tokio::test]
async fn test_session_clear_messages() {
    let mut session = Session::new(SessionId::new());

    session.queue_message("msg1".to_string()).await;
    session.queue_message("msg2".to_string()).await;
    session.queue_message("msg3".to_string()).await;

    session.clear_messages().await;

    assert_eq!(session.pending_messages_count(), 0);
}
