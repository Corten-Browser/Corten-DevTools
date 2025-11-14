//! ProfilerDomain implementation
//!
//! Handles CPU profiling and code coverage for the Chrome DevTools Protocol.

use async_trait::async_trait;
use cdp_types::CdpError;
use parking_lot::RwLock;
use protocol_handler::DomainHandler;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, warn};

use crate::types::{CoverageRange, FunctionCoverage, Profile, ProfileNode, ScriptCoverage};

/// ProfilerDomain handles CPU profiling and code coverage
#[derive(Debug)]
pub struct ProfilerDomain {
    /// Whether profiling is currently active
    profiling_active: Arc<AtomicBool>,
    /// Whether code coverage is currently active
    coverage_active: Arc<AtomicBool>,
    /// Stored coverage data
    coverage_data: Arc<RwLock<Vec<ScriptCoverage>>>,
    /// Start time of profiling
    profile_start_time: Arc<RwLock<f64>>,
    /// Whether the domain is enabled
    enabled: Arc<AtomicBool>,
}

impl ProfilerDomain {
    /// Create a new ProfilerDomain instance
    pub fn new() -> Self {
        Self {
            profiling_active: Arc::new(AtomicBool::new(false)),
            coverage_active: Arc::new(AtomicBool::new(false)),
            coverage_data: Arc::new(RwLock::new(Vec::new())),
            profile_start_time: Arc::new(RwLock::new(0.0)),
            enabled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Check if profiling is currently active
    pub fn is_profiling(&self) -> bool {
        self.profiling_active.load(Ordering::SeqCst)
    }

    /// Check if coverage is currently active
    pub fn is_coverage_active(&self) -> bool {
        self.coverage_active.load(Ordering::SeqCst)
    }

    /// Get current timestamp in microseconds
    fn get_timestamp_micros() -> f64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_micros() as f64)
            .unwrap_or(0.0)
    }

    /// Handle the enable method
    fn handle_enable(&self) -> Result<Value, CdpError> {
        debug!("Profiler.enable called");
        self.enabled.store(true, Ordering::SeqCst);
        Ok(json!({}))
    }

    /// Handle the disable method
    fn handle_disable(&self) -> Result<Value, CdpError> {
        debug!("Profiler.disable called");
        self.enabled.store(false, Ordering::SeqCst);
        self.profiling_active.store(false, Ordering::SeqCst);
        self.coverage_active.store(false, Ordering::SeqCst);
        Ok(json!({}))
    }

    /// Handle the start method
    fn handle_start(&self) -> Result<Value, CdpError> {
        debug!("Profiler.start called");

        if !self.enabled.load(Ordering::SeqCst) {
            return Err(CdpError::invalid_request());
        }

        self.profiling_active.store(true, Ordering::SeqCst);
        *self.profile_start_time.write() = Self::get_timestamp_micros();

        Ok(json!({}))
    }

    /// Handle the stop method
    fn handle_stop(&self) -> Result<Value, CdpError> {
        debug!("Profiler.stop called");

        if !self.profiling_active.load(Ordering::SeqCst) {
            return Err(CdpError::invalid_request());
        }

        self.profiling_active.store(false, Ordering::SeqCst);

        let start_time = *self.profile_start_time.read();
        let end_time = Self::get_timestamp_micros();

        // Generate mock profile data
        let profile = self.generate_mock_profile(start_time, end_time);

        Ok(json!({ "profile": profile }))
    }

    /// Handle the startPreciseCoverage method
    fn handle_start_precise_coverage(&self, _params: Option<Value>) -> Result<Value, CdpError> {
        debug!("Profiler.startPreciseCoverage called");

        if !self.enabled.load(Ordering::SeqCst) {
            return Err(CdpError::invalid_request());
        }

        self.coverage_active.store(true, Ordering::SeqCst);

        // Clear previous coverage data
        self.coverage_data.write().clear();

        Ok(json!({ "timestamp": 0.0 }))
    }

    /// Handle the stopPreciseCoverage method
    fn handle_stop_precise_coverage(&self) -> Result<Value, CdpError> {
        debug!("Profiler.stopPreciseCoverage called");

        self.coverage_active.store(false, Ordering::SeqCst);
        self.coverage_data.write().clear();

        Ok(json!({}))
    }

