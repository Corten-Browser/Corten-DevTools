//! Unit tests for CDP message validation module

use protocol_handler::{
    validate_cdp_request, validate_cdp_request_detailed, validate_method_name,
    MessageValidator, MessageValidatorConfig,
};
use serde_json::json;

// =========================================
// Request Validation Tests
// =========================================

mod request_validation {
    use super::*;

    #[test]
    fn test_valid_minimal_request() {
        let json = r#"{"id": 1, "method": "Runtime.evaluate"}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_ok());

        let request = result.unwrap();
        assert_eq!(request.id, 1);
        assert_eq!(request.method, "Runtime.evaluate");
        assert_eq!(request.domain, "Runtime");
        assert_eq!(request.method_name, "evaluate");
    }

    #[test]
    fn test_valid_request_with_params() {
        let json = r#"{"id": 42, "method": "DOM.getDocument", "params": {"depth": 1}}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_ok());

        let request = result.unwrap();
        assert_eq!(request.id, 42);
        assert_eq!(request.domain, "DOM");
        assert_eq!(request.method_name, "getDocument");
        assert!(request.params.is_some());
    }

    #[test]
    fn test_valid_request_with_empty_params() {
        let json = r#"{"id": 1, "method": "Network.enable", "params": {}}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_valid_request_with_null_params() {
        let json = r#"{"id": 1, "method": "Network.enable", "params": null}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_valid_request_with_complex_params() {
        let json = r#"{
            "id": 1,
            "method": "Runtime.evaluate",
            "params": {
                "expression": "document.title",
                "returnByValue": true,
                "awaitPromise": false,
                "contextId": 1
            }
        }"#;
        let result = validate_cdp_request(json);
        assert!(result.is_ok());
    }
}

// =========================================
// ID Validation Tests
// =========================================

mod id_validation {
    use super::*;

    #[test]
    fn test_missing_id() {
        let json = r#"{"method": "Runtime.evaluate"}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, -32600);
    }

    #[test]
    fn test_id_zero() {
        let json = r#"{"id": 0, "method": "Runtime.evaluate"}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, 0);
    }

    #[test]
    fn test_id_positive() {
        let json = r#"{"id": 123, "method": "Runtime.evaluate"}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, 123);
    }

    #[test]
    fn test_id_large() {
        let json = r#"{"id": 999999999999, "method": "Runtime.evaluate"}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_id_negative() {
        let json = r#"{"id": -1, "method": "Runtime.evaluate"}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_id_float() {
        let json = r#"{"id": 1.5, "method": "Runtime.evaluate"}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_id_string() {
        let json = r#"{"id": "1", "method": "Runtime.evaluate"}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_id_null() {
        let json = r#"{"id": null, "method": "Runtime.evaluate"}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_id_boolean() {
        let json = r#"{"id": true, "method": "Runtime.evaluate"}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_id_object() {
        let json = r#"{"id": {}, "method": "Runtime.evaluate"}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_err());
    }
}

// =========================================
// Method Validation Tests
// =========================================

mod method_validation {
    use super::*;

    #[test]
    fn test_missing_method() {
        let json = r#"{"id": 1}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, -32600);
    }

    #[test]
    fn test_empty_method() {
        let json = r#"{"id": 1, "method": ""}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_method_no_dot() {
        let json = r#"{"id": 1, "method": "evaluate"}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_method_domain_lowercase() {
        let json = r#"{"id": 1, "method": "runtime.evaluate"}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_method_name_uppercase() {
        let json = r#"{"id": 1, "method": "Runtime.Evaluate"}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_method_valid_format() {
        let json = r#"{"id": 1, "method": "Runtime.evaluate"}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_ok());

        let json = r#"{"id": 1, "method": "DOM.getDocument"}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_ok());

        let json = r#"{"id": 1, "method": "Network.requestWillBeSent"}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_method_with_numbers() {
        let json = r#"{"id": 1, "method": "DOM2.getNode123"}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_method_non_string() {
        let json = r#"{"id": 1, "method": 123}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_err());

        let json = r#"{"id": 1, "method": null}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_err());

        let json = r#"{"id": 1, "method": true}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_method_multiple_dots() {
        let json = r#"{"id": 1, "method": "DOM.Node.get"}"#;
        let result = validate_cdp_request(json);
        // splitn(2, '.') means this is parsed as domain="DOM", method="Node.get"
        // "Node.get" starts with uppercase, so it's invalid
        assert!(result.is_err());
    }
}

