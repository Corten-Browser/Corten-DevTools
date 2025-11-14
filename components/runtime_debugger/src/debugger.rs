//! Debugger domain implementation for JavaScript debugging
//!
//! Handles breakpoint management, stepping controls, and call frame inspection.

use async_trait::async_trait;
use cdp_types::domains::debugger::*;
use cdp_types::domains::runtime::{RemoteObject, RemoteObjectId, RemoteObjectType};
use cdp_types::CdpError;
use dashmap::DashMap;
use parking_lot::RwLock;
use protocol_handler::DomainHandler;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::{Result, RuntimeDebuggerError};

/// Debugger domain handler for JavaScript debugging
pub struct DebuggerDomain {
    /// Map of breakpoint IDs to breakpoints
    breakpoints: Arc<DashMap<BreakpointId, Breakpoint>>,
    /// Current call frames (when paused)
    call_frames: Arc<RwLock<Vec<CallFrame>>>,
    /// Enabled state
    enabled: Arc<AtomicBool>,
    /// Paused state
    paused: Arc<AtomicBool>,
    /// Breakpoint ID counter
    breakpoint_counter: Arc<AtomicU32>,
}

/// Breakpoint information
#[derive(Debug, Clone)]
pub struct Breakpoint {
    /// Breakpoint ID
    pub id: BreakpointId,
    /// Location
    pub location: Location,
    /// Optional condition
    pub condition: Option<String>,
    /// Hit count
    pub hit_count: u32,
}

impl DebuggerDomain {
    /// Create a new DebuggerDomain
    pub fn new() -> Self {
        Self {
            breakpoints: Arc::new(DashMap::new()),
            call_frames: Arc::new(RwLock::new(Vec::new())),
            enabled: Arc::new(AtomicBool::new(false)),
            paused: Arc::new(AtomicBool::new(false)),
            breakpoint_counter: Arc::new(AtomicU32::new(1)),
        }
    }

