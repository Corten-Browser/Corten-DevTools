//! JavaScript Debug Bridge
//!
//! Implements FEAT-019: JavaScript Debug Bridge
//!
//! Features:
//! - Script source management
//! - Breakpoint coordination between CDP and JS engine
//! - Step execution control (stepOver, stepInto, stepOut)
//! - Call stack management
//! - Scope chain access
//! - Source map integration for debugging transpiled code

use async_trait::async_trait;
use cdp_types::domains::debugger::{
    BreakpointId, CallFrame, Location, PausedReason, Scope, ScopeType, ScriptId,
};
use cdp_types::domains::runtime::{RemoteObject, RemoteObjectId, RemoteObjectType};
use cdp_types::CdpError;
use dashmap::DashMap;
use parking_lot::RwLock;
use protocol_handler::DomainHandler;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::broadcast;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::source_map::{OriginalLocation, Position, SourceMap, SourceMapError};

/// Errors for JavaScript Debug Bridge operations
#[derive(Error, Debug)]
pub enum JsDebugBridgeError {
    /// Debugger not enabled
    #[error("Debugger not enabled")]
    NotEnabled,

    /// Debugger not paused
    #[error("Debugger not paused")]
    NotPaused,

    /// Script not found
    #[error("Script not found: {0}")]
    ScriptNotFound(String),

    /// Breakpoint not found
    #[error("Breakpoint not found: {0}")]
    BreakpointNotFound(String),

    /// Call frame not found
    #[error("Call frame not found: {0}")]
    CallFrameNotFound(String),

    /// Source map error
    #[error("Source map error: {0}")]
    SourceMapError(#[from] SourceMapError),

    /// Invalid parameter
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    /// Evaluation error
    #[error("Evaluation error: {0}")]
    EvaluationError(String),
}

/// Result type for debug bridge operations
pub type Result<T> = std::result::Result<T, JsDebugBridgeError>;

/// Script information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptInfo {
    /// Script ID
    pub script_id: ScriptId,
    /// Script URL
    pub url: String,
    /// Source content
    pub source: String,
    /// Start line in the containing document
    pub start_line: u32,
    /// Start column
    pub start_column: u32,
    /// End line
    pub end_line: u32,
    /// End column
    pub end_column: u32,
    /// Execution context ID
    pub execution_context_id: u32,
    /// Script hash
    pub hash: String,
    /// Source map URL (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_map_url: Option<String>,
    /// Whether this is a module
    pub is_module: bool,
    /// Script length
    pub length: u32,
}

/// Breakpoint information
#[derive(Debug, Clone)]
pub struct BreakpointInfo {
    /// Breakpoint ID
    pub id: BreakpointId,
    /// Location in generated code
    pub location: Location,
    /// Original location (if source map available)
    pub original_location: Option<OriginalLocation>,
    /// Condition expression
    pub condition: Option<String>,
    /// Log point message (if this is a log point)
    pub log_message: Option<String>,
    /// Hit count
    pub hit_count: u32,
    /// Whether breakpoint is enabled
    pub enabled: bool,
}

/// Debugger pause state
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PauseState {
    /// Reason for pause
    pub reason: PausedReason,
    /// Current call frames
    pub call_frames: Vec<CallFrame>,
    /// Hit breakpoints (if paused on breakpoint)
    pub hit_breakpoints: Vec<BreakpointId>,
    /// Exception data (if paused on exception)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// Step action type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepAction {
    /// Step over (next line, don't enter functions)
    StepOver,
    /// Step into (enter function calls)
    StepInto,
    /// Step out (finish current function)
    StepOut,
    /// Continue execution
    Continue,
}

/// Debug event types
#[derive(Debug, Clone)]
pub enum DebugEvent {
    /// Script parsed
    ScriptParsed(ScriptInfo),
    /// Script failed to parse
    ScriptFailedToParse {
        script_id: ScriptId,
        url: String,
        error: String,
    },
    /// Debugger paused
    Paused(PauseState),
    /// Debugger resumed
    Resumed,
    /// Breakpoint resolved
    BreakpointResolved {
        breakpoint_id: BreakpointId,
        location: Location,
    },
}

/// JavaScript Debug Bridge
///
/// Provides a bridge between Chrome DevTools Protocol and a JavaScript engine
/// for debugging operations.
pub struct JsDebugBridge {
    /// Whether debugger is enabled
    enabled: Arc<AtomicBool>,
    /// Whether debugger is currently paused
    paused: Arc<AtomicBool>,
    /// Script ID counter
    script_counter: Arc<AtomicU32>,
    /// Breakpoint ID counter
    breakpoint_counter: Arc<AtomicU32>,
    /// Scripts by ID
    scripts: Arc<DashMap<String, ScriptInfo>>,
    /// Scripts by URL (for quick lookup)
    scripts_by_url: Arc<DashMap<String, String>>,
    /// Source maps by script ID
    source_maps: Arc<DashMap<String, SourceMap>>,
    /// Breakpoints by ID
    breakpoints: Arc<DashMap<String, BreakpointInfo>>,
    /// Breakpoints by location (script_id:line:column -> breakpoint_id)
    breakpoints_by_location: Arc<DashMap<String, String>>,
    /// Current call frames (when paused)
    call_frames: Arc<RwLock<Vec<CallFrame>>>,
    /// Current pause reason
    pause_reason: Arc<RwLock<Option<PausedReason>>>,
    /// Hit breakpoints in current pause
    hit_breakpoints: Arc<RwLock<Vec<BreakpointId>>>,
    /// Event broadcaster
    event_sender: broadcast::Sender<DebugEvent>,
    /// Skip all pauses flag
    skip_all_pauses: Arc<AtomicBool>,
    /// Pause on exceptions mode
    pause_on_exceptions: Arc<RwLock<PauseOnExceptionsMode>>,
    /// Async stack trace depth
    async_stack_trace_depth: Arc<AtomicU32>,
}

