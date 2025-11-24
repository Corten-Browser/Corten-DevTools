//! CDP message validation module
//!
//! This module provides comprehensive validation for CDP (Chrome DevTools Protocol)
//! messages, ensuring they conform to the protocol specification.

use cdp_types::CdpError;
use serde_json::Value;
use std::collections::HashSet;

/// Known CDP domains for validation
static KNOWN_DOMAINS: &[&str] = &[
    "Browser",
    "Console",
    "CSS",
    "Debugger",
    "DOM",
    "DOMDebugger",
    "DOMSnapshot",
    "DOMStorage",
    "Emulation",
    "HeadlessExperimental",
    "HeapProfiler",
    "IndexedDB",
    "Input",
    "IO",
    "Inspector",
    "Log",
    "Memory",
    "Network",
    "Overlay",
    "Page",
    "Performance",
    "Profiler",
    "Runtime",
    "Schema",
    "Security",
    "ServiceWorker",
    "Storage",
    "SystemInfo",
    "Target",
    "Tracing",
    "WebAudio",
    "WebAuthn",
    "Animation",
    "Accessibility",
    "Audits",
    "BackgroundService",
    "CacheStorage",
    "Cast",
    "Database",
    "DeviceOrientation",
    "Fetch",
    "LayerTree",
    "Media",
    "Tethering",
];

/// CDP message validation configuration
#[derive(Debug, Clone)]
pub struct MessageValidatorConfig {
    /// Maximum method name length
    pub max_method_length: usize,
    /// Maximum params size in bytes (when serialized)
    pub max_params_size: usize,
    /// Whether to enforce known domains only
    pub enforce_known_domains: bool,
    /// Whether to allow empty params
    pub allow_empty_params: bool,
    /// Custom allowed domains (in addition to known ones)
    pub custom_domains: Vec<String>,
}

impl Default for MessageValidatorConfig {
    fn default() -> Self {
        Self {
            max_method_length: 256,
            max_params_size: 10 * 1024 * 1024, // 10MB
            enforce_known_domains: false,
            allow_empty_params: true,
            custom_domains: vec![],
        }
    }
}

/// CDP message validator
#[derive(Debug, Clone)]
pub struct MessageValidator {
    config: MessageValidatorConfig,
    known_domains: HashSet<String>,
}

impl MessageValidator {
    /// Create a new message validator with the given configuration
    pub fn new(config: MessageValidatorConfig) -> Self {
        let mut known_domains: HashSet<String> =
            KNOWN_DOMAINS.iter().map(|s| s.to_string()).collect();

        for domain in &config.custom_domains {
            known_domains.insert(domain.clone());
        }

        Self {
            config,
            known_domains,
        }
    }

    /// Create a strict validator that enforces known domains
    pub fn strict() -> Self {
        Self::new(MessageValidatorConfig {
            enforce_known_domains: true,
            ..Default::default()
        })
    }

    /// Validate a CDP request
    pub fn validate_request(&self, json: &Value) -> Result<ValidatedRequest, CdpError> {
        // Validate it's an object
        let obj = json.as_object().ok_or_else(|| {
            CdpError::invalid_request()
        })?;

        // Validate 'id' field
        let id = self.validate_id(obj)?;

        // Validate 'method' field
        let method = self.validate_method(obj)?;

        // Parse and validate method format (clone to avoid borrow issues)
        let method_clone = method.clone();
        let (domain, method_name) = self.validate_method_format(&method_clone)?;

        // Validate 'params' field (optional)
        let params = self.validate_params(obj)?;

        Ok(ValidatedRequest {
            id,
            method,
            domain: domain.to_string(),
            method_name: method_name.to_string(),
            params,
        })
    }

