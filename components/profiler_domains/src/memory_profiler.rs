//! Memory Profiler implementation
//!
//! Provides memory allocation tracking, timeline recording, and
//! basic leak detection capabilities.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::debug;

use crate::types::{
    AllocationEntry, AllocationSummary, AllocationTrackingState, CallFrame, MemoryProfilerConfig,
    MemorySnapshot, MemoryTimelineEntry, PotentialLeak,
};

/// Memory Profiler for tracking allocations and detecting leaks
#[derive(Debug)]
pub struct MemoryProfiler {
    /// Whether tracking is currently active
    tracking_active: Arc<AtomicBool>,
    /// Allocation tracking state
    state: Arc<RwLock<AllocationTrackingState>>,
    /// Next allocation ID
    next_alloc_id: Arc<AtomicU64>,
    /// Last timeline sample timestamp
    last_timeline_sample: Arc<RwLock<f64>>,
}

impl MemoryProfiler {
    /// Create a new memory profiler instance
    pub fn new() -> Self {
        Self {
            tracking_active: Arc::new(AtomicBool::new(false)),
            state: Arc::new(RwLock::new(AllocationTrackingState::default())),
            next_alloc_id: Arc::new(AtomicU64::new(1)),
            last_timeline_sample: Arc::new(RwLock::new(0.0)),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: MemoryProfilerConfig) -> Self {
        let profiler = Self::new();
        profiler.state.write().config = config;
        profiler
    }

    /// Get current timestamp in microseconds
    fn get_timestamp_micros() -> f64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_micros() as f64)
            .unwrap_or(0.0)
    }

    /// Check if tracking is active
    pub fn is_tracking(&self) -> bool {
        self.tracking_active.load(Ordering::SeqCst)
    }

    /// Get current configuration
    pub fn get_config(&self) -> MemoryProfilerConfig {
        self.state.read().config.clone()
    }

    /// Update configuration
    pub fn set_config(&self, config: MemoryProfilerConfig) {
        self.state.write().config = config;
    }

    /// Start allocation tracking
    pub fn start_tracking(&self) -> Result<(), String> {
        if self.tracking_active.load(Ordering::SeqCst) {
            return Err("Tracking already active".to_string());
        }

        debug!("Starting memory profiler tracking");

        // Clear previous state
        {
            let mut state = self.state.write();
            state.allocations.clear();
            state.timeline.clear();
            state.total_allocated = 0;
            state.total_freed = 0;
        }

        self.next_alloc_id.store(1, Ordering::SeqCst);
        *self.last_timeline_sample.write() = Self::get_timestamp_micros();
        self.tracking_active.store(true, Ordering::SeqCst);

        // Record initial timeline entry
        self.record_timeline_entry();

        Ok(())
    }

    /// Stop allocation tracking
    pub fn stop_tracking(&self) -> Result<MemorySnapshot, String> {
        if !self.tracking_active.load(Ordering::SeqCst) {
            return Err("Tracking not active".to_string());
        }

        debug!("Stopping memory profiler tracking");
        self.tracking_active.store(false, Ordering::SeqCst);

        // Generate final snapshot
        self.take_snapshot()
    }

    /// Record an allocation
    pub fn record_allocation(&self, size: u64, stack_trace: Vec<CallFrame>) -> Result<u64, String> {
        if !self.tracking_active.load(Ordering::SeqCst) {
            return Err("Tracking not active".to_string());
        }

        let config = self.state.read().config.clone();

        // Skip small allocations based on config
        if size < config.min_allocation_size {
            return Ok(0);
        }

        let id = self.next_alloc_id.fetch_add(1, Ordering::SeqCst);
        let timestamp = Self::get_timestamp_micros();

        // Truncate stack trace if needed
        let truncated_trace: Vec<CallFrame> = if config.capture_stack_traces {
            stack_trace
                .into_iter()
                .take(config.max_stack_depth as usize)
                .collect()
        } else {
            Vec::new()
        };

        let entry = AllocationEntry {
            id,
            size,
            timestamp,
            stack_trace: truncated_trace,
            freed: false,
            freed_timestamp: None,
        };

        {
            let mut state = self.state.write();
            state.allocations.insert(id, entry);
            state.total_allocated += size;
        }

        // Maybe record timeline entry
        self.maybe_record_timeline();

        Ok(id)
    }

    /// Record a deallocation
    pub fn record_deallocation(&self, allocation_id: u64) -> Result<(), String> {
        if !self.tracking_active.load(Ordering::SeqCst) {
            return Err("Tracking not active".to_string());
        }

        let timestamp = Self::get_timestamp_micros();

        let mut state = self.state.write();

        if let Some(entry) = state.allocations.get_mut(&allocation_id) {
            if entry.freed {
                return Err(format!("Allocation {} already freed (double free)", allocation_id));
            }

            entry.freed = true;
            entry.freed_timestamp = Some(timestamp);
            state.total_freed += entry.size;
        } else {
            return Err(format!("Unknown allocation ID: {}", allocation_id));
        }

        drop(state);

        // Maybe record timeline entry
        self.maybe_record_timeline();

        Ok(())
    }

    /// Maybe record a timeline entry based on interval
    fn maybe_record_timeline(&self) {
        let config = self.state.read().config.clone();
        let current_time = Self::get_timestamp_micros();
        let last_sample = *self.last_timeline_sample.read();

        if current_time - last_sample >= config.timeline_interval {
            self.record_timeline_entry();
            *self.last_timeline_sample.write() = current_time;
        }
    }

    /// Record a timeline entry
    fn record_timeline_entry(&self) {
        let timestamp = Self::get_timestamp_micros();
        let state = self.state.read();

        let live_allocations: u32 = state
            .allocations
            .values()
            .filter(|a| !a.freed)
            .count() as u32;

        let current_heap_size: u64 = state
            .allocations
            .values()
            .filter(|a| !a.freed)
            .map(|a| a.size)
            .sum();

        drop(state);

        let entry = MemoryTimelineEntry {
            timestamp,
            total_allocated: self.state.read().total_allocated,
            total_freed: self.state.read().total_freed,
            current_heap_size,
            live_allocations,
        };

        self.state.write().timeline.push(entry);
    }

    /// Take a memory snapshot
    pub fn take_snapshot(&self) -> Result<MemorySnapshot, String> {
        let timestamp = Self::get_timestamp_micros();
        let state = self.state.read();
        let config = state.config.clone();

        // Calculate summary by call site
        let allocation_summaries = self.compute_allocation_summaries(&state.allocations);

        // Detect potential leaks
        let potential_leaks = self.detect_potential_leaks(&state.allocations, timestamp, &config);

        // Calculate heap sizes
        let total_heap_size = state.total_allocated;
        let used_heap_size: u64 = state
            .allocations
            .values()
            .filter(|a| !a.freed)
            .map(|a| a.size)
            .sum();

        let timeline = state.timeline.clone();

        Ok(MemorySnapshot {
            timestamp,
            total_heap_size,
            used_heap_size,
            allocation_summaries,
            potential_leaks,
            timeline,
        })
    }

    /// Compute allocation summaries by call site
    fn compute_allocation_summaries(
        &self,
        allocations: &HashMap<u64, AllocationEntry>,
    ) -> Vec<AllocationSummary> {
        let mut by_call_site: HashMap<String, (u64, u64, u64)> = HashMap::new();

        for alloc in allocations.values() {
            let call_site = if alloc.stack_trace.is_empty() {
                "(unknown)".to_string()
            } else {
                let frame = &alloc.stack_trace[0];
                format!("{}:{}", frame.function_name, frame.line_number)
            };

            let entry = by_call_site.entry(call_site).or_insert((0, 0, 0));
            entry.0 += 1; // count
            entry.1 += alloc.size; // total bytes
            if !alloc.freed {
                entry.2 += alloc.size; // live bytes
            }
        }

        by_call_site
            .into_iter()
            .map(|(call_site, (count, total, live))| AllocationSummary {
                call_site,
                allocation_count: count,
                total_bytes: total,
                live_bytes: live,
                average_size: if count > 0 { total as f64 / count as f64 } else { 0.0 },
            })
            .collect()
    }

    /// Detect potential memory leaks
    fn detect_potential_leaks(
        &self,
        allocations: &HashMap<u64, AllocationEntry>,
        current_time: f64,
        config: &MemoryProfilerConfig,
    ) -> Vec<PotentialLeak> {
        let mut leaks = Vec::new();

        for alloc in allocations.values() {
            if alloc.freed {
                continue;
            }

            let age = current_time - alloc.timestamp;

            // Check if allocation is old enough to be suspicious
            if age < config.leak_threshold_age {
                continue;
            }

            // Calculate leak score based on various factors
            let mut leak_score = 0u32;
            let mut reasons = Vec::new();

            // Age-based scoring
            let age_factor = (age / config.leak_threshold_age) as u32;
            leak_score += (age_factor * 10).min(50);
            if age_factor > 1 {
                reasons.push(format!("{}x longer than threshold", age_factor));
            }

            // Size-based scoring (larger allocations are more suspicious)
            if alloc.size > 1_000_000 {
                // > 1MB
                leak_score += 30;
                reasons.push("Large allocation (>1MB)".to_string());
            } else if alloc.size > 100_000 {
                // > 100KB
                leak_score += 15;
                reasons.push("Medium allocation (>100KB)".to_string());
            }

            // Unknown origin is suspicious
            if alloc.stack_trace.is_empty() {
                leak_score += 10;
                reasons.push("Unknown allocation source".to_string());
            }

            // Only report if score is significant
            if leak_score >= 20 {
                leaks.push(PotentialLeak {
                    allocation_id: alloc.id,
                    size: alloc.size,
                    age,
                    stack_trace: alloc.stack_trace.clone(),
                    leak_score: leak_score.min(100),
                    reason: reasons.join("; "),
                });
            }
        }

        // Sort by leak score (highest first)
        leaks.sort_by(|a, b| b.leak_score.cmp(&a.leak_score));

        // Return top potential leaks
        leaks.truncate(50);
        leaks
    }

    /// Get current memory statistics
    pub fn get_stats(&self) -> MemoryStats {
        let state = self.state.read();

        let live_allocations: Vec<&AllocationEntry> =
            state.allocations.values().filter(|a| !a.freed).collect();

        let live_count = live_allocations.len() as u64;
        let live_bytes: u64 = live_allocations.iter().map(|a| a.size).sum();

        MemoryStats {
            total_allocations: state.allocations.len() as u64,
            live_allocations: live_count,
            freed_allocations: state.allocations.len() as u64 - live_count,
            total_allocated_bytes: state.total_allocated,
            total_freed_bytes: state.total_freed,
            current_heap_bytes: live_bytes,
            timeline_entries: state.timeline.len() as u64,
        }
    }

    /// Force garbage collection (simulation for CDP compatibility)
    pub fn force_gc(&self) {
        debug!("Memory profiler: force_gc called (simulated)");
        // In a real implementation, this would trigger actual GC
        // For now, we just record a timeline entry
        if self.tracking_active.load(Ordering::SeqCst) {
            self.record_timeline_entry();
        }
    }

    /// Get the memory timeline
    pub fn get_timeline(&self) -> Vec<MemoryTimelineEntry> {
        self.state.read().timeline.clone()
    }

    /// Get all live allocations
    pub fn get_live_allocations(&self) -> Vec<AllocationEntry> {
        self.state
            .read()
            .allocations
            .values()
            .filter(|a| !a.freed)
            .cloned()
            .collect()
    }
}