    /// Enable the Debugger domain
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::SeqCst);
        debug!("Debugger domain enabled");
    }

    /// Disable the Debugger domain
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::SeqCst);
        self.paused.store(false, Ordering::SeqCst);
        self.breakpoints.clear();
        self.call_frames.write().clear();
        debug!("Debugger domain disabled");
    }

    /// Check if debugger is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    /// Check if debugger is paused
    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }

    /// Set a breakpoint at the specified location
    pub fn set_breakpoint(
        &self,
        location: Location,
        condition: Option<String>,
    ) -> Result<SetBreakpointResponse> {
        if !self.is_enabled() {
            return Err(RuntimeDebuggerError::DebuggerNotEnabled);
        }

        let id = self.breakpoint_counter.fetch_add(1, Ordering::SeqCst);
        let breakpoint_id = BreakpointId(format!("bp-{}", id));

        let breakpoint = Breakpoint {
            id: breakpoint_id.clone(),
            location: location.clone(),
            condition,
            hit_count: 0,
        };

        self.breakpoints.insert(breakpoint_id.clone(), breakpoint);

        debug!("Set breakpoint {:?} at {:?}", breakpoint_id, location);

        Ok(SetBreakpointResponse {
            breakpoint_id,
            actual_location: location,
        })
    }

    /// Remove a breakpoint
    pub fn remove_breakpoint(&self, breakpoint_id: &BreakpointId) -> Result<()> {
        if !self.is_enabled() {
            return Err(RuntimeDebuggerError::DebuggerNotEnabled);
        }

        self.breakpoints
            .remove(breakpoint_id)
            .ok_or_else(|| RuntimeDebuggerError::BreakpointNotFound(breakpoint_id.0.clone()))?;

        debug!("Removed breakpoint {:?}", breakpoint_id);
        Ok(())
    }

    /// Get all breakpoints
    pub fn get_breakpoints(&self) -> Vec<Breakpoint> {
        self.breakpoints
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Step over (next line, not entering functions)
    pub fn step_over(&self) -> Result<()> {
        if !self.is_enabled() {
            return Err(RuntimeDebuggerError::DebuggerNotEnabled);
        }

        if !self.is_paused() {
            return Err(RuntimeDebuggerError::DebuggerNotPaused);
        }

        debug!("Step over");
        // Mock implementation - in real version this would interact with JS engine
        self.paused.store(false, Ordering::SeqCst);
        Ok(())
    }

    /// Step into (enter function calls)
    pub fn step_into(&self) -> Result<()> {
        if !self.is_enabled() {
            return Err(RuntimeDebuggerError::DebuggerNotEnabled);
        }

        if !self.is_paused() {
            return Err(RuntimeDebuggerError::DebuggerNotPaused);
        }

        debug!("Step into");
        // Mock implementation
        self.paused.store(false, Ordering::SeqCst);
        Ok(())
    }

    /// Step out (finish current function)
    pub fn step_out(&self) -> Result<()> {
        if !self.is_enabled() {
            return Err(RuntimeDebuggerError::DebuggerNotEnabled);
        }

        if !self.is_paused() {
            return Err(RuntimeDebuggerError::DebuggerNotPaused);
        }

        debug!("Step out");
        // Mock implementation
        self.paused.store(false, Ordering::SeqCst);
        Ok(())
    }

    /// Resume execution
    pub fn resume(&self) -> Result<()> {
        if !self.is_enabled() {
            return Err(RuntimeDebuggerError::DebuggerNotEnabled);
        }

        if !self.is_paused() {
            return Err(RuntimeDebuggerError::DebuggerNotPaused);
        }

        debug!("Resume execution");
        self.paused.store(false, Ordering::SeqCst);
        self.call_frames.write().clear();
        Ok(())
    }

    /// Pause execution
    pub fn pause(&self) -> Result<()> {
        if !self.is_enabled() {
            return Err(RuntimeDebuggerError::DebuggerNotEnabled);
        }

        debug!("Pause execution");
        self.paused.store(true, Ordering::SeqCst);
        // Mock call frames
        self.create_mock_call_frames();
        Ok(())
    }

    /// Evaluate expression on a call frame
    pub fn evaluate_on_call_frame(
        &self,
        call_frame_id: &str,
        expression: &str,
    ) -> Result<EvaluateOnCallFrameResponse> {
        if !self.is_enabled() {
            return Err(RuntimeDebuggerError::DebuggerNotEnabled);
        }

        if !self.is_paused() {
            return Err(RuntimeDebuggerError::DebuggerNotPaused);
        }

        let call_frames = self.call_frames.read();
        let _frame = call_frames
            .iter()
            .find(|f| f.call_frame_id == call_frame_id)
            .ok_or_else(|| RuntimeDebuggerError::CallFrameNotFound(call_frame_id.to_string()))?;

        debug!("Evaluating on call frame {}: {}", call_frame_id, expression);

        // Mock evaluation result
        let result = self.mock_evaluate(expression);

        Ok(EvaluateOnCallFrameResponse {
            result,
            exception_details: None,
        })
    }

    /// Get current call frames
    pub fn get_call_frames(&self) -> Vec<CallFrame> {
        self.call_frames.read().clone()
    }

    /// Create mock call frames for testing
    fn create_mock_call_frames(&self) {
        let mut frames = self.call_frames.write();
        frames.clear();

        // Create a mock call frame
        frames.push(CallFrame {
            call_frame_id: "frame-0".to_string(),
            function_name: "main".to_string(),
            location: Location {
                script_id: ScriptId("script-1".to_string()),
                line_number: 10,
                column_number: Some(5),
            },
            url: "file:///test.js".to_string(),
            scope_chain: vec![],
            this: RemoteObject {
                object_type: RemoteObjectType::Object,
                subtype: None,
                class_name: Some("global".to_string()),
                value: None,
                unserializable_value: None,
                description: Some("global".to_string()),
                object_id: Some(RemoteObjectId(format!("obj-{}", Uuid::new_v4()))),
                preview: None,
            },
            return_value: None,
        });
    }

    /// Mock evaluation for testing
    fn mock_evaluate(&self, expression: &str) -> RemoteObject {
        match expression.trim() {
            "42" => RemoteObject {
                object_type: RemoteObjectType::Number,
                subtype: None,
                class_name: None,
                value: Some(json!(42)),
                unserializable_value: None,
                description: Some("42".to_string()),
                object_id: None,
                preview: None,
            },
            "true" => RemoteObject {
                object_type: RemoteObjectType::Boolean,
                subtype: None,
                class_name: None,
                value: Some(json!(true)),
                unserializable_value: None,
                description: Some("true".to_string()),
                object_id: None,
                preview: None,
            },
            _ => RemoteObject {
                object_type: RemoteObjectType::Undefined,
                subtype: None,
                class_name: None,
                value: None,
                unserializable_value: Some("undefined".to_string()),
                description: Some("undefined".to_string()),
                object_id: None,
                preview: None,
            },
        }
    }
}

