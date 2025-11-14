// Integration tests for Console and Storage domains with ProtocolHandler

use console_storage::{ConsoleDomain, StorageDomain};
use protocol_handler::ProtocolHandler;
use std::sync::Arc;

#[tokio::test]
async fn test_console_domain_registration() {
    let handler = ProtocolHandler::new();
    let console_domain = Arc::new(ConsoleDomain::new());

    handler.register_domain(console_domain);

    // Test that Console domain responds to enable method
    let request = r#"{"id": 1, "method": "Console.enable"}"#;
    let response = handler.handle_message(request).await;

    assert!(response.contains(r#""id":1"#));
    assert!(response.contains(r#""result""#));
}

#[tokio::test]
async fn test_storage_domain_registration() {
    let handler = ProtocolHandler::new();
    let storage_domain = Arc::new(StorageDomain::new());

    handler.register_domain(storage_domain);

    // Test that Storage domain responds to getCookies method
    let request = r#"{"id": 2, "method": "Storage.getCookies"}"#;
    let response = handler.handle_message(request).await;

    assert!(response.contains(r#""id":2"#));
    assert!(response.contains(r#""result""#));
    assert!(response.contains(r#""cookies""#));
}

#[tokio::test]
async fn test_both_domains_registered() {
    let handler = ProtocolHandler::new();
    let console_domain = Arc::new(ConsoleDomain::new());
    let storage_domain = Arc::new(StorageDomain::new());

    handler.register_domain(console_domain);
    handler.register_domain(storage_domain);

    // Test Console domain
    let console_request = r#"{"id": 1, "method": "Console.enable"}"#;
    let console_response = handler.handle_message(console_request).await;
    assert!(console_response.contains(r#""result""#));

    // Test Storage domain
    let storage_request = r#"{"id": 2, "method": "Storage.getCookies"}"#;
    let storage_response = handler.handle_message(storage_request).await;
    assert!(storage_response.contains(r#""cookies""#));
}

#[tokio::test]
async fn test_console_message_workflow() {
    let handler = ProtocolHandler::new();
    let console_domain = Arc::new(ConsoleDomain::new());

    handler.register_domain(console_domain);

    // Enable console
    let enable_request = r#"{"id": 1, "method": "Console.enable"}"#;
    let enable_response = handler.handle_message(enable_request).await;
    assert!(enable_response.contains(r#""result""#));

    // Add a message
    let add_message_request = r#"{"id": 2, "method": "Console.messageAdded", "params": {"message": {"source": "console", "level": "log", "text": "Test message"}}}"#;
    let add_message_response = handler.handle_message(add_message_request).await;
    assert!(add_message_response.contains(r#""result""#));

    // Get messages
    let get_messages_request = r#"{"id": 3, "method": "Console.getMessages"}"#;
    let get_messages_response = handler.handle_message(get_messages_request).await;
    assert!(get_messages_response.contains(r#""messages""#));
    assert!(get_messages_response.contains("Test message"));

    // Clear messages
    let clear_request = r#"{"id": 4, "method": "Console.clearMessages"}"#;
    let clear_response = handler.handle_message(clear_request).await;
    assert!(clear_response.contains(r#""result""#));

    // Verify messages cleared
    let verify_request = r#"{"id": 5, "method": "Console.getMessages"}"#;
    let verify_response = handler.handle_message(verify_request).await;
    assert!(verify_response.contains(r#""messages":[]"#));
}

#[tokio::test]
async fn test_storage_cookie_workflow() {
    let handler = ProtocolHandler::new();
    let storage_domain = Arc::new(StorageDomain::new());

    handler.register_domain(storage_domain);

    // Initially, no cookies
    let get_initial_request = r#"{"id": 1, "method": "Storage.getCookies"}"#;
    let get_initial_response = handler.handle_message(get_initial_request).await;
    assert!(get_initial_response.contains(r#""cookies":[]"#));

    // Set a cookie
    let set_cookie_request = r#"{"id": 2, "method": "Storage.setCookie", "params": {"name": "session", "value": "abc123", "domain": "example.com", "path": "/"}}"#;
    let set_cookie_response = handler.handle_message(set_cookie_request).await;
    assert!(set_cookie_response.contains(r#""result""#));

    // Get cookies and verify
    let get_cookies_request = r#"{"id": 3, "method": "Storage.getCookies"}"#;
    let get_cookies_response = handler.handle_message(get_cookies_request).await;
    assert!(get_cookies_response.contains("session"));
    assert!(get_cookies_response.contains("abc123"));

    // Delete the cookie
    let delete_cookie_request = r#"{"id": 4, "method": "Storage.deleteCookie", "params": {"name": "session", "domain": "example.com"}}"#;
    let delete_cookie_response = handler.handle_message(delete_cookie_request).await;
    assert!(delete_cookie_response.contains(r#""result""#));

    // Verify cookie deleted
    let verify_request = r#"{"id": 5, "method": "Storage.getCookies"}"#;
    let verify_response = handler.handle_message(verify_request).await;
    assert!(verify_response.contains(r#""cookies":[]"#));
}
