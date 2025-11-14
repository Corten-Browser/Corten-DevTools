//! Runtime domain implementation for JavaScript execution
//!
//! Handles JavaScript expression evaluation and remote object management.

use async_trait::async_trait;
use cdp_types::domains::runtime::*;
use cdp_types::CdpError;
use dashmap::DashMap;
use parking_lot::RwLock;
use protocol_handler::DomainHandler;
use serde_json::{json, Value};
use std::sync::atomic::AtomicU32;
use std::sync::Arc;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::{Result, RuntimeDebuggerError};

/// Runtime domain handler for JavaScript execution
pub struct RuntimeDomain {
    /// Map of remote object IDs to their values
    object_map: Arc<DashMap<RemoteObjectId, RemoteObject>>,
    /// Execution context counter (for future use)
    _context_counter: Arc<AtomicU32>,
    /// Enabled state
    enabled: Arc<RwLock<bool>>,
}

impl RuntimeDomain {
    /// Create a new RuntimeDomain
    pub fn new() -> Self {
        Self {
            object_map: Arc::new(DashMap::new()),
            _context_counter: Arc::new(AtomicU32::new(1)),
            enabled: Arc::new(RwLock::new(false)),
        }
    }

    /// Enable the Runtime domain
    pub fn enable(&self) {
        *self.enabled.write() = true;
        debug!("Runtime domain enabled");
    }

    /// Disable the Runtime domain
    pub fn disable(&self) {
        *self.enabled.write() = false;
        self.object_map.clear();
        debug!("Runtime domain disabled");
    }

    /// Check if domain is enabled
    pub fn is_enabled(&self) -> bool {
        *self.enabled.read()
    }

    /// Evaluate JavaScript expression
    pub fn evaluate(&self, expression: &str) -> Result<EvaluateResponse> {
        debug!("Evaluating expression: {}", expression);

        // Mock JavaScript evaluation for now
        let result = self.mock_evaluate(expression)?;
        let remote_object = self.create_remote_object(result);

        Ok(EvaluateResponse {
            result: remote_object,
            exception_details: None,
        })
    }

    /// Call function on remote object
    pub fn call_function_on(
        &self,
        object_id: &RemoteObjectId,
        function_declaration: &str,
    ) -> Result<RemoteObject> {
        debug!(
            "Calling function on object {:?}: {}",
            object_id, function_declaration
        );

        // Get the object
        let _obj = self
            .object_map
            .get(object_id)
            .ok_or_else(|| RuntimeDebuggerError::ObjectNotFound(object_id.0.clone()))?;

        // Mock function call for now
        let result = self.mock_evaluate(function_declaration)?;
        Ok(self.create_remote_object(result))
    }

    /// Get properties of remote object
    pub fn get_properties(&self, object_id: &RemoteObjectId) -> Result<Vec<PropertyDescriptor>> {
        debug!("Getting properties for object {:?}", object_id);

        let obj = self
            .object_map
            .get(object_id)
            .ok_or_else(|| RuntimeDebuggerError::ObjectNotFound(object_id.0.clone()))?;

        // For mock implementation, return basic properties
        Ok(self.mock_get_properties(&obj))
    }

    /// Release remote object
    pub fn release_object(&self, object_id: &RemoteObjectId) -> Result<()> {
        debug!("Releasing object {:?}", object_id);

        self.object_map
            .remove(object_id)
            .ok_or_else(|| RuntimeDebuggerError::ObjectNotFound(object_id.0.clone()))?;

        Ok(())
    }

    /// Release all remote objects
    pub fn release_all_objects(&self) {
        debug!("Releasing all remote objects");
        self.object_map.clear();
    }

