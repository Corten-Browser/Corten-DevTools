//! Type definitions for profiler domains
//!
//! Contains all data structures used by ProfilerDomain and HeapProfilerDomain.

use serde::{Deserialize, Serialize};
use serde_json::Value;

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
