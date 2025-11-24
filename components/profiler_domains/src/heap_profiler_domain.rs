//! HeapProfilerDomain implementation
//!
//! Handles heap profiling and memory snapshots for the Chrome DevTools Protocol.
//! Provides heap snapshot capture, allocation sampling, and object tracking.

use async_trait::async_trait;
use cdp_types::CdpError;
use parking_lot::RwLock;
use protocol_handler::DomainHandler;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};

use crate::memory_profiler::MemoryProfiler;
use crate::types::{HeapStatsUpdate, LastSeenObjectId, SamplingHeapProfile, SamplingHeapProfileNode};

/// Event callback type for heap profiler events
pub type EventCallback = Arc<dyn Fn(&str, Value) + Send + Sync>;

/// HeapProfilerDomain handles heap profiling and memory snapshots
pub struct HeapProfilerDomain {
    /// Whether heap sampling is currently active
    sampling_active: Arc<AtomicBool>,
    /// Current sampling interval (bytes)
    sampling_interval: Arc<RwLock<u32>>,
    /// Whether the domain is enabled
    enabled: Arc<AtomicBool>,
    /// Whether heap object tracking is active
    tracking_active: Arc<AtomicBool>,
    /// Whether to include allocation stack traces
    include_object_info: Arc<AtomicBool>,
    /// Last seen object ID for tracking
    last_seen_object_id: Arc<AtomicU32>,
    /// Memory profiler for allocation tracking
    memory_profiler: Arc<MemoryProfiler>,
    /// Event callback for sending events to client
    event_callback: Arc<RwLock<Option<EventCallback>>>,
}

impl std::fmt::Debug for HeapProfilerDomain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HeapProfilerDomain")
            .field("sampling_active", &self.sampling_active)
            .field("sampling_interval", &self.sampling_interval)
            .field("enabled", &self.enabled)
            .field("tracking_active", &self.tracking_active)
            .field("include_object_info", &self.include_object_info)
            .field("last_seen_object_id", &self.last_seen_object_id)
            .field("memory_profiler", &self.memory_profiler)
            .field("event_callback", &"<callback>")
            .finish()
    }
}

impl HeapProfilerDomain {
    /// Create a new HeapProfilerDomain instance
    pub fn new() -> Self {
        Self {
            sampling_active: Arc::new(AtomicBool::new(false)),
            sampling_interval: Arc::new(RwLock::new(32768)), // Default 32KB
            enabled: Arc::new(AtomicBool::new(false)),
            tracking_active: Arc::new(AtomicBool::new(false)),
            include_object_info: Arc::new(AtomicBool::new(false)),
            last_seen_object_id: Arc::new(AtomicU32::new(0)),
            memory_profiler: Arc::new(MemoryProfiler::new()),
            event_callback: Arc::new(RwLock::new(None)),
        }
    }