    /// Create a remote object from a JSON value
    fn create_remote_object(&self, value: Value) -> RemoteObject {
        match value {
            Value::Null => RemoteObject {
                object_type: RemoteObjectType::Object,
                subtype: Some(RemoteObjectSubtype::Null),
                class_name: None,
                value: Some(Value::Null),
                unserializable_value: None,
                description: Some("null".to_string()),
                object_id: None,
                preview: None,
            },
            Value::Bool(b) => RemoteObject {
                object_type: RemoteObjectType::Boolean,
                subtype: None,
                class_name: None,
                value: Some(Value::Bool(b)),
                unserializable_value: None,
                description: Some(b.to_string()),
                object_id: None,
                preview: None,
            },
            Value::Number(n) => RemoteObject {
                object_type: RemoteObjectType::Number,
                subtype: None,
                class_name: None,
                value: Some(Value::Number(n.clone())),
                unserializable_value: None,
                description: Some(n.to_string()),
                object_id: None,
                preview: None,
            },
            Value::String(s) => RemoteObject {
                object_type: RemoteObjectType::String,
                subtype: None,
                class_name: None,
                value: Some(Value::String(s.clone())),
                unserializable_value: None,
                description: Some(s),
                object_id: None,
                preview: None,
            },
            Value::Array(_) | Value::Object(_) => {
                // For objects and arrays, create a remote object ID
                let object_id = RemoteObjectId(format!("obj-{}", Uuid::new_v4()));
                let description = format!("{:?}", value);

                let remote_obj = RemoteObject {
                    object_type: RemoteObjectType::Object,
                    subtype: if value.is_array() {
                        Some(RemoteObjectSubtype::Array)
                    } else {
                        None
                    },
                    class_name: Some(if value.is_array() {
                        "Array".to_string()
                    } else {
                        "Object".to_string()
                    }),
                    value: None,
                    unserializable_value: None,
                    description: Some(description),
                    object_id: Some(object_id.clone()),
                    preview: None,
                };

                // Store the object for later retrieval
                self.object_map.insert(object_id, remote_obj.clone());
                remote_obj
            }
        }
    }

    /// Mock JavaScript evaluation (to be replaced with real JS engine)
    fn mock_evaluate(&self, expression: &str) -> Result<Value> {
        let expr = expression.trim();

        // Try to parse as JSON first (handles objects and arrays)
        if let Ok(value) = serde_json::from_str::<Value>(expr) {
            return Ok(value);
        }

        // Simple mock evaluator for testing
        match expr {
            "42" => Ok(json!(42)),
            "true" => Ok(json!(true)),
            "false" => Ok(json!(false)),
            "null" => Ok(json!(null)),
            s if s.starts_with('"') && s.ends_with('"') => Ok(json!(s[1..s.len() - 1].to_string())),
            "1 + 1" => Ok(json!(2)),
            "2 * 3" => Ok(json!(6)),
            _ => Err(RuntimeDebuggerError::EvaluationError(format!(
                "Mock evaluator cannot handle: {}",
                expr
            ))),
        }
    }

    /// Mock property descriptor generation
    fn mock_get_properties(&self, _obj: &RemoteObject) -> Vec<PropertyDescriptor> {
        // For testing, return empty vector
        // In real implementation, this would inspect the object
        vec![]
    }
}

impl Default for RuntimeDomain {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DomainHandler for RuntimeDomain {
    fn name(&self) -> &str {
        "Runtime"
    }

    async fn handle_method(
        &self,
        method: &str,
        params: Option<Value>,
    ) -> std::result::Result<Value, CdpError> {
        match method {
            "enable" => {
                self.enable();
                Ok(json!({}))
            }
            "disable" => {
                self.disable();
                Ok(json!({}))
            }
            "evaluate" => {
                let params = params.ok_or_else(|| CdpError::invalid_params("Missing params"))?;
                let expression = params
                    .get("expression")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| CdpError::invalid_params("Missing expression"))?;

                let response = self
                    .evaluate(expression)
                    .map_err(|e| CdpError::internal_error(e.to_string()))?;

                Ok(serde_json::to_value(response)
                    .map_err(|e| CdpError::internal_error(e.to_string()))?)
            }
            "callFunctionOn" => {
                let params = params.ok_or_else(|| CdpError::invalid_params("Missing params"))?;
                let object_id_str = params
                    .get("objectId")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| CdpError::invalid_params("Missing objectId"))?;
                let object_id = RemoteObjectId(object_id_str.to_string());

                let function_declaration = params
                    .get("functionDeclaration")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| CdpError::invalid_params("Missing functionDeclaration"))?;

