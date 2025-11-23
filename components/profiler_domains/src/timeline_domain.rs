//! TimelineDomain implementation (FEAT-034)
//!
//! Provides performance timeline recording for the Chrome DevTools Protocol.
//! Records events with categories, timing, memory snapshots, and frame timing.

use async_trait::async_trait;
use cdp_types::CdpError;
use parking_lot::RwLock;
use protocol_handler::DomainHandler;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};

use crate::types::{
    FrameTiming, TimelineConfig, TimelineEvent, TimelineEventCategory,
    TimelineMemorySnapshot, TimelineRecording,
};

/// TimelineDomain handles performance timeline recording
#[derive(Debug)]
pub struct TimelineDomain {
    /// Whether the domain is enabled
    enabled: Arc<AtomicBool>,
    /// Whether recording is active
    recording: Arc<AtomicBool>,
    /// Recording configuration
    config: Arc<RwLock<TimelineConfig>>,
    /// Recorded events
    events: Arc<RwLock<Vec<TimelineEvent>>>,
    /// Memory snapshots
    memory_snapshots: Arc<RwLock<Vec<TimelineMemorySnapshot>>>,
    /// Frame timing data
    frames: Arc<RwLock<Vec<FrameTiming>>>,
    /// Recording start time
    start_time: Arc<RwLock<f64>>,
    /// Last memory snapshot time
    last_memory_snapshot: Arc<RwLock<f64>>,
    /// Event counter for IDs
    event_counter: Arc<AtomicU64>,
    /// Frame counter for IDs
    frame_counter: Arc<AtomicU64>,
}