impl Default for MemoryProfiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory statistics summary
#[derive(Debug, Clone)]
pub struct MemoryStats {
    /// Total allocations tracked
    pub total_allocations: u64,
    /// Currently live allocations
    pub live_allocations: u64,
    /// Freed allocations
    pub freed_allocations: u64,
    /// Total bytes ever allocated
    pub total_allocated_bytes: u64,
    /// Total bytes freed
    pub total_freed_bytes: u64,
    /// Current heap size (live allocations)
    pub current_heap_bytes: u64,
    /// Number of timeline entries
    pub timeline_entries: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_profiler_new() {
        let profiler = MemoryProfiler::new();
        assert!(!profiler.is_tracking());
    }

    #[test]
    fn test_start_stop_tracking() {
        let profiler = MemoryProfiler::new();

        assert!(profiler.start_tracking().is_ok());
        assert!(profiler.is_tracking());

        let snapshot = profiler.stop_tracking();
        assert!(snapshot.is_ok());
        assert!(!profiler.is_tracking());
    }

    #[test]
    fn test_record_allocation() {
        let profiler = MemoryProfiler::new();
        profiler.start_tracking().unwrap();

        let stack_trace = vec![CallFrame {
            function_name: "test_func".to_string(),
            script_id: "1".to_string(),
            url: "test.js".to_string(),
            line_number: 10,
            column_number: 5,
        }];

        let id = profiler.record_allocation(2048, stack_trace).unwrap();
        assert!(id > 0);

        let stats = profiler.get_stats();
        assert_eq!(stats.live_allocations, 1);
        assert_eq!(stats.current_heap_bytes, 2048);
    }