    /// Validate a raw JSON string as a CDP request
    pub fn validate_request_str(&self, json_str: &str) -> Result<ValidatedRequest, CdpError> {
        let json: Value = serde_json::from_str(json_str).map_err(|e| {
            CdpError::with_data(
                -32700,
                "Parse error",
                serde_json::json!({ "details": e.to_string() }),
            )
        })?;

        self.validate_request(&json)
    }

    /// Validate the 'id' field
    fn validate_id(&self, obj: &serde_json::Map<String, Value>) -> Result<u64, CdpError> {
        let id = obj.get("id").ok_or_else(|| {
            CdpError::with_data(
                -32600,
                "Invalid Request",
                serde_json::json!({ "details": "Missing 'id' field" }),
            )
        })?;

        // CDP spec: id should be a positive integer
        match id {
            Value::Number(n) => {
                if let Some(id_u64) = n.as_u64() {
                    Ok(id_u64)
                } else if let Some(id_i64) = n.as_i64() {
                    if id_i64 >= 0 {
                        Ok(id_i64 as u64)
                    } else {
                        Err(CdpError::with_data(
                            -32600,
                            "Invalid Request",
                            serde_json::json!({ "details": "Request 'id' must be a non-negative integer" }),
                        ))
                    }
                } else {
                    Err(CdpError::with_data(
                        -32600,
                        "Invalid Request",
                        serde_json::json!({ "details": "Request 'id' must be an integer, not a float" }),
                    ))
                }
            }
            _ => Err(CdpError::with_data(
                -32600,
                "Invalid Request",
                serde_json::json!({ "details": "Request 'id' must be a number" }),
            )),
        }
    }

    /// Validate the 'method' field
    fn validate_method(&self, obj: &serde_json::Map<String, Value>) -> Result<String, CdpError> {
        let method = obj.get("method").ok_or_else(|| {
            CdpError::with_data(
                -32600,
                "Invalid Request",
                serde_json::json!({ "details": "Missing 'method' field" }),
            )
        })?;

        let method_str = method.as_str().ok_or_else(|| {
            CdpError::with_data(
                -32600,
                "Invalid Request",
                serde_json::json!({ "details": "'method' must be a string" }),
            )
        })?;

        // Check method length
        if method_str.len() > self.config.max_method_length {
            return Err(CdpError::with_data(
                -32600,
                "Invalid Request",
                serde_json::json!({
                    "details": format!(
                        "Method name exceeds maximum length of {} characters",
                        self.config.max_method_length
                    )
                }),
            ));
        }

        // Check for empty method
        if method_str.is_empty() {
            return Err(CdpError::with_data(
                -32600,
                "Invalid Request",
                serde_json::json!({ "details": "'method' cannot be empty" }),
            ));
        }

        Ok(method_str.to_string())
    }