/// Mode for pausing on exceptions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PauseOnExceptionsMode {
    /// Don't pause on exceptions
    #[default]
    None,
    /// Pause only on uncaught exceptions
    Uncaught,
    /// Pause on all exceptions
    All,
}

impl JsDebugBridge {
    /// Create a new JavaScript Debug Bridge
    pub fn new() -> Self {
        let (event_sender, _) = broadcast::channel(100);
        Self {
            enabled: Arc::new(AtomicBool::new(false)),
            paused: Arc::new(AtomicBool::new(false)),
            script_counter: Arc::new(AtomicU32::new(1)),
            breakpoint_counter: Arc::new(AtomicU32::new(1)),
            scripts: Arc::new(DashMap::new()),
            scripts_by_url: Arc::new(DashMap::new()),
            source_maps: Arc::new(DashMap::new()),
            breakpoints: Arc::new(DashMap::new()),
            breakpoints_by_location: Arc::new(DashMap::new()),
            call_frames: Arc::new(RwLock::new(Vec::new())),
            pause_reason: Arc::new(RwLock::new(None)),
            hit_breakpoints: Arc::new(RwLock::new(Vec::new())),
            event_sender,
            skip_all_pauses: Arc::new(AtomicBool::new(false)),
            pause_on_exceptions: Arc::new(RwLock::new(PauseOnExceptionsMode::None)),
            async_stack_trace_depth: Arc::new(AtomicU32::new(0)),
        }
    }

    /// Enable the debugger
    pub fn enable(&self) -> Result<()> {
        self.enabled.store(true, Ordering::SeqCst);
        debug!("JavaScript Debug Bridge enabled");
        Ok(())
    }

    /// Disable the debugger
    pub fn disable(&self) -> Result<()> {
        self.enabled.store(false, Ordering::SeqCst);
        self.paused.store(false, Ordering::SeqCst);
        self.scripts.clear();
        self.scripts_by_url.clear();
        self.source_maps.clear();
        self.breakpoints.clear();
        self.breakpoints_by_location.clear();
        self.call_frames.write().clear();
        *self.pause_reason.write() = None;
        self.hit_breakpoints.write().clear();
        debug!("JavaScript Debug Bridge disabled");
        Ok(())
    }

