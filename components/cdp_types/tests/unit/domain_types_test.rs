// Unit tests for CDP domain types
// These tests should FAIL initially (RED phase of TDD)

#[cfg(test)]
mod browser_domain {

    #[test]
    fn test_get_version_response() {
        let response = cdp_types::domains::browser::GetVersionResponse {
            protocol_version: "1.3".to_string(),
            product: "CortenBrowser/1.0".to_string(),
            revision: "abc123".to_string(),
            user_agent: "Mozilla/5.0".to_string(),
            js_version: "V8/11.0".to_string(),
        };

        let json_str = serde_json::to_string(&response).unwrap();
        assert!(json_str.contains("1.3"));
        assert!(json_str.contains("CortenBrowser"));
    }

    #[test]
    fn test_get_version_deserialization() {
        let json_str = r#"{
            "protocolVersion": "1.3",
            "product": "CortenBrowser/1.0",
            "revision": "abc123",
            "userAgent": "Mozilla/5.0",
            "jsVersion": "V8/11.0"
        }"#;

        let response: cdp_types::domains::browser::GetVersionResponse =
            serde_json::from_str(json_str).unwrap();

        assert_eq!(response.protocol_version, "1.3");
        assert_eq!(response.product, "CortenBrowser/1.0");
    }
}

#[cfg(test)]
mod dom_domain {

    #[test]
    fn test_node_id_type() {
        let node_id = cdp_types::domains::dom::NodeId(123);
        let json_str = serde_json::to_string(&node_id).unwrap();
        assert_eq!(json_str, "123");
    }

    #[test]
    fn test_node_type_enum() {
        let element = cdp_types::domains::dom::NodeType::Element;
        let document = cdp_types::domains::dom::NodeType::Document;
        let text = cdp_types::domains::dom::NodeType::Text;

        assert_eq!(element as u32, 1);
        assert_eq!(document as u32, 9);
        assert_eq!(text as u32, 3);
    }

    #[test]
    fn test_node_serialization() {
        let node = cdp_types::domains::dom::Node {
            node_id: cdp_types::domains::dom::NodeId(1),
            node_type: cdp_types::domains::dom::NodeType::Element,
            node_name: "div".to_string(),
            local_name: Some("div".to_string()),
            node_value: None,
            child_node_count: Some(2),
            children: None,
            attributes: Some(vec!["class".to_string(), "container".to_string()]),
        };

        let json_str = serde_json::to_string(&node).unwrap();
        assert!(json_str.contains("\"nodeId\":1"));
        assert!(json_str.contains("\"nodeName\":\"div\""));
    }

    #[test]
    fn test_get_document_response() {
        let node = cdp_types::domains::dom::Node {
            node_id: cdp_types::domains::dom::NodeId(1),
            node_type: cdp_types::domains::dom::NodeType::Document,
            node_name: "#document".to_string(),
            local_name: None,
            node_value: None,
            child_node_count: Some(1),
            children: None,
            attributes: None,
        };

        let response = cdp_types::domains::dom::GetDocumentResponse { root: node };

        let json_str = serde_json::to_string(&response).unwrap();
        assert!(json_str.contains("#document"));
    }
}

#[cfg(test)]
mod css_domain {

    #[test]
    fn test_style_sheet_id() {
        let id = cdp_types::domains::css::StyleSheetId("stylesheet-1".to_string());
        let json_str = serde_json::to_string(&id).unwrap();
        assert_eq!(json_str, "\"stylesheet-1\"");
    }

    #[test]
    fn test_css_property() {
        let prop = cdp_types::domains::css::CSSProperty {
            name: "color".to_string(),
            value: "red".to_string(),
            important: Some(false),
            implicit: Some(false),
            text: Some("color: red".to_string()),
            parsed_ok: Some(true),
            disabled: Some(false),
            range: None,
        };

        let json_str = serde_json::to_string(&prop).unwrap();
        assert!(json_str.contains("\"name\":\"color\""));
        assert!(json_str.contains("\"value\":\"red\""));
    }

    #[test]
    fn test_css_rule() {
        let rule = cdp_types::domains::css::CSSRule {
            style_sheet_id: Some(cdp_types::domains::css::StyleSheetId("1".to_string())),
            selector_list: cdp_types::domains::css::SelectorList {
                selectors: vec![cdp_types::domains::css::Value {
                    text: ".container".to_string(),
                }],
                text: ".container".to_string(),
            },
            origin: cdp_types::domains::css::StyleSheetOrigin::Regular,
            style: cdp_types::domains::css::CSSStyle {
                style_sheet_id: Some(cdp_types::domains::css::StyleSheetId("1".to_string())),
                css_properties: vec![],
                short_hand_entries: vec![],
                css_text: Some("".to_string()),
                range: None,
            },
        };

        let json_str = serde_json::to_string(&rule).unwrap();
        assert!(json_str.contains(".container"));
    }
}