    /// Validate method format (Domain.methodName)
    fn validate_method_format<'a>(&self, method: &'a str) -> Result<(&'a str, &'a str), CdpError> {
        let parts: Vec<&str> = method.splitn(2, '.').collect();

        if parts.len() != 2 {
            return Err(CdpError::with_data(
                -32600,
                "Invalid Request",
                serde_json::json!({
                    "details": "Method must be in format 'Domain.methodName'",
                    "method": method
                }),
            ));
        }

        let domain = parts[0];
        let method_name = parts[1];

        // Validate domain name (PascalCase: starts with uppercase letter)
        if !self.is_valid_domain_name(domain) {
            return Err(CdpError::with_data(
                -32600,
                "Invalid Request",
                serde_json::json!({
                    "details": "Domain name must start with an uppercase letter",
                    "domain": domain
                }),
            ));
        }

        // Validate method name (camelCase: starts with lowercase letter)
        if !self.is_valid_method_name(method_name) {
            return Err(CdpError::with_data(
                -32600,
                "Invalid Request",
                serde_json::json!({
                    "details": "Method name must start with a lowercase letter",
                    "method_name": method_name
                }),
            ));
        }

        // Check against known domains if enforcement is enabled
        if self.config.enforce_known_domains && !self.known_domains.contains(domain) {
            return Err(CdpError::with_data(
                -32601,
                "Method not found",
                serde_json::json!({
                    "details": "Unknown domain",
                    "domain": domain
                }),
            ));
        }

        Ok((domain, method_name))
    }

    /// Check if a domain name is valid (PascalCase)
    fn is_valid_domain_name(&self, name: &str) -> bool {
        if name.is_empty() {
            return false;
        }

        let first_char = name.chars().next().unwrap();
        first_char.is_ascii_uppercase() && name.chars().all(|c| c.is_ascii_alphanumeric())
    }

    /// Check if a method name is valid (camelCase)
    fn is_valid_method_name(&self, name: &str) -> bool {
        if name.is_empty() {
            return false;
        }

        let first_char = name.chars().next().unwrap();
        first_char.is_ascii_lowercase() && name.chars().all(|c| c.is_ascii_alphanumeric())
    }

    /// Validate the 'params' field
    fn validate_params(
        &self,
        obj: &serde_json::Map<String, Value>,
    ) -> Result<Option<Value>, CdpError> {
        let params = match obj.get("params") {
            Some(p) => p,
            None => return Ok(None),
        };

        // Params must be an object or null
        match params {
            Value::Object(_) => {
                // Check params size
                let params_str = serde_json::to_string(params).unwrap_or_default();
                if params_str.len() > self.config.max_params_size {
                    return Err(CdpError::with_data(
                        -32600,
                        "Invalid Request",
                        serde_json::json!({
                            "details": format!(
                                "Params size exceeds maximum of {} bytes",
                                self.config.max_params_size
                            )
                        }),
                    ));
                }
                Ok(Some(params.clone()))
            }
            Value::Null => {
                if self.config.allow_empty_params {
                    Ok(None)
                } else {
                    Err(CdpError::with_data(
                        -32600,
                        "Invalid Request",
                        serde_json::json!({ "details": "Null params are not allowed" }),
                    ))
                }
            }
            _ => Err(CdpError::with_data(
                -32600,
                "Invalid Request",
                serde_json::json!({ "details": "'params' must be an object or null" }),
            )),
        }
    }

    /// Check if a domain is known
    pub fn is_known_domain(&self, domain: &str) -> bool {
        self.known_domains.contains(domain)
    }

    /// Get list of known domains
    pub fn get_known_domains(&self) -> Vec<&str> {
        self.known_domains.iter().map(|s| s.as_str()).collect()
    }
}

impl Default for MessageValidator {
    fn default() -> Self {
        Self::new(MessageValidatorConfig::default())
    }
}

/// Validated CDP request with parsed components
#[derive(Debug, Clone)]
pub struct ValidatedRequest {
    /// Request ID
    pub id: u64,
    /// Full method name (Domain.method)
    pub method: String,
    /// Domain name (e.g., "DOM")
    pub domain: String,
    /// Method name without domain (e.g., "getDocument")
    pub method_name: String,
    /// Optional parameters
    pub params: Option<Value>,
}

/// Quick validation function for CDP requests
pub fn validate_cdp_request(json_str: &str) -> Result<ValidatedRequest, CdpError> {
    let validator = MessageValidator::default();
    validator.validate_request_str(json_str)
}

/// Validate method name format without full request validation
pub fn validate_method_name(method: &str) -> Result<(String, String), CdpError> {
    let validator = MessageValidator::default();
    let (domain, method_name) = validator.validate_method_format(method)?;
    Ok((domain.to_string(), method_name.to_string()))
}

/// Validation result with detailed information
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the message is valid
    pub is_valid: bool,
    /// Validation errors (if any)
    pub errors: Vec<String>,
    /// Parsed request (if valid)
    pub request: Option<ValidatedRequest>,
}