    #[test]
    fn test_record_deallocation() {
        let profiler = MemoryProfiler::new();
        profiler.start_tracking().unwrap();

        let id = profiler.record_allocation(2048, vec![]).unwrap();
        assert!(profiler.record_deallocation(id).is_ok());

        let stats = profiler.get_stats();
        assert_eq!(stats.live_allocations, 0);
        assert_eq!(stats.freed_allocations, 1);
    }

    #[test]
    fn test_double_free_detection() {
        let profiler = MemoryProfiler::new();
        profiler.start_tracking().unwrap();

        let id = profiler.record_allocation(2048, vec![]).unwrap();
        assert!(profiler.record_deallocation(id).is_ok());
        assert!(profiler.record_deallocation(id).is_err());
    }

    #[test]
    fn test_take_snapshot() {
        let profiler = MemoryProfiler::new();
        profiler.start_tracking().unwrap();

        profiler.record_allocation(1024, vec![]).unwrap();
        profiler.record_allocation(2048, vec![]).unwrap();

        let snapshot = profiler.take_snapshot().unwrap();
        assert!(snapshot.used_heap_size > 0);
        assert!(!snapshot.allocation_summaries.is_empty());
    }

    #[test]
    fn test_potential_leak_detection() {
        let mut config = MemoryProfilerConfig::default();
        config.leak_threshold_age = 1.0; // Very short threshold for testing

        let profiler = MemoryProfiler::with_config(config);
        profiler.start_tracking().unwrap();

        // Record a large allocation
        profiler.record_allocation(2_000_000, vec![]).unwrap();

        // Wait a tiny bit
        std::thread::sleep(std::time::Duration::from_micros(10));

        let snapshot = profiler.take_snapshot().unwrap();

        // Should detect the large, old allocation as potential leak
        assert!(!snapshot.potential_leaks.is_empty());
    }

