//! Type definitions for profiler domains
//!
//! Contains all data structures used by ProfilerDomain, HeapProfilerDomain,
//! CpuProfiler, and MemoryProfiler.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

// ============================================================================
// Profiler Domain Types
// ============================================================================

/// Represents a range of code coverage
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CoverageRange {
    /// Start offset of the range (inclusive)
    pub start_offset: u32,
    /// End offset of the range (exclusive)
    pub end_offset: u32,
    /// Number of times this range was executed
    pub count: u32,
}

/// Coverage data for a single function
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FunctionCoverage {
    /// Name of the function
    pub function_name: String,
    /// Coverage ranges for this function
    pub ranges: Vec<CoverageRange>,
    /// Whether this is block coverage (true) or function coverage (false)
    pub is_block_coverage: bool,
}

/// Coverage data for a single script
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ScriptCoverage {
    /// Script identifier
    pub script_id: String,
    /// URL of the script
    pub url: String,
    /// Functions in this script with coverage data
    pub functions: Vec<FunctionCoverage>,
}

/// A single node in a CPU profile tree
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileNode {
    /// Unique identifier for this node
    pub id: u32,
    /// Call frame information
    pub call_frame: Value,
    /// Number of samples where this was the top frame
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hit_count: Option<u32>,
    /// Child node IDs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<u32>>,
    /// Reason for deoptimization (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deopt_reason: Option<String>,
    /// Position ticks for this node
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_ticks: Option<Vec<Value>>,
}

/// A complete CPU profile
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    /// All profile nodes
    pub nodes: Vec<ProfileNode>,
    /// Start time of profiling (in microseconds)
    pub start_time: f64,
    /// End time of profiling (in microseconds)
    pub end_time: f64,
    /// Sample IDs (indices into nodes array)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub samples: Option<Vec<u32>>,
    /// Timestamps for each sample (in microseconds)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_deltas: Option<Vec<u32>>,
}

// ============================================================================
// HeapProfiler Domain Types
// ============================================================================

/// A node in the sampling heap profile tree
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SamplingHeapProfileNode {
    /// Call frame information
    pub call_frame: Value,
    /// Size of allocations in this node (excluding children)
    pub self_size: u64,
    /// Unique identifier for this node
    pub id: u32,
    /// Child nodes
    pub children: Vec<SamplingHeapProfileNode>,
}

/// A sampling heap profile
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SamplingHeapProfile {
    /// Root node of the profile tree
    pub head: SamplingHeapProfileNode,
    /// Individual samples
    pub samples: Vec<Value>,
}

/// A heap snapshot sample
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeapSnapshotSample {
    /// Size of the allocation
    pub size: u64,
    /// Node ID
    pub node_id: u32,
    /// Ordinal number
    pub ordinal: u32,
}

/// Heap statistics update data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeapStatsUpdate {
    /// Array of triplets: index, count, size
    pub stats_update: Vec<u64>,
}

/// Last seen object ID event data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LastSeenObjectId {
    /// Last seen object ID
    pub last_seen_object_id: u32,
    /// Timestamp in microseconds
    pub timestamp: f64,
}

/// Heap snapshot chunk for streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeapSnapshotChunk {
    /// Chunk of snapshot data
    pub chunk: String,
}

/// Heap snapshot progress report
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeapSnapshotProgress {
    /// Number of done steps
    pub done: u32,
    /// Total number of steps
    pub total: u32,
    /// Whether the snapshot is finished
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished: Option<bool>,
}

// ============================================================================
// CPU Profiler Enhanced Types
// ============================================================================

/// Call frame information for CPU profiling
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallFrame {
    /// Function name
    pub function_name: String,
    /// Script ID
    pub script_id: String,
    /// URL of the script
    pub url: String,
    /// Line number (0-based)
    pub line_number: i32,
    /// Column number (0-based)
    pub column_number: i32,
}

impl Default for CallFrame {
    fn default() -> Self {
        Self {
            function_name: String::new(),
            script_id: "0".to_string(),
            url: String::new(),
            line_number: 0,
            column_number: 0,
        }
    }
}

/// Position tick information for a profile node
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionTickInfo {
    /// Source line number (1-based)
    pub line: u32,
    /// Number of samples at this position
    pub ticks: u32,
}

/// Enhanced profile node with typed call frame
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnhancedProfileNode {
    /// Unique identifier for this node
    pub id: u32,
    /// Call frame information
    pub call_frame: CallFrame,
    /// Number of samples where this was the top frame
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hit_count: Option<u32>,
    /// Child node IDs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<u32>>,
    /// Reason for deoptimization (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deopt_reason: Option<String>,
    /// Position ticks for this node
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_ticks: Option<Vec<PositionTickInfo>>,
}

/// CPU profiling sample with timing information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuSample {
    /// Node ID in the profile tree
    pub node_id: u32,
    /// Timestamp when sample was taken (microseconds)
    pub timestamp: f64,
}