#[cfg(test)]
mod network_domain {

    #[test]
    fn test_request_id() {
        let id = cdp_types::domains::network::RequestId("request-123".to_string());
        let json_str = serde_json::to_string(&id).unwrap();
        assert_eq!(json_str, "\"request-123\"");
    }

    #[test]
    fn test_timestamp() {
        let timestamp = cdp_types::domains::network::Timestamp(1234567890.123);
        let json_str = serde_json::to_string(&timestamp).unwrap();
        assert!(json_str.contains("1234567890.123"));
    }

    #[test]
    fn test_resource_type() {
        let doc = cdp_types::domains::network::ResourceType::Document;
        let script = cdp_types::domains::network::ResourceType::Script;
        let xhr = cdp_types::domains::network::ResourceType::XHR;

        // Test serialization
        assert_eq!(serde_json::to_string(&doc).unwrap(), "\"Document\"");
        assert_eq!(serde_json::to_string(&script).unwrap(), "\"Script\"");
        assert_eq!(serde_json::to_string(&xhr).unwrap(), "\"XHR\"");
    }

    #[test]
    fn test_request() {
        let request = cdp_types::domains::network::Request {
            url: "https://example.com".to_string(),
            method: "GET".to_string(),
            headers: std::collections::HashMap::new(),
            post_data: None,
            has_post_data: Some(false),
            mixed_content_type: None,
            initial_priority: cdp_types::domains::network::ResourcePriority::High,
            referrer_policy: cdp_types::domains::network::ReferrerPolicy::NoReferrerWhenDowngrade,
        };

        let json_str = serde_json::to_string(&request).unwrap();
        assert!(json_str.contains("https://example.com"));
        assert!(json_str.contains("GET"));
    }

    #[test]
    fn test_response() {
        let response = cdp_types::domains::network::Response {
            url: "https://example.com".to_string(),
            status: 200,
            status_text: "OK".to_string(),
            headers: std::collections::HashMap::new(),
            mime_type: "text/html".to_string(),
            request_headers: None,
            connection_reused: true,
            connection_id: 1,
            from_disk_cache: Some(false),
            from_service_worker: Some(false),
            encoded_data_length: 1024.0,
            timing: None,
            protocol: Some("http/1.1".to_string()),
            security_state: cdp_types::domains::network::SecurityState::Secure,
        };

        let json_str = serde_json::to_string(&response).unwrap();
        assert!(json_str.contains("200"));
        assert!(json_str.contains("OK"));
    }
}

#[cfg(test)]
mod runtime_domain {
    use serde_json::json;

    #[test]
    fn test_execution_context_id() {
        let id = cdp_types::domains::runtime::ExecutionContextId(1);
        let json_str = serde_json::to_string(&id).unwrap();
        assert_eq!(json_str, "1");
    }

    #[test]
    fn test_remote_object_id() {
        let id = cdp_types::domains::runtime::RemoteObjectId("obj-123".to_string());
        let json_str = serde_json::to_string(&id).unwrap();
        assert_eq!(json_str, "\"obj-123\"");
    }

    #[test]
    fn test_remote_object_type() {
        let obj = cdp_types::domains::runtime::RemoteObjectType::Object;
        let func = cdp_types::domains::runtime::RemoteObjectType::Function;
        let num = cdp_types::domains::runtime::RemoteObjectType::Number;

        assert_eq!(serde_json::to_string(&obj).unwrap(), "\"object\"");
        assert_eq!(serde_json::to_string(&func).unwrap(), "\"function\"");
        assert_eq!(serde_json::to_string(&num).unwrap(), "\"number\"");
    }

    #[test]
    fn test_remote_object() {
        let obj = cdp_types::domains::runtime::RemoteObject {
            object_type: cdp_types::domains::runtime::RemoteObjectType::Number,
            subtype: None,
            class_name: None,
            value: Some(json!(42)),
            unserializable_value: None,
            description: Some("42".to_string()),
            object_id: None,
            preview: None,
        };

        let json_str = serde_json::to_string(&obj).unwrap();
        assert!(json_str.contains("\"type\":\"number\""));
        assert!(json_str.contains("42"));
    }

    #[test]
    fn test_evaluate_response() {
        let response = cdp_types::domains::runtime::EvaluateResponse {
            result: cdp_types::domains::runtime::RemoteObject {
                object_type: cdp_types::domains::runtime::RemoteObjectType::Number,
                subtype: None,
                class_name: None,
                value: Some(json!(2)),
                unserializable_value: None,
                description: Some("2".to_string()),
                object_id: None,
                preview: None,
            },
            exception_details: None,
        };

        let json_str = serde_json::to_string(&response).unwrap();
        assert!(json_str.contains("\"result\""));
    }
}

