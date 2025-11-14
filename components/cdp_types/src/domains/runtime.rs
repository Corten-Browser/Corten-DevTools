// Runtime domain types (JavaScript execution)

use serde::{Deserialize, Serialize};

/// Execution context identifier
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ExecutionContextId(pub u32);

/// Remote object identifier
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RemoteObjectId(pub String);

/// Object type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RemoteObjectType {
    Object,
    Function,
    Undefined,
    String,
    Number,
    Boolean,
    Symbol,
    Bigint,
}

/// Object subtype
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RemoteObjectSubtype {
    Array,
    Null,
    Node,
    Regexp,
    Date,
    Map,
    Set,
    Weakmap,
    Weakset,
    Iterator,
    Generator,
    Error,
    Proxy,
    Promise,
    Typedarray,
    Arraybuffer,
    Dataview,
}

/// Remote object representing JavaScript value
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RemoteObject {
    /// Object type
    #[serde(rename = "type")]
    pub object_type: RemoteObjectType,
    /// Object subtype
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtype: Option<RemoteObjectSubtype>,
    /// Object class name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_name: Option<String>,
    /// Primitive value (for primitives)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
    /// Unserializable value (NaN, Infinity, -Infinity, -0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unserializable_value: Option<String>,
    /// String representation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Remote object ID (for objects)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_id: Option<RemoteObjectId>,
    /// Object preview
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview: Option<ObjectPreview>,
}

/// Object preview
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ObjectPreview {
    /// Object type
    #[serde(rename = "type")]
    pub object_type: RemoteObjectType,
    /// Object subtype
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtype: Option<RemoteObjectSubtype>,
    /// String representation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Whether preview is truncated
    pub overflow: bool,
    /// Preview properties
    pub properties: Vec<PropertyPreview>,
}

/// Property preview
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PropertyPreview {
    /// Property name
    pub name: String,
    /// Property type
    #[serde(rename = "type")]
    pub property_type: RemoteObjectType,
    /// Property value (short string)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Property subtype
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtype: Option<RemoteObjectSubtype>,
}

/// Response for Runtime.evaluate
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateResponse {
    /// Evaluation result
    pub result: RemoteObject,
    /// Exception details (if thrown)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exception_details: Option<ExceptionDetails>,
}

/// Exception details
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExceptionDetails {
    /// Exception ID
    pub exception_id: u32,
    /// Exception text
    pub text: String,
    /// Line number
    pub line_number: u32,
    /// Column number
    pub column_number: u32,
    /// Script ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script_id: Option<String>,
    /// URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Stack trace
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack_trace: Option<StackTrace>,
    /// Exception object
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exception: Option<RemoteObject>,
}

/// Stack trace
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StackTrace {
    /// Stack frames
    pub call_frames: Vec<CallFrame>,
    /// Parent stack trace
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<Box<StackTrace>>,
}

/// Call frame
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CallFrame {
    /// Function name
    pub function_name: String,
    /// Script ID
    pub script_id: String,
    /// Script URL
    pub url: String,
    /// Line number (0-based)
    pub line_number: u32,
    /// Column number (0-based)
    pub column_number: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_context_id() {
        let id = ExecutionContextId(1);
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "1");
    }

    #[test]
    fn test_remote_object_id() {
        let id = RemoteObjectId("obj-123".to_string());
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"obj-123\"");
    }

    #[test]
    fn test_remote_object_type() {
        let obj = RemoteObjectType::Object;
        let json = serde_json::to_string(&obj).unwrap();
        assert_eq!(json, "\"object\"");

        let num = RemoteObjectType::Number;
        let json = serde_json::to_string(&num).unwrap();
        assert_eq!(json, "\"number\"");
    }
}