    /// Handle the takePreciseCoverage method
    fn handle_take_precise_coverage(&self) -> Result<Value, CdpError> {
        debug!("Profiler.takePreciseCoverage called");

        if !self.coverage_active.load(Ordering::SeqCst) {
            return Err(CdpError::invalid_request());
        }

        // Generate mock coverage data
        let coverage_data = self.generate_mock_coverage();
        let timestamp = Self::get_timestamp_micros();

        Ok(json!({
            "result": coverage_data,
            "timestamp": timestamp
        }))
    }

    /// Handle the getBestEffortCoverage method
    fn handle_get_best_effort_coverage(&self) -> Result<Value, CdpError> {
        debug!("Profiler.getBestEffortCoverage called");

        // Generate mock best-effort coverage data
        let coverage_data = self.generate_mock_coverage();

        Ok(json!({
            "result": coverage_data
        }))
    }

    /// Generate mock profile data for testing
    fn generate_mock_profile(&self, start_time: f64, end_time: f64) -> Profile {
        Profile {
            nodes: vec![
                ProfileNode {
                    id: 1,
                    call_frame: json!({
                        "functionName": "(root)",
                        "scriptId": "0",
                        "url": "",
                        "lineNumber": 0,
                        "columnNumber": 0
                    }),
                    hit_count: Some(0),
                    children: Some(vec![2]),
                    deopt_reason: None,
                    position_ticks: None,
                },
                ProfileNode {
                    id: 2,
                    call_frame: json!({
                        "functionName": "main",
                        "scriptId": "1",
                        "url": "http://example.com/script.js",
                        "lineNumber": 10,
                        "columnNumber": 5
                    }),
                    hit_count: Some(10),
                    children: None,
                    deopt_reason: None,
                    position_ticks: None,
                },
            ],
            start_time,
            end_time,
            samples: Some(vec![1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2]),
            time_deltas: Some(vec![1000, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100]),
        }
    }

    /// Generate mock coverage data for testing
    fn generate_mock_coverage(&self) -> Vec<ScriptCoverage> {
        vec![
            ScriptCoverage {
                script_id: "1".to_string(),
                url: "http://example.com/script.js".to_string(),
                functions: vec![FunctionCoverage {
                    function_name: "main".to_string(),
                    ranges: vec![
                        CoverageRange {
                            start_offset: 0,
                            end_offset: 100,
                            count: 5,
                        },
                        CoverageRange {
                            start_offset: 100,
                            end_offset: 200,
                            count: 3,
                        },
                    ],
                    is_block_coverage: true,
                }],
            },
            ScriptCoverage {
                script_id: "2".to_string(),
                url: "http://example.com/utils.js".to_string(),
                functions: vec![FunctionCoverage {
                    function_name: "helper".to_string(),
                    ranges: vec![CoverageRange {
                        start_offset: 0,
                        end_offset: 50,
                        count: 10,
                    }],
                    is_block_coverage: true,
                }],
            },
        ]
    }
}

impl Default for ProfilerDomain {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DomainHandler for ProfilerDomain {
    fn name(&self) -> &str {
        "Profiler"
    }

    async fn handle_method(&self, method: &str, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("Profiler domain handling method: {}", method);

        match method {
            "enable" => self.handle_enable(),
            "disable" => self.handle_disable(),
            "start" => self.handle_start(),
            "stop" => self.handle_stop(),
            "startPreciseCoverage" => self.handle_start_precise_coverage(params),
            "stopPreciseCoverage" => self.handle_stop_precise_coverage(),
            "takePreciseCoverage" => self.handle_take_precise_coverage(),
            "getBestEffortCoverage" => self.handle_get_best_effort_coverage(),
            _ => {
                warn!("Unknown Profiler method: {}", method);
                Err(CdpError::method_not_found(format!("Profiler.{}", method)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profiler_domain_new() {
        let profiler = ProfilerDomain::new();
        assert_eq!(profiler.name(), "Profiler");
        assert!(!profiler.is_profiling());
        assert!(!profiler.is_coverage_active());
    }

    #[test]
    fn test_get_timestamp_micros() {
        let timestamp = ProfilerDomain::get_timestamp_micros();
        assert!(timestamp > 0.0);
    }

    #[tokio::test]
    async fn test_enable_disable() {
        let profiler = ProfilerDomain::new();

        let enable_result = profiler.handle_method("enable", None).await;
        assert!(enable_result.is_ok());
        assert!(profiler.enabled.load(Ordering::SeqCst));

        let disable_result = profiler.handle_method("disable", None).await;
        assert!(disable_result.is_ok());
        assert!(!profiler.enabled.load(Ordering::SeqCst));
    }
}
