//! Object Preview Generation (FEAT-038)
//!
//! Generates previews for complex JavaScript objects including:
//! - Arrays with element previews
//! - Objects with property previews
//! - Special types (Date, Map, Set, RegExp, etc.)
//! - Nested structures with configurable depth limits

use cdp_types::domains::runtime::{
    ObjectPreview, PropertyPreview, RemoteObject, RemoteObjectSubtype, RemoteObjectType,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Configuration for object preview generation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewConfig {
    /// Maximum depth for nested object previews
    pub max_depth: u32,
    /// Maximum number of properties to include in preview
    pub max_properties: usize,
    /// Maximum string length before truncation
    pub max_string_length: usize,
    /// Whether to include accessor properties
    pub include_accessors: bool,
    /// Whether to skip internal properties
    pub skip_internal: bool,
}

impl Default for PreviewConfig {
    fn default() -> Self {
        Self {
            max_depth: 3,
            max_properties: 5,
            max_string_length: 100,
            include_accessors: false,
            skip_internal: true,
        }
    }
}

/// Object preview generator
pub struct PreviewGenerator {
    config: PreviewConfig,
}

impl PreviewGenerator {
    /// Create a new preview generator with default config
    pub fn new() -> Self {
        Self {
            config: PreviewConfig::default(),
        }
    }

    /// Create a new preview generator with custom config
    pub fn with_config(config: PreviewConfig) -> Self {
        Self { config }
    }

    /// Generate preview for a RemoteObject
    pub fn generate_preview(&self, obj: &RemoteObject, value: &Value) -> Option<ObjectPreview> {
        self.generate_preview_at_depth(obj, value, 0)
    }

    /// Generate preview at a specific depth
    fn generate_preview_at_depth(
        &self,
        obj: &RemoteObject,
        value: &Value,
        depth: u32,
    ) -> Option<ObjectPreview> {
        if depth > self.config.max_depth {
            return None;
        }

        match &obj.object_type {
            RemoteObjectType::Object => {
                self.generate_object_preview(obj, value, depth)
            }
            RemoteObjectType::Function => {
                Some(self.generate_function_preview(obj))
            }
            _ => None, // Primitives don't need previews
        }
    }

    /// Generate preview for an object
    fn generate_object_preview(
        &self,
        obj: &RemoteObject,
        value: &Value,
        depth: u32,
    ) -> Option<ObjectPreview> {
        match &obj.subtype {
            Some(RemoteObjectSubtype::Array) => {
                self.generate_array_preview(value, depth)
            }
            Some(RemoteObjectSubtype::Date) => {
                Some(self.generate_date_preview(obj))
            }
            Some(RemoteObjectSubtype::Regexp) => {
                Some(self.generate_regexp_preview(obj))
            }
            Some(RemoteObjectSubtype::Map) => {
                self.generate_map_preview(value, depth)
            }
            Some(RemoteObjectSubtype::Set) => {
                self.generate_set_preview(value, depth)
            }
            Some(RemoteObjectSubtype::Error) => {
                Some(self.generate_error_preview(obj))
            }
            Some(RemoteObjectSubtype::Promise) => {
                Some(self.generate_promise_preview(obj))
            }
            Some(RemoteObjectSubtype::Typedarray) => {
                self.generate_typed_array_preview(value, depth)
            }
            Some(RemoteObjectSubtype::Null) => None,
            _ => self.generate_plain_object_preview(value, depth),
        }
    }

    /// Generate preview for a plain object
    fn generate_plain_object_preview(&self, value: &Value, depth: u32) -> Option<ObjectPreview> {
        let obj = value.as_object()?;
        let mut properties = Vec::new();
        let mut count = 0;

        for (key, val) in obj.iter() {
            if count >= self.config.max_properties {
                break;
            }

            if self.config.skip_internal && key.starts_with('_') {
                continue;
            }

            properties.push(self.value_to_property_preview(key, val, depth + 1));
            count += 1;
        }

        Some(ObjectPreview {
            object_type: RemoteObjectType::Object,
            subtype: None,
            description: Some(format!("Object {{{}}}", obj.len())),
            overflow: obj.len() > self.config.max_properties,
            properties,
        })
    }

    /// Generate preview for an array
    fn generate_array_preview(&self, value: &Value, depth: u32) -> Option<ObjectPreview> {
        let arr = value.as_array()?;
        let mut properties = Vec::new();

        for (i, val) in arr.iter().enumerate() {
            if i >= self.config.max_properties {
                break;
            }

            properties.push(self.value_to_property_preview(&i.to_string(), val, depth + 1));
        }

        Some(ObjectPreview {
            object_type: RemoteObjectType::Object,
            subtype: Some(RemoteObjectSubtype::Array),
            description: Some(format!("Array({})", arr.len())),
            overflow: arr.len() > self.config.max_properties,
            properties,
        })
    }

    /// Generate preview for a Date object
    fn generate_date_preview(&self, obj: &RemoteObject) -> ObjectPreview {
        ObjectPreview {
            object_type: RemoteObjectType::Object,
            subtype: Some(RemoteObjectSubtype::Date),
            description: obj.description.clone(),
            overflow: false,
            properties: vec![],
        }
    }

    /// Generate preview for a RegExp
    fn generate_regexp_preview(&self, obj: &RemoteObject) -> ObjectPreview {
        ObjectPreview {
            object_type: RemoteObjectType::Object,
            subtype: Some(RemoteObjectSubtype::Regexp),
            description: obj.description.clone(),
            overflow: false,
            properties: vec![],
        }
    }

    /// Generate preview for a Map
    fn generate_map_preview(&self, value: &Value, _depth: u32) -> Option<ObjectPreview> {
        // For maps represented as arrays of [key, value] pairs
        let entries = value.get("entries").and_then(|e| e.as_array())?;
        let mut properties = Vec::new();

        for (i, entry) in entries.iter().enumerate() {
            if i >= self.config.max_properties {
                break;
            }

            if let Some(arr) = entry.as_array() {
                if arr.len() >= 2 {
                    let key_str = self.value_to_short_string(&arr[0]);
                    let val_str = self.value_to_short_string(&arr[1]);
                    properties.push(PropertyPreview {
                        name: key_str,
                        property_type: self.value_to_type(&arr[1]),
                        value: Some(val_str),
                        subtype: self.value_to_subtype(&arr[1]),
                    });
                }
            }
        }

        Some(ObjectPreview {
            object_type: RemoteObjectType::Object,
            subtype: Some(RemoteObjectSubtype::Map),
            description: Some(format!("Map({})", entries.len())),
            overflow: entries.len() > self.config.max_properties,
            properties,
        })
    }

    /// Generate preview for a Set
    fn generate_set_preview(&self, value: &Value, depth: u32) -> Option<ObjectPreview> {
        // For sets represented as arrays of values
        let values = value.get("values").and_then(|v| v.as_array())?;
        let mut properties = Vec::new();

        for (i, val) in values.iter().enumerate() {
            if i >= self.config.max_properties {
                break;
            }

            properties.push(self.value_to_property_preview(&i.to_string(), val, depth + 1));
        }

        Some(ObjectPreview {
            object_type: RemoteObjectType::Object,
            subtype: Some(RemoteObjectSubtype::Set),
            description: Some(format!("Set({})", values.len())),
            overflow: values.len() > self.config.max_properties,
            properties,
        })
    }

    /// Generate preview for an Error
    fn generate_error_preview(&self, obj: &RemoteObject) -> ObjectPreview {
        let mut properties = vec![];

        // Add message property if available
        if let Some(desc) = &obj.description {
            properties.push(PropertyPreview {
                name: "message".to_string(),
                property_type: RemoteObjectType::String,
                value: Some(desc.clone()),
                subtype: None,
            });
        }

        ObjectPreview {
            object_type: RemoteObjectType::Object,
            subtype: Some(RemoteObjectSubtype::Error),
            description: obj.description.clone(),
            overflow: false,
            properties,
        }
    }

    /// Generate preview for a Promise
    fn generate_promise_preview(&self, obj: &RemoteObject) -> ObjectPreview {
        ObjectPreview {
            object_type: RemoteObjectType::Object,
            subtype: Some(RemoteObjectSubtype::Promise),
            description: obj.description.clone().or(Some("Promise".to_string())),
            overflow: false,
            properties: vec![
                PropertyPreview {
                    name: "[[PromiseState]]".to_string(),
                    property_type: RemoteObjectType::String,
                    value: Some("pending".to_string()),
                    subtype: None,
                },
            ],
        }
    }

    /// Generate preview for a TypedArray
    fn generate_typed_array_preview(&self, value: &Value, _depth: u32) -> Option<ObjectPreview> {
        let arr = value.as_array()?;
        let mut properties = Vec::new();

        for (i, val) in arr.iter().enumerate() {
            if i >= self.config.max_properties {
                break;
            }

            properties.push(PropertyPreview {
                name: i.to_string(),
                property_type: RemoteObjectType::Number,
                value: val.as_f64().map(|n| n.to_string()),
                subtype: None,
            });
        }

        Some(ObjectPreview {
            object_type: RemoteObjectType::Object,
            subtype: Some(RemoteObjectSubtype::Typedarray),
            description: Some(format!("TypedArray({})", arr.len())),
            overflow: arr.len() > self.config.max_properties,
            properties,
        })
    }

    /// Generate preview for a function
    fn generate_function_preview(&self, obj: &RemoteObject) -> ObjectPreview {
        ObjectPreview {
            object_type: RemoteObjectType::Function,
            subtype: None,
            description: obj.description.clone(),
            overflow: false,
            properties: vec![],
        }
    }

    /// Convert a Value to a PropertyPreview
    fn value_to_property_preview(
        &self,
        name: &str,
        value: &Value,
        depth: u32,
    ) -> PropertyPreview {
        let value_str = if depth < self.config.max_depth {
            Some(self.value_to_short_string(value))
        } else {
            Some("...".to_string())
        };

        PropertyPreview {
            name: name.to_string(),
            property_type: self.value_to_type(value),
            value: value_str,
            subtype: self.value_to_subtype(value),
        }
    }

    /// Convert a Value to a short string representation
    fn value_to_short_string(&self, value: &Value) -> String {
        match value {
            Value::Null => "null".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => n.to_string(),
            Value::String(s) => {
                if s.len() > self.config.max_string_length {
                    format!("\"{}...\"", &s[..self.config.max_string_length])
                } else {
                    format!("\"{}\"", s)
                }
            }
            Value::Array(arr) => format!("Array({})", arr.len()),
            Value::Object(obj) => format!("{{...}} ({} keys)", obj.len()),
        }
    }

    /// Get the RemoteObjectType for a Value
    fn value_to_type(&self, value: &Value) -> RemoteObjectType {
        match value {
            Value::Null => RemoteObjectType::Object,
            Value::Bool(_) => RemoteObjectType::Boolean,
            Value::Number(_) => RemoteObjectType::Number,
            Value::String(_) => RemoteObjectType::String,
            Value::Array(_) | Value::Object(_) => RemoteObjectType::Object,
        }
    }

    /// Get the RemoteObjectSubtype for a Value (if any)
    fn value_to_subtype(&self, value: &Value) -> Option<RemoteObjectSubtype> {
        match value {
            Value::Null => Some(RemoteObjectSubtype::Null),
            Value::Array(_) => Some(RemoteObjectSubtype::Array),
            _ => None,
        }
    }
}

impl Default for PreviewGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate a formatted description for various object types
pub fn generate_description(obj_type: &RemoteObjectType, subtype: Option<&RemoteObjectSubtype>, value: &Value) -> String {
    match (obj_type, subtype) {
        (RemoteObjectType::Object, Some(RemoteObjectSubtype::Array)) => {
            if let Some(arr) = value.as_array() {
                format!("Array({})", arr.len())
            } else {
                "Array".to_string()
            }
        }
        (RemoteObjectType::Object, Some(RemoteObjectSubtype::Date)) => {
            "Date".to_string()
        }
        (RemoteObjectType::Object, Some(RemoteObjectSubtype::Regexp)) => {
            value.as_str().unwrap_or("/regex/").to_string()
        }
        (RemoteObjectType::Object, Some(RemoteObjectSubtype::Map)) => {
            "Map".to_string()
        }
        (RemoteObjectType::Object, Some(RemoteObjectSubtype::Set)) => {
            "Set".to_string()
        }
        (RemoteObjectType::Object, Some(RemoteObjectSubtype::Error)) => {
            value.get("message")
                .and_then(|m| m.as_str())
                .map(|s| format!("Error: {}", s))
                .unwrap_or_else(|| "Error".to_string())
        }
        (RemoteObjectType::Object, Some(RemoteObjectSubtype::Null)) => {
            "null".to_string()
        }
        (RemoteObjectType::Object, None) => {
            if let Some(obj) = value.as_object() {
                format!("Object {{{}}}", obj.len())
            } else {
                "Object".to_string()
            }
        }
        (RemoteObjectType::Function, _) => {
            "function".to_string()
        }
        (RemoteObjectType::String, _) => {
            if let Some(s) = value.as_str() {
                if s.len() > 100 {
                    format!("\"{}...\"", &s[..100])
                } else {
                    format!("\"{}\"", s)
                }
            } else {
                "string".to_string()
            }
        }
        (RemoteObjectType::Number, _) => {
            value.as_f64()
                .map(|n| n.to_string())
                .unwrap_or_else(|| "number".to_string())
        }
        (RemoteObjectType::Boolean, _) => {
            value.as_bool()
                .map(|b| b.to_string())
                .unwrap_or_else(|| "boolean".to_string())
        }
        (RemoteObjectType::Undefined, _) => {
            "undefined".to_string()
        }
        (RemoteObjectType::Symbol, _) => {
            "Symbol()".to_string()
        }
        (RemoteObjectType::Bigint, _) => {
            format!("{}n", value.as_str().unwrap_or("0"))
        }
        // Catch-all for remaining combinations
        (_, _) => "object".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_preview_config_default() {
        let config = PreviewConfig::default();
        assert_eq!(config.max_depth, 3);
        assert_eq!(config.max_properties, 5);
        assert_eq!(config.max_string_length, 100);
    }

    #[test]
    fn test_preview_generator_new() {
        let gen = PreviewGenerator::new();
        assert_eq!(gen.config.max_depth, 3);
    }

    #[test]
    fn test_preview_generator_with_config() {
        let config = PreviewConfig {
            max_depth: 5,
            max_properties: 10,
            ..Default::default()
        };
        let gen = PreviewGenerator::with_config(config);
        assert_eq!(gen.config.max_depth, 5);
        assert_eq!(gen.config.max_properties, 10);
    }

    #[test]
    fn test_generate_plain_object_preview() {
        let gen = PreviewGenerator::new();
        let value = json!({"a": 1, "b": 2, "c": 3});

        let preview = gen.generate_plain_object_preview(&value, 0).unwrap();
        assert_eq!(preview.object_type, RemoteObjectType::Object);
        assert_eq!(preview.properties.len(), 3);
        assert!(!preview.overflow);
    }

    #[test]
    fn test_generate_array_preview() {
        let gen = PreviewGenerator::new();
        let value = json!([1, 2, 3, 4, 5]);

        let preview = gen.generate_array_preview(&value, 0).unwrap();
        assert_eq!(preview.object_type, RemoteObjectType::Object);
        assert_eq!(preview.subtype, Some(RemoteObjectSubtype::Array));
        assert_eq!(preview.properties.len(), 5);
        assert!(!preview.overflow);
        assert_eq!(preview.description, Some("Array(5)".to_string()));
    }

    #[test]
    fn test_generate_array_preview_overflow() {
        let gen = PreviewGenerator::new();
        let value = json!([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);

        let preview = gen.generate_array_preview(&value, 0).unwrap();
        assert_eq!(preview.properties.len(), 5); // max_properties
        assert!(preview.overflow);
        assert_eq!(preview.description, Some("Array(10)".to_string()));
    }

    #[test]
    fn test_generate_date_preview() {
        let gen = PreviewGenerator::new();
        let obj = RemoteObject {
            object_type: RemoteObjectType::Object,
            subtype: Some(RemoteObjectSubtype::Date),
            class_name: Some("Date".to_string()),
            value: None,
            unserializable_value: None,
            description: Some("Wed Jan 01 2025".to_string()),
            object_id: None,
            preview: None,
        };

        let preview = gen.generate_date_preview(&obj);
        assert_eq!(preview.subtype, Some(RemoteObjectSubtype::Date));
        assert_eq!(preview.description, Some("Wed Jan 01 2025".to_string()));
    }

    #[test]
    fn test_generate_error_preview() {
        let gen = PreviewGenerator::new();
        let obj = RemoteObject {
            object_type: RemoteObjectType::Object,
            subtype: Some(RemoteObjectSubtype::Error),
            class_name: Some("Error".to_string()),
            value: None,
            unserializable_value: None,
            description: Some("Something went wrong".to_string()),
            object_id: None,
            preview: None,
        };

        let preview = gen.generate_error_preview(&obj);
        assert_eq!(preview.subtype, Some(RemoteObjectSubtype::Error));
        assert_eq!(preview.properties.len(), 1);
        assert_eq!(preview.properties[0].name, "message");
    }

    #[test]
    fn test_generate_promise_preview() {
        let gen = PreviewGenerator::new();
        let obj = RemoteObject {
            object_type: RemoteObjectType::Object,
            subtype: Some(RemoteObjectSubtype::Promise),
            class_name: Some("Promise".to_string()),
            value: None,
            unserializable_value: None,
            description: None,
            object_id: None,
            preview: None,
        };

        let preview = gen.generate_promise_preview(&obj);
        assert_eq!(preview.subtype, Some(RemoteObjectSubtype::Promise));
        assert_eq!(preview.description, Some("Promise".to_string()));
    }

    #[test]
    fn test_generate_function_preview() {
        let gen = PreviewGenerator::new();
        let obj = RemoteObject {
            object_type: RemoteObjectType::Function,
            subtype: None,
            class_name: Some("Function".to_string()),
            value: None,
            unserializable_value: None,
            description: Some("function foo() {}".to_string()),
            object_id: None,
            preview: None,
        };

        let preview = gen.generate_function_preview(&obj);
        assert_eq!(preview.object_type, RemoteObjectType::Function);
        assert_eq!(preview.description, Some("function foo() {}".to_string()));
    }

    #[test]
    fn test_value_to_short_string() {
        let gen = PreviewGenerator::new();

        assert_eq!(gen.value_to_short_string(&json!(null)), "null");
        assert_eq!(gen.value_to_short_string(&json!(true)), "true");
        assert_eq!(gen.value_to_short_string(&json!(42)), "42");
        assert_eq!(gen.value_to_short_string(&json!("hello")), "\"hello\"");
        assert_eq!(gen.value_to_short_string(&json!([1, 2, 3])), "Array(3)");
        assert_eq!(gen.value_to_short_string(&json!({"a": 1})), "{...} (1 keys)");
    }

    #[test]
    fn test_value_to_short_string_truncation() {
        let config = PreviewConfig {
            max_string_length: 10,
            ..Default::default()
        };
        let gen = PreviewGenerator::with_config(config);

        let long_string = "a".repeat(50);
        let result = gen.value_to_short_string(&json!(long_string));
        assert!(result.len() < 20);
        assert!(result.ends_with("...\""));
    }

    #[test]
    fn test_value_to_type() {
        let gen = PreviewGenerator::new();

        assert_eq!(gen.value_to_type(&json!(null)), RemoteObjectType::Object);
        assert_eq!(gen.value_to_type(&json!(true)), RemoteObjectType::Boolean);
        assert_eq!(gen.value_to_type(&json!(42)), RemoteObjectType::Number);
        assert_eq!(gen.value_to_type(&json!("hello")), RemoteObjectType::String);
        assert_eq!(gen.value_to_type(&json!([1, 2])), RemoteObjectType::Object);
        assert_eq!(gen.value_to_type(&json!({"a": 1})), RemoteObjectType::Object);
    }

    #[test]
    fn test_value_to_subtype() {
        let gen = PreviewGenerator::new();

        assert_eq!(gen.value_to_subtype(&json!(null)), Some(RemoteObjectSubtype::Null));
        assert_eq!(gen.value_to_subtype(&json!([1, 2])), Some(RemoteObjectSubtype::Array));
        assert_eq!(gen.value_to_subtype(&json!(42)), None);
        assert_eq!(gen.value_to_subtype(&json!("hello")), None);
    }

    #[test]
    fn test_generate_description_array() {
        let desc = generate_description(
            &RemoteObjectType::Object,
            Some(&RemoteObjectSubtype::Array),
            &json!([1, 2, 3]),
        );
        assert_eq!(desc, "Array(3)");
    }

    #[test]
    fn test_generate_description_object() {
        let desc = generate_description(
            &RemoteObjectType::Object,
            None,
            &json!({"a": 1, "b": 2}),
        );
        assert_eq!(desc, "Object {2}");
    }

    #[test]
    fn test_generate_description_null() {
        let desc = generate_description(
            &RemoteObjectType::Object,
            Some(&RemoteObjectSubtype::Null),
            &json!(null),
        );
        assert_eq!(desc, "null");
    }

    #[test]
    fn test_generate_description_string() {
        let desc = generate_description(
            &RemoteObjectType::String,
            None,
            &json!("hello"),
        );
        assert_eq!(desc, "\"hello\"");
    }

    #[test]
    fn test_nested_object_preview() {
        let gen = PreviewGenerator::new();
        let value = json!({
            "level1": {
                "level2": {
                    "level3": {
                        "level4": "deep"
                    }
                }
            }
        });

        let preview = gen.generate_plain_object_preview(&value, 0).unwrap();
        assert_eq!(preview.properties.len(), 1);
        assert_eq!(preview.properties[0].name, "level1");
    }

    #[test]
    fn test_skip_internal_properties() {
        let config = PreviewConfig {
            skip_internal: true,
            ..Default::default()
        };
        let gen = PreviewGenerator::with_config(config);
        let value = json!({
            "public": 1,
            "_internal": 2,
            "__private": 3
        });

        let preview = gen.generate_plain_object_preview(&value, 0).unwrap();
        assert_eq!(preview.properties.len(), 1);
        assert_eq!(preview.properties[0].name, "public");
    }

    #[test]
    fn test_include_internal_properties() {
        let config = PreviewConfig {
            skip_internal: false,
            ..Default::default()
        };
        let gen = PreviewGenerator::with_config(config);
        let value = json!({
            "public": 1,
            "_internal": 2
        });

        let preview = gen.generate_plain_object_preview(&value, 0).unwrap();
        assert_eq!(preview.properties.len(), 2);
    }
}