/// Validate with detailed result
pub fn validate_cdp_request_detailed(json_str: &str) -> ValidationResult {
    let validator = MessageValidator::default();

    match validator.validate_request_str(json_str) {
        Ok(request) => ValidationResult {
            is_valid: true,
            errors: vec![],
            request: Some(request),
        },
        Err(e) => ValidationResult {
            is_valid: false,
            errors: vec![e.message.clone()],
            request: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // =========================================
    // Basic Request Validation Tests
    // =========================================

    #[test]
    fn test_validate_valid_request() {
        let json = r#"{"id": 1, "method": "Runtime.evaluate"}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_ok());

        let request = result.unwrap();
        assert_eq!(request.id, 1);
        assert_eq!(request.domain, "Runtime");
        assert_eq!(request.method_name, "evaluate");
    }

    #[test]
    fn test_validate_request_with_params() {
        let json = r#"{"id": 1, "method": "Runtime.evaluate", "params": {"expression": "1+1"}}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_ok());

        let request = result.unwrap();
        assert!(request.params.is_some());
    }

    #[test]
    fn test_validate_request_without_params() {
        let json = r#"{"id": 1, "method": "Runtime.enable"}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_ok());

        let request = result.unwrap();
        assert!(request.params.is_none());
    }

    // =========================================
    // ID Validation Tests
    // =========================================

    #[test]
    fn test_missing_id() {
        let json = r#"{"method": "Runtime.evaluate"}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert_eq!(error.code, -32600);
    }

    #[test]
    fn test_invalid_id_string() {
        let json = r#"{"id": "not-a-number", "method": "Runtime.evaluate"}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_id_negative() {
        let json = r#"{"id": -1, "method": "Runtime.evaluate"}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_id_float() {
        let json = r#"{"id": 1.5, "method": "Runtime.evaluate"}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_valid_id_zero() {
        let json = r#"{"id": 0, "method": "Runtime.evaluate"}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, 0);
    }

    #[test]
    fn test_valid_id_large() {
        let json = r#"{"id": 9999999999, "method": "Runtime.evaluate"}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_ok());
    }

    // =========================================
    // Method Validation Tests
    // =========================================

    #[test]
    fn test_missing_method() {
        let json = r#"{"id": 1}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert_eq!(error.code, -32600);
    }

    #[test]
    fn test_empty_method() {
        let json = r#"{"id": 1, "method": ""}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_method_format_no_dot() {
        let json = r#"{"id": 1, "method": "evaluate"}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_method_format_multiple_dots() {
        let json = r#"{"id": 1, "method": "Runtime.sub.evaluate"}"#;
        let result = validate_cdp_request(json);
        // This should fail - after splitn(2, '.'), method name is "sub.evaluate"
        // which contains a dot (non-alphanumeric character)
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_domain_lowercase() {
        let json = r#"{"id": 1, "method": "runtime.evaluate"}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_method_name_uppercase() {
        let json = r#"{"id": 1, "method": "Runtime.Evaluate"}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_method_non_string() {
        let json = r#"{"id": 1, "method": 123}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_err());
    }

    // =========================================
    // Params Validation Tests
    // =========================================

    #[test]
    fn test_params_object() {
        let json = r#"{"id": 1, "method": "Runtime.evaluate", "params": {"a": 1}}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_params_null() {
        let json = r#"{"id": 1, "method": "Runtime.evaluate", "params": null}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_params_array_invalid() {
        let json = r#"{"id": 1, "method": "Runtime.evaluate", "params": [1, 2, 3]}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_params_string_invalid() {
        let json = r#"{"id": 1, "method": "Runtime.evaluate", "params": "invalid"}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_err());
    }

    // =========================================
    // JSON Parse Error Tests
    // =========================================

    #[test]
    fn test_invalid_json() {
        let json = "not valid json";
        let result = validate_cdp_request(json);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert_eq!(error.code, -32700);
    }

    #[test]
    fn test_incomplete_json() {
        let json = r#"{"id": 1, "method"#;
        let result = validate_cdp_request(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_not_an_object() {
        let json = r#"[1, 2, 3]"#;
        let result = validate_cdp_request(json);
        assert!(result.is_err());
    }

    // =========================================
    // Known Domains Tests
    // =========================================

    #[test]
    fn test_known_domains() {
        let validator = MessageValidator::default();

        assert!(validator.is_known_domain("Runtime"));
        assert!(validator.is_known_domain("DOM"));
        assert!(validator.is_known_domain("Network"));
        assert!(!validator.is_known_domain("UnknownDomain"));
    }

    #[test]
    fn test_enforce_known_domains() {
        let validator = MessageValidator::strict();

        let valid = json!({"id": 1, "method": "Runtime.evaluate"});
        assert!(validator.validate_request(&valid).is_ok());

        let invalid = json!({"id": 1, "method": "CustomDomain.method"});
        assert!(validator.validate_request(&invalid).is_err());
    }

    #[test]
    fn test_custom_domains() {
        let config = MessageValidatorConfig {
            enforce_known_domains: true,
            custom_domains: vec!["CustomDomain".to_string()],
            ..Default::default()
        };
        let validator = MessageValidator::new(config);

        let request = json!({"id": 1, "method": "CustomDomain.doSomething"});
        assert!(validator.validate_request(&request).is_ok());
    }

    // =========================================
    // Method Name Function Tests
    // =========================================

    #[test]
    fn test_validate_method_name_valid() {
        let result = validate_method_name("Runtime.evaluate");
        assert!(result.is_ok());

        let (domain, method) = result.unwrap();
        assert_eq!(domain, "Runtime");
        assert_eq!(method, "evaluate");
    }

    #[test]
    fn test_validate_method_name_invalid() {
        assert!(validate_method_name("invalid").is_err());
        assert!(validate_method_name("").is_err());
        assert!(validate_method_name(".").is_err());
    }

    // =========================================
    // Detailed Validation Tests
    // =========================================

    #[test]
    fn test_validate_detailed_valid() {
        let json = r#"{"id": 1, "method": "Runtime.evaluate"}"#;
        let result = validate_cdp_request_detailed(json);

        assert!(result.is_valid);
        assert!(result.errors.is_empty());
        assert!(result.request.is_some());
    }

    #[test]
    fn test_validate_detailed_invalid() {
        let json = r#"{"id": 1}"#; // Missing method
        let result = validate_cdp_request_detailed(json);

        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());
        assert!(result.request.is_none());
    }

    // =========================================
    // Configuration Tests
    // =========================================

    #[test]
    fn test_max_method_length() {
        let config = MessageValidatorConfig {
            max_method_length: 20,
            ..Default::default()
        };
        let validator = MessageValidator::new(config);

        // Short method should work
        let short = json!({"id": 1, "method": "Runtime.run"});
        assert!(validator.validate_request(&short).is_ok());

        // Long method should fail
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

        let with_null_params = json!({"id": 1, "method": "Runtime.evaluate", "params": null});
        assert!(validator.validate_request(&with_null_params).is_err());

        let with_object_params = json!({"id": 1, "method": "Runtime.evaluate", "params": {}});
        assert!(validator.validate_request(&with_object_params).is_ok());
    }

    // =========================================
    // Edge Cases
    // =========================================

    #[test]
    fn test_extra_fields_allowed() {
        let json = r#"{"id": 1, "method": "Runtime.evaluate", "sessionId": "abc", "extra": true}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_unicode_in_params() {
        let json = r#"{"id": 1, "method": "Runtime.evaluate", "params": {"expression": "console.log('Hello')"}}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_nested_params() {
        let json = r#"{"id": 1, "method": "Runtime.evaluate", "params": {"a": {"b": {"c": 1}}}}"#;
        let result = validate_cdp_request(json);
        assert!(result.is_ok());
    }
}
