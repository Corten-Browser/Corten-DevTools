// Unit tests for core CDP message types
// These tests should FAIL initially (RED phase of TDD)

use serde_json::json;

#[cfg(test)]
mod core_message_types {
    use super::*;

    #[test]
    fn test_cdp_request_serialization() {
        // Test that CdpRequest can be serialized to JSON
        let request = cdp_types::CdpRequest {
            id: 1,
            method: "Runtime.evaluate".to_string(),
            params: Some(json!({
                "expression": "1 + 1"
            })),
        };

        let json_str = serde_json::to_string(&request).unwrap();
        assert!(json_str.contains("\"id\":1"));
        assert!(json_str.contains("\"method\":\"Runtime.evaluate\""));
        assert!(json_str.contains("\"params\""));
    }

    #[test]
    fn test_cdp_request_deserialization() {
        // Test that JSON can be deserialized to CdpRequest
        let json_str = r#"{"id":1,"method":"DOM.getDocument","params":null}"#;
        let request: cdp_types::CdpRequest = serde_json::from_str(json_str).unwrap();

        assert_eq!(request.id, 1);
        assert_eq!(request.method, "DOM.getDocument");
        assert!(request.params.is_none());
    }

    #[test]
    fn test_cdp_request_with_params() {
        // Test request with parameters
        let json_str = r#"{
            "id": 2,
            "method": "Network.enable",
            "params": {
                "maxTotalBufferSize": 10000000
            }
        }"#;

        let request: cdp_types::CdpRequest = serde_json::from_str(json_str).unwrap();
        assert_eq!(request.id, 2);
        assert_eq!(request.method, "Network.enable");
        assert!(request.params.is_some());

        let params = request.params.unwrap();
        assert_eq!(params["maxTotalBufferSize"], 10000000);
    }

    #[test]
    fn test_cdp_response_success() {
        // Test successful response with result
        let response = cdp_types::CdpResponse {
            id: 1,
            result: Some(json!({
                "result": {
                    "type": "number",
                    "value": 2
                }
            })),
            error: None,
        };

        let json_str = serde_json::to_string(&response).unwrap();
        assert!(json_str.contains("\"id\":1"));
        assert!(json_str.contains("\"result\""));
        assert!(!json_str.contains("\"error\""));
    }

    #[test]
    fn test_cdp_response_error() {
        // Test error response
        let response = cdp_types::CdpResponse {
            id: 1,
            result: None,
            error: Some(cdp_types::CdpError {
                code: -32601,
                message: "Method not found".to_string(),
                data: None,
            }),
        };

        let json_str = serde_json::to_string(&response).unwrap();
        assert!(json_str.contains("\"id\":1"));
        assert!(json_str.contains("\"error\""));
        assert!(json_str.contains("-32601"));
    }

    #[test]
    fn test_cdp_response_deserialization() {
        // Test deserializing response from JSON
        let json_str = r##"{
            "id": 3,
            "result": {
                "root": {
                    "nodeId": 1,
                    "nodeType": 9,
                    "nodeName": "#document"
                }
            }
        }"##;

        let response: cdp_types::CdpResponse = serde_json::from_str(json_str).unwrap();
        assert_eq!(response.id, 3);
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_cdp_event_serialization() {
        // Test event serialization
        let event = cdp_types::CdpEvent {
            method: "Network.requestWillBeSent".to_string(),
            params: json!({
                "requestId": "1234.5",
                "documentURL": "https://example.com"
            }),
        };

        let json_str = serde_json::to_string(&event).unwrap();
        assert!(json_str.contains("\"method\":\"Network.requestWillBeSent\""));
        assert!(json_str.contains("\"params\""));
        assert!(!json_str.contains("\"id\"")); // Events don't have id
    }

    #[test]
    fn test_cdp_event_deserialization() {
        // Test deserializing event from JSON
        let json_str = r#"{
            "method": "DOM.attributeModified",
            "params": {
                "nodeId": 10,
                "name": "class",
                "value": "active"
            }
        }"#;

        let event: cdp_types::CdpEvent = serde_json::from_str(json_str).unwrap();
        assert_eq!(event.method, "DOM.attributeModified");
        assert_eq!(event.params["nodeId"], 10);
        assert_eq!(event.params["name"], "class");
        assert_eq!(event.params["value"], "active");
    }

    #[test]
    fn test_cdp_message_variants() {
        // Test that we can distinguish between request, response, and event
        let request_json = r#"{"id":1,"method":"test","params":null}"#;
        let response_json = r#"{"id":1,"result":{}}"#;
        let event_json = r#"{"method":"test","params":{}}"#;

        // These should be deserializable to appropriate types
        assert!(serde_json::from_str::<cdp_types::CdpRequest>(request_json).is_ok());
        assert!(serde_json::from_str::<cdp_types::CdpResponse>(response_json).is_ok());
        assert!(serde_json::from_str::<cdp_types::CdpEvent>(event_json).is_ok());
    }
}

#[cfg(test)]
mod cdp_error_types {
    use super::*;

    #[test]
    fn test_cdp_error_serialization() {
        // Test error serialization
        let error = cdp_types::CdpError {
            code: -32601,
            message: "Method not found".to_string(),
            data: Some(json!({"details": "Unknown method: test"})),
        };

        let json_str = serde_json::to_string(&error).unwrap();
        assert!(json_str.contains("-32601"));
        assert!(json_str.contains("Method not found"));
        assert!(json_str.contains("details"));
    }

    #[test]
    fn test_cdp_error_standard_codes() {
        // Test standard JSON-RPC error codes
        assert_eq!(cdp_types::CdpError::parse_error().code, -32700);
        assert_eq!(cdp_types::CdpError::invalid_request().code, -32600);
        assert_eq!(cdp_types::CdpError::method_not_found("test").code, -32601);
        assert_eq!(
            cdp_types::CdpError::invalid_params("bad params").code,
            -32602
        );
        assert_eq!(cdp_types::CdpError::internal_error("error").code, -32603);
    }

    #[test]
    fn test_cdp_error_custom_codes() {
        // Test custom CDP error codes
        let error = cdp_types::CdpError::server_error(-32000, "Custom error");
        assert_eq!(error.code, -32000);
        assert_eq!(error.message, "Custom error");
    }
}

#[cfg(test)]
mod type_safety {
    use super::*;

    #[test]
    fn test_request_id_type() {
        // Test that request IDs are properly typed
        let request = cdp_types::CdpRequest {
            id: 42,
            method: "test".to_string(),
            params: None,
        };

        assert_eq!(request.id, 42);
    }

    #[test]
    fn test_method_name_string() {
        // Test that method names are strings
        let request = cdp_types::CdpRequest {
            id: 1,
            method: "Domain.method".to_string(),
            params: None,
        };

        assert!(request.method.contains('.'));
        assert_eq!(request.method.split('.').count(), 2);
    }

    #[test]
    fn test_params_optional() {
        // Test that params is optional
        let request_no_params = cdp_types::CdpRequest {
            id: 1,
            method: "test".to_string(),
            params: None,
        };

        let request_with_params = cdp_types::CdpRequest {
            id: 1,
            method: "test".to_string(),
            params: Some(json!({"key": "value"})),
        };

        assert!(request_no_params.params.is_none());
        assert!(request_with_params.params.is_some());
    }
}
