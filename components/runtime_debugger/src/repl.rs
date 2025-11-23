//! JavaScript REPL (Read-Eval-Print-Loop) implementation
//!
//! Provides interactive JavaScript console evaluation with support for:
//! - Multi-line expression support
//! - Command history tracking
//! - Auto-completion hints
//! - REPL mode evaluation

use cdp_types::domains::runtime::{
    ObjectPreview, PropertyPreview, RemoteObject, RemoteObjectId, RemoteObjectSubtype,
    RemoteObjectType,
};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tracing::debug;

use crate::{Result, RuntimeDebuggerError};

/// Maximum number of history entries to keep
const MAX_HISTORY_SIZE: usize = 1000;

/// Default preview depth for objects
const DEFAULT_PREVIEW_DEPTH: u32 = 3;

/// Maximum properties to show in preview
const MAX_PREVIEW_PROPERTIES: usize = 5;

/// REPL session state
#[derive(Debug)]
pub struct ReplSession {
    /// Command history
    history: Arc<RwLock<VecDeque<HistoryEntry>>>,
    /// Current multi-line buffer
    multiline_buffer: Arc<RwLock<String>>,
    /// Whether we're in multi-line mode
    in_multiline: Arc<RwLock<bool>>,
    /// Session ID
    session_id: String,
    /// Evaluation counter
    eval_counter: Arc<AtomicU32>,
    /// Auto-completion suggestions cache
    completion_cache: Arc<RwLock<Vec<CompletionItem>>>,
}

/// History entry for REPL commands
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    /// The expression that was evaluated
    pub expression: String,
    /// Timestamp of evaluation (Unix milliseconds)
    pub timestamp: u64,
    /// Whether evaluation was successful
    pub success: bool,
    /// Result preview (if successful)
    pub result_preview: Option<String>,
}

/// Auto-completion item
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionItem {
    /// The completion text
    pub text: String,
    /// Display label
    pub label: String,
    /// Type of completion (property, method, keyword)
    pub kind: CompletionKind,
    /// Documentation (if available)
    pub documentation: Option<String>,
}

/// Types of auto-completion items
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CompletionKind {
    Property,
    Method,
    Keyword,
    Variable,
    Class,
    Constant,
}

/// REPL evaluation options
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReplEvaluateOptions {
    /// Enable REPL mode (allows statements without return)
    #[serde(default)]
    pub repl_mode: bool,
    /// Generate object preview
    #[serde(default)]
    pub generate_preview: bool,
    /// Preview depth for nested objects
    pub preview_depth: Option<u32>,
    /// Whether to include command-line API
    #[serde(default)]
    pub include_command_line_api: bool,
    /// Throw on side effect (for safe evaluation)
    #[serde(default)]
    pub throw_on_side_effect: bool,
    /// Disable breaks during evaluation
    #[serde(default)]
    pub disable_breaks: bool,
}

/// REPL evaluation result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplEvaluateResult {
    /// The evaluated result as a RemoteObject
    pub result: RemoteObject,
    /// Whether the expression had side effects
    pub had_side_effects: bool,
    /// Whether the result is from REPL mode transformation
    pub repl_mode: bool,
    /// Suggestions for auto-completion (based on result)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_hints: Option<Vec<CompletionItem>>,
}

impl ReplSession {
    /// Create a new REPL session
    pub fn new() -> Self {
        Self {
            history: Arc::new(RwLock::new(VecDeque::with_capacity(MAX_HISTORY_SIZE))),
            multiline_buffer: Arc::new(RwLock::new(String::new())),
            in_multiline: Arc::new(RwLock::new(false)),
            session_id: uuid::Uuid::new_v4().to_string(),
            eval_counter: Arc::new(AtomicU32::new(0)),
            completion_cache: Arc::new(RwLock::new(Self::default_completions())),
        }
    }