/// Call tree node for hierarchical CPU profile view
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallTreeNode {
    /// Function name
    pub function_name: String,
    /// Total time spent in this function and children (microseconds)
    pub total_time: f64,
    /// Self time spent only in this function (microseconds)
    pub self_time: f64,
    /// Hit count
    pub hit_count: u32,
    /// Children nodes
    pub children: Vec<CallTreeNode>,
    /// Script URL
    pub url: String,
    /// Line number
    pub line_number: i32,
}

/// Exportable CPU profile format
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportableProfile {
    /// Profile nodes
    pub nodes: Vec<EnhancedProfileNode>,
    /// Start time in microseconds
    pub start_time: f64,
    /// End time in microseconds
    pub end_time: f64,
    /// Sample node IDs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub samples: Option<Vec<u32>>,
    /// Time deltas between samples (microseconds)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_deltas: Option<Vec<u32>>,
    /// Profile title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

// ============================================================================
// Memory Profiler Types
// ============================================================================

/// Memory allocation entry
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllocationEntry {
    /// Allocation ID
    pub id: u64,
    /// Size in bytes
    pub size: u64,
    /// Timestamp when allocated (microseconds)
    pub timestamp: f64,
    /// Stack trace at allocation time
    pub stack_trace: Vec<CallFrame>,
    /// Whether this allocation has been freed
    pub freed: bool,
    /// Timestamp when freed (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub freed_timestamp: Option<f64>,
}

/// Memory timeline entry
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryTimelineEntry {
    /// Timestamp in microseconds
    pub timestamp: f64,
    /// Total allocated memory (bytes)
    pub total_allocated: u64,
    /// Total freed memory (bytes)
    pub total_freed: u64,
    /// Current heap size (bytes)
    pub current_heap_size: u64,
    /// Number of live allocations
    pub live_allocations: u32,
}

/// Potential memory leak information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PotentialLeak {
    /// Allocation ID
    pub allocation_id: u64,
    /// Size in bytes
    pub size: u64,
    /// Age in microseconds
    pub age: f64,
    /// Stack trace at allocation
    pub stack_trace: Vec<CallFrame>,
    /// Leak score (0-100, higher = more likely a leak)
    pub leak_score: u32,
    /// Reason for flagging as potential leak
    pub reason: String,
}

/// Memory allocation summary by call site
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllocationSummary {
    /// Call site identifier (function:line)
    pub call_site: String,
    /// Total allocations count
    pub allocation_count: u64,
    /// Total bytes allocated
    pub total_bytes: u64,
    /// Currently live bytes
    pub live_bytes: u64,
    /// Average allocation size
    pub average_size: f64,
}

/// Memory profiler snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemorySnapshot {
    /// Snapshot timestamp
    pub timestamp: f64,
    /// Total heap size
    pub total_heap_size: u64,
    /// Used heap size
    pub used_heap_size: u64,
    /// Allocation summaries by call site
    pub allocation_summaries: Vec<AllocationSummary>,
    /// Potential leaks detected
    pub potential_leaks: Vec<PotentialLeak>,
    /// Timeline since last snapshot
    pub timeline: Vec<MemoryTimelineEntry>,
}

/// Memory profiler configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryProfilerConfig {
    /// Enable stack trace capture for allocations
    pub capture_stack_traces: bool,
    /// Maximum stack trace depth
    pub max_stack_depth: u32,
    /// Minimum allocation size to track (bytes)
    pub min_allocation_size: u64,
    /// Leak detection threshold (microseconds)
    pub leak_threshold_age: f64,
    /// Timeline sampling interval (microseconds)
    pub timeline_interval: f64,
}

impl Default for MemoryProfilerConfig {
    fn default() -> Self {
        Self {
            capture_stack_traces: true,
            max_stack_depth: 16,
            min_allocation_size: 1024,           // 1KB minimum
            leak_threshold_age: 60_000_000.0,    // 60 seconds
            timeline_interval: 100_000.0,        // 100ms
        }
    }
}

/// Allocation tracking state
#[derive(Debug, Clone, Default)]
pub struct AllocationTrackingState {
    /// Active allocations by ID
    pub allocations: HashMap<u64, AllocationEntry>,
    /// Next allocation ID
    pub next_id: u64,
    /// Total allocated bytes (all time)
    pub total_allocated: u64,
    /// Total freed bytes
    pub total_freed: u64,
    /// Timeline entries
    pub timeline: Vec<MemoryTimelineEntry>,
    /// Configuration
    pub config: MemoryProfilerConfig,
}

// ============================================================================
// Timeline Domain Types (FEAT-034)
// ============================================================================

/// Event category for timeline recording
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TimelineEventCategory {
    /// Script execution events
    Scripting,
    /// Rendering and layout events
    Rendering,
    /// Paint operations
    Painting,
    /// Resource loading events
    Loading,
    /// Other/general events
    Other,
}