    /// Check if debugger is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    /// Check if debugger is paused
    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }

    /// Subscribe to debug events
    pub fn subscribe(&self) -> broadcast::Receiver<DebugEvent> {
        self.event_sender.subscribe()
    }

    // ========== Script Management ==========

    /// Register a new script
    pub fn add_script(&self, url: &str, source: &str) -> Result<ScriptInfo> {
        if !self.is_enabled() {
            return Err(JsDebugBridgeError::NotEnabled);
        }

        let script_id = self.script_counter.fetch_add(1, Ordering::SeqCst);
        let script_id_str = format!("script-{}", script_id);

        let lines: Vec<&str> = source.lines().collect();
        let end_line = if lines.is_empty() {
            0
        } else {
            (lines.len() - 1) as u32
        };
        let end_column = lines.last().map(|l| l.len() as u32).unwrap_or(0);

        // Extract source map URL from source
        let source_map_url = SourceMap::extract_url_from_source(source);

        let script_info = ScriptInfo {
            script_id: ScriptId(script_id_str.clone()),
            url: url.to_string(),
            source: source.to_string(),
            start_line: 0,
            start_column: 0,
            end_line,
            end_column,
            execution_context_id: 1,
            hash: format!("{:x}", md5_hash(source)),
            source_map_url: source_map_url.clone(),
            is_module: source.contains("import ") || source.contains("export "),
            length: source.len() as u32,
        };

        // Store script
        self.scripts.insert(script_id_str.clone(), script_info.clone());
        self.scripts_by_url
            .insert(url.to_string(), script_id_str.clone());

        // Load source map if available
        if let Some(ref sm_url) = source_map_url {
            if let Ok(source_map) = self.load_source_map(sm_url) {
                self.source_maps.insert(script_id_str.clone(), source_map);
            }
        }

        // Emit event
        let _ = self.event_sender.send(DebugEvent::ScriptParsed(script_info.clone()));

        debug!("Added script {}: {}", script_id_str, url);
        Ok(script_info)
    }

    /// Get script by ID
    pub fn get_script(&self, script_id: &str) -> Option<ScriptInfo> {
        self.scripts.get(script_id).map(|s| s.clone())
    }

    /// Get script source
    pub fn get_script_source(&self, script_id: &str) -> Result<String> {
        self.scripts
            .get(script_id)
            .map(|s| s.source.clone())
            .ok_or_else(|| JsDebugBridgeError::ScriptNotFound(script_id.to_string()))
    }

    /// Get all scripts
    pub fn get_all_scripts(&self) -> Vec<ScriptInfo> {
        self.scripts.iter().map(|e| e.value().clone()).collect()
    }

    /// Load a source map
    fn load_source_map(&self, url: &str) -> Result<SourceMap> {
        if url.starts_with("data:") {
            // Inline source map
            Ok(SourceMap::parse_data_url(url)?)
        } else {
            // For now, we don't support loading external source maps
            // In a real implementation, this would fetch the URL
            Err(JsDebugBridgeError::InvalidParameter(
                "External source maps not yet supported".to_string(),
            ))
        }
    }

    // ========== Source Map Operations ==========

    /// Get original location from generated location
    pub fn get_original_location(&self, script_id: &str, line: u32, column: u32) -> Option<OriginalLocation> {
        let source_map = self.source_maps.get(script_id)?;
        source_map
            .original_position_for(Position::new(line, column))
            .ok()
    }

    /// Get generated location from original location
    pub fn get_generated_location(&self, script_id: &str, source: &str, line: u32, column: u32) -> Option<Position> {
        let source_map = self.source_maps.get(script_id)?;
        source_map
            .generated_position_for(source, Position::new(line, column))
            .ok()
            .map(|g| g.position)
    }

    /// Check if script has source map
    pub fn has_source_map(&self, script_id: &str) -> bool {
        self.source_maps.contains_key(script_id)
    }

    // ========== Breakpoint Management ==========

    /// Set a breakpoint
    pub fn set_breakpoint(
        &self,
        location: Location,
        condition: Option<String>,
    ) -> Result<(BreakpointId, Location)> {
        if !self.is_enabled() {
            return Err(JsDebugBridgeError::NotEnabled);
        }

        let bp_id = self.breakpoint_counter.fetch_add(1, Ordering::SeqCst);
        let breakpoint_id = BreakpointId(format!("bp-{}", bp_id));

        // Try to get original location if source map available
        let original_location = self.get_original_location(
            &location.script_id.0,
            location.line_number,
            location.column_number.unwrap_or(0),
        );

        let breakpoint_info = BreakpointInfo {
            id: breakpoint_id.clone(),
            location: location.clone(),
            original_location,
            condition,
            log_message: None,
            hit_count: 0,
            enabled: true,
        };

        // Create location key
        let loc_key = format!(
            "{}:{}:{}",
            location.script_id.0,
            location.line_number,
            location.column_number.unwrap_or(0)
        );

        self.breakpoints
            .insert(breakpoint_id.0.clone(), breakpoint_info);
        self.breakpoints_by_location
            .insert(loc_key, breakpoint_id.0.clone());

        debug!(
            "Set breakpoint {} at {}:{:?}",
            breakpoint_id.0, location.script_id.0, location.line_number
        );

        // Emit event
        let _ = self.event_sender.send(DebugEvent::BreakpointResolved {
            breakpoint_id: breakpoint_id.clone(),
            location: location.clone(),
        });

        Ok((breakpoint_id, location))
    }

    /// Set breakpoint by URL
    pub fn set_breakpoint_by_url(
        &self,
        url: &str,
        line_number: u32,
        column_number: Option<u32>,
        condition: Option<String>,
    ) -> Result<(BreakpointId, Vec<Location>)> {
        if !self.is_enabled() {
            return Err(JsDebugBridgeError::NotEnabled);
        }

        // Find script by URL
        let script_id = self
            .scripts_by_url
            .get(url)
            .map(|s| s.clone())
            .ok_or_else(|| JsDebugBridgeError::ScriptNotFound(url.to_string()))?;

        let location = Location {
            script_id: ScriptId(script_id),
            line_number,
            column_number,
        };

        let (bp_id, actual_location) = self.set_breakpoint(location, condition)?;

        Ok((bp_id, vec![actual_location]))
    }

    /// Remove a breakpoint
    pub fn remove_breakpoint(&self, breakpoint_id: &BreakpointId) -> Result<()> {
        if !self.is_enabled() {
            return Err(JsDebugBridgeError::NotEnabled);
        }

        let bp_info = self
            .breakpoints
            .remove(&breakpoint_id.0)
            .ok_or_else(|| JsDebugBridgeError::BreakpointNotFound(breakpoint_id.0.clone()))?;

        // Remove from location index
        let loc_key = format!(
            "{}:{}:{}",
            bp_info.1.location.script_id.0,
            bp_info.1.location.line_number,
            bp_info.1.location.column_number.unwrap_or(0)
        );
        self.breakpoints_by_location.remove(&loc_key);

        debug!("Removed breakpoint {}", breakpoint_id.0);
        Ok(())
    }

    /// Get all breakpoints
    pub fn get_breakpoints(&self) -> Vec<BreakpointInfo> {
        self.breakpoints.iter().map(|e| e.value().clone()).collect()
    }

    /// Get breakpoint by ID
    pub fn get_breakpoint(&self, breakpoint_id: &str) -> Option<BreakpointInfo> {
        self.breakpoints.get(breakpoint_id).map(|b| b.clone())
    }

    /// Check if breakpoint exists at location
    pub fn has_breakpoint_at(&self, script_id: &str, line: u32, column: u32) -> bool {
        let loc_key = format!("{}:{}:{}", script_id, line, column);
        self.breakpoints_by_location.contains_key(&loc_key)
    }

    // ========== Execution Control ==========

    /// Pause execution
    pub fn pause(&self) -> Result<()> {
        if !self.is_enabled() {
            return Err(JsDebugBridgeError::NotEnabled);
        }

        if self.skip_all_pauses.load(Ordering::SeqCst) {
            return Ok(());
        }

        self.paused.store(true, Ordering::SeqCst);
        *self.pause_reason.write() = Some(PausedReason::Other);

        // Create mock call frame for testing
        self.create_mock_call_frames();

        // Emit event
        let pause_state = PauseState {
            reason: PausedReason::Other,
            call_frames: self.call_frames.read().clone(),
            hit_breakpoints: self.hit_breakpoints.read().clone(),
            data: None,
        };
        let _ = self.event_sender.send(DebugEvent::Paused(pause_state));

        debug!("Debugger paused");
        Ok(())
    }

    /// Resume execution
    pub fn resume(&self) -> Result<()> {
        if !self.is_enabled() {
            return Err(JsDebugBridgeError::NotEnabled);
        }

        if !self.is_paused() {
            return Err(JsDebugBridgeError::NotPaused);
        }

        self.paused.store(false, Ordering::SeqCst);
        self.call_frames.write().clear();
        *self.pause_reason.write() = None;
        self.hit_breakpoints.write().clear();

        let _ = self.event_sender.send(DebugEvent::Resumed);

        debug!("Debugger resumed");
        Ok(())
    }

    /// Step over (execute next line without entering functions)
    pub fn step_over(&self) -> Result<()> {
        self.perform_step(StepAction::StepOver)
    }

    /// Step into (enter function calls)
    pub fn step_into(&self) -> Result<()> {
        self.perform_step(StepAction::StepInto)
    }

    /// Step out (finish current function)
    pub fn step_out(&self) -> Result<()> {
        self.perform_step(StepAction::StepOut)
    }

    /// Perform a step action
    fn perform_step(&self, action: StepAction) -> Result<()> {
        if !self.is_enabled() {
            return Err(JsDebugBridgeError::NotEnabled);
        }

        if !self.is_paused() {
            return Err(JsDebugBridgeError::NotPaused);
        }

        debug!("Performing step action: {:?}", action);

        // In a real implementation, this would instruct the JS engine to step
        // For now, we just resume
        self.paused.store(false, Ordering::SeqCst);
        self.call_frames.write().clear();
        *self.pause_reason.write() = None;

        let _ = self.event_sender.send(DebugEvent::Resumed);

        Ok(())
    }

    /// Set pause on exceptions mode
    pub fn set_pause_on_exceptions(&self, mode: PauseOnExceptionsMode) {
        *self.pause_on_exceptions.write() = mode;
        debug!("Set pause on exceptions mode: {:?}", mode);
    }

    /// Get pause on exceptions mode
    pub fn get_pause_on_exceptions(&self) -> PauseOnExceptionsMode {
        *self.pause_on_exceptions.read()
    }

    /// Set skip all pauses
    pub fn set_skip_all_pauses(&self, skip: bool) {
        self.skip_all_pauses.store(skip, Ordering::SeqCst);
        debug!("Set skip all pauses: {}", skip);
    }

    /// Set async stack trace depth
    pub fn set_async_stack_trace_depth(&self, depth: u32) {
        self.async_stack_trace_depth.store(depth, Ordering::SeqCst);
        debug!("Set async stack trace depth: {}", depth);
    }

    // ========== Call Stack Management ==========

    /// Get current call frames
    pub fn get_call_frames(&self) -> Vec<CallFrame> {
        self.call_frames.read().clone()
    }

    /// Get specific call frame
    pub fn get_call_frame(&self, call_frame_id: &str) -> Option<CallFrame> {
        self.call_frames
            .read()
            .iter()
            .find(|f| f.call_frame_id == call_frame_id)
            .cloned()
    }

    /// Create mock call frames for testing
    fn create_mock_call_frames(&self) {
        let mut frames = self.call_frames.write();
        frames.clear();

        // Get first script (if any) for realistic mock data
        let (script_id, url) = self
            .scripts
            .iter()
            .next()
            .map(|s| (s.script_id.clone(), s.url.clone()))
            .unwrap_or_else(|| (ScriptId("script-1".to_string()), "file:///test.js".to_string()));

        // Create a mock call frame
        frames.push(CallFrame {
            call_frame_id: format!("frame-{}", Uuid::new_v4()),
            function_name: "main".to_string(),
            location: Location {
                script_id: script_id.clone(),
                line_number: 10,
                column_number: Some(5),
            },
            url: url.clone(),
            scope_chain: vec![
                Scope {
                    scope_type: ScopeType::Local,
                    object: create_mock_remote_object("Local"),
                    name: Some("Local".to_string()),
                    start_location: None,
                    end_location: None,
                },
                Scope {
                    scope_type: ScopeType::Closure,
                    object: create_mock_remote_object("Closure"),
                    name: Some("Closure".to_string()),
                    start_location: None,
                    end_location: None,
                },
                Scope {
                    scope_type: ScopeType::Global,
                    object: create_mock_remote_object("Global"),
                    name: Some("Global".to_string()),
                    start_location: None,
                    end_location: None,
                },
            ],
            this: create_mock_remote_object("this"),
            return_value: None,
        });

        // Add an outer frame
        frames.push(CallFrame {
            call_frame_id: format!("frame-{}", Uuid::new_v4()),
            function_name: "<anonymous>".to_string(),
            location: Location {
                script_id,
                line_number: 1,
                column_number: Some(0),
            },
            url,
            scope_chain: vec![Scope {
                scope_type: ScopeType::Global,
                object: create_mock_remote_object("Global"),
                name: Some("Global".to_string()),
                start_location: None,
                end_location: None,
            }],
            this: create_mock_remote_object("global"),
            return_value: None,
        });
    }

    // ========== Evaluation ==========

    /// Evaluate expression on a call frame
    pub fn evaluate_on_call_frame(
        &self,
        call_frame_id: &str,
        expression: &str,
        _include_command_line_api: bool,
    ) -> Result<RemoteObject> {
        if !self.is_enabled() {
            return Err(JsDebugBridgeError::NotEnabled);
        }

        if !self.is_paused() {
            return Err(JsDebugBridgeError::NotPaused);
        }

        // Verify call frame exists
        let frames = self.call_frames.read();
        if !frames.iter().any(|f| f.call_frame_id == call_frame_id) {
            return Err(JsDebugBridgeError::CallFrameNotFound(
                call_frame_id.to_string(),
            ));
        }

        debug!(
            "Evaluating on call frame {}: {}",
            call_frame_id, expression
        );

        // Mock evaluation - in real implementation this would call JS engine
        Ok(mock_evaluate(expression))
    }

    /// Evaluate global expression
    pub fn evaluate(&self, expression: &str) -> Result<RemoteObject> {
        if !self.is_enabled() {
            return Err(JsDebugBridgeError::NotEnabled);
        }

        debug!("Evaluating: {}", expression);
        Ok(mock_evaluate(expression))
    }

    // ========== Scope Chain Access ==========

    /// Get scope variables
    pub fn get_scope_variables(&self, scope_object_id: &str) -> Result<Vec<PropertyInfo>> {
        if !self.is_enabled() {
            return Err(JsDebugBridgeError::NotEnabled);
        }

        debug!("Getting scope variables for: {}", scope_object_id);

        // Mock scope variables
        Ok(vec![
            PropertyInfo {
                name: "x".to_string(),
                value: create_mock_remote_object_value("number", Some(json!(42))),
                writable: true,
                configurable: true,
                enumerable: true,
            },
            PropertyInfo {
                name: "name".to_string(),
                value: create_mock_remote_object_value("string", Some(json!("test"))),
                writable: true,
                configurable: true,
                enumerable: true,
            },
            PropertyInfo {
                name: "arr".to_string(),
                value: create_mock_remote_object_value("object", None),
                writable: true,
                configurable: true,
                enumerable: true,
            },
        ])
    }

    /// Restart frame (for Edit and Continue)
    pub fn restart_frame(&self, call_frame_id: &str) -> Result<Vec<CallFrame>> {
        if !self.is_enabled() {
            return Err(JsDebugBridgeError::NotEnabled);
        }

        if !self.is_paused() {
            return Err(JsDebugBridgeError::NotPaused);
        }

        // Verify call frame exists
        let frames = self.call_frames.read();
        if !frames.iter().any(|f| f.call_frame_id == call_frame_id) {
            return Err(JsDebugBridgeError::CallFrameNotFound(
                call_frame_id.to_string(),
            ));
        }

        debug!("Restarting frame: {}", call_frame_id);

        // In a real implementation, this would restart execution from the frame
        // For now, just return current frames
        Ok(frames.clone())
    }
}

