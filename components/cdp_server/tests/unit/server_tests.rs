//! Unit tests for CDP WebSocket server

use cdp_server::*;

#[test]
fn test_server_config_default() {
    let config = ServerConfig::default();
    assert_eq!(config.port, 9222);
    assert_eq!(config.max_message_size, 100 * 1024 * 1024); // 100MB
    assert_eq!(config.allowed_origins.len(), 1);
    assert_eq!(config.allowed_origins[0], "http://localhost:*");
}

#[test]
fn test_server_config_custom() {
    let config = ServerConfig {
        port: 8080,
        max_message_size: 1024,
        allowed_origins: vec!["https://example.com".to_string()],
        bind_address: "127.0.0.1".to_string(),
    };

    assert_eq!(config.port, 8080);
    assert_eq!(config.max_message_size, 1024);
    assert_eq!(config.allowed_origins[0], "https://example.com");
}

#[test]
fn test_origin_validation_exact_match() {
    let config = ServerConfig {
        allowed_origins: vec!["http://localhost:3000".to_string()],
        ..Default::default()
    };

    assert!(validate_origin("http://localhost:3000", &config.allowed_origins));
    assert!(!validate_origin("http://localhost:4000", &config.allowed_origins));
    assert!(!validate_origin("https://localhost:3000", &config.allowed_origins));
}

#[test]
fn test_origin_validation_wildcard() {
    let config = ServerConfig {
        allowed_origins: vec!["http://localhost:*".to_string()],
        ..Default::default()
    };

    assert!(validate_origin("http://localhost:3000", &config.allowed_origins));
    assert!(validate_origin("http://localhost:9222", &config.allowed_origins));
    assert!(!validate_origin("http://example.com:3000", &config.allowed_origins));
}

#[test]
fn test_session_id_generation() {
    let id1 = SessionId::new();
    let id2 = SessionId::new();

    // Session IDs should be unique
    assert_ne!(id1, id2);

    // Session ID should be valid UUID
    assert!(!id1.to_string().is_empty());
}

#[test]
fn test_session_id_from_string() {
    let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
    let session_id = SessionId::from_string(uuid_str).unwrap();

    assert_eq!(session_id.to_string(), uuid_str);
}

#[test]
fn test_session_id_invalid_string() {
    let invalid = "not-a-uuid";
    let result = SessionId::from_string(invalid);

    assert!(result.is_err());
}
