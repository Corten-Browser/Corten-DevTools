//! CPU Profiler implementation
//!
//! Provides enhanced CPU profiling with sample-based profiling,
//! call tree generation, and profile export capabilities.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::debug;

use crate::types::{
    CallFrame, CallTreeNode, CpuSample, EnhancedProfileNode, ExportableProfile, PositionTickInfo,
};

/// CPU Profiler state and functionality
#[derive(Debug)]
pub struct CpuProfiler {
    /// Whether profiling is currently active
    profiling_active: Arc<AtomicBool>,
    /// Sampling interval in microseconds
    sampling_interval: Arc<AtomicU32>,
    /// Profile start time
    start_time: Arc<RwLock<f64>>,
    /// Collected samples
    samples: Arc<RwLock<Vec<CpuSample>>>,
    /// Profile nodes by ID
    nodes: Arc<RwLock<HashMap<u32, EnhancedProfileNode>>>,
    /// Next node ID
    next_node_id: Arc<AtomicU32>,
    /// Profile title
    title: Arc<RwLock<Option<String>>>,
}

impl CpuProfiler {
    /// Create a new CPU profiler instance
    pub fn new() -> Self {
        Self {
            profiling_active: Arc::new(AtomicBool::new(false)),
            sampling_interval: Arc::new(AtomicU32::new(100)), // 100 microseconds default
            start_time: Arc::new(RwLock::new(0.0)),
            samples: Arc::new(RwLock::new(Vec::new())),
            nodes: Arc::new(RwLock::new(HashMap::new())),
            next_node_id: Arc::new(AtomicU32::new(1)),
            title: Arc::new(RwLock::new(None)),
        }
    }