impl Default for DebuggerDomain {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DomainHandler for DebuggerDomain {
    fn name(&self) -> &str {
        "Debugger"
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
            "setBreakpoint" => {
                let params = params.ok_or_else(|| CdpError::invalid_params("Missing params"))?;
                let location: Location = serde_json::from_value(
                    params
                        .get("location")
                        .cloned()
                        .ok_or_else(|| CdpError::invalid_params("Missing location"))?,
                )
                .map_err(|e| CdpError::invalid_params(e.to_string()))?;

                let condition = params
                    .get("condition")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let response = self
                    .set_breakpoint(location, condition)
                    .map_err(|e| CdpError::internal_error(e.to_string()))?;

                Ok(serde_json::to_value(response)
                    .map_err(|e| CdpError::internal_error(e.to_string()))?)
            }
            "removeBreakpoint" => {
                let params = params.ok_or_else(|| CdpError::invalid_params("Missing params"))?;
                let breakpoint_id_str = params
                    .get("breakpointId")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| CdpError::invalid_params("Missing breakpointId"))?;
                let breakpoint_id = BreakpointId(breakpoint_id_str.to_string());

                self.remove_breakpoint(&breakpoint_id)
                    .map_err(|e| CdpError::internal_error(e.to_string()))?;

                Ok(json!({}))
            }
            "stepOver" => {
                self.step_over()
                    .map_err(|e| CdpError::internal_error(e.to_string()))?;
                Ok(json!({}))
            }
            "stepInto" => {
                self.step_into()
                    .map_err(|e| CdpError::internal_error(e.to_string()))?;
                Ok(json!({}))
            }
            "stepOut" => {
                self.step_out()
                    .map_err(|e| CdpError::internal_error(e.to_string()))?;
                Ok(json!({}))
            }
            "resume" => {
                self.resume()
                    .map_err(|e| CdpError::internal_error(e.to_string()))?;
                Ok(json!({}))
            }
            "pause" => {
                self.pause()
                    .map_err(|e| CdpError::internal_error(e.to_string()))?;
                Ok(json!({}))
            }
            "evaluateOnCallFrame" => {
                let params = params.ok_or_else(|| CdpError::invalid_params("Missing params"))?;
                let call_frame_id = params
                    .get("callFrameId")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| CdpError::invalid_params("Missing callFrameId"))?;

                let expression = params
                    .get("expression")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| CdpError::invalid_params("Missing expression"))?;

                let response = self
                    .evaluate_on_call_frame(call_frame_id, expression)
                    .map_err(|e| CdpError::internal_error(e.to_string()))?;