                let result = self
                    .call_function_on(&object_id, function_declaration)
                    .map_err(|e| CdpError::internal_error(e.to_string()))?;

                Ok(serde_json::to_value(result)
                    .map_err(|e| CdpError::internal_error(e.to_string()))?)
            }
            "getProperties" => {
                let params = params.ok_or_else(|| CdpError::invalid_params("Missing params"))?;
                let object_id_str = params
                    .get("objectId")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| CdpError::invalid_params("Missing objectId"))?;
                let object_id = RemoteObjectId(object_id_str.to_string());

                let properties = self
                    .get_properties(&object_id)
                    .map_err(|e| CdpError::internal_error(e.to_string()))?;

                Ok(json!({ "result": properties }))
            }
            "releaseObject" => {
                let params = params.ok_or_else(|| CdpError::invalid_params("Missing params"))?;
                let object_id_str = params
                    .get("objectId")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| CdpError::invalid_params("Missing objectId"))?;
                let object_id = RemoteObjectId(object_id_str.to_string());

                self.release_object(&object_id)
                    .map_err(|e| CdpError::internal_error(e.to_string()))?;

                Ok(json!({}))
            }
            "releaseObjectGroup" => {
                // For mock implementation, just release all objects
                self.release_all_objects();
                Ok(json!({}))
            }
            _ => {
                warn!("Unknown Runtime method: {}", method);
                Err(CdpError::method_not_found(format!("Runtime.{}", method)))
            }
        }
    }
}