    /// Get the session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Evaluate an expression in REPL mode
    pub fn evaluate(
        &self,
        expression: &str,
        options: &ReplEvaluateOptions,
    ) -> Result<ReplEvaluateResult> {
        let eval_id = self.eval_counter.fetch_add(1, Ordering::SeqCst);
        debug!(
            "REPL evaluate #{} (repl_mode={}): {}",
            eval_id, options.repl_mode, expression
        );

        // Check if this is a continuation of multi-line input
        let full_expression = if *self.in_multiline.read() {
            let mut buffer = self.multiline_buffer.write();
            buffer.push_str(expression);
            buffer.push('\n');

            if Self::is_expression_complete(&buffer) {
                *self.in_multiline.write() = false;
                let expr = buffer.clone();
                buffer.clear();
                expr
            } else {
                return Err(RuntimeDebuggerError::EvaluationError(
                    "MULTILINE_CONTINUE".to_string(),
                ));
            }
        } else if !Self::is_expression_complete(expression) {
            // Start multi-line mode
            *self.in_multiline.write() = true;
            let mut buffer = self.multiline_buffer.write();
            buffer.clear();
            buffer.push_str(expression);
            buffer.push('\n');
            return Err(RuntimeDebuggerError::EvaluationError(
                "MULTILINE_CONTINUE".to_string(),
            ));
        } else {
            expression.to_string()
        };

        // Transform expression for REPL mode if needed
        let transformed = if options.repl_mode {
            Self::transform_for_repl(&full_expression)
        } else {
            full_expression.clone()
        };

        // Mock evaluation
        let (result, had_side_effects) = self.mock_evaluate(&transformed, options)?;

        // Add to history
        let history_entry = HistoryEntry {
            expression: full_expression,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            success: true,
            result_preview: result.description.clone(),
        };
        self.add_to_history(history_entry);

        // Generate completion hints based on result
        let completion_hints = if options.include_command_line_api {
            Some(self.generate_completion_hints(&result))
        } else {
            None
        };

        Ok(ReplEvaluateResult {
            result,
            had_side_effects,
            repl_mode: options.repl_mode,
            completion_hints,
        })
    }

    /// Check if an expression is syntactically complete
    fn is_expression_complete(expression: &str) -> bool {
        let expr = expression.trim();

        // Check for unclosed brackets/braces/parentheses
        let mut brace_count = 0i32;
        let mut bracket_count = 0i32;
        let mut paren_count = 0i32;
        let mut in_string = false;
        let mut string_char = ' ';
        let mut escape_next = false;

        for ch in expr.chars() {
            if escape_next {
                escape_next = false;
                continue;
            }

            if ch == '\\' {
                escape_next = true;
                continue;
            }

            if in_string {
                if ch == string_char {
                    in_string = false;
                }
                continue;
            }

            match ch {
                '"' | '\'' | '`' => {
                    in_string = true;
                    string_char = ch;
                }
                '{' => brace_count += 1,
                '}' => brace_count -= 1,
                '[' => bracket_count += 1,
                ']' => bracket_count -= 1,
                '(' => paren_count += 1,
                ')' => paren_count -= 1,
                _ => {}
            }
        }

        // Expression is complete if all brackets are balanced and we're not in a string
        !in_string && brace_count == 0 && bracket_count == 0 && paren_count == 0
    }

    /// Transform expression for REPL mode
    ///
    /// In REPL mode, expressions like `{a: 1}` should be treated as objects,
    /// not as blocks with labels. We wrap them in parentheses.
    fn transform_for_repl(expression: &str) -> String {
        let trimmed = expression.trim();

        // If it looks like an object literal starting with {, wrap in parens
        if trimmed.starts_with('{') && !trimmed.starts_with("{\"") {
            // Check if it looks like an object literal (has : after first identifier)
            if let Some(colon_pos) = trimmed.find(':') {
                let before_colon = &trimmed[1..colon_pos];
                if before_colon
                    .trim()
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_' || c == ' ')
                {
                    return format!("({})", trimmed);
                }
            }
        }

        expression.to_string()
    }

