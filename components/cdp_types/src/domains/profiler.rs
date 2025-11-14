// Profiler domain types

use super::runtime::CallFrame;
use serde::{Deserialize, Serialize};

/// Profile node identifier
pub type ProfileNodeId = u32;

/// Profile node
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProfileNode {
    /// Unique identifier
    pub id: ProfileNodeId,
    /// Function call frame
    pub call_frame: CallFrame,
    /// Number of samples where this node was on top of the call stack
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hit_count: Option<u32>,
    /// Child node IDs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<ProfileNodeId>>,
}

/// Profile
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    /// Profile nodes
    pub nodes: Vec<ProfileNode>,
    /// Profile start time (microseconds)
    pub start_time: f64,
    /// Profile end time (microseconds)
    pub end_time: f64,
    /// Sample node IDs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub samples: Option<Vec<ProfileNodeId>>,
    /// Time deltas between samples (microseconds)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_deltas: Option<Vec<u32>>,
}

/// Position in source code
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PositionTickInfo {
    /// Source line number (0-based)
    pub line: u32,
    /// Number of samples on this line
    pub ticks: u32,
}

/// Coverage range
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CoverageRange {
    /// Start offset (inclusive)
    pub start_offset: u32,
    /// End offset (exclusive)
    pub end_offset: u32,
    /// Execution count
    pub count: u32,
}

/// Script coverage
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ScriptCoverage {
    /// Script identifier
    pub script_id: String,
    /// Script URL
    pub url: String,
    /// Coverage ranges
    pub functions: Vec<FunctionCoverage>,
}

/// Function coverage
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FunctionCoverage {
    /// Function name
    pub function_name: String,
    /// Coverage ranges
    pub ranges: Vec<CoverageRange>,
    /// Whether coverage is block-level
    pub is_block_coverage: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_node() {
        let node = ProfileNode {
            id: 1,
            call_frame: CallFrame {
                function_name: "main".to_string(),
                script_id: "1".to_string(),
                url: "file.js".to_string(),
                line_number: 0,
                column_number: 0,
            },
            hit_count: Some(10),
            children: Some(vec![2, 3]),
        };

        let json = serde_json::to_string(&node).unwrap();
        assert!(json.contains("\"id\":1"));
    }

    #[test]
    fn test_profile() {
        let profile = Profile {
            nodes: vec![],
            start_time: 1000.0,
            end_time: 2000.0,
            samples: Some(vec![1, 2, 3]),
            time_deltas: Some(vec![10, 20, 30]),
        };

        let json = serde_json::to_string(&profile).unwrap();
        assert!(json.contains("\"startTime\":1000"));
    }
}