    #[test]
    fn test_allocation_summaries() {
        let profiler = MemoryProfiler::new();
        profiler.start_tracking().unwrap();

        let stack_trace = vec![CallFrame {
            function_name: "allocate".to_string(),
            script_id: "1".to_string(),
            url: "test.js".to_string(),
            line_number: 42,
            column_number: 0,
        }];

        // Multiple allocations from same call site
        for _ in 0..5 {
            profiler.record_allocation(1024, stack_trace.clone()).unwrap();
        }

        let snapshot = profiler.take_snapshot().unwrap();
        let summary = snapshot
            .allocation_summaries
            .iter()
            .find(|s| s.call_site.contains("allocate"))
            .unwrap();

        assert_eq!(summary.allocation_count, 5);
        assert_eq!(summary.total_bytes, 5 * 1024);
    }

    #[test]
    fn test_min_allocation_size_filter() {
        let mut config = MemoryProfilerConfig::default();
        config.min_allocation_size = 2048; // Only track allocations >= 2KB

        let profiler = MemoryProfiler::with_config(config);
        profiler.start_tracking().unwrap();

        // This should be skipped (too small)
        let id1 = profiler.record_allocation(1024, vec![]).unwrap();
        assert_eq!(id1, 0);

        // This should be tracked
        let id2 = profiler.record_allocation(4096, vec![]).unwrap();
        assert!(id2 > 0);

        let stats = profiler.get_stats();
        assert_eq!(stats.live_allocations, 1);
    }

    #[test]
    fn test_memory_timeline() {
        let mut config = MemoryProfilerConfig::default();
        config.timeline_interval = 1.0; // 1 microsecond for testing

        let profiler = MemoryProfiler::with_config(config);
        profiler.start_tracking().unwrap();

        for _ in 0..5 {
            profiler.record_allocation(1024, vec![]).unwrap();
            std::thread::sleep(std::time::Duration::from_micros(5));
        }

        let timeline = profiler.get_timeline();
        assert!(!timeline.is_empty());
    }
}