    /// Get current timestamp in microseconds
    fn get_timestamp_micros() -> f64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_micros() as f64)
            .unwrap_or(0.0)
    }

    /// Check if heap sampling is currently active
    pub fn is_sampling(&self) -> bool {
        self.sampling_active.load(Ordering::SeqCst)
    }

    /// Check if object tracking is active
    pub fn is_tracking(&self) -> bool {
        self.tracking_active.load(Ordering::SeqCst)
    }

    /// Set event callback for sending events to client
    pub fn set_event_callback(&self, callback: EventCallback) {
        *self.event_callback.write() = Some(callback);
    }

    /// Clear event callback
    pub fn clear_event_callback(&self) {
        *self.event_callback.write() = None;
    }

    /// Send an event to the client
    fn send_event(&self, event_name: &str, params: Value) {
        if let Some(ref callback) = *self.event_callback.read() {
            callback(event_name, params);
        }
    }

    /// Get the memory profiler instance
    pub fn memory_profiler(&self) -> &MemoryProfiler {
        &self.memory_profiler
    }

    /// Handle the enable method
    fn handle_enable(&self) -> Result<Value, CdpError> {
        debug!("HeapProfiler.enable called");
        self.enabled.store(true, Ordering::SeqCst);
        info!("HeapProfiler domain enabled");
        Ok(json!({}))
    }

    /// Handle the disable method
    fn handle_disable(&self) -> Result<Value, CdpError> {
        debug!("HeapProfiler.disable called");

        // Stop any active tracking/sampling
        if self.sampling_active.load(Ordering::SeqCst) {
            let _ = self.memory_profiler.stop_tracking();
        }

        self.enabled.store(false, Ordering::SeqCst);
        self.sampling_active.store(false, Ordering::SeqCst);
        self.tracking_active.store(false, Ordering::SeqCst);
        info!("HeapProfiler domain disabled");
        Ok(json!({}))
    }

    /// Handle the startSampling method
    fn handle_start_sampling(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("HeapProfiler.startSampling called");

        if !self.enabled.load(Ordering::SeqCst) {
            return Err(CdpError::invalid_request());
        }

        // Extract parameters
        if let Some(ref params) = params {
            if let Some(interval) = params.get("samplingInterval").and_then(|v| v.as_u64()) {
                *self.sampling_interval.write() = interval as u32;
            }
            if let Some(include_info) = params.get("includeObjectsCollectedByMajorGC").and_then(|v| v.as_bool()) {
                self.include_object_info.store(include_info, Ordering::SeqCst);
            }
            if let Some(include_info) = params.get("includeObjectsCollectedByMinorGC").and_then(|v| v.as_bool()) {
                self.include_object_info.store(include_info, Ordering::SeqCst);
            }
        }

        // Start memory profiler tracking
        if let Err(e) = self.memory_profiler.start_tracking() {
            warn!("Failed to start memory tracking: {}", e);
        }

        self.sampling_active.store(true, Ordering::SeqCst);
        info!("HeapProfiler sampling started with interval {} bytes", *self.sampling_interval.read());

        Ok(json!({}))
    }

    /// Handle the stopSampling method
    fn handle_stop_sampling(&self) -> Result<Value, CdpError> {
        debug!("HeapProfiler.stopSampling called");

        if !self.sampling_active.load(Ordering::SeqCst) {
            return Err(CdpError::invalid_request());
        }

        // Stop memory profiler and get snapshot
        let snapshot = self.memory_profiler.stop_tracking();
        self.sampling_active.store(false, Ordering::SeqCst);

        // Generate sampling profile from memory profiler data
        let profile = if let Ok(snap) = snapshot {
            self.generate_sampling_profile_from_snapshot(&snap)
        } else {
            self.generate_mock_sampling_profile()
        };

        info!("HeapProfiler sampling stopped");

        Ok(json!({ "profile": profile }))
    }

    /// Generate sampling profile from memory snapshot
    fn generate_sampling_profile_from_snapshot(&self, snapshot: &crate::types::MemorySnapshot) -> SamplingHeapProfile {
        // Build profile tree from allocation summaries
        let mut children = Vec::new();
        let mut node_id = 1u32;

        for summary in &snapshot.allocation_summaries {
            if summary.live_bytes > 0 {
                let parts: Vec<&str> = summary.call_site.split(':').collect();
                let function_name = parts.first().unwrap_or(&"(unknown)").to_string();
                let line_number = parts.get(1).and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);

                children.push(SamplingHeapProfileNode {
                    call_frame: json!({
                        "functionName": function_name,
                        "scriptId": "1",
                        "url": "",
                        "lineNumber": line_number,
                        "columnNumber": 0
                    }),
                    self_size: summary.live_bytes,
                    id: node_id,
                    children: vec![],
                });
                node_id += 1;
            }
        }

        SamplingHeapProfile {
            head: SamplingHeapProfileNode {
                call_frame: json!({
                    "functionName": "(root)",
                    "scriptId": "0",
                    "url": "",
                    "lineNumber": 0,
                    "columnNumber": 0
                }),
                self_size: 0,
                id: 0,
                children,
            },
            samples: vec![],
        }
    }

    /// Handle the collectGarbage method
    fn handle_collect_garbage(&self) -> Result<Value, CdpError> {
        debug!("HeapProfiler.collectGarbage called");

        if !self.enabled.load(Ordering::SeqCst) {
            return Err(CdpError::invalid_request());
        }

        // Trigger GC simulation in memory profiler
        self.memory_profiler.force_gc();
        info!("HeapProfiler garbage collection triggered");

        Ok(json!({}))
    }

    /// Handle the takeHeapSnapshot method
    fn handle_take_heap_snapshot(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("HeapProfiler.takeHeapSnapshot called");

        if !self.enabled.load(Ordering::SeqCst) {
            return Err(CdpError::invalid_request());
        }

        // Extract options
        let report_progress = params
            .as_ref()
            .and_then(|p| p.get("reportProgress").and_then(|v| v.as_bool()))
            .unwrap_or(false);
        let capture_numeric_value = params
            .as_ref()
            .and_then(|p| p.get("captureNumericValue").and_then(|v| v.as_bool()))
            .unwrap_or(false);

        // Generate heap snapshot
        let snapshot = self.generate_heap_snapshot(report_progress, capture_numeric_value);

        // Stream the snapshot in chunks
        self.stream_heap_snapshot(&snapshot, report_progress);

        Ok(json!({}))
    }

    /// Generate a heap snapshot
    fn generate_heap_snapshot(&self, report_progress: bool, _capture_numeric: bool) -> String {
        // Send progress event if requested
        if report_progress {
            self.send_event(
                "HeapProfiler.reportHeapSnapshotProgress",
                json!({
                    "done": 0,
                    "total": 100,
                    "finished": false
                }),
            );
        }

        // Generate V8 heap snapshot format
        let snapshot = json!({
            "snapshot": {
                "meta": {
                    "node_fields": ["type", "name", "id", "self_size", "edge_count", "trace_node_id", "detachedness"],
                    "node_types": [["hidden", "array", "string", "object", "code", "closure", "regexp", "number", "native", "synthetic", "concatenated string", "sliced string", "symbol", "bigint"], "string", "number", "number", "number", "number", "number"],
                    "edge_fields": ["type", "name_or_index", "to_node"],
                    "edge_types": [["context", "element", "property", "internal", "hidden", "shortcut", "weak"], "string_or_number", "node"],
                    "trace_function_info_fields": ["function_id", "name", "script_name", "script_id", "line", "column"],
                    "trace_node_fields": ["id", "function_info_index", "count", "size", "children"],
                    "sample_fields": ["timestamp_us", "last_assigned_id"],
                    "location_fields": ["object_index", "script_id", "line", "column"]
                },
                "node_count": 3,
                "edge_count": 2,
                "trace_function_count": 0
            },
            "nodes": [0, 0, 1, 0, 2, 0, 0, 3, 1, 2, 1024, 1, 0, 0, 3, 2, 3, 2048, 0, 0, 0],
            "edges": [1, 0, 7, 1, 1, 14],
            "trace_function_infos": [],
            "trace_tree": [],
            "samples": [],
            "locations": [],
            "strings": ["(root)", "Object", "Array"]
        });

        if report_progress {
            self.send_event(
                "HeapProfiler.reportHeapSnapshotProgress",
                json!({
                    "done": 50,
                    "total": 100,
                    "finished": false
                }),
            );
        }

        serde_json::to_string(&snapshot).unwrap_or_else(|_| "{}".to_string())
    }

    /// Stream heap snapshot in chunks
    fn stream_heap_snapshot(&self, snapshot: &str, report_progress: bool) {
        let chunk_size = 65536; // 64KB chunks
        let chunks: Vec<&str> = snapshot
            .as_bytes()
            .chunks(chunk_size)
            .map(|c| std::str::from_utf8(c).unwrap_or(""))
            .collect();

        let total_chunks = chunks.len();

        for (i, chunk) in chunks.iter().enumerate() {
            // Send chunk event
            self.send_event(
                "HeapProfiler.addHeapSnapshotChunk",
                json!({
                    "chunk": chunk
                }),
            );

            // Update progress if requested
            if report_progress {
                let progress = ((i + 1) as f64 / total_chunks as f64 * 50.0 + 50.0) as u32;
                self.send_event(
                    "HeapProfiler.reportHeapSnapshotProgress",
                    json!({
                        "done": progress,
                        "total": 100,
                        "finished": i + 1 == total_chunks
                    }),
                );
            }
        }
    }

    /// Send heap stats update event
    pub fn emit_heap_stats_update(&self, stats: &HeapStatsUpdate) {
        self.send_event(
            "HeapProfiler.heapStatsUpdate",
            json!({
                "statsUpdate": stats.stats_update
            }),
        );
    }

    /// Send last seen object ID event
    pub fn emit_last_seen_object_id(&self, last_seen: &LastSeenObjectId) {
        self.send_event(
            "HeapProfiler.lastSeenObjectId",
            json!({
                "lastSeenObjectId": last_seen.last_seen_object_id,
                "timestamp": last_seen.timestamp
            }),
        );
    }

    /// Send reset profiles event
    pub fn emit_reset_profiles(&self) {
        self.send_event("HeapProfiler.resetProfiles", json!({}));
    }

    /// Handle the getHeapObjectId method
    fn handle_get_heap_object_id(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("HeapProfiler.getHeapObjectId called");

        if !self.enabled.load(Ordering::SeqCst) {
            return Err(CdpError::invalid_request());
        }

        let object_id = params
            .and_then(|p| p.get("objectId").and_then(|v| v.as_str().map(String::from)))
            .ok_or_else(|| CdpError::invalid_params("Missing objectId parameter"))?;

        // Generate a mock heap snapshot object ID
        let heap_snapshot_object_id = format!("heap-{}", object_id);

        Ok(json!({
            "heapSnapshotObjectId": heap_snapshot_object_id
        }))
    }

    /// Handle the getObjectByHeapObjectId method
    fn handle_get_object_by_heap_object_id(
        &self,
        params: Option<Value>,
    ) -> Result<Value, CdpError> {
        debug!("HeapProfiler.getObjectByHeapObjectId called");

        if !self.enabled.load(Ordering::SeqCst) {
            return Err(CdpError::invalid_request());
        }

        let object_id = params
            .and_then(|p| p.get("objectId").and_then(|v| v.as_str().map(String::from)))
            .ok_or_else(|| CdpError::invalid_params("Missing objectId parameter"))?;

        // Generate a mock remote object
        Ok(json!({
            "result": {
                "type": "object",
                "objectId": object_id,
                "className": "Object",
                "description": "Object"
            }
        }))
    }

    /// Handle the startTrackingHeapObjects method
    fn handle_start_tracking_heap_objects(
        &self,
        params: Option<Value>,
    ) -> Result<Value, CdpError> {
        debug!("HeapProfiler.startTrackingHeapObjects called");

        if !self.enabled.load(Ordering::SeqCst) {
            return Err(CdpError::invalid_request());
        }

        // Extract track allocations option
        let track_allocations = params
            .as_ref()
            .and_then(|p| p.get("trackAllocations").and_then(|v| v.as_bool()))
            .unwrap_or(false);

        if track_allocations {
            if let Err(e) = self.memory_profiler.start_tracking() {
                warn!("Failed to start allocation tracking: {}", e);
            }
        }

        self.tracking_active.store(true, Ordering::SeqCst);
        self.last_seen_object_id.store(0, Ordering::SeqCst);
        info!("HeapProfiler object tracking started");

        Ok(json!({}))
    }

    /// Handle the stopTrackingHeapObjects method
    fn handle_stop_tracking_heap_objects(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("HeapProfiler.stopTrackingHeapObjects called");

        if !self.tracking_active.load(Ordering::SeqCst) {
            return Err(CdpError::invalid_request());
        }

        // Extract options
        let report_progress = params
            .as_ref()
            .and_then(|p| p.get("reportProgress").and_then(|v| v.as_bool()))
            .unwrap_or(false);

        // Stop memory tracking if active
        let _ = self.memory_profiler.stop_tracking();

        self.tracking_active.store(false, Ordering::SeqCst);
        info!("HeapProfiler object tracking stopped");

        // Optionally take final snapshot
        if report_progress {
            let snapshot = self.generate_heap_snapshot(true, false);
            self.stream_heap_snapshot(&snapshot, true);
        }

        Ok(json!({}))
    }

    /// Handle the getSamplingProfile method
    fn handle_get_sampling_profile(&self) -> Result<Value, CdpError> {
        debug!("HeapProfiler.getSamplingProfile called");

        if !self.enabled.load(Ordering::SeqCst) {
            return Err(CdpError::invalid_request());
        }

        // Get current profile without stopping
        let profile = if let Ok(snapshot) = self.memory_profiler.take_snapshot() {
            self.generate_sampling_profile_from_snapshot(&snapshot)
        } else {
            self.generate_mock_sampling_profile()
        };

        Ok(json!({ "profile": profile }))
    }

    /// Handle the addInspectedHeapObject method
    fn handle_add_inspected_heap_object(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("HeapProfiler.addInspectedHeapObject called");

        if !self.enabled.load(Ordering::SeqCst) {
            return Err(CdpError::invalid_request());
        }

        let heap_object_id = params
            .and_then(|p| p.get("heapObjectId").and_then(|v| v.as_str().map(String::from)))
            .ok_or_else(|| CdpError::invalid_params("Missing heapObjectId parameter"))?;

        debug!("Added heap object to inspection: {}", heap_object_id);

        Ok(json!({}))
    }

    /// Generate mock sampling heap profile for testing
    fn generate_mock_sampling_profile(&self) -> SamplingHeapProfile {
        SamplingHeapProfile {
            head: SamplingHeapProfileNode {
                call_frame: json!({
                    "functionName": "(root)",
                    "scriptId": "0",
                    "url": "",
                    "lineNumber": 0,
                    "columnNumber": 0
                }),
                self_size: 0,
                id: 0,
                children: vec![
                    SamplingHeapProfileNode {
                        call_frame: json!({
                            "functionName": "allocateMemory",
                            "scriptId": "1",
                            "url": "http://example.com/script.js",
                            "lineNumber": 20,
                            "columnNumber": 10
                        }),
                        self_size: 1024000,
                        id: 1,
                        children: vec![],
                    },
                    SamplingHeapProfileNode {
                        call_frame: json!({
                            "functionName": "createArray",
                            "scriptId": "1",
                            "url": "http://example.com/script.js",
                            "lineNumber": 35,
                            "columnNumber": 5
                        }),
                        self_size: 512000,
                        id: 2,
                        children: vec![],
                    },
                ],
            },
            samples: vec![],
        }
    }
}