#[cfg(test)]
mod debugger_domain {

    #[test]
    fn test_breakpoint_id() {
        let id = cdp_types::domains::debugger::BreakpointId("bp-123".to_string());
        let json_str = serde_json::to_string(&id).unwrap();
        assert_eq!(json_str, "\"bp-123\"");
    }

    #[test]
    fn test_script_id() {
        let id = cdp_types::domains::debugger::ScriptId("script-456".to_string());
        let json_str = serde_json::to_string(&id).unwrap();
        assert_eq!(json_str, "\"script-456\"");
    }

    #[test]
    fn test_location() {
        let location = cdp_types::domains::debugger::Location {
            script_id: cdp_types::domains::debugger::ScriptId("1".to_string()),
            line_number: 10,
            column_number: Some(5),
        };

        let json_str = serde_json::to_string(&location).unwrap();
        assert!(json_str.contains("\"scriptId\":\"1\""));
        assert!(json_str.contains("\"lineNumber\":10"));
        assert!(json_str.contains("\"columnNumber\":5"));
    }

    #[test]
    fn test_call_frame() {
        let frame = cdp_types::domains::debugger::CallFrame {
            call_frame_id: "frame-1".to_string(),
            function_name: "myFunction".to_string(),
            location: cdp_types::domains::debugger::Location {
                script_id: cdp_types::domains::debugger::ScriptId("1".to_string()),
                line_number: 10,
                column_number: Some(0),
            },
            url: "https://example.com/script.js".to_string(),
            scope_chain: vec![],
            this: cdp_types::domains::runtime::RemoteObject {
                object_type: cdp_types::domains::runtime::RemoteObjectType::Object,
                subtype: None,
                class_name: None,
                value: None,
                unserializable_value: None,
                description: None,
                object_id: Some(cdp_types::domains::runtime::RemoteObjectId(
                    "obj-1".to_string(),
                )),
                preview: None,
            },
            return_value: None,
        };

        let json_str = serde_json::to_string(&frame).unwrap();
        assert!(json_str.contains("myFunction"));
    }
}

#[cfg(test)]
mod profiler_domain {

    #[test]
    fn test_profile_node() {
        let node = cdp_types::domains::profiler::ProfileNode {
            id: 1,
            call_frame: cdp_types::domains::runtime::CallFrame {
                function_name: "main".to_string(),
                script_id: "1".to_string(),
                url: "file.js".to_string(),
                line_number: 0,
                column_number: 0,
            },
            hit_count: Some(10),
            children: Some(vec![2, 3]),
        };

        let json_str = serde_json::to_string(&node).unwrap();
        assert!(json_str.contains("\"id\":1"));
        assert!(json_str.contains("\"hitCount\":10"));
    }

    #[test]
    fn test_profile() {
        let profile = cdp_types::domains::profiler::Profile {
            nodes: vec![],
            start_time: 1000.0,
            end_time: 2000.0,
            samples: Some(vec![1, 2, 3]),
            time_deltas: Some(vec![10, 20, 30]),
        };

        let json_str = serde_json::to_string(&profile).unwrap();
        assert!(json_str.contains("\"startTime\":1000"));
        assert!(json_str.contains("\"endTime\":2000"));
    }
}

#[cfg(test)]
mod console_domain {

    #[test]
    fn test_console_message_source() {
        let source = cdp_types::domains::console::ConsoleMessageSource::Console;
        let json_str = serde_json::to_string(&source).unwrap();
        assert_eq!(json_str, "\"console\"");
    }

    #[test]
    fn test_console_message_level() {
        let level = cdp_types::domains::console::ConsoleMessageLevel::Error;
        let json_str = serde_json::to_string(&level).unwrap();
        assert_eq!(json_str, "\"error\"");
    }

    #[test]
    fn test_console_message() {
        let message = cdp_types::domains::console::ConsoleMessage {
            source: cdp_types::domains::console::ConsoleMessageSource::Console,
            level: cdp_types::domains::console::ConsoleMessageLevel::Log,
            text: "Hello, world!".to_string(),
            url: Some("https://example.com".to_string()),
            line: Some(10),
            column: Some(5),
        };

        let json_str = serde_json::to_string(&message).unwrap();
        assert!(json_str.contains("Hello, world!"));
        assert!(json_str.contains("\"source\":\"console\""));
        assert!(json_str.contains("\"level\":\"log\""));
    }
}