// =========================================
// Params Validation Tests
// =========================================

mod params_validation {
    use super::*;

    #[test]
    fn test_params_missing() {
        let json = r#"{"id": 1, "method": "Runtime.evaluate"}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_ok());
        assert!(result.unwrap().params.is_none());
    }

    #[test]
    fn test_params_null() {
        let json = r#"{"id": 1, "method": "Runtime.evaluate", "params": null}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_ok());
        assert!(result.unwrap().params.is_none());
    }

    #[test]
    fn test_params_empty_object() {
        let json = r#"{"id": 1, "method": "Runtime.evaluate", "params": {}}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_ok());
        assert!(result.unwrap().params.is_some());
    }

    #[test]
    fn test_params_array() {
        let json = r#"{"id": 1, "method": "Runtime.evaluate", "params": [1, 2, 3]}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_params_string() {
        let json = r#"{"id": 1, "method": "Runtime.evaluate", "params": "invalid"}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_params_number() {
        let json = r#"{"id": 1, "method": "Runtime.evaluate", "params": 123}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_params_boolean() {
        let json = r#"{"id": 1, "method": "Runtime.evaluate", "params": true}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_err());
    }
}

// =========================================
// JSON Parse Error Tests
// =========================================

mod json_parsing {
    use super::*;

    #[test]
    fn test_invalid_json() {
        let json = "not valid json";
        let result = validate_cdp_request(json);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, -32700);
    }

    #[test]
    fn test_incomplete_json() {
        let json = r#"{"id": 1, "method"#;
        let result = validate_cdp_request(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_json_array() {
        let json = r#"[{"id": 1, "method": "Runtime.evaluate"}]"#;
        let result = validate_cdp_request(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_string() {
        let json = "";
        let result = validate_cdp_request(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_json_primitive() {
        let json = "123";
        let result = validate_cdp_request(json);
        assert!(result.is_err());

        let json = "\"string\"";
        let result = validate_cdp_request(json);
        assert!(result.is_err());

        let json = "null";
        let result = validate_cdp_request(json);
        assert!(result.is_err());
    }
}

// =========================================
// Known Domains Tests
// =========================================

mod known_domains {
    use super::*;

    #[test]
    fn test_validator_knows_standard_domains() {
        let validator = MessageValidator::default();

        assert!(validator.is_known_domain("Runtime"));
        assert!(validator.is_known_domain("DOM"));
        assert!(validator.is_known_domain("Network"));
        assert!(validator.is_known_domain("Page"));
        assert!(validator.is_known_domain("Console"));
        assert!(validator.is_known_domain("Debugger"));
        assert!(validator.is_known_domain("Profiler"));
        assert!(validator.is_known_domain("HeapProfiler"));
    }

    #[test]
    fn test_unknown_domain() {
        let validator = MessageValidator::default();
        assert!(!validator.is_known_domain("CustomDomain"));
        assert!(!validator.is_known_domain("Unknown"));
    }

    #[test]
    fn test_strict_mode_enforces_known_domains() {
        let validator = MessageValidator::strict();

        let valid = json!({"id": 1, "method": "Runtime.evaluate"});
        assert!(validator.validate_request(&valid).is_ok());

        let invalid = json!({"id": 1, "method": "CustomDomain.method"});
        let result = validator.validate_request(&invalid);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, -32601);
    }

    #[test]
    fn test_custom_domains() {
        let config = MessageValidatorConfig {
            enforce_known_domains: true,
            custom_domains: vec!["MyDomain".to_string(), "AnotherDomain".to_string()],
            ..Default::default()
        };
        let validator = MessageValidator::new(config);

        assert!(validator.is_known_domain("MyDomain"));
        assert!(validator.is_known_domain("AnotherDomain"));

        let request = json!({"id": 1, "method": "MyDomain.doSomething"});
        assert!(validator.validate_request(&request).is_ok());
    }

    #[test]
    fn test_get_known_domains() {
        let validator = MessageValidator::default();
        let domains = validator.get_known_domains();

        assert!(domains.contains(&"Runtime"));
        assert!(domains.contains(&"DOM"));
        assert!(domains.len() > 10); // Should have many domains
    }
}

// =========================================
// Method Name Validation Tests
// =========================================

mod method_name_function {
    use super::*;

    #[test]
    fn test_validate_method_name_valid() {
        let result = validate_method_name("Runtime.evaluate");
        assert!(result.is_ok());

        let (domain, method) = result.unwrap();
        assert_eq!(domain, "Runtime");
        assert_eq!(method, "evaluate");
    }

    #[test]
    fn test_validate_method_name_invalid_format() {
        assert!(validate_method_name("invalid").is_err());
        assert!(validate_method_name("").is_err());
        assert!(validate_method_name(".").is_err());
        assert!(validate_method_name("Domain.").is_err());
        assert!(validate_method_name(".method").is_err());
    }

    #[test]
    fn test_validate_method_name_case() {
        assert!(validate_method_name("runtime.evaluate").is_err()); // lowercase domain
        assert!(validate_method_name("Runtime.Evaluate").is_err()); // uppercase method
        assert!(validate_method_name("Runtime.evaluate").is_ok()); // correct
    }
}

// =========================================
// Detailed Validation Tests
// =========================================

mod detailed_validation {
    use super::*;

    #[test]
    fn test_detailed_valid() {
        let json = r#"{"id": 1, "method": "Runtime.evaluate"}"#;
        let result = validate_cdp_request_detailed(json);

        assert!(result.is_valid);
        assert!(result.errors.is_empty());
        assert!(result.request.is_some());

        let request = result.request.unwrap();
        assert_eq!(request.id, 1);
        assert_eq!(request.domain, "Runtime");
    }

    #[test]
    fn test_detailed_invalid_returns_errors() {
        let json = r#"{"id": 1}"#; // Missing method
        let result = validate_cdp_request_detailed(json);

        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());
        assert!(result.request.is_none());
    }

    #[test]
    fn test_detailed_parse_error() {
        let json = "invalid json";
        let result = validate_cdp_request_detailed(json);

        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());
    }
}

// =========================================
// Configuration Tests
// =========================================

mod configuration {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = MessageValidatorConfig::default();
        assert_eq!(config.max_method_length, 256);
        assert_eq!(config.max_params_size, 10 * 1024 * 1024);
        assert!(!config.enforce_known_domains);
        assert!(config.allow_empty_params);
    }

    #[test]
    fn test_max_method_length() {
        let config = MessageValidatorConfig {
            max_method_length: 20,
            ..Default::default()
        };
        let validator = MessageValidator::new(config);

        let short = json!({"id": 1, "method": "Runtime.run"});
        assert!(validator.validate_request(&short).is_ok());

        let long = json!({"id": 1, "method": "VeryLongDomainName.veryLongMethodName"});
        assert!(validator.validate_request(&long).is_err());
    }

    #[test]
    fn test_disallow_empty_params() {
        let config = MessageValidatorConfig {
            allow_empty_params: false,
            ..Default::default()
        };
        let validator = MessageValidator::new(config);

        let with_null = json!({"id": 1, "method": "Runtime.evaluate", "params": null});
        assert!(validator.validate_request(&with_null).is_err());

        let with_object = json!({"id": 1, "method": "Runtime.evaluate", "params": {}});
        assert!(validator.validate_request(&with_object).is_ok());
    }
}

// =========================================
// Edge Cases Tests
// =========================================

mod edge_cases {
    use super::*;

    #[test]
    fn test_extra_fields_allowed() {
        let json =
            r#"{"id": 1, "method": "Runtime.evaluate", "sessionId": "abc", "custom": "field"}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_unicode_in_params() {
        let json =
            r#"{"id": 1, "method": "Runtime.evaluate", "params": {"expression": "console.log('Hello')"}}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_nested_params() {
        let json =
            r#"{"id": 1, "method": "Runtime.evaluate", "params": {"a": {"b": {"c": {"d": 1}}}}}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_large_id() {
        // Test with maximum u64 value
        let json = format!(
            r#"{{"id": {}, "method": "Runtime.evaluate"}}"#,
            u64::MAX
        );
        let result = validate_cdp_request(&json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_whitespace_handling() {
        let json = r#"
            {
                "id" : 1 ,
                "method" : "Runtime.evaluate"
            }
        "#;
        let result = validate_cdp_request(json);
        assert!(result.is_ok());
    }
}