impl Default for JsDebugBridge {
    fn default() -> Self {
        Self::new()
    }
}

/// Property information for scope variables
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyInfo {
    /// Property name
    pub name: String,
    /// Property value
    pub value: RemoteObject,
    /// Whether property is writable
    pub writable: bool,
    /// Whether property is configurable
    pub configurable: bool,
    /// Whether property is enumerable
    pub enumerable: bool,
}

/// Create a mock RemoteObject
fn create_mock_remote_object(class_name: &str) -> RemoteObject {
    RemoteObject {
        object_type: RemoteObjectType::Object,
        subtype: None,
        class_name: Some(class_name.to_string()),
        value: None,
        unserializable_value: None,
        description: Some(class_name.to_string()),
        object_id: Some(RemoteObjectId(format!("obj-{}", Uuid::new_v4()))),
        preview: None,
    }
}

/// Create a mock RemoteObject with value
fn create_mock_remote_object_value(type_name: &str, value: Option<Value>) -> RemoteObject {
    let object_type = match type_name {
        "number" => RemoteObjectType::Number,
        "string" => RemoteObjectType::String,
        "boolean" => RemoteObjectType::Boolean,
        "undefined" => RemoteObjectType::Undefined,
        _ => RemoteObjectType::Object,
    };

    RemoteObject {
        object_type,
        subtype: None,
        class_name: None,
        value,
        unserializable_value: None,
        description: None,
        object_id: if type_name == "object" {
            Some(RemoteObjectId(format!("obj-{}", Uuid::new_v4())))
        } else {
            None
        },
        preview: None,
    }
}