                Ok(serde_json::to_value(response)
                    .map_err(|e| CdpError::internal_error(e.to_string()))?)
            }
            _ => {
                warn!("Unknown Debugger method: {}", method);
                Err(CdpError::method_not_found(format!("Debugger.{}", method)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debugger_domain_new() {
        let debugger = DebuggerDomain::new();
        assert!(!debugger.is_enabled());
        assert!(!debugger.is_paused());
        assert_eq!(debugger.breakpoints.len(), 0);
    }

    #[test]
    fn test_debugger_enable_disable() {
        let debugger = DebuggerDomain::new();

        assert!(!debugger.is_enabled());

        debugger.enable();
        assert!(debugger.is_enabled());

        debugger.disable();
        assert!(!debugger.is_enabled());
    }

    #[test]
    fn test_set_breakpoint() {
        let debugger = DebuggerDomain::new();
        debugger.enable();

        let location = Location {
            script_id: ScriptId("script-1".to_string()),
            line_number: 10,
            column_number: Some(5),
        };

        let response = debugger.set_breakpoint(location.clone(), None);
        assert!(response.is_ok());

        let response = response.unwrap();
        assert_eq!(response.actual_location.script_id, location.script_id);
        assert_eq!(response.actual_location.line_number, location.line_number);
        assert_eq!(debugger.breakpoints.len(), 1);
    }

    #[test]
    fn test_set_breakpoint_not_enabled() {
        let debugger = DebuggerDomain::new();

        let location = Location {
            script_id: ScriptId("script-1".to_string()),
            line_number: 10,
            column_number: Some(5),
        };

        let result = debugger.set_breakpoint(location, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_breakpoint() {
        let debugger = DebuggerDomain::new();
        debugger.enable();

        let location = Location {
            script_id: ScriptId("script-1".to_string()),
            line_number: 10,
            column_number: None,
        };

        let response = debugger.set_breakpoint(location, None).unwrap();
        let breakpoint_id = response.breakpoint_id;

        assert_eq!(debugger.breakpoints.len(), 1);

        let result = debugger.remove_breakpoint(&breakpoint_id);
        assert!(result.is_ok());
        assert_eq!(debugger.breakpoints.len(), 0);
    }

    #[test]
    fn test_remove_nonexistent_breakpoint() {
        let debugger = DebuggerDomain::new();
        debugger.enable();

        let breakpoint_id = BreakpointId("nonexistent".to_string());
        let result = debugger.remove_breakpoint(&breakpoint_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_pause_resume() {
        let debugger = DebuggerDomain::new();
        debugger.enable();

        assert!(!debugger.is_paused());

        let result = debugger.pause();
        assert!(result.is_ok());
        assert!(debugger.is_paused());
        assert!(!debugger.get_call_frames().is_empty());

        let result = debugger.resume();
        assert!(result.is_ok());
        assert!(!debugger.is_paused());
        assert!(debugger.get_call_frames().is_empty());
    }

    #[test]
    fn test_step_over() {
        let debugger = DebuggerDomain::new();
        debugger.enable();
        debugger.pause().unwrap();

        assert!(debugger.is_paused());

        let result = debugger.step_over();
        assert!(result.is_ok());
        assert!(!debugger.is_paused());
    }

    #[test]
    fn test_step_into() {
        let debugger = DebuggerDomain::new();
        debugger.enable();
        debugger.pause().unwrap();

        let result = debugger.step_into();
        assert!(result.is_ok());
        assert!(!debugger.is_paused());
    }

    #[test]
    fn test_step_out() {
        let debugger = DebuggerDomain::new();
        debugger.enable();
        debugger.pause().unwrap();

        let result = debugger.step_out();
        assert!(result.is_ok());
        assert!(!debugger.is_paused());
    }

    #[test]
    fn test_step_not_paused() {
        let debugger = DebuggerDomain::new();
        debugger.enable();

        let result = debugger.step_over();
        assert!(result.is_err());
    }

    #[test]
    fn test_step_not_enabled() {
        let debugger = DebuggerDomain::new();

        let result = debugger.step_over();
        assert!(result.is_err());
    }

    #[test]
    fn test_evaluate_on_call_frame() {
        let debugger = DebuggerDomain::new();
        debugger.enable();
        debugger.pause().unwrap();

        let call_frames = debugger.get_call_frames();
        assert!(!call_frames.is_empty());

        let frame_id = &call_frames[0].call_frame_id;
        let response = debugger.evaluate_on_call_frame(frame_id, "42");

        assert!(response.is_ok());
        let response = response.unwrap();
        assert_eq!(response.result.object_type, RemoteObjectType::Number);
        assert_eq!(response.result.value, Some(json!(42)));
    }

    #[test]
    fn test_evaluate_on_nonexistent_frame() {
        let debugger = DebuggerDomain::new();
        debugger.enable();
        debugger.pause().unwrap();

        let result = debugger.evaluate_on_call_frame("nonexistent", "42");
        assert!(result.is_err());
    }

    #[test]
    fn test_evaluate_not_paused() {
        let debugger = DebuggerDomain::new();
        debugger.enable();

        let result = debugger.evaluate_on_call_frame("frame-0", "42");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_breakpoints() {
        let debugger = DebuggerDomain::new();
        debugger.enable();

        let location1 = Location {
            script_id: ScriptId("script-1".to_string()),
            line_number: 10,
            column_number: None,
        };

        let location2 = Location {
            script_id: ScriptId("script-1".to_string()),
            line_number: 20,
            column_number: None,
        };

        debugger.set_breakpoint(location1, None).unwrap();
        debugger.set_breakpoint(location2, None).unwrap();

        let breakpoints = debugger.get_breakpoints();
        assert_eq!(breakpoints.len(), 2);
    }

    #[tokio::test]
    async fn test_domain_handler_name() {
        let debugger = DebuggerDomain::new();
        assert_eq!(debugger.name(), "Debugger");
    }

    #[tokio::test]
    async fn test_domain_handler_enable() {
        let debugger = DebuggerDomain::new();
        let result = debugger.handle_method("enable", None).await;
        assert!(result.is_ok());
        assert!(debugger.is_enabled());
    }

    #[tokio::test]
    async fn test_domain_handler_disable() {
        let debugger = DebuggerDomain::new();
        debugger.enable();

        let result = debugger.handle_method("disable", None).await;
        assert!(result.is_ok());
        assert!(!debugger.is_enabled());
    }

    #[tokio::test]
    async fn test_domain_handler_set_breakpoint() {
        let debugger = DebuggerDomain::new();
        debugger.enable();

        let params = json!({
            "location": {
                "scriptId": "script-1",
                "lineNumber": 10
            }
        });

        let result = debugger.handle_method("setBreakpoint", Some(params)).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        let response: SetBreakpointResponse = serde_json::from_value(value).unwrap();
        assert_eq!(response.actual_location.line_number, 10);
    }

    #[tokio::test]
    async fn test_domain_handler_pause() {
        let debugger = DebuggerDomain::new();
        debugger.enable();

        let result = debugger.handle_method("pause", None).await;
        assert!(result.is_ok());
        assert!(debugger.is_paused());
    }

    #[tokio::test]
    async fn test_domain_handler_unknown_method() {
        let debugger = DebuggerDomain::new();

        let result = debugger.handle_method("unknownMethod", None).await;
        assert!(result.is_err());
    }
}