impl TimelineDomain {
    /// Create a new TimelineDomain instance
    pub fn new() -> Self {
        Self {
            enabled: Arc::new(AtomicBool::new(false)),
            recording: Arc::new(AtomicBool::new(false)),
            config: Arc::new(RwLock::new(TimelineConfig::default())),
            events: Arc::new(RwLock::new(Vec::new())),
            memory_snapshots: Arc::new(RwLock::new(Vec::new())),
            frames: Arc::new(RwLock::new(Vec::new())),
            start_time: Arc::new(RwLock::new(0.0)),
            last_memory_snapshot: Arc::new(RwLock::new(0.0)),
            event_counter: Arc::new(AtomicU64::new(0)),
            frame_counter: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Check if timeline recording is active
    pub fn is_recording(&self) -> bool {
        self.recording.load(Ordering::SeqCst)
    }

    /// Check if the domain is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    /// Get the current configuration
    pub fn get_config(&self) -> TimelineConfig {
        self.config.read().clone()
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
        debug!("Timeline.enable called");
        self.enabled.store(true, Ordering::SeqCst);
        info!("Timeline domain enabled");
        Ok(json!({}))
    }

    /// Handle the disable method
    fn handle_disable(&self) -> Result<Value, CdpError> {
        debug!("Timeline.disable called");
        self.enabled.store(false, Ordering::SeqCst);
        self.recording.store(false, Ordering::SeqCst);
        info!("Timeline domain disabled");
        Ok(json!({}))
    }

    /// Handle the start method
    fn handle_start(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("Timeline.start called");

        if !self.enabled.load(Ordering::SeqCst) {
            return Err(CdpError::invalid_request());
        }

        if self.recording.load(Ordering::SeqCst) {
            return Err(CdpError::invalid_request());
        }

        // Parse configuration from params
        if let Some(ref p) = params {
            let mut config = self.config.write();

            if let Some(max_events) = p.get("maxCallStackDepth").and_then(|v| v.as_u64()) {
                config.max_stack_depth = max_events as u32;
            }

            if let Some(capture_memory) = p.get("includeCounters").and_then(|v| v.as_bool()) {
                config.capture_memory = capture_memory;
            }
        }

        // Clear previous data
        self.events.write().clear();
        self.memory_snapshots.write().clear();
        self.frames.write().clear();
        self.event_counter.store(0, Ordering::SeqCst);
        self.frame_counter.store(0, Ordering::SeqCst);

        let now = Self::get_timestamp_micros();
        *self.start_time.write() = now;
        *self.last_memory_snapshot.write() = now;

        self.recording.store(true, Ordering::SeqCst);
        info!("Timeline recording started");

        Ok(json!({}))
    }

    /// Handle the stop method
    fn handle_stop(&self) -> Result<Value, CdpError> {
        debug!("Timeline.stop called");

        if !self.recording.load(Ordering::SeqCst) {
            return Err(CdpError::invalid_request());
        }

        self.recording.store(false, Ordering::SeqCst);

        let start_time = *self.start_time.read();
        let end_time = Self::get_timestamp_micros();

        let recording = TimelineRecording {
            start_time,
            end_time,
            events: self.events.read().clone(),
            memory_snapshots: self.memory_snapshots.read().clone(),
            frames: self.frames.read().clone(),
        };

        info!("Timeline recording stopped with {} events", recording.events.len());

        Ok(json!({ "timeline": recording }))
    }

    /// Handle the recordEvent method
    fn handle_record_event(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("Timeline.recordEvent called");

        if !self.recording.load(Ordering::SeqCst) {
            return Err(CdpError::invalid_request());
        }

        let params = params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?;

        let event_type = params
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CdpError::invalid_params("Missing type parameter"))?
            .to_string();

        let category = params
            .get("category")
            .and_then(|v| v.as_str())
            .map(|c| match c.to_lowercase().as_str() {
                "scripting" => TimelineEventCategory::Scripting,
                "rendering" => TimelineEventCategory::Rendering,
                "painting" => TimelineEventCategory::Painting,
                "loading" => TimelineEventCategory::Loading,
                _ => TimelineEventCategory::Other,
            })
            .unwrap_or(TimelineEventCategory::Other);

        let now = Self::get_timestamp_micros();
        let start_time = params
            .get("startTime")
            .and_then(|v| v.as_f64())
            .unwrap_or(now);

        let duration = params
            .get("duration")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let config = self.config.read();

        // Check if we should record this category
        if !config.categories.is_empty() && !config.categories.contains(&category) {
            return Ok(json!({}));
        }

        // Check max events
        let events_count = self.events.read().len();
        if events_count >= config.max_events {
            return Ok(json!({}));
        }
        drop(config);

        let mut event = TimelineEvent::new(event_type, category, start_time)
            .with_duration(duration);

        if let Some(thread_id) = params.get("threadId").and_then(|v| v.as_u64()) {
            event = event.with_thread_id(thread_id as u32);
        }

        if let Some(frame_id) = params.get("frameId").and_then(|v| v.as_str()) {
            event = event.with_frame_id(frame_id.to_string());
        }

        if let Some(data) = params.get("data").cloned() {
            event = event.with_data(data);
        }

        self.events.write().push(event);
        self.event_counter.fetch_add(1, Ordering::SeqCst);

        Ok(json!({}))
    }

    /// Handle memory snapshot request
    fn handle_take_memory_snapshot(&self) -> Result<Value, CdpError> {
        debug!("Timeline.takeMemorySnapshot called");

        if !self.recording.load(Ordering::SeqCst) {
            return Err(CdpError::invalid_request());
        }

        let snapshot = self.create_memory_snapshot();
        self.memory_snapshots.write().push(snapshot.clone());

        Ok(json!({ "snapshot": snapshot }))
    }

    /// Handle frame timing recording
    fn handle_record_frame(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("Timeline.recordFrame called");

        if !self.recording.load(Ordering::SeqCst) {
            return Err(CdpError::invalid_request());
        }

        let params = params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?;

        let frame_id = params
            .get("frameId")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| {
                // Generate a frame ID
                "frame-auto"
            })
            .to_string();

        let now = Self::get_timestamp_micros();
        let start_time = params
            .get("startTime")
            .and_then(|v| v.as_f64())
            .unwrap_or(now);

        let end_time = params
            .get("endTime")
            .and_then(|v| v.as_f64())
            .unwrap_or(now);

        let cpu_time = params
            .get("cpuTime")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let dropped = params
            .get("dropped")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mut frame = FrameTiming::new(frame_id, start_time);
        frame.complete(end_time, cpu_time, dropped);

        self.frames.write().push(frame.clone());
        self.frame_counter.fetch_add(1, Ordering::SeqCst);

        Ok(json!({ "frame": frame }))
    }

    /// Handle get events
    fn handle_get_events(&self) -> Result<Value, CdpError> {
        debug!("Timeline.getEvents called");

        let events = self.events.read().clone();
        Ok(json!({ "events": events }))
    }

    /// Handle get memory snapshots
    fn handle_get_memory_snapshots(&self) -> Result<Value, CdpError> {
        debug!("Timeline.getMemorySnapshots called");

        let snapshots = self.memory_snapshots.read().clone();
        Ok(json!({ "snapshots": snapshots }))
    }