    /// Mock JavaScript evaluation
    fn mock_evaluate(
        &self,
        expression: &str,
        options: &ReplEvaluateOptions,
    ) -> Result<(RemoteObject, bool)> {
        let expr = expression.trim();
        let preview_depth = options.preview_depth.unwrap_or(DEFAULT_PREVIEW_DEPTH);

        // Try to parse as JSON first
        if let Ok(value) = serde_json::from_str::<Value>(expr) {
            let result = self.value_to_remote_object(&value, options.generate_preview, preview_depth);
            return Ok((result, false));
        }

        // Handle wrapped object literals
        if expr.starts_with('(') && expr.ends_with(')') {
            let inner = &expr[1..expr.len() - 1];
            if let Ok(value) = serde_json::from_str::<Value>(inner) {
                let result =
                    self.value_to_remote_object(&value, options.generate_preview, preview_depth);
                return Ok((result, false));
            }
        }

        // Simple mock evaluation for common expressions
        let (result, side_effects) = match expr {
            "42" => (
                RemoteObject {
                    object_type: RemoteObjectType::Number,
                    subtype: None,
                    class_name: None,
                    value: Some(json!(42)),
                    unserializable_value: None,
                    description: Some("42".to_string()),
                    object_id: None,
                    preview: None,
                },
                false,
            ),
            "true" | "false" => {
                let b = expr == "true";
                (
                    RemoteObject {
                        object_type: RemoteObjectType::Boolean,
                        subtype: None,
                        class_name: None,
                        value: Some(json!(b)),
                        unserializable_value: None,
                        description: Some(expr.to_string()),
                        object_id: None,
                        preview: None,
                    },
                    false,
                )
            }
            "null" => (
                RemoteObject {
                    object_type: RemoteObjectType::Object,
                    subtype: Some(RemoteObjectSubtype::Null),
                    class_name: None,
                    value: Some(Value::Null),
                    unserializable_value: None,
                    description: Some("null".to_string()),
                    object_id: None,
                    preview: None,
                },
                false,
            ),
            "undefined" => (
                RemoteObject {
                    object_type: RemoteObjectType::Undefined,
                    subtype: None,
                    class_name: None,
                    value: None,
                    unserializable_value: Some("undefined".to_string()),
                    description: Some("undefined".to_string()),
                    object_id: None,
                    preview: None,
                },
                false,
            ),
            "NaN" => (
                RemoteObject {
                    object_type: RemoteObjectType::Number,
                    subtype: None,
                    class_name: None,
                    value: None,
                    unserializable_value: Some("NaN".to_string()),
                    description: Some("NaN".to_string()),
                    object_id: None,
                    preview: None,
                },
                false,
            ),
            "Infinity" => (
                RemoteObject {
                    object_type: RemoteObjectType::Number,
                    subtype: None,
                    class_name: None,
                    value: None,
                    unserializable_value: Some("Infinity".to_string()),
                    description: Some("Infinity".to_string()),
                    object_id: None,
                    preview: None,
                },
                false,
            ),
            "-Infinity" => (
                RemoteObject {
                    object_type: RemoteObjectType::Number,
                    subtype: None,
                    class_name: None,
                    value: None,
                    unserializable_value: Some("-Infinity".to_string()),
                    description: Some("-Infinity".to_string()),
                    object_id: None,
                    preview: None,
                },
                false,
            ),
            s if s.starts_with('"') && s.ends_with('"') => {
                let string_val = &s[1..s.len() - 1];
                (
                    RemoteObject {
                        object_type: RemoteObjectType::String,
                        subtype: None,
                        class_name: None,
                        value: Some(json!(string_val)),
                        unserializable_value: None,
                        description: Some(string_val.to_string()),
                        object_id: None,
                        preview: None,
                    },
                    false,
                )
            }
            "1 + 1" => (
                RemoteObject {
                    object_type: RemoteObjectType::Number,
                    subtype: None,
                    class_name: None,
                    value: Some(json!(2)),
                    unserializable_value: None,
                    description: Some("2".to_string()),
                    object_id: None,
                    preview: None,
                },
                false,
            ),
            "2 * 3" => (
                RemoteObject {
                    object_type: RemoteObjectType::Number,
                    subtype: None,
                    class_name: None,
                    value: Some(json!(6)),
                    unserializable_value: None,
                    description: Some("6".to_string()),
                    object_id: None,
                    preview: None,
                },
                false,
            ),
            "console.log" => (
                RemoteObject {
                    object_type: RemoteObjectType::Function,
                    subtype: None,
                    class_name: Some("Function".to_string()),
                    value: None,
                    unserializable_value: None,
                    description: Some("function log() { [native code] }".to_string()),
                    object_id: Some(RemoteObjectId(format!("func-{}", uuid::Uuid::new_v4()))),
                    preview: None,
                },
                false,
            ),
            "new Date()" | "Date.now()" => {
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                (
                    RemoteObject {
                        object_type: RemoteObjectType::Object,
                        subtype: Some(RemoteObjectSubtype::Date),
                        class_name: Some("Date".to_string()),
                        value: None,
                        unserializable_value: None,
                        description: Some(format!("Date {}", timestamp)),
                        object_id: Some(RemoteObjectId(format!("date-{}", uuid::Uuid::new_v4()))),
                        preview: if options.generate_preview {
                            Some(ObjectPreview {
                                object_type: RemoteObjectType::Object,
                                subtype: Some(RemoteObjectSubtype::Date),
                                description: Some(format!("Date {}", timestamp)),
                                overflow: false,
                                properties: vec![],
                            })
                        } else {
                            None
                        },
                    },
                    false,
                )
            }
            "new Map()" => (
                RemoteObject {
                    object_type: RemoteObjectType::Object,
                    subtype: Some(RemoteObjectSubtype::Map),
                    class_name: Some("Map".to_string()),
                    value: None,
                    unserializable_value: None,
                    description: Some("Map(0)".to_string()),
                    object_id: Some(RemoteObjectId(format!("map-{}", uuid::Uuid::new_v4()))),
                    preview: if options.generate_preview {
                        Some(ObjectPreview {
                            object_type: RemoteObjectType::Object,
                            subtype: Some(RemoteObjectSubtype::Map),
                            description: Some("Map(0)".to_string()),
                            overflow: false,
                            properties: vec![],
                        })
                    } else {
                        None
                    },
                },
                false,
            ),
            "new Set()" => (
                RemoteObject {
                    object_type: RemoteObjectType::Object,
                    subtype: Some(RemoteObjectSubtype::Set),
                    class_name: Some("Set".to_string()),
                    value: None,
                    unserializable_value: None,
                    description: Some("Set(0)".to_string()),
                    object_id: Some(RemoteObjectId(format!("set-{}", uuid::Uuid::new_v4()))),
                    preview: if options.generate_preview {
                        Some(ObjectPreview {
                            object_type: RemoteObjectType::Object,
                            subtype: Some(RemoteObjectSubtype::Set),
                            description: Some("Set(0)".to_string()),
                            overflow: false,
                            properties: vec![],
                        })
                    } else {
                        None
                    },
                },
                false,
            ),
            _ => {
                // Try to parse as a number
                if let Ok(n) = expr.parse::<f64>() {
                    (
                        RemoteObject {
                            object_type: RemoteObjectType::Number,
                            subtype: None,
                            class_name: None,
                            value: Some(json!(n)),
                            unserializable_value: None,
                            description: Some(n.to_string()),
                            object_id: None,
                            preview: None,
                        },
                        false,
                    )
                } else {
                    return Err(RuntimeDebuggerError::EvaluationError(format!(
                        "Cannot evaluate: {}",
                        expr
                    )));
                }
            }
        };

        Ok((result, side_effects))
    }