    /// Get current timestamp in microseconds
    fn get_timestamp_micros() -> f64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_micros() as f64)
            .unwrap_or(0.0)
    }

    /// Check if profiling is active
    pub fn is_profiling(&self) -> bool {
        self.profiling_active.load(Ordering::SeqCst)
    }

    /// Get the sampling interval in microseconds
    pub fn get_sampling_interval(&self) -> u32 {
        self.sampling_interval.load(Ordering::SeqCst)
    }

    /// Set the sampling interval in microseconds
    pub fn set_sampling_interval(&self, interval: u32) {
        debug!("Setting CPU profiler sampling interval to {} microseconds", interval);
        self.sampling_interval.store(interval, Ordering::SeqCst);
    }

    /// Set the profile title
    pub fn set_title(&self, title: String) {
        *self.title.write() = Some(title);
    }

    /// Start CPU profiling
    pub fn start(&self) -> Result<(), String> {
        if self.profiling_active.load(Ordering::SeqCst) {
            return Err("Profiling already active".to_string());
        }

        debug!("Starting CPU profiler");

        // Clear previous data
        self.samples.write().clear();
        self.nodes.write().clear();
        self.next_node_id.store(1, Ordering::SeqCst);

        // Initialize root node
        let root_node = EnhancedProfileNode {
            id: 0,
            call_frame: CallFrame {
                function_name: "(root)".to_string(),
                script_id: "0".to_string(),
                url: String::new(),
                line_number: 0,
                column_number: 0,
            },
            hit_count: Some(0),
            children: Some(Vec::new()),
            deopt_reason: None,
            position_ticks: None,
        };
        self.nodes.write().insert(0, root_node);

        *self.start_time.write() = Self::get_timestamp_micros();
        self.profiling_active.store(true, Ordering::SeqCst);

        Ok(())
    }

    /// Stop CPU profiling and return the profile
    pub fn stop(&self) -> Result<ExportableProfile, String> {
        if !self.profiling_active.load(Ordering::SeqCst) {
            return Err("Profiling not active".to_string());
        }

        debug!("Stopping CPU profiler");
        self.profiling_active.store(false, Ordering::SeqCst);

        let start_time = *self.start_time.read();
        let end_time = Self::get_timestamp_micros();
        let samples = self.samples.read().clone();
        let nodes = self.nodes.read().clone();
        let title = self.title.read().clone();

        // Convert nodes to vec, sorted by ID
        let mut nodes_vec: Vec<EnhancedProfileNode> = nodes.into_values().collect();
        nodes_vec.sort_by_key(|n| n.id);

        // Extract sample data
        let sample_ids: Vec<u32> = samples.iter().map(|s| s.node_id).collect();
        let time_deltas: Vec<u32> = samples
            .windows(2)
            .map(|w| (w[1].timestamp - w[0].timestamp) as u32)
            .collect();

        Ok(ExportableProfile {
            nodes: nodes_vec,
            start_time,
            end_time,
            samples: Some(sample_ids),
            time_deltas: Some(time_deltas),
            title,
        })
    }

    /// Add a sample to the profile
    pub fn add_sample(&self, call_stack: Vec<CallFrame>) -> Result<(), String> {
        if !self.profiling_active.load(Ordering::SeqCst) {
            return Err("Profiling not active".to_string());
        }

        let timestamp = Self::get_timestamp_micros();
        let mut nodes = self.nodes.write();

        // Build the node path from root to leaf
        let mut current_parent_id = 0u32;
        let mut leaf_node_id = 0u32;

        for frame in call_stack.iter() {
            // Check if this call frame already exists as a child of current parent
            let existing_child = {
                if let Some(parent) = nodes.get(&current_parent_id) {
                    if let Some(ref children) = parent.children {
                        children.iter().find(|&&child_id| {
                            if let Some(child) = nodes.get(&child_id) {
                                child.call_frame.function_name == frame.function_name
                                    && child.call_frame.script_id == frame.script_id
                                    && child.call_frame.line_number == frame.line_number
                            } else {
                                false
                            }
                        }).copied()
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            if let Some(child_id) = existing_child {
                // Use existing node
                current_parent_id = child_id;
                leaf_node_id = child_id;
            } else {
                // Create new node
                let new_id = self.next_node_id.fetch_add(1, Ordering::SeqCst);
                let new_node = EnhancedProfileNode {
                    id: new_id,
                    call_frame: frame.clone(),
                    hit_count: Some(0),
                    children: Some(Vec::new()),
                    deopt_reason: None,
                    position_ticks: None,
                };

                // Add new node
                nodes.insert(new_id, new_node);

                // Update parent's children
                if let Some(parent) = nodes.get_mut(&current_parent_id) {
                    if let Some(ref mut children) = parent.children {
                        children.push(new_id);
                    }
                }

                current_parent_id = new_id;
                leaf_node_id = new_id;
            }
        }

        // Increment hit count on leaf node
        if let Some(leaf) = nodes.get_mut(&leaf_node_id) {
            if let Some(ref mut hit_count) = leaf.hit_count {
                *hit_count += 1;
            }

            // Add position tick
            if leaf.call_frame.line_number > 0 {
                let line = leaf.call_frame.line_number as u32;
                if let Some(ref mut ticks) = leaf.position_ticks {
                    if let Some(tick) = ticks.iter_mut().find(|t| t.line == line) {
                        tick.ticks += 1;
                    } else {
                        ticks.push(PositionTickInfo { line, ticks: 1 });
                    }
                } else {
                    leaf.position_ticks = Some(vec![PositionTickInfo { line, ticks: 1 }]);
                }
            }
        }

        drop(nodes);

        // Record sample
        self.samples.write().push(CpuSample {
            node_id: leaf_node_id,
            timestamp,
        });

        Ok(())
    }

    /// Generate a call tree from the current profile
    pub fn generate_call_tree(&self) -> CallTreeNode {
        let nodes = self.nodes.read();
        let samples = self.samples.read();

        // Calculate timing
        let total_samples = samples.len() as f64;
        let sampling_interval = self.sampling_interval.load(Ordering::SeqCst) as f64;
        let total_time = total_samples * sampling_interval;

        // Build tree recursively from root
        self.build_call_tree_node(0, &nodes, total_time, sampling_interval)
    }

    fn build_call_tree_node(
        &self,
        node_id: u32,
        nodes: &HashMap<u32, EnhancedProfileNode>,
        total_time: f64,
        sampling_interval: f64,
    ) -> CallTreeNode {
        let node = nodes.get(&node_id).cloned().unwrap_or_else(|| EnhancedProfileNode {
            id: node_id,
            call_frame: CallFrame::default(),
            hit_count: Some(0),
            children: None,
            deopt_reason: None,
            position_ticks: None,
        });

        let hit_count = node.hit_count.unwrap_or(0);
        let self_time = hit_count as f64 * sampling_interval;

        let children: Vec<CallTreeNode> = node
            .children
            .unwrap_or_default()
            .iter()
            .map(|&child_id| {
                self.build_call_tree_node(child_id, nodes, total_time, sampling_interval)
            })
            .collect();

        let children_time: f64 = children.iter().map(|c| c.total_time).sum();

        CallTreeNode {
            function_name: node.call_frame.function_name,
            total_time: self_time + children_time,
            self_time,
            hit_count,
            children,
            url: node.call_frame.url,
            line_number: node.call_frame.line_number,
        }
    }

    /// Export profile to Chrome DevTools format (JSON)
    pub fn export_profile(&self) -> Result<String, String> {
        if self.profiling_active.load(Ordering::SeqCst) {
            return Err("Cannot export while profiling is active".to_string());
        }

        let start_time = *self.start_time.read();
        let samples = self.samples.read().clone();
        let nodes = self.nodes.read().clone();
        let title = self.title.read().clone();

        if nodes.is_empty() {
            return Err("No profile data available".to_string());
        }

        let end_time = samples.last().map(|s| s.timestamp).unwrap_or(start_time);

        let mut nodes_vec: Vec<EnhancedProfileNode> = nodes.into_values().collect();
        nodes_vec.sort_by_key(|n| n.id);

        let sample_ids: Vec<u32> = samples.iter().map(|s| s.node_id).collect();
        let time_deltas: Vec<u32> = samples
            .windows(2)
            .map(|w| (w[1].timestamp - w[0].timestamp) as u32)
            .collect();

        let profile = ExportableProfile {
            nodes: nodes_vec,
            start_time,
            end_time,
            samples: Some(sample_ids),
            time_deltas: Some(time_deltas),
            title,
        };

        serde_json::to_string_pretty(&profile)
            .map_err(|e| format!("Failed to serialize profile: {}", e))
    }

    /// Get profile statistics
    pub fn get_stats(&self) -> ProfileStats {
        let samples = self.samples.read();
        let nodes = self.nodes.read();

        let total_samples = samples.len() as u64;
        let total_nodes = nodes.len() as u64;
        let start_time = *self.start_time.read();
        let end_time = samples.last().map(|s| s.timestamp).unwrap_or(start_time);
        let duration = end_time - start_time;

        // Find hottest functions
        let mut hot_functions: Vec<(String, u32)> = nodes
            .values()
            .filter_map(|n| {
                n.hit_count.map(|count| (n.call_frame.function_name.clone(), count))
            })
            .filter(|(name, _)| !name.is_empty() && name != "(root)")
            .collect();
        hot_functions.sort_by(|a, b| b.1.cmp(&a.1));
        hot_functions.truncate(10);

        ProfileStats {
            total_samples,
            total_nodes,
            duration_micros: duration,
            hot_functions,
        }
    }
}

impl Default for CpuProfiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Profile statistics summary
#[derive(Debug, Clone)]
pub struct ProfileStats {
    /// Total number of samples collected
    pub total_samples: u64,
    /// Total number of unique call tree nodes
    pub total_nodes: u64,
    /// Duration of profiling in microseconds
    pub duration_micros: f64,
    /// Top 10 hottest functions (name, hit count)
    pub hot_functions: Vec<(String, u32)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_profiler_new() {
        let profiler = CpuProfiler::new();
        assert!(!profiler.is_profiling());
        assert_eq!(profiler.get_sampling_interval(), 100);
    }

    #[test]
    fn test_set_sampling_interval() {
        let profiler = CpuProfiler::new();
        profiler.set_sampling_interval(500);
        assert_eq!(profiler.get_sampling_interval(), 500);
    }

    #[test]
    fn test_start_stop_profiling() {
        let profiler = CpuProfiler::new();

        assert!(profiler.start().is_ok());
        assert!(profiler.is_profiling());

        let profile = profiler.stop();
        assert!(profile.is_ok());
        assert!(!profiler.is_profiling());
    }

    #[test]
    fn test_add_sample() {
        let profiler = CpuProfiler::new();
        profiler.start().unwrap();

        let call_stack = vec![
            CallFrame {
                function_name: "main".to_string(),
                script_id: "1".to_string(),
                url: "http://example.com/script.js".to_string(),
                line_number: 10,
                column_number: 5,
            },
            CallFrame {
                function_name: "helper".to_string(),
                script_id: "1".to_string(),
                url: "http://example.com/script.js".to_string(),
                line_number: 50,
                column_number: 10,
            },
        ];

        assert!(profiler.add_sample(call_stack).is_ok());

        let profile = profiler.stop().unwrap();
        assert!(profile.nodes.len() > 1);
    }

    #[test]
    fn test_generate_call_tree() {
        let profiler = CpuProfiler::new();
        profiler.start().unwrap();

        for _ in 0..5 {
            let call_stack = vec![CallFrame {
                function_name: "main".to_string(),
                script_id: "1".to_string(),
                url: "test.js".to_string(),
                line_number: 1,
                column_number: 0,
            }];
            profiler.add_sample(call_stack).unwrap();
        }

        profiler.stop().unwrap();

        let tree = profiler.generate_call_tree();
        assert_eq!(tree.function_name, "(root)");
    }

    #[test]
    fn test_export_requires_stopped() {
        let profiler = CpuProfiler::new();
        profiler.start().unwrap();

        let result = profiler.export_profile();
        assert!(result.is_err());
    }

    #[test]
    fn test_profile_stats() {
        let profiler = CpuProfiler::new();
        profiler.start().unwrap();

        for i in 0..10 {
            let call_stack = vec![CallFrame {
                function_name: format!("func_{}", i % 3),
                script_id: "1".to_string(),
                url: "test.js".to_string(),
                line_number: i,
                column_number: 0,
            }];
            profiler.add_sample(call_stack).unwrap();
        }

        profiler.stop().unwrap();

        let stats = profiler.get_stats();
        assert_eq!(stats.total_samples, 10);
        assert!(!stats.hot_functions.is_empty());
    }
}