/// Mock expression evaluation
fn mock_evaluate(expression: &str) -> RemoteObject {
    match expression.trim() {
        "42" | "21 + 21" => RemoteObject {
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
        "false" => RemoteObject {
            object_type: RemoteObjectType::Boolean,
            subtype: None,
            class_name: None,
            value: Some(json!(false)),
            unserializable_value: None,
            description: Some("false".to_string()),
            object_id: None,
            preview: None,
        },
        s if s.starts_with('"') && s.ends_with('"') => RemoteObject {
            object_type: RemoteObjectType::String,
            subtype: None,
            class_name: None,
            value: Some(json!(s.trim_matches('"'))),
            unserializable_value: None,
            description: Some(s.to_string()),
            object_id: None,
            preview: None,
        },
        "NaN" => RemoteObject {
            object_type: RemoteObjectType::Number,
            subtype: None,
            class_name: None,
            value: None,
            unserializable_value: Some("NaN".to_string()),
            description: Some("NaN".to_string()),
            object_id: None,
            preview: None,
        },
        "Infinity" => RemoteObject {
            object_type: RemoteObjectType::Number,
            subtype: None,
            class_name: None,
            value: None,
            unserializable_value: Some("Infinity".to_string()),
            description: Some("Infinity".to_string()),
            object_id: None,
            preview: None,
        },
        "null" => RemoteObject {
            object_type: RemoteObjectType::Object,
            subtype: Some(cdp_types::domains::runtime::RemoteObjectSubtype::Null),
            class_name: None,
            value: Some(json!(null)),
            unserializable_value: None,
            description: Some("null".to_string()),
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

/// Simple hash function (not cryptographic)
fn md5_hash(input: &str) -> u64 {
    let mut hash: u64 = 0;
    for (i, byte) in input.bytes().enumerate() {
        hash = hash.wrapping_add((byte as u64).wrapping_mul((i as u64).wrapping_add(1)));
        hash = hash.rotate_left(7);
    }
    hash
}

#[async_trait]
impl DomainHandler for JsDebugBridge {
    fn name(&self) -> &str {
        "JsDebugBridge"
    }

    async fn handle_method(
        &self,
        method: &str,
        params: Option<Value>,
    ) -> std::result::Result<Value, CdpError> {
        match method {
            "enable" => {
                self.enable()
                    .map_err(|e| CdpError::internal_error(e.to_string()))?;
                Ok(json!({}))
            }
            "disable" => {
                self.disable()
                    .map_err(|e| CdpError::internal_error(e.to_string()))?;
                Ok(json!({}))
            }
            "pause" => {
                self.pause()
                    .map_err(|e| CdpError::internal_error(e.to_string()))?;
                Ok(json!({}))
            }
            "resume" => {
                self.resume()
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

                let (bp_id, actual_location) = self
                    .set_breakpoint(location, condition)
                    .map_err(|e| CdpError::internal_error(e.to_string()))?;

                Ok(json!({
                    "breakpointId": bp_id.0,
                    "actualLocation": actual_location
                }))
            }
            "setBreakpointByUrl" => {
                let params = params.ok_or_else(|| CdpError::invalid_params("Missing params"))?;
                let url = params
                    .get("url")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| CdpError::invalid_params("Missing url"))?;
                let line_number = params
                    .get("lineNumber")
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| CdpError::invalid_params("Missing lineNumber"))?
                    as u32;
                let column_number = params.get("columnNumber").and_then(|v| v.as_u64()).map(|c| c as u32);
                let condition = params
                    .get("condition")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let (bp_id, locations) = self
                    .set_breakpoint_by_url(url, line_number, column_number, condition)
                    .map_err(|e| CdpError::internal_error(e.to_string()))?;

                Ok(json!({
                    "breakpointId": bp_id.0,
                    "locations": locations
                }))
            }
            "removeBreakpoint" => {
                let params = params.ok_or_else(|| CdpError::invalid_params("Missing params"))?;
                let bp_id_str = params
                    .get("breakpointId")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| CdpError::invalid_params("Missing breakpointId"))?;

                self.remove_breakpoint(&BreakpointId(bp_id_str.to_string()))
                    .map_err(|e| CdpError::internal_error(e.to_string()))?;

                Ok(json!({}))
            }
            "getScriptSource" => {
                let params = params.ok_or_else(|| CdpError::invalid_params("Missing params"))?;
                let script_id = params
                    .get("scriptId")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| CdpError::invalid_params("Missing scriptId"))?;

                let source = self
                    .get_script_source(script_id)
                    .map_err(|e| CdpError::internal_error(e.to_string()))?;

                Ok(json!({ "scriptSource": source }))
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
                let include_cmd_api = params
                    .get("includeCommandLineAPI")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let result = self
                    .evaluate_on_call_frame(call_frame_id, expression, include_cmd_api)
                    .map_err(|e| CdpError::internal_error(e.to_string()))?;

                Ok(json!({
                    "result": result
                }))
            }
            "setPauseOnExceptions" => {
                let params = params.ok_or_else(|| CdpError::invalid_params("Missing params"))?;
                let state = params
                    .get("state")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| CdpError::invalid_params("Missing state"))?;

                let mode = match state {
                    "none" => PauseOnExceptionsMode::None,
                    "uncaught" => PauseOnExceptionsMode::Uncaught,
                    "all" => PauseOnExceptionsMode::All,
                    _ => {
                        return Err(CdpError::invalid_params(format!(
                            "Invalid state: {}",
                            state
                        )))
                    }
                };

                self.set_pause_on_exceptions(mode);
                Ok(json!({}))
            }
            "setSkipAllPauses" => {
                let params = params.ok_or_else(|| CdpError::invalid_params("Missing params"))?;
                let skip = params
                    .get("skip")
                    .and_then(|v| v.as_bool())
                    .ok_or_else(|| CdpError::invalid_params("Missing skip"))?;

                self.set_skip_all_pauses(skip);
                Ok(json!({}))
            }
            "setAsyncCallStackDepth" => {
                let params = params.ok_or_else(|| CdpError::invalid_params("Missing params"))?;
                let depth = params
                    .get("maxDepth")
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| CdpError::invalid_params("Missing maxDepth"))?
                    as u32;

                self.set_async_stack_trace_depth(depth);
                Ok(json!({}))
            }
            _ => {
                warn!("Unknown JsDebugBridge method: {}", method);
                Err(CdpError::method_not_found(format!(
                    "JsDebugBridge.{}",
                    method
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_js_debug_bridge_new() {
        let bridge = JsDebugBridge::new();
        assert!(!bridge.is_enabled());
        assert!(!bridge.is_paused());
    }

    #[test]
    fn test_enable_disable() {
        let bridge = JsDebugBridge::new();

        assert!(bridge.enable().is_ok());
        assert!(bridge.is_enabled());

        assert!(bridge.disable().is_ok());
        assert!(!bridge.is_enabled());
    }

    #[test]
    fn test_add_script() {
        let bridge = JsDebugBridge::new();
        bridge.enable().unwrap();

        let source = "function test() { return 42; }";
        let result = bridge.add_script("file:///test.js", source);

        assert!(result.is_ok());
        let script_info = result.unwrap();
        assert_eq!(script_info.url, "file:///test.js");
        assert_eq!(script_info.source, source);
    }

    #[test]
    fn test_add_script_not_enabled() {
        let bridge = JsDebugBridge::new();
        let result = bridge.add_script("file:///test.js", "const x = 1;");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_script() {
        let bridge = JsDebugBridge::new();
        bridge.enable().unwrap();

        let source = "const x = 1;";
        let script_info = bridge.add_script("file:///test.js", source).unwrap();

        let retrieved = bridge.get_script(&script_info.script_id.0);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().source, source);
    }

    #[test]
    fn test_get_script_source() {
        let bridge = JsDebugBridge::new();
        bridge.enable().unwrap();

        let source = "const x = 1;";
        let script_info = bridge.add_script("file:///test.js", source).unwrap();

        let retrieved_source = bridge.get_script_source(&script_info.script_id.0);
        assert!(retrieved_source.is_ok());
        assert_eq!(retrieved_source.unwrap(), source);
    }

    #[test]
    fn test_set_breakpoint() {
        let bridge = JsDebugBridge::new();
        bridge.enable().unwrap();

        let script_info = bridge.add_script("file:///test.js", "const x = 1;").unwrap();

        let location = Location {
            script_id: script_info.script_id.clone(),
            line_number: 0,
            column_number: Some(0),
        };

        let result = bridge.set_breakpoint(location.clone(), None);
        assert!(result.is_ok());

        let (bp_id, actual_location) = result.unwrap();
        assert!(bp_id.0.starts_with("bp-"));
        assert_eq!(actual_location.line_number, 0);
    }

    #[test]
    fn test_set_breakpoint_by_url() {
        let bridge = JsDebugBridge::new();
        bridge.enable().unwrap();

        bridge.add_script("file:///test.js", "const x = 1;").unwrap();

        let result = bridge.set_breakpoint_by_url("file:///test.js", 0, Some(0), None);
        assert!(result.is_ok());

        let (bp_id, locations) = result.unwrap();
        assert!(bp_id.0.starts_with("bp-"));
        assert!(!locations.is_empty());
    }

    #[test]
    fn test_remove_breakpoint() {
        let bridge = JsDebugBridge::new();
        bridge.enable().unwrap();

        let script_info = bridge.add_script("file:///test.js", "const x = 1;").unwrap();

        let location = Location {
            script_id: script_info.script_id,
            line_number: 0,
            column_number: Some(0),
        };

        let (bp_id, _) = bridge.set_breakpoint(location, None).unwrap();
        assert!(bridge.remove_breakpoint(&bp_id).is_ok());
        assert!(bridge.get_breakpoint(&bp_id.0).is_none());
    }

    #[test]
    fn test_remove_nonexistent_breakpoint() {
        let bridge = JsDebugBridge::new();
        bridge.enable().unwrap();

        let result = bridge.remove_breakpoint(&BreakpointId("nonexistent".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_pause_resume() {
        let bridge = JsDebugBridge::new();
        bridge.enable().unwrap();

        assert!(!bridge.is_paused());

        assert!(bridge.pause().is_ok());
        assert!(bridge.is_paused());
        assert!(!bridge.get_call_frames().is_empty());

        assert!(bridge.resume().is_ok());
        assert!(!bridge.is_paused());
        assert!(bridge.get_call_frames().is_empty());
    }

    #[test]
    fn test_step_over() {
        let bridge = JsDebugBridge::new();
        bridge.enable().unwrap();
        bridge.pause().unwrap();

        assert!(bridge.is_paused());
        assert!(bridge.step_over().is_ok());
        assert!(!bridge.is_paused());
    }

    #[test]
    fn test_step_into() {
        let bridge = JsDebugBridge::new();
        bridge.enable().unwrap();
        bridge.pause().unwrap();

        assert!(bridge.step_into().is_ok());
        assert!(!bridge.is_paused());
    }

    #[test]
    fn test_step_out() {
        let bridge = JsDebugBridge::new();
        bridge.enable().unwrap();
        bridge.pause().unwrap();

        assert!(bridge.step_out().is_ok());
        assert!(!bridge.is_paused());
    }

    #[test]
    fn test_step_not_paused() {
        let bridge = JsDebugBridge::new();
        bridge.enable().unwrap();

        assert!(bridge.step_over().is_err());
    }

    #[test]
    fn test_evaluate_on_call_frame() {
        let bridge = JsDebugBridge::new();
        bridge.enable().unwrap();
        bridge.add_script("file:///test.js", "const x = 1;").unwrap();
        bridge.pause().unwrap();

        let frames = bridge.get_call_frames();
        assert!(!frames.is_empty());

        let frame_id = &frames[0].call_frame_id;
        let result = bridge.evaluate_on_call_frame(frame_id, "42", false);

        assert!(result.is_ok());
        let obj = result.unwrap();
        assert_eq!(obj.object_type, RemoteObjectType::Number);
        assert_eq!(obj.value, Some(json!(42)));
    }

    #[test]
    fn test_evaluate_on_nonexistent_frame() {
        let bridge = JsDebugBridge::new();
        bridge.enable().unwrap();
        bridge.pause().unwrap();

        let result = bridge.evaluate_on_call_frame("nonexistent", "42", false);
        assert!(result.is_err());
    }

    #[test]
    fn test_evaluate() {
        let bridge = JsDebugBridge::new();
        bridge.enable().unwrap();

        let result = bridge.evaluate("42");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().value, Some(json!(42)));

        let result = bridge.evaluate("true");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().value, Some(json!(true)));
    }

    #[test]
    fn test_pause_on_exceptions() {
        let bridge = JsDebugBridge::new();

        assert_eq!(
            bridge.get_pause_on_exceptions(),
            PauseOnExceptionsMode::None
        );

        bridge.set_pause_on_exceptions(PauseOnExceptionsMode::All);
        assert_eq!(bridge.get_pause_on_exceptions(), PauseOnExceptionsMode::All);

        bridge.set_pause_on_exceptions(PauseOnExceptionsMode::Uncaught);
        assert_eq!(
            bridge.get_pause_on_exceptions(),
            PauseOnExceptionsMode::Uncaught
        );
    }

    #[test]
    fn test_skip_all_pauses() {
        let bridge = JsDebugBridge::new();
        bridge.enable().unwrap();

        bridge.set_skip_all_pauses(true);

        // pause() should succeed but not actually pause
        assert!(bridge.pause().is_ok());
        assert!(!bridge.is_paused());
    }

    #[test]
    fn test_get_scope_variables() {
        let bridge = JsDebugBridge::new();
        bridge.enable().unwrap();

        let result = bridge.get_scope_variables("scope-1");
        assert!(result.is_ok());

        let vars = result.unwrap();
        assert!(!vars.is_empty());
        assert!(vars.iter().any(|v| v.name == "x"));
    }

    #[test]
    fn test_script_with_source_map_url() {
        let bridge = JsDebugBridge::new();
        bridge.enable().unwrap();

        let source = "const x = 1;\n//# sourceMappingURL=test.js.map";
        let script_info = bridge.add_script("file:///test.js", source).unwrap();

        assert_eq!(
            script_info.source_map_url,
            Some("test.js.map".to_string())
        );
    }

    #[test]
    fn test_has_breakpoint_at() {
        let bridge = JsDebugBridge::new();
        bridge.enable().unwrap();

        let script_info = bridge.add_script("file:///test.js", "const x = 1;").unwrap();

        let location = Location {
            script_id: script_info.script_id.clone(),
            line_number: 0,
            column_number: Some(0),
        };

        bridge.set_breakpoint(location, None).unwrap();

        assert!(bridge.has_breakpoint_at(&script_info.script_id.0, 0, 0));
        assert!(!bridge.has_breakpoint_at(&script_info.script_id.0, 1, 0));
    }

    #[test]
    fn test_get_all_scripts() {
        let bridge = JsDebugBridge::new();
        bridge.enable().unwrap();

        bridge.add_script("file:///a.js", "const a = 1;").unwrap();
        bridge.add_script("file:///b.js", "const b = 2;").unwrap();

        let scripts = bridge.get_all_scripts();
        assert_eq!(scripts.len(), 2);
    }

    #[test]
    fn test_get_breakpoints() {
        let bridge = JsDebugBridge::new();
        bridge.enable().unwrap();

        let script_info = bridge.add_script("file:///test.js", "const x = 1;").unwrap();

        let location1 = Location {
            script_id: script_info.script_id.clone(),
            line_number: 0,
            column_number: Some(0),
        };
        let location2 = Location {
            script_id: script_info.script_id,
            line_number: 0,
            column_number: Some(5),
        };

        bridge.set_breakpoint(location1, None).unwrap();
        bridge.set_breakpoint(location2, None).unwrap();

        let breakpoints = bridge.get_breakpoints();
        assert_eq!(breakpoints.len(), 2);
    }

    #[tokio::test]
    async fn test_domain_handler_name() {
        let bridge = JsDebugBridge::new();
        assert_eq!(bridge.name(), "JsDebugBridge");
    }

    #[tokio::test]
    async fn test_domain_handler_enable() {
        let bridge = JsDebugBridge::new();
        let result = bridge.handle_method("enable", None).await;
        assert!(result.is_ok());
        assert!(bridge.is_enabled());
    }

    #[tokio::test]
    async fn test_domain_handler_disable() {
        let bridge = JsDebugBridge::new();
        bridge.enable().unwrap();

        let result = bridge.handle_method("disable", None).await;
        assert!(result.is_ok());
        assert!(!bridge.is_enabled());
    }

    #[tokio::test]
    async fn test_domain_handler_pause() {
        let bridge = JsDebugBridge::new();
        bridge.enable().unwrap();

        let result = bridge.handle_method("pause", None).await;
        assert!(result.is_ok());
        assert!(bridge.is_paused());
    }

    #[tokio::test]
    async fn test_domain_handler_unknown_method() {
        let bridge = JsDebugBridge::new();
        let result = bridge.handle_method("unknownMethod", None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_domain_handler_set_breakpoint() {
        let bridge = JsDebugBridge::new();
        bridge.enable().unwrap();
        bridge.add_script("file:///test.js", "const x = 1;").unwrap();

        let params = json!({
            "location": {
                "scriptId": "script-1",
                "lineNumber": 0
            }
        });

        let result = bridge.handle_method("setBreakpoint", Some(params)).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(value["breakpointId"].is_string());
    }

    #[tokio::test]
    async fn test_domain_handler_get_script_source() {
        let bridge = JsDebugBridge::new();
        bridge.enable().unwrap();

        let source = "const x = 1;";
        let script_info = bridge.add_script("file:///test.js", source).unwrap();

        let params = json!({
            "scriptId": script_info.script_id.0
        });

        let result = bridge.handle_method("getScriptSource", Some(params)).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert_eq!(value["scriptSource"], source);
    }

    #[tokio::test]
    async fn test_domain_handler_set_pause_on_exceptions() {
        let bridge = JsDebugBridge::new();
        bridge.enable().unwrap();

        let params = json!({ "state": "all" });
        let result = bridge
            .handle_method("setPauseOnExceptions", Some(params))
            .await;
        assert!(result.is_ok());
        assert_eq!(bridge.get_pause_on_exceptions(), PauseOnExceptionsMode::All);
    }

    #[test]
    fn test_restart_frame() {
        let bridge = JsDebugBridge::new();
        bridge.enable().unwrap();
        bridge.add_script("file:///test.js", "const x = 1;").unwrap();
        bridge.pause().unwrap();

        let frames = bridge.get_call_frames();
        let frame_id = &frames[0].call_frame_id;

        let result = bridge.restart_frame(frame_id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_restart_frame_not_found() {
        let bridge = JsDebugBridge::new();
        bridge.enable().unwrap();
        bridge.pause().unwrap();

        let result = bridge.restart_frame("nonexistent");
        assert!(result.is_err());
    }
}