    /// Convert a JSON Value to a RemoteObject with optional preview
    fn value_to_remote_object(
        &self,
        value: &Value,
        generate_preview: bool,
        depth: u32,
    ) -> RemoteObject {
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
                value: Some(Value::Bool(*b)),
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
                description: Some(s.clone()),
                object_id: None,
                preview: None,
            },
            Value::Array(arr) => {
                let object_id = RemoteObjectId(format!("arr-{}", uuid::Uuid::new_v4()));
                let description = format!("Array({})", arr.len());

                let preview = if generate_preview && depth > 0 {
                    Some(self.generate_array_preview(arr, depth - 1))
                } else {
                    None
                };

                RemoteObject {
                    object_type: RemoteObjectType::Object,
                    subtype: Some(RemoteObjectSubtype::Array),
                    class_name: Some("Array".to_string()),
                    value: None,
                    unserializable_value: None,
                    description: Some(description),
                    object_id: Some(object_id),
                    preview,
                }
            }
            Value::Object(obj) => {
                let object_id = RemoteObjectId(format!("obj-{}", uuid::Uuid::new_v4()));
                let description = format!("Object {{{}}}", obj.len());

                let preview = if generate_preview && depth > 0 {
                    Some(self.generate_object_preview(obj, depth - 1))
                } else {
                    None
                };

                RemoteObject {
                    object_type: RemoteObjectType::Object,
                    subtype: None,
                    class_name: Some("Object".to_string()),
                    value: None,
                    unserializable_value: None,
                    description: Some(description),
                    object_id: Some(object_id),
                    preview,
                }
            }
        }
    }

    /// Generate preview for an array
    fn generate_array_preview(&self, arr: &[Value], depth: u32) -> ObjectPreview {
        let properties: Vec<PropertyPreview> = arr
            .iter()
            .take(MAX_PREVIEW_PROPERTIES)
            .enumerate()
            .map(|(i, v)| self.value_to_property_preview(&i.to_string(), v, depth))
            .collect();

        ObjectPreview {
            object_type: RemoteObjectType::Object,
            subtype: Some(RemoteObjectSubtype::Array),
            description: Some(format!("Array({})", arr.len())),
            overflow: arr.len() > MAX_PREVIEW_PROPERTIES,
            properties,
        }
    }

    /// Generate preview for an object
    fn generate_object_preview(
        &self,
        obj: &serde_json::Map<String, Value>,
        depth: u32,
    ) -> ObjectPreview {
        let properties: Vec<PropertyPreview> = obj
            .iter()
            .take(MAX_PREVIEW_PROPERTIES)
            .map(|(k, v)| self.value_to_property_preview(k, v, depth))
            .collect();

        ObjectPreview {
            object_type: RemoteObjectType::Object,
            subtype: None,
            description: Some(format!("Object {{{}}}", obj.len())),
            overflow: obj.len() > MAX_PREVIEW_PROPERTIES,
            properties,
        }
    }

    /// Convert a value to a property preview
    fn value_to_property_preview(&self, name: &str, value: &Value, _depth: u32) -> PropertyPreview {
        match value {
            Value::Null => PropertyPreview {
                name: name.to_string(),
                property_type: RemoteObjectType::Object,
                value: Some("null".to_string()),
                subtype: Some(RemoteObjectSubtype::Null),
            },
            Value::Bool(b) => PropertyPreview {
                name: name.to_string(),
                property_type: RemoteObjectType::Boolean,
                value: Some(b.to_string()),
                subtype: None,
            },
            Value::Number(n) => PropertyPreview {
                name: name.to_string(),
                property_type: RemoteObjectType::Number,
                value: Some(n.to_string()),
                subtype: None,
            },
            Value::String(s) => PropertyPreview {
                name: name.to_string(),
                property_type: RemoteObjectType::String,
                value: Some(format!("\"{}\"", s)),
                subtype: None,
            },
            Value::Array(arr) => PropertyPreview {
                name: name.to_string(),
                property_type: RemoteObjectType::Object,
                value: Some(format!("Array({})", arr.len())),
                subtype: Some(RemoteObjectSubtype::Array),
            },
            Value::Object(obj) => PropertyPreview {
                name: name.to_string(),
                property_type: RemoteObjectType::Object,
                value: Some(format!("{{...}} ({} keys)", obj.len())),
                subtype: None,
            },
        }
    }

    /// Add entry to history
    fn add_to_history(&self, entry: HistoryEntry) {
        let mut history = self.history.write();
        if history.len() >= MAX_HISTORY_SIZE {
            history.pop_front();
        }
        history.push_back(entry);
    }

    /// Get command history
    pub fn get_history(&self, count: Option<usize>) -> Vec<HistoryEntry> {
        let history = self.history.read();
        let count = count.unwrap_or(history.len());
        history.iter().rev().take(count).cloned().collect()
    }

    /// Clear history
    pub fn clear_history(&self) {
        self.history.write().clear();
    }

    /// Get auto-completion suggestions for a partial expression
    pub fn get_completions(&self, partial: &str) -> Vec<CompletionItem> {
        let cache = self.completion_cache.read();
        let partial_lower = partial.to_lowercase();

        cache
            .iter()
            .filter(|item| item.text.to_lowercase().starts_with(&partial_lower))
            .cloned()
            .collect()
    }

    /// Generate completion hints based on an evaluation result
    fn generate_completion_hints(&self, result: &RemoteObject) -> Vec<CompletionItem> {
        let mut hints = Vec::new();

        // Add type-specific completions
        match result.object_type {
            RemoteObjectType::String => {
                hints.extend(Self::string_methods());
            }
            RemoteObjectType::Number => {
                hints.extend(Self::number_methods());
            }
            RemoteObjectType::Object => {
                if let Some(ref subtype) = result.subtype {
                    match subtype {
                        RemoteObjectSubtype::Array => {
                            hints.extend(Self::array_methods());
                        }
                        RemoteObjectSubtype::Map => {
                            hints.extend(Self::map_methods());
                        }
                        RemoteObjectSubtype::Set => {
                            hints.extend(Self::set_methods());
                        }
                        RemoteObjectSubtype::Date => {
                            hints.extend(Self::date_methods());
                        }
                        _ => {
                            hints.extend(Self::object_methods());
                        }
                    }
                } else {
                    hints.extend(Self::object_methods());
                }
            }
            _ => {}
        }

        hints
    }

    /// Default completions (global objects and keywords)
    fn default_completions() -> Vec<CompletionItem> {
        vec![
            CompletionItem {
                text: "console".to_string(),
                label: "console".to_string(),
                kind: CompletionKind::Variable,
                documentation: Some("Console API for logging".to_string()),
            },
            CompletionItem {
                text: "console.log".to_string(),
                label: "console.log()".to_string(),
                kind: CompletionKind::Method,
                documentation: Some("Log message to console".to_string()),
            },
            CompletionItem {
                text: "console.error".to_string(),
                label: "console.error()".to_string(),
                kind: CompletionKind::Method,
                documentation: Some("Log error to console".to_string()),
            },
            CompletionItem {
                text: "JSON".to_string(),
                label: "JSON".to_string(),
                kind: CompletionKind::Class,
                documentation: Some("JSON parsing and stringification".to_string()),
            },
            CompletionItem {
                text: "JSON.parse".to_string(),
                label: "JSON.parse()".to_string(),
                kind: CompletionKind::Method,
                documentation: Some("Parse JSON string".to_string()),
            },
            CompletionItem {
                text: "JSON.stringify".to_string(),
                label: "JSON.stringify()".to_string(),
                kind: CompletionKind::Method,
                documentation: Some("Convert to JSON string".to_string()),
            },
            CompletionItem {
                text: "Math".to_string(),
                label: "Math".to_string(),
                kind: CompletionKind::Class,
                documentation: Some("Mathematical functions".to_string()),
            },
            CompletionItem {
                text: "Date".to_string(),
                label: "Date".to_string(),
                kind: CompletionKind::Class,
                documentation: Some("Date and time".to_string()),
            },
            CompletionItem {
                text: "Array".to_string(),
                label: "Array".to_string(),
                kind: CompletionKind::Class,
                documentation: Some("Array constructor".to_string()),
            },
            CompletionItem {
                text: "Object".to_string(),
                label: "Object".to_string(),
                kind: CompletionKind::Class,
                documentation: Some("Object constructor".to_string()),
            },
            CompletionItem {
                text: "let".to_string(),
                label: "let".to_string(),
                kind: CompletionKind::Keyword,
                documentation: Some("Declare block-scoped variable".to_string()),
            },
            CompletionItem {
                text: "const".to_string(),
                label: "const".to_string(),
                kind: CompletionKind::Keyword,
                documentation: Some("Declare constant".to_string()),
            },
            CompletionItem {
                text: "function".to_string(),
                label: "function".to_string(),
                kind: CompletionKind::Keyword,
                documentation: Some("Declare function".to_string()),
            },
        ]
    }

    /// String methods for completion
    fn string_methods() -> Vec<CompletionItem> {
        vec![
            CompletionItem {
                text: ".length".to_string(),
                label: ".length".to_string(),
                kind: CompletionKind::Property,
                documentation: Some("String length".to_string()),
            },
            CompletionItem {
                text: ".split".to_string(),
                label: ".split()".to_string(),
                kind: CompletionKind::Method,
                documentation: Some("Split string".to_string()),
            },
            CompletionItem {
                text: ".trim".to_string(),
                label: ".trim()".to_string(),
                kind: CompletionKind::Method,
                documentation: Some("Trim whitespace".to_string()),
            },
            CompletionItem {
                text: ".toUpperCase".to_string(),
                label: ".toUpperCase()".to_string(),
                kind: CompletionKind::Method,
                documentation: Some("Convert to uppercase".to_string()),
            },
            CompletionItem {
                text: ".toLowerCase".to_string(),
                label: ".toLowerCase()".to_string(),
                kind: CompletionKind::Method,
                documentation: Some("Convert to lowercase".to_string()),
            },
        ]
    }

    /// Number methods for completion
    fn number_methods() -> Vec<CompletionItem> {
        vec![
            CompletionItem {
                text: ".toFixed".to_string(),
                label: ".toFixed()".to_string(),
                kind: CompletionKind::Method,
                documentation: Some("Format with fixed decimals".to_string()),
            },
            CompletionItem {
                text: ".toString".to_string(),
                label: ".toString()".to_string(),
                kind: CompletionKind::Method,
                documentation: Some("Convert to string".to_string()),
            },
        ]
    }

    /// Array methods for completion
    fn array_methods() -> Vec<CompletionItem> {
        vec![
            CompletionItem {
                text: ".length".to_string(),
                label: ".length".to_string(),
                kind: CompletionKind::Property,
                documentation: Some("Array length".to_string()),
            },
            CompletionItem {
                text: ".push".to_string(),
                label: ".push()".to_string(),
                kind: CompletionKind::Method,
                documentation: Some("Add to end".to_string()),
            },
            CompletionItem {
                text: ".pop".to_string(),
                label: ".pop()".to_string(),
                kind: CompletionKind::Method,
                documentation: Some("Remove from end".to_string()),
            },
            CompletionItem {
                text: ".map".to_string(),
                label: ".map()".to_string(),
                kind: CompletionKind::Method,
                documentation: Some("Map elements".to_string()),
            },
            CompletionItem {
                text: ".filter".to_string(),
                label: ".filter()".to_string(),
                kind: CompletionKind::Method,
                documentation: Some("Filter elements".to_string()),
            },
            CompletionItem {
                text: ".reduce".to_string(),
                label: ".reduce()".to_string(),
                kind: CompletionKind::Method,
                documentation: Some("Reduce to single value".to_string()),
            },
        ]
    }

    /// Object methods for completion
    fn object_methods() -> Vec<CompletionItem> {
        vec![
            CompletionItem {
                text: ".keys".to_string(),
                label: "Object.keys()".to_string(),
                kind: CompletionKind::Method,
                documentation: Some("Get object keys".to_string()),
            },
            CompletionItem {
                text: ".values".to_string(),
                label: "Object.values()".to_string(),
                kind: CompletionKind::Method,
                documentation: Some("Get object values".to_string()),
            },
            CompletionItem {
                text: ".entries".to_string(),
                label: "Object.entries()".to_string(),
                kind: CompletionKind::Method,
                documentation: Some("Get key-value pairs".to_string()),
            },
        ]
    }

    /// Map methods for completion
    fn map_methods() -> Vec<CompletionItem> {
        vec![
            CompletionItem {
                text: ".size".to_string(),
                label: ".size".to_string(),
                kind: CompletionKind::Property,
                documentation: Some("Map size".to_string()),
            },
            CompletionItem {
                text: ".get".to_string(),
                label: ".get()".to_string(),
                kind: CompletionKind::Method,
                documentation: Some("Get value by key".to_string()),
            },
            CompletionItem {
                text: ".set".to_string(),
                label: ".set()".to_string(),
                kind: CompletionKind::Method,
                documentation: Some("Set key-value pair".to_string()),
            },
            CompletionItem {
                text: ".has".to_string(),
                label: ".has()".to_string(),
                kind: CompletionKind::Method,
                documentation: Some("Check if key exists".to_string()),
            },
            CompletionItem {
                text: ".delete".to_string(),
                label: ".delete()".to_string(),
                kind: CompletionKind::Method,
                documentation: Some("Delete by key".to_string()),
            },
        ]
    }

    /// Set methods for completion
    fn set_methods() -> Vec<CompletionItem> {
        vec![
            CompletionItem {
                text: ".size".to_string(),
                label: ".size".to_string(),
                kind: CompletionKind::Property,
                documentation: Some("Set size".to_string()),
            },
            CompletionItem {
                text: ".add".to_string(),
                label: ".add()".to_string(),
                kind: CompletionKind::Method,
                documentation: Some("Add value".to_string()),
            },
            CompletionItem {
                text: ".has".to_string(),
                label: ".has()".to_string(),
                kind: CompletionKind::Method,
                documentation: Some("Check if value exists".to_string()),
            },
            CompletionItem {
                text: ".delete".to_string(),
                label: ".delete()".to_string(),
                kind: CompletionKind::Method,
                documentation: Some("Delete value".to_string()),
            },
        ]
    }

    /// Date methods for completion
    fn date_methods() -> Vec<CompletionItem> {
        vec![
            CompletionItem {
                text: ".getTime".to_string(),
                label: ".getTime()".to_string(),
                kind: CompletionKind::Method,
                documentation: Some("Get timestamp".to_string()),
            },
            CompletionItem {
                text: ".toISOString".to_string(),
                label: ".toISOString()".to_string(),
                kind: CompletionKind::Method,
                documentation: Some("Get ISO string".to_string()),
            },
            CompletionItem {
                text: ".getFullYear".to_string(),
                label: ".getFullYear()".to_string(),
                kind: CompletionKind::Method,
                documentation: Some("Get year".to_string()),
            },
            CompletionItem {
                text: ".getMonth".to_string(),
                label: ".getMonth()".to_string(),
                kind: CompletionKind::Method,
                documentation: Some("Get month (0-11)".to_string()),
            },
        ]
    }

    /// Cancel multi-line input
    pub fn cancel_multiline(&self) {
        *self.in_multiline.write() = false;
        self.multiline_buffer.write().clear();
    }

    /// Check if in multi-line mode
    pub fn is_multiline(&self) -> bool {
        *self.in_multiline.read()
    }
}