impl Default for TimelineEventCategory {
    fn default() -> Self {
        Self::Other
    }
}

impl std::fmt::Display for TimelineEventCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Scripting => write!(f, "scripting"),
            Self::Rendering => write!(f, "rendering"),
            Self::Painting => write!(f, "painting"),
            Self::Loading => write!(f, "loading"),
            Self::Other => write!(f, "other"),
        }
    }
}

/// A single timeline event
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineEvent {
    /// Event type name
    pub event_type: String,
    /// Category of the event
    pub category: TimelineEventCategory,
    /// Start timestamp in microseconds
    pub start_time: f64,
    /// Duration in microseconds (0 for instant events)
    pub duration: f64,
    /// Thread ID where event occurred
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<u32>,
    /// Frame ID associated with the event
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame_id: Option<String>,
    /// Stack trace at event time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack_trace: Option<Vec<CallFrame>>,
    /// Additional event data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl TimelineEvent {
    /// Create a new timeline event
    pub fn new(event_type: String, category: TimelineEventCategory, start_time: f64) -> Self {
        Self {
            event_type,
            category,
            start_time,
            duration: 0.0,
            thread_id: None,
            frame_id: None,
            stack_trace: None,
            data: None,
        }
    }

    /// Set the duration of this event
    pub fn with_duration(mut self, duration: f64) -> Self {
        self.duration = duration;
        self
    }

    /// Set the thread ID
    pub fn with_thread_id(mut self, thread_id: u32) -> Self {
        self.thread_id = Some(thread_id);
        self
    }

    /// Set the frame ID
    pub fn with_frame_id(mut self, frame_id: String) -> Self {
        self.frame_id = Some(frame_id);
        self
    }

    /// Set additional event data
    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }

    /// Set stack trace
    pub fn with_stack_trace(mut self, stack_trace: Vec<CallFrame>) -> Self {
        self.stack_trace = Some(stack_trace);
        self
    }
}

/// Memory snapshot for timeline
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineMemorySnapshot {
    /// Timestamp when snapshot was taken
    pub timestamp: f64,
    /// JS heap size in bytes
    pub js_heap_size_used: u64,
    /// Total JS heap size in bytes
    pub js_heap_size_total: u64,
    /// Documents count
    pub documents: u32,
    /// Nodes count
    pub nodes: u32,
    /// Event listeners count
    pub listeners: u32,
}

impl Default for TimelineMemorySnapshot {
    fn default() -> Self {
        Self {
            timestamp: 0.0,
            js_heap_size_used: 0,
            js_heap_size_total: 0,
            documents: 0,
            nodes: 0,
            listeners: 0,
        }
    }
}

/// Frame timing information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameTiming {
    /// Frame ID
    pub frame_id: String,
    /// Frame start timestamp
    pub start_time: f64,
    /// Frame end timestamp
    pub end_time: f64,
    /// Duration of the frame
    pub duration: f64,
    /// CPU time spent in this frame
    pub cpu_time: f64,
    /// Whether the frame was dropped
    pub dropped: bool,
}

impl FrameTiming {
    /// Create new frame timing
    pub fn new(frame_id: String, start_time: f64) -> Self {
        Self {
            frame_id,
            start_time,
            end_time: start_time,
            duration: 0.0,
            cpu_time: 0.0,
            dropped: false,
        }
    }

    /// Mark frame as complete
    pub fn complete(&mut self, end_time: f64, cpu_time: f64, dropped: bool) {
        self.end_time = end_time;
        self.duration = end_time - self.start_time;
        self.cpu_time = cpu_time;
        self.dropped = dropped;
    }
}

/// Timeline recording configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineConfig {
    /// Maximum events to record
    pub max_events: usize,
    /// Whether to capture memory snapshots
    pub capture_memory: bool,
    /// Memory snapshot interval in microseconds
    pub memory_interval: f64,
    /// Whether to capture stack traces
    pub capture_stacks: bool,
    /// Maximum stack depth
    pub max_stack_depth: u32,
    /// Categories to record (empty = all)
    pub categories: Vec<TimelineEventCategory>,
}

impl Default for TimelineConfig {
    fn default() -> Self {
        Self {
            max_events: 10000,
            capture_memory: true,
            memory_interval: 100_000.0, // 100ms
            capture_stacks: true,
            max_stack_depth: 16,
            categories: vec![],
        }
    }
}

/// Complete timeline recording
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineRecording {
    /// Recording start time
    pub start_time: f64,
    /// Recording end time
    pub end_time: f64,
    /// All recorded events
    pub events: Vec<TimelineEvent>,
    /// Memory snapshots
    pub memory_snapshots: Vec<TimelineMemorySnapshot>,
    /// Frame timing information
    pub frames: Vec<FrameTiming>,
}