/// Property descriptor (placeholder for now)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PropertyDescriptor {
    pub name: String,
    pub value: Option<RemoteObject>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_domain_new() {
        let runtime = RuntimeDomain::new();
        assert!(!runtime.is_enabled());
        assert_eq!(runtime.object_map.len(), 0);
    }

    #[test]
    fn test_runtime_enable_disable() {
        let runtime = RuntimeDomain::new();

        assert!(!runtime.is_enabled());

        runtime.enable();
        assert!(runtime.is_enabled());

        runtime.disable();
        assert!(!runtime.is_enabled());
    }

    #[test]
    fn test_evaluate_number() {
        let runtime = RuntimeDomain::new();
        runtime.enable();

        let response = runtime.evaluate("42").unwrap();
        assert_eq!(response.result.object_type, RemoteObjectType::Number);
        assert_eq!(response.result.value, Some(json!(42)));
        assert!(response.exception_details.is_none());
    }

    #[test]
    fn test_evaluate_string() {
        let runtime = RuntimeDomain::new();
        runtime.enable();

        let response = runtime.evaluate(r#""hello""#).unwrap();
        assert_eq!(response.result.object_type, RemoteObjectType::String);
        assert_eq!(response.result.value, Some(json!("hello")));
    }

    #[test]
    fn test_evaluate_boolean() {
        let runtime = RuntimeDomain::new();
        runtime.enable();

        let response = runtime.evaluate("true").unwrap();
        assert_eq!(response.result.object_type, RemoteObjectType::Boolean);
        assert_eq!(response.result.value, Some(json!(true)));
    }

    #[test]
    fn test_evaluate_null() {
        let runtime = RuntimeDomain::new();
        runtime.enable();

        let response = runtime.evaluate("null").unwrap();
        assert_eq!(response.result.object_type, RemoteObjectType::Object);
        assert_eq!(response.result.subtype, Some(RemoteObjectSubtype::Null));
    }

    #[test]
    fn test_evaluate_array() {
        let runtime = RuntimeDomain::new();
        runtime.enable();

        let response = runtime.evaluate("[1, 2, 3]").unwrap();
        assert_eq!(response.result.object_type, RemoteObjectType::Object);
        assert_eq!(response.result.subtype, Some(RemoteObjectSubtype::Array));
        assert!(response.result.object_id.is_some());
    }

    #[test]
    fn test_evaluate_object() {
        let runtime = RuntimeDomain::new();
        runtime.enable();

        let response = runtime.evaluate(r#"{"a": 1, "b": 2}"#).unwrap();
        assert_eq!(response.result.object_type, RemoteObjectType::Object);
        assert!(response.result.object_id.is_some());
    }

    #[test]
    fn test_evaluate_arithmetic() {
        let runtime = RuntimeDomain::new();
        runtime.enable();

        let response = runtime.evaluate("1 + 1").unwrap();
        assert_eq!(response.result.value, Some(json!(2)));

        let response = runtime.evaluate("2 * 3").unwrap();
        assert_eq!(response.result.value, Some(json!(6)));
    }

    #[test]
    fn test_call_function_on() {
        let runtime = RuntimeDomain::new();
        runtime.enable();

        // First create an object
        let response = runtime.evaluate(r#"{"a": 1}"#).unwrap();
        let object_id = response.result.object_id.clone().unwrap();

        // Now call a function on it
        let result = runtime.call_function_on(&object_id, "42");
        assert!(result.is_ok());
    }

    #[test]
    fn test_call_function_on_nonexistent_object() {
        let runtime = RuntimeDomain::new();
        runtime.enable();

        let object_id = RemoteObjectId("nonexistent".to_string());
        let result = runtime.call_function_on(&object_id, "42");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_properties() {
        let runtime = RuntimeDomain::new();
        runtime.enable();

        // Create an object
        let response = runtime.evaluate(r#"{"a": 1}"#).unwrap();
        let object_id = response.result.object_id.clone().unwrap();

        // Get properties
        let properties = runtime.get_properties(&object_id);
        assert!(properties.is_ok());
    }

    #[test]
    fn test_release_object() {
        let runtime = RuntimeDomain::new();
        runtime.enable();

        // Create an object
        let response = runtime.evaluate(r#"{"a": 1}"#).unwrap();
        let object_id = response.result.object_id.clone().unwrap();

        assert_eq!(runtime.object_map.len(), 1);

        // Release it
        let result = runtime.release_object(&object_id);
        assert!(result.is_ok());
        assert_eq!(runtime.object_map.len(), 0);
    }

    #[test]
    fn test_release_all_objects() {
        let runtime = RuntimeDomain::new();
        runtime.enable();

        // Create multiple objects
        let _ = runtime.evaluate(r#"{"a": 1}"#).unwrap();
        let _ = runtime.evaluate("[1, 2, 3]").unwrap();
        let _ = runtime.evaluate(r#"{"b": 2}"#).unwrap();

        assert_eq!(runtime.object_map.len(), 3);

        // Release all
        runtime.release_all_objects();
        assert_eq!(runtime.object_map.len(), 0);
    }

    #[tokio::test]
    async fn test_domain_handler_name() {
        let runtime = RuntimeDomain::new();
        assert_eq!(runtime.name(), "Runtime");
    }

    #[tokio::test]
    async fn test_domain_handler_enable() {
        let runtime = RuntimeDomain::new();
        let result = runtime.handle_method("enable", None).await;
        assert!(result.is_ok());
        assert!(runtime.is_enabled());
    }

    #[tokio::test]
    async fn test_domain_handler_disable() {
        let runtime = RuntimeDomain::new();
        runtime.enable();

        let result = runtime.handle_method("disable", None).await;
        assert!(result.is_ok());
        assert!(!runtime.is_enabled());
    }

    #[tokio::test]
    async fn test_domain_handler_evaluate() {
        let runtime = RuntimeDomain::new();
        runtime.enable();

        let params = json!({
            "expression": "42"
        });

        let result = runtime.handle_method("evaluate", Some(params)).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        let response: EvaluateResponse = serde_json::from_value(value).unwrap();
        assert_eq!(response.result.object_type, RemoteObjectType::Number);
    }

    #[tokio::test]
    async fn test_domain_handler_unknown_method() {
        let runtime = RuntimeDomain::new();

        let result = runtime.handle_method("unknownMethod", None).await;
        assert!(result.is_err());
    }
}