impl Default for ReplSession {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repl_session_new() {
        let session = ReplSession::new();
        assert!(!session.session_id().is_empty());
        assert!(!session.is_multiline());
    }

    #[test]
    fn test_repl_evaluate_simple() {
        let session = ReplSession::new();
        let options = ReplEvaluateOptions::default();

        let result = session.evaluate("42", &options);
        assert!(result.is_ok());

        let result = result.unwrap();
        assert_eq!(result.result.object_type, RemoteObjectType::Number);
        assert_eq!(result.result.value, Some(json!(42)));
    }

    #[test]
    fn test_repl_evaluate_string() {
        let session = ReplSession::new();
        let options = ReplEvaluateOptions::default();

        let result = session.evaluate("\"hello\"", &options);
        assert!(result.is_ok());

        let result = result.unwrap();
        assert_eq!(result.result.object_type, RemoteObjectType::String);
        assert_eq!(result.result.value, Some(json!("hello")));
    }

    #[test]
    fn test_repl_evaluate_json_object() {
        let session = ReplSession::new();
        let options = ReplEvaluateOptions {
            generate_preview: true,
            ..Default::default()
        };

        let result = session.evaluate(r#"{"a": 1, "b": 2}"#, &options);
        assert!(result.is_ok());

        let result = result.unwrap();
        assert_eq!(result.result.object_type, RemoteObjectType::Object);
        assert!(result.result.object_id.is_some());
        assert!(result.result.preview.is_some());
    }

    #[test]
    fn test_repl_evaluate_array() {
        let session = ReplSession::new();
        let options = ReplEvaluateOptions {
            generate_preview: true,
            ..Default::default()
        };

        let result = session.evaluate("[1, 2, 3]", &options);
        assert!(result.is_ok());

        let result = result.unwrap();
        assert_eq!(result.result.object_type, RemoteObjectType::Object);
        assert_eq!(result.result.subtype, Some(RemoteObjectSubtype::Array));
        assert!(result.result.preview.is_some());

        let preview = result.result.preview.unwrap();
        assert_eq!(preview.properties.len(), 3);
    }

    #[test]
    fn test_repl_mode_object_literal() {
        let session = ReplSession::new();
        let options = ReplEvaluateOptions {
            repl_mode: true,
            generate_preview: true,
            ..Default::default()
        };

        // In REPL mode with JSON syntax for mock evaluator
        // Note: Real JS engine would handle {a: 1}, mock requires JSON
        let result = session.evaluate(r#"({"a": 1})"#, &options);
        assert!(result.is_ok());
        assert!(result.unwrap().repl_mode);
    }

    #[test]
    fn test_repl_history() {
        let session = ReplSession::new();
        let options = ReplEvaluateOptions::default();

        session.evaluate("42", &options).unwrap();
        session.evaluate("true", &options).unwrap();

        let history = session.get_history(None);
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].expression, "true");
        assert_eq!(history[1].expression, "42");
    }

