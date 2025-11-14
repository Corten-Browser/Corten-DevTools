//! HeapProfilerDomain implementation
//!
//! Handles heap profiling and memory snapshots for the Chrome DevTools Protocol.

use async_trait::async_trait;
use cdp_types::CdpError;
use parking_lot::RwLock;
use protocol_handler::DomainHandler;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{debug, warn};

use crate::types::{SamplingHeapProfile, SamplingHeapProfileNode};

/// HeapProfilerDomain handles heap profiling and memory snapshots
#[derive(Debug)]
pub struct HeapProfilerDomain {
    /// Whether heap sampling is currently active
    sampling_active: Arc<AtomicBool>,
    /// Current sampling interval
    sampling_interval: Arc<RwLock<u32>>,
    /// Whether the domain is enabled
    enabled: Arc<AtomicBool>,
    /// Whether heap object tracking is active
    tracking_active: Arc<AtomicBool>,
}

impl HeapProfilerDomain {
    /// Create a new HeapProfilerDomain instance
    pub fn new() -> Self {
        Self {
            sampling_active: Arc::new(AtomicBool::new(false)),
            sampling_interval: Arc::new(RwLock::new(32768)), // Default 32KB
            enabled: Arc::new(AtomicBool::new(false)),
            tracking_active: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Check if heap sampling is currently active
    pub fn is_sampling(&self) -> bool {
        self.sampling_active.load(Ordering::SeqCst)
    }

    /// Handle the enable method
    fn handle_enable(&self) -> Result<Value, CdpError> {
        debug!("HeapProfiler.enable called");
        self.enabled.store(true, Ordering::SeqCst);
        Ok(json!({}))
    }

    /// Handle the disable method
    fn handle_disable(&self) -> Result<Value, CdpError> {
        debug!("HeapProfiler.disable called");
        self.enabled.store(false, Ordering::SeqCst);
        self.sampling_active.store(false, Ordering::SeqCst);
        self.tracking_active.store(false, Ordering::SeqCst);
        Ok(json!({}))
    }

    /// Handle the startSampling method
    fn handle_start_sampling(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("HeapProfiler.startSampling called");

        if !self.enabled.load(Ordering::SeqCst) {
            return Err(CdpError::invalid_request());
        }

        // Extract sampling interval if provided
        if let Some(params) = params {
            if let Some(interval) = params.get("samplingInterval").and_then(|v| v.as_u64()) {
                *self.sampling_interval.write() = interval as u32;
            }
        }

        self.sampling_active.store(true, Ordering::SeqCst);

        Ok(json!({}))
    }

    /// Handle the stopSampling method
    fn handle_stop_sampling(&self) -> Result<Value, CdpError> {
        debug!("HeapProfiler.stopSampling called");

        if !self.sampling_active.load(Ordering::SeqCst) {
            return Err(CdpError::invalid_request());
        }

        self.sampling_active.store(false, Ordering::SeqCst);

        // Generate mock sampling profile
        let profile = self.generate_mock_sampling_profile();

        Ok(json!({ "profile": profile }))
    }

    /// Handle the collectGarbage method
    fn handle_collect_garbage(&self) -> Result<Value, CdpError> {
        debug!("HeapProfiler.collectGarbage called");

        if !self.enabled.load(Ordering::SeqCst) {
            return Err(CdpError::invalid_request());
        }

        // Mock garbage collection (in real implementation, this would trigger GC)
        Ok(json!({}))
    }

    /// Handle the takeHeapSnapshot method
    fn handle_take_heap_snapshot(&self, _params: Option<Value>) -> Result<Value, CdpError> {
        debug!("HeapProfiler.takeHeapSnapshot called");

        if !self.enabled.load(Ordering::SeqCst) {
            return Err(CdpError::invalid_request());
        }

        // In a real implementation, this would stream the heap snapshot
        // For now, we just return success
        Ok(json!({}))
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
        _params: Option<Value>,
    ) -> Result<Value, CdpError> {
        debug!("HeapProfiler.startTrackingHeapObjects called");

        if !self.enabled.load(Ordering::SeqCst) {
            return Err(CdpError::invalid_request());
        }

        self.tracking_active.store(true, Ordering::SeqCst);

        Ok(json!({}))
    }

    /// Handle the stopTrackingHeapObjects method
    fn handle_stop_tracking_heap_objects(&self, _params: Option<Value>) -> Result<Value, CdpError> {
        debug!("HeapProfiler.stopTrackingHeapObjects called");

        if !self.tracking_active.load(Ordering::SeqCst) {
            return Err(CdpError::invalid_request());
        }

        self.tracking_active.store(false, Ordering::SeqCst);

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
            "collectGarbage" => self.handle_collect_garbage(),
            "takeHeapSnapshot" => self.handle_take_heap_snapshot(params),
            "getHeapObjectId" => self.handle_get_heap_object_id(params),
            "getObjectByHeapObjectId" => self.handle_get_object_by_heap_object_id(params),
            "startTrackingHeapObjects" => self.handle_start_tracking_heap_objects(params),
            "stopTrackingHeapObjects" => self.handle_stop_tracking_heap_objects(params),
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