    /// Handle get frames
    fn handle_get_frames(&self) -> Result<Value, CdpError> {
        debug!("Timeline.getFrames called");

        let frames = self.frames.read().clone();
        Ok(json!({ "frames": frames }))
    }

    /// Create a memory snapshot
    fn create_memory_snapshot(&self) -> TimelineMemorySnapshot {
        let timestamp = Self::get_timestamp_micros();
        *self.last_memory_snapshot.write() = timestamp;

        // In a real implementation, this would query actual memory usage
        // For now, generate mock data
        TimelineMemorySnapshot {
            timestamp,
            js_heap_size_used: 50_000_000,    // 50MB
            js_heap_size_total: 100_000_000,  // 100MB
            documents: 1,
            nodes: 500,
            listeners: 100,
        }
    }

    /// Record a scripting event
    pub fn record_scripting_event(&self, event_type: &str, duration: f64, data: Option<Value>) {
        if !self.is_recording() {
            return;
        }

        let now = Self::get_timestamp_micros();
        let mut event = TimelineEvent::new(
            event_type.to_string(),
            TimelineEventCategory::Scripting,
            now,
        )
        .with_duration(duration);

        if let Some(d) = data {
            event = event.with_data(d);
        }

        self.events.write().push(event);
    }

    /// Record a rendering event
    pub fn record_rendering_event(&self, event_type: &str, duration: f64, data: Option<Value>) {
        if !self.is_recording() {
            return;
        }

        let now = Self::get_timestamp_micros();
        let mut event = TimelineEvent::new(
            event_type.to_string(),
            TimelineEventCategory::Rendering,
            now,
        )
        .with_duration(duration);

        if let Some(d) = data {
            event = event.with_data(d);
        }

        self.events.write().push(event);
    }

    /// Record a painting event
    pub fn record_painting_event(&self, event_type: &str, duration: f64, data: Option<Value>) {
        if !self.is_recording() {
            return;
        }

        let now = Self::get_timestamp_micros();
        let mut event = TimelineEvent::new(
            event_type.to_string(),
            TimelineEventCategory::Painting,
            now,
        )
        .with_duration(duration);

        if let Some(d) = data {
            event = event.with_data(d);
        }

        self.events.write().push(event);
    }

    /// Record a loading event
    pub fn record_loading_event(&self, event_type: &str, duration: f64, url: Option<String>) {
        if !self.is_recording() {
            return;
        }

        let now = Self::get_timestamp_micros();
        let mut event = TimelineEvent::new(
            event_type.to_string(),
            TimelineEventCategory::Loading,
            now,
        )
        .with_duration(duration);

        if let Some(u) = url {
            event = event.with_data(json!({ "url": u }));
        }

        self.events.write().push(event);
    }

    /// Get the event count
    pub fn event_count(&self) -> usize {
        self.events.read().len()
    }

    /// Get the frame count
    pub fn frame_count(&self) -> usize {
        self.frames.read().len()
    }

    /// Get the memory snapshot count
    pub fn memory_snapshot_count(&self) -> usize {
        self.memory_snapshots.read().len()
    }
}

impl Default for TimelineDomain {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DomainHandler for TimelineDomain {
    fn name(&self) -> &str {
        "Timeline"
    }

