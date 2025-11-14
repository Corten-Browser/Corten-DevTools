//! Unit tests for message transport layer

use cdp_server::*;
use serde_json::json;

#[test]
fn test_parse_cdp_request() {
    let json = r#"{"id": 1, "method": "Runtime.evaluate", "params": {"expression": "1+1"}}"#;

    let result = parse_cdp_message(json);
    assert!(result.is_ok());

    let msg = result.unwrap();
    match msg {
        cdp_types::CdpMessage::Request(req) => {
            assert_eq!(req.id, 1);
            assert_eq!(req.method, "Runtime.evaluate");
            assert!(req.params.is_some());
        }
        _ => panic!("Expected Request variant"),
    }
}

#[test]
fn test_parse_cdp_response() {
    let json = r#"{"id": 1, "result": {"value": 2}}"#;

    let result = parse_cdp_message(json);
    assert!(result.is_ok());

    let msg = result.unwrap();
    match msg {
        cdp_types::CdpMessage::Response(resp) => {
            assert_eq!(resp.id, 1);
            assert!(resp.result.is_some());
            assert!(resp.error.is_none());
        }
        _ => panic!("Expected Response variant"),
    }
}

#[test]
fn test_parse_cdp_event() {
    let json = r#"{"method": "Network.requestWillBeSent", "params": {"requestId": "123"}}"#;

    let result = parse_cdp_message(json);
    assert!(result.is_ok());

    let msg = result.unwrap();
    match msg {
        cdp_types::CdpMessage::Event(event) => {
            assert_eq!(event.method, "Network.requestWillBeSent");
        }
        _ => panic!("Expected Event variant"),
    }
}

#[test]
fn test_parse_invalid_json() {
    let json = "not valid json";

    let result = parse_cdp_message(json);
    assert!(result.is_err());
}

#[test]
fn test_message_size_validation() {
    let small_msg = "a".repeat(100);
    assert!(validate_message_size(&small_msg, 1024).is_ok());

    let large_msg = "a".repeat(2000);
    assert!(validate_message_size(&large_msg, 1024).is_err());
}

#[test]
fn test_message_size_exact_limit() {
    let msg = "a".repeat(1024);
    assert!(validate_message_size(&msg, 1024).is_ok());

    let msg = "a".repeat(1025);
    assert!(validate_message_size(&msg, 1024).is_err());
}

#[test]
fn test_serialize_cdp_response() {
    let response = cdp_types::CdpResponse {
        id: 1,
        result: Some(json!({"value": 42})),
        error: None,
    };

    let json = serialize_cdp_message(&cdp_types::CdpMessage::Response(response));
    assert!(json.is_ok());

    let json_str = json.unwrap();
    assert!(json_str.contains("\"id\":1"));
    assert!(json_str.contains("\"result\""));
}

#[test]
fn test_serialize_cdp_event() {
    let event = cdp_types::CdpEvent {
        method: "DOM.documentUpdated".to_string(),
        params: json!({}),
    };

    let json = serialize_cdp_message(&cdp_types::CdpMessage::Event(event));
    assert!(json.is_ok());

    let json_str = json.unwrap();
    assert!(json_str.contains("\"method\":\"DOM.documentUpdated\""));
}