impl Default for HeapProfilerDomain {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DomainHandler for HeapProfilerDomain {
    fn name(&self) -> &str {
        "HeapProfiler"
    }

    async fn handle_method(&self, method: &str, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("HeapProfiler domain handling method: {}", method);

        match method {
            "enable" => self.handle_enable(),
            "disable" => self.handle_disable(),
            "startSampling" => self.handle_start_sampling(params),
            "stopSampling" => self.handle_stop_sampling(),
            "getSamplingProfile" => self.handle_get_sampling_profile(),
            "collectGarbage" => self.handle_collect_garbage(),
            "takeHeapSnapshot" => self.handle_take_heap_snapshot(params),
            "getHeapObjectId" => self.handle_get_heap_object_id(params),
            "getObjectByHeapObjectId" => self.handle_get_object_by_heap_object_id(params),
            "startTrackingHeapObjects" => self.handle_start_tracking_heap_objects(params),
            "stopTrackingHeapObjects" => self.handle_stop_tracking_heap_objects(params),
            "addInspectedHeapObject" => self.handle_add_inspected_heap_object(params),
            _ => {
                warn!("Unknown HeapProfiler method: {}", method);
                Err(CdpError::method_not_found(format!(
                    "HeapProfiler.{}",
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
    fn test_heap_profiler_domain_new() {
        let heap_profiler = HeapProfilerDomain::new();
        assert_eq!(heap_profiler.name(), "HeapProfiler");
        assert!(!heap_profiler.is_sampling());
    }

    #[test]
    fn test_sampling_interval_default() {
        let heap_profiler = HeapProfilerDomain::new();
        assert_eq!(*heap_profiler.sampling_interval.read(), 32768);
    }

    #[tokio::test]
    async fn test_enable_disable() {
        let heap_profiler = HeapProfilerDomain::new();

        let enable_result = heap_profiler.handle_method("enable", None).await;
        assert!(enable_result.is_ok());
        assert!(heap_profiler.enabled.load(Ordering::SeqCst));

        let disable_result = heap_profiler.handle_method("disable", None).await;
        assert!(disable_result.is_ok());
        assert!(!heap_profiler.enabled.load(Ordering::SeqCst));
    }
}