    #[test]
    fn test_repl_clear_history() {
        let session = ReplSession::new();
        let options = ReplEvaluateOptions::default();

        session.evaluate("42", &options).unwrap();
        assert_eq!(session.get_history(None).len(), 1);

        session.clear_history();
        assert_eq!(session.get_history(None).len(), 0);
    }

    #[test]
    fn test_repl_completions() {
        let session = ReplSession::new();

        let completions = session.get_completions("con");
        assert!(!completions.is_empty());
        assert!(completions.iter().any(|c| c.text.starts_with("console")));
    }

    #[test]
    fn test_expression_complete() {
        assert!(ReplSession::is_expression_complete("42"));
        assert!(ReplSession::is_expression_complete("{a: 1}"));
        assert!(ReplSession::is_expression_complete("[1, 2, 3]"));
        assert!(ReplSession::is_expression_complete("\"hello\""));

        // Incomplete expressions
        assert!(!ReplSession::is_expression_complete("{a: 1"));
        assert!(!ReplSession::is_expression_complete("[1, 2"));
        assert!(!ReplSession::is_expression_complete("\"hello"));
        assert!(!ReplSession::is_expression_complete("function() {"));
    }

    #[test]
    fn test_multiline_detection() {
        let session = ReplSession::new();
        let options = ReplEvaluateOptions::default();

        // Start incomplete expression
        let result = session.evaluate("{", &options);
        assert!(result.is_err());
        assert!(session.is_multiline());

        // Cancel multiline
        session.cancel_multiline();
        assert!(!session.is_multiline());
    }

