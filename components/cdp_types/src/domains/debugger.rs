// Debugger domain types

use super::runtime::RemoteObject;
use serde::{Deserialize, Serialize};

/// Breakpoint identifier
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct BreakpointId(pub String);

/// Script identifier
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ScriptId(pub String);

/// Location in source code
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Location {
    /// Script identifier
    pub script_id: ScriptId,
    /// Line number (0-based)
    pub line_number: u32,
    /// Column number (0-based, optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column_number: Option<u32>,
}

/// Scope type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ScopeType {
    Global,
    Local,
    With,
    Closure,
    Catch,
    Block,
    Script,
    Eval,
    Module,
}

/// Scope description
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Scope {
    /// Scope type
    #[serde(rename = "type")]
    pub scope_type: ScopeType,
    /// Object representing the scope
    pub object: RemoteObject,
    /// Scope name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Location where scope was defined
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_location: Option<Location>,
    /// Location where scope ends
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_location: Option<Location>,
}

/// Call frame in the call stack
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CallFrame {
    /// Call frame identifier
    pub call_frame_id: String,
    /// Function name
    pub function_name: String,
    /// Location in source code
    pub location: Location,
    /// Script URL
    pub url: String,
    /// Scope chain
    pub scope_chain: Vec<Scope>,
    /// 'this' object
    pub this: RemoteObject,
    /// Return value (if function is at return point)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_value: Option<RemoteObject>,
}

/// Parameters for Debugger.setBreakpoint
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SetBreakpointParams {
    /// Location to set breakpoint at
    pub location: Location,
    /// Breakpoint condition (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
}

/// Response for Debugger.setBreakpoint
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SetBreakpointResponse {
    /// Breakpoint identifier
    pub breakpoint_id: BreakpointId,
    /// Actual location where breakpoint was set
    pub actual_location: Location,
}

/// Parameters for Debugger.evaluateOnCallFrame
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateOnCallFrameParams {
    /// Call frame ID to evaluate on
    pub call_frame_id: String,
    /// Expression to evaluate
    pub expression: String,
}

/// Response for Debugger.evaluateOnCallFrame
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateOnCallFrameResponse {
    /// Evaluation result
    pub result: RemoteObject,
    /// Exception details (if thrown)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exception_details: Option<super::runtime::ExceptionDetails>,
}

/// Paused event reason
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PausedReason {
    Ambiguous,
    Assert,
    Debugcommand,
    Dom,
    Eventlistener,
    Exception,
    Instrumentation,
    Oom,
    Other,
    Promiserejection,
    Xhr,
    #[serde(rename = "breakpoint")]
    Breakpoint,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_breakpoint_id() {
        let id = BreakpointId("bp-123".to_string());
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"bp-123\"");
    }

    #[test]
    fn test_script_id() {
        let id = ScriptId("script-456".to_string());
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"script-456\"");
    }

    #[test]
    fn test_location() {
        let loc = Location {
            script_id: ScriptId("1".to_string()),
            line_number: 10,
            column_number: Some(5),
        };

        let json = serde_json::to_string(&loc).unwrap();
        assert!(json.contains("\"scriptId\":\"1\""));
        assert!(json.contains("\"lineNumber\":10"));
    }
}