    async fn handle_method(&self, method: &str, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("Timeline domain handling method: {}", method);

        match method {
            "enable" => self.handle_enable(),
            "disable" => self.handle_disable(),
            "start" => self.handle_start(params),
            "stop" => self.handle_stop(),
            "recordEvent" => self.handle_record_event(params),
            "takeMemorySnapshot" => self.handle_take_memory_snapshot(),
            "recordFrame" => self.handle_record_frame(params),
            "getEvents" => self.handle_get_events(),
            "getMemorySnapshots" => self.handle_get_memory_snapshots(),
            "getFrames" => self.handle_get_frames(),
            _ => {
                warn!("Unknown Timeline method: {}", method);
                Err(CdpError::method_not_found(format!("Timeline.{}", method)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeline_domain_new() {
        let timeline = TimelineDomain::new();
        assert_eq!(timeline.name(), "Timeline");
        assert!(!timeline.is_recording());
        assert!(!timeline.is_enabled());
    }

    #[tokio::test]
    async fn test_enable_disable() {
        let timeline = TimelineDomain::new();

        let enable_result = timeline.handle_method("enable", None).await;
        assert!(enable_result.is_ok());
        assert!(timeline.is_enabled());

        let disable_result = timeline.handle_method("disable", None).await;
        assert!(disable_result.is_ok());
        assert!(!timeline.is_enabled());
    }

    #[tokio::test]
    async fn test_start_stop_recording() {
        let timeline = TimelineDomain::new();

        // Enable first
        timeline.handle_method("enable", None).await.unwrap();

        // Start recording
        let start_result = timeline.handle_method("start", None).await;
        assert!(start_result.is_ok());
        assert!(timeline.is_recording());

        // Stop recording
        let stop_result = timeline.handle_method("stop", None).await;
        assert!(stop_result.is_ok());
        assert!(!timeline.is_recording());
    }

    #[tokio::test]
    async fn test_record_event() {
        let timeline = TimelineDomain::new();
        timeline.handle_method("enable", None).await.unwrap();
        timeline.handle_method("start", None).await.unwrap();

        let event_params = json!({
            "type": "FunctionCall",
            "category": "scripting",
            "duration": 1000.0
        });

        let result = timeline.handle_method("recordEvent", Some(event_params)).await;
        assert!(result.is_ok());
        assert_eq!(timeline.event_count(), 1);
    }

    #[tokio::test]
    async fn test_record_frame() {
        let timeline = TimelineDomain::new();
        timeline.handle_method("enable", None).await.unwrap();
        timeline.handle_method("start", None).await.unwrap();

        let frame_params = json!({
            "frameId": "frame-1",
            "startTime": 1000.0,
            "endTime": 1016.67,
            "cpuTime": 10.0,
            "dropped": false
        });

        let result = timeline.handle_method("recordFrame", Some(frame_params)).await;
        assert!(result.is_ok());
        assert_eq!(timeline.frame_count(), 1);
    }

    #[tokio::test]
    async fn test_memory_snapshot() {
        let timeline = TimelineDomain::new();
        timeline.handle_method("enable", None).await.unwrap();
        timeline.handle_method("start", None).await.unwrap();

        let result = timeline.handle_method("takeMemorySnapshot", None).await;
        assert!(result.is_ok());
        assert_eq!(timeline.memory_snapshot_count(), 1);
    }

    #[tokio::test]
    async fn test_recording_helper_methods() {
        let timeline = TimelineDomain::new();
        timeline.handle_method("enable", None).await.unwrap();
        timeline.handle_method("start", None).await.unwrap();

        timeline.record_scripting_event("FunctionCall", 100.0, None);
        timeline.record_rendering_event("Layout", 50.0, None);
        timeline.record_painting_event("Paint", 30.0, None);
        timeline.record_loading_event("ResourceLoad", 200.0, Some("http://example.com".to_string()));

        assert_eq!(timeline.event_count(), 4);
    }

    #[test]
    fn test_timeline_event_builder() {
        let event = TimelineEvent::new(
            "TestEvent".to_string(),
            TimelineEventCategory::Scripting,
            1000.0,
        )
        .with_duration(500.0)
        .with_thread_id(1)
        .with_frame_id("frame-1".to_string())
        .with_data(json!({"key": "value"}));

        assert_eq!(event.event_type, "TestEvent");
        assert_eq!(event.category, TimelineEventCategory::Scripting);
        assert_eq!(event.start_time, 1000.0);
        assert_eq!(event.duration, 500.0);
        assert_eq!(event.thread_id, Some(1));
        assert_eq!(event.frame_id, Some("frame-1".to_string()));
        assert!(event.data.is_some());
    }

    #[test]
    fn test_frame_timing() {
        let mut frame = FrameTiming::new("frame-1".to_string(), 1000.0);
        frame.complete(1016.67, 10.0, false);

        assert_eq!(frame.frame_id, "frame-1");
        assert_eq!(frame.start_time, 1000.0);
        assert_eq!(frame.end_time, 1016.67);
        assert!((frame.duration - 16.67).abs() < 0.01);
        assert_eq!(frame.cpu_time, 10.0);
        assert!(!frame.dropped);
    }

    #[test]
    fn test_timeline_event_category_display() {
        assert_eq!(TimelineEventCategory::Scripting.to_string(), "scripting");
        assert_eq!(TimelineEventCategory::Rendering.to_string(), "rendering");
        assert_eq!(TimelineEventCategory::Painting.to_string(), "painting");
        assert_eq!(TimelineEventCategory::Loading.to_string(), "loading");
        assert_eq!(TimelineEventCategory::Other.to_string(), "other");
    }
}