    #[test]
    fn test_special_values() {
        let session = ReplSession::new();
        let options = ReplEvaluateOptions::default();

        // NaN
        let result = session.evaluate("NaN", &options).unwrap();
        assert_eq!(
            result.result.unserializable_value,
            Some("NaN".to_string())
        );

        // Infinity
        let result = session.evaluate("Infinity", &options).unwrap();
        assert_eq!(
            result.result.unserializable_value,
            Some("Infinity".to_string())
        );

        // undefined
        let result = session.evaluate("undefined", &options).unwrap();
        assert_eq!(result.result.object_type, RemoteObjectType::Undefined);
    }

    #[test]
    fn test_date_evaluation() {
        let session = ReplSession::new();
        let options = ReplEvaluateOptions {
            generate_preview: true,
            ..Default::default()
        };

        let result = session.evaluate("new Date()", &options).unwrap();
        assert_eq!(result.result.subtype, Some(RemoteObjectSubtype::Date));
        assert!(result.result.preview.is_some());
    }

    #[test]
    fn test_map_evaluation() {
        let session = ReplSession::new();
        let options = ReplEvaluateOptions {
            generate_preview: true,
            ..Default::default()
        };

        let result = session.evaluate("new Map()", &options).unwrap();
        assert_eq!(result.result.subtype, Some(RemoteObjectSubtype::Map));
    }

    #[test]
    fn test_set_evaluation() {
        let session = ReplSession::new();
        let options = ReplEvaluateOptions {
            generate_preview: true,
            ..Default::default()
        };

        let result = session.evaluate("new Set()", &options).unwrap();
        assert_eq!(result.result.subtype, Some(RemoteObjectSubtype::Set));
    }

    #[test]
    fn test_function_evaluation() {
        let session = ReplSession::new();
        let options = ReplEvaluateOptions::default();

        let result = session.evaluate("console.log", &options).unwrap();
        assert_eq!(result.result.object_type, RemoteObjectType::Function);
        assert!(result.result.object_id.is_some());
    }

    #[test]
    fn test_completion_hints() {
        let session = ReplSession::new();
        let options = ReplEvaluateOptions {
            include_command_line_api: true,
            ..Default::default()
        };

        let result = session.evaluate("[1, 2, 3]", &options).unwrap();
        assert!(result.completion_hints.is_some());

        let hints = result.completion_hints.unwrap();
        assert!(hints.iter().any(|h| h.text == ".map"));
    }
}
