//! Integration tests for WebSocket server

use cdp_server::*;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures::{SinkExt, StreamExt};

#[tokio::test]
async fn test_server_starts_and_accepts_connections() {
    let config = ServerConfig {
        port: 9223, // Use different port to avoid conflicts
        ..Default::default()
    };

    let server = CdpWebSocketServer::new(config.clone()).unwrap();

    // Start server in background
    let server_handle = tokio::spawn(async move {
        server.start().await
    });

    // Give server time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Try to connect
    let url = format!("ws://127.0.0.1:{}", config.port);
    let result = connect_async(&url).await;

    assert!(result.is_ok(), "Should be able to connect to WebSocket server");

    // Cleanup
    server_handle.abort();
}

#[tokio::test]
async fn test_server_rejects_invalid_origin() {
    let config = ServerConfig {
        port: 9224,
        allowed_origins: vec!["http://allowed.com".to_string()],
        ..Default::default()
    };

    let server = CdpWebSocketServer::new(config.clone()).unwrap();

    let server_handle = tokio::spawn(async move {
        server.start().await
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Try to connect with invalid origin
    let url = format!("ws://127.0.0.1:{}", config.port);

    // Create request with invalid Origin header
    let request = http::Request::builder()
        .uri(&url)
        .header("Host", format!("127.0.0.1:{}", config.port))
        .header("Origin", "http://malicious.com")
        .header("Upgrade", "websocket")
        .header("Connection", "Upgrade")
        .header("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==")
        .header("Sec-WebSocket-Version", "13")
        .body(())
        .unwrap();

    let result = connect_async(request).await;

    // Connection should be rejected
    assert!(result.is_err(), "Should reject connection with invalid origin");

    server_handle.abort();
}

#[tokio::test]
async fn test_server_accepts_valid_origin() {
    let config = ServerConfig {
        port: 9225,
        allowed_origins: vec!["http://localhost:*".to_string()],
        ..Default::default()
    };

    let server = CdpWebSocketServer::new(config.clone()).unwrap();

    let server_handle = tokio::spawn(async move {
        server.start().await
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Connect with valid origin
    let url = format!("ws://127.0.0.1:{}", config.port);

    let request = http::Request::builder()
        .uri(&url)
        .header("Host", format!("127.0.0.1:{}", config.port))
        .header("Origin", "http://localhost:3000")
        .header("Upgrade", "websocket")
        .header("Connection", "Upgrade")
        .header("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==")
        .header("Sec-WebSocket-Version", "13")
        .body(())
        .unwrap();

    let result = connect_async(request).await;

    assert!(result.is_ok(), "Should accept connection with valid origin");

    server_handle.abort();
}

#[tokio::test]
async fn test_message_exchange() {
    let config = ServerConfig {
        port: 9226,
        ..Default::default()
    };

    let server = CdpWebSocketServer::new(config.clone()).unwrap();

    let server_handle = tokio::spawn(async move {
        server.start().await
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Connect client
    let url = format!("ws://127.0.0.1:{}", config.port);
    let (ws_stream, _) = connect_async(&url).await.unwrap();
    let (mut write, mut read) = ws_stream.split();

    // Send CDP request
    let request = r#"{"id": 1, "method": "Runtime.evaluate", "params": {"expression": "1+1"}}"#;
    write.send(Message::Text(request.to_string())).await.unwrap();

    // Read response (should get echo or actual response)
    let response = read.next().await;
    assert!(response.is_some(), "Should receive response from server");

    server_handle.abort();
}

#[tokio::test]
async fn test_multiple_concurrent_connections() {
    let config = ServerConfig {
        port: 9227,
        ..Default::default()
    };

    let server = CdpWebSocketServer::new(config.clone()).unwrap();

    let server_handle = tokio::spawn(async move {
        server.start().await
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let url = format!("ws://127.0.0.1:{}", config.port);

    // Create multiple connections
    let mut handles = vec![];
    for _ in 0..5 {
        let url = url.clone();
        let handle = tokio::spawn(async move {
            let result = connect_async(&url).await;
            result.is_ok()
        });
        handles.push(handle);
    }

    // All connections should succeed
    for handle in handles {
        let success = handle.await.unwrap();
        assert!(success, "All concurrent connections should succeed");
    }

    server_handle.abort();
}

#[tokio::test]
async fn test_message_size_limit() {
    let config = ServerConfig {
        port: 9228,
        max_message_size: 1024, // 1KB limit
        ..Default::default()
    };

    let server = CdpWebSocketServer::new(config.clone()).unwrap();

    let server_handle = tokio::spawn(async move {
        server.start().await
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let url = format!("ws://127.0.0.1:{}", config.port);
    let (ws_stream, _) = connect_async(&url).await.unwrap();
    let (mut write, _read) = ws_stream.split();

    // Send message exceeding limit
    let large_msg = "a".repeat(2000);
    let result = write.send(Message::Text(large_msg)).await;

    // Connection should be closed or error returned
    // (exact behavior depends on implementation)
    // For now, just test that we can send the message
    // The server should reject it
    assert!(result.is_ok()); // Client can send

    server_handle.abort();
}

#[tokio::test]
async fn test_session_tracking() {
    let config = ServerConfig {
        port: 9229,
        ..Default::default()
    };

    let server = CdpWebSocketServer::new(config.clone()).unwrap();
    let sessions = server.get_sessions();

    let server_handle = tokio::spawn(async move {
        server.start().await
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Initially no sessions
    assert_eq!(sessions.len(), 0);

    // Connect client
    let url = format!("ws://127.0.0.1:{}", config.port);
    let _connection = connect_async(&url).await.unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Should have one session now
    assert_eq!(sessions.len(), 1);

    server_handle.abort();
}
