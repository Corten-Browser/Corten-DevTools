//! Message Batching (FEAT-041)
//!
//! Batches multiple CDP events into single messages for efficiency.
//! Supports configurable batch size and timing.

use cdp_types::CdpError;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::debug;

/// Default maximum batch size
const DEFAULT_MAX_BATCH_SIZE: usize = 10;

/// Default batch timeout in milliseconds
const DEFAULT_BATCH_TIMEOUT_MS: u64 = 50;

/// Configuration for message batching
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchConfig {
    /// Maximum number of messages in a batch
    pub max_batch_size: usize,
    /// Maximum time to wait before sending a batch (milliseconds)
    pub batch_timeout_ms: u64,
    /// Whether batching is enabled
    pub enabled: bool,
    /// Whether to batch events by domain
    pub group_by_domain: bool,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_batch_size: DEFAULT_MAX_BATCH_SIZE,
            batch_timeout_ms: DEFAULT_BATCH_TIMEOUT_MS,
            enabled: true,
            group_by_domain: false,
        }
    }
}

/// A batched CDP event
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchedEvent {
    /// Event method name (e.g., "Network.requestWillBeSent")
    pub method: String,
    /// Event parameters
    pub params: Value,
    /// Timestamp when event was created
    pub timestamp: u64,
}

impl BatchedEvent {
    /// Create a new batched event
    pub fn new(method: &str, params: Value) -> Self {
        Self {
            method: method.to_string(),
            params,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        }
    }

    /// Get the domain of this event
    pub fn domain(&self) -> Option<&str> {
        self.method.split('.').next()
    }
}

/// A batch of CDP events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventBatch {
    /// The events in this batch
    pub events: Vec<BatchedEvent>,
    /// When this batch was created
    pub created_at: u64,
    /// When this batch was sent
    pub sent_at: Option<u64>,
    /// Batch sequence number
    pub sequence: u64,
}

impl EventBatch {
    /// Create a new batch
    pub fn new(sequence: u64) -> Self {
        Self {
            events: Vec::new(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            sent_at: None,
            sequence,
        }
    }

    /// Add an event to the batch
    pub fn add_event(&mut self, event: BatchedEvent) {
        self.events.push(event);
    }

    /// Check if the batch is empty
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Get the number of events in the batch
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Mark the batch as sent
    pub fn mark_sent(&mut self) {
        self.sent_at = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        );
    }

    /// Convert to JSON value for sending
    pub fn to_json(&self) -> Value {
        json!({
            "method": "CDP.batchedEvents",
            "params": {
                "events": self.events,
                "batchSequence": self.sequence,
                "eventCount": self.events.len()
            }
        })
    }
}

/// Statistics about batching
#[derive(Debug, Clone, Default)]
pub struct BatchStats {
    /// Total number of events batched
    pub total_events: u64,
    /// Total number of batches sent
    pub batches_sent: u64,
    /// Number of events sent unbatched (when disabled or immediate)
    pub unbatched_events: u64,
    /// Average batch size
    pub avg_batch_size: f64,
    /// Number of batches triggered by size limit
    pub size_triggered: u64,
    /// Number of batches triggered by timeout
    pub timeout_triggered: u64,
}

impl BatchStats {
    /// Update average batch size
    pub fn update_avg(&mut self, batch_size: usize) {
        let total = self.total_events + batch_size as u64;
        let count = self.batches_sent + 1;
        self.avg_batch_size = total as f64 / count as f64;
    }
}

/// Message batcher for CDP events
pub struct MessageBatcher {
    /// Configuration
    config: BatchConfig,
    /// Current batch being built
    current_batch: Arc<Mutex<EventBatch>>,
    /// When the current batch was started
    batch_start: Arc<Mutex<Option<Instant>>>,
    /// Sequence counter for batches
    sequence: Arc<AtomicU64>,
    /// Statistics
    stats: Arc<Mutex<BatchStats>>,
    /// Whether the batcher is running
    running: Arc<AtomicBool>,
    /// Pending batches ready to send
    pending_batches: Arc<Mutex<VecDeque<EventBatch>>>,
}

impl MessageBatcher {
    /// Create a new message batcher with default configuration
    pub fn new() -> Self {
        Self::with_config(BatchConfig::default())
    }

    /// Create a new message batcher with custom configuration
    pub fn with_config(config: BatchConfig) -> Self {
        Self {
            config,
            current_batch: Arc::new(Mutex::new(EventBatch::new(0))),
            batch_start: Arc::new(Mutex::new(None)),
            sequence: Arc::new(AtomicU64::new(1)),
            stats: Arc::new(Mutex::new(BatchStats::default())),
            running: Arc::new(AtomicBool::new(true)),
            pending_batches: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    /// Get the configuration
    pub fn config(&self) -> &BatchConfig {
        &self.config
    }

    /// Add an event to be batched
    pub fn add_event(&self, method: &str, params: Value) -> Option<EventBatch> {
        if !self.config.enabled {
            // Return single event as batch when disabled
            let event = BatchedEvent::new(method, params);
            let mut batch = EventBatch::new(self.sequence.fetch_add(1, Ordering::SeqCst));
            batch.add_event(event);
            batch.mark_sent();

            let mut stats = self.stats.lock();
            stats.unbatched_events += 1;

            return Some(batch);
        }

        let event = BatchedEvent::new(method, params);

        let should_flush = {
            let mut batch = self.current_batch.lock();
            let mut batch_start = self.batch_start.lock();

            // Start batch timer if this is the first event
            if batch.is_empty() {
                *batch_start = Some(Instant::now());
            }

            batch.add_event(event);

            // Check if we should flush due to size
            batch.len() >= self.config.max_batch_size
        };

        if should_flush {
            debug!("Flushing batch due to size limit");
            {
                let mut stats = self.stats.lock();
                stats.size_triggered += 1;
            } // Drop stats lock before flush
            return self.flush_current_batch();
        }

        // Check timeout
        if self.should_flush_timeout() {
            debug!("Flushing batch due to timeout");
            {
                let mut stats = self.stats.lock();
                stats.timeout_triggered += 1;
            } // Drop stats lock before flush
            return self.flush_current_batch();
        }

        None
    }

    /// Check if we should flush due to timeout
    fn should_flush_timeout(&self) -> bool {
        let batch_start = self.batch_start.lock();
        if let Some(start) = *batch_start {
            start.elapsed() > Duration::from_millis(self.config.batch_timeout_ms)
        } else {
            false
        }
    }

    /// Flush the current batch and return it
    pub fn flush_current_batch(&self) -> Option<EventBatch> {
        let mut batch = self.current_batch.lock();
        let mut batch_start = self.batch_start.lock();

        if batch.is_empty() {
            return None;
        }

        let mut completed_batch = std::mem::replace(
            &mut *batch,
            EventBatch::new(self.sequence.fetch_add(1, Ordering::SeqCst)),
        );
        completed_batch.mark_sent();
        *batch_start = None;

        // Update stats
        {
            let mut stats = self.stats.lock();
            stats.total_events += completed_batch.len() as u64;
            stats.batches_sent += 1;
            stats.update_avg(completed_batch.len());
        }

        Some(completed_batch)
    }

    /// Force flush all pending events
    pub fn flush_all(&self) -> Vec<EventBatch> {
        let mut batches = Vec::new();

        if let Some(batch) = self.flush_current_batch() {
            batches.push(batch);
        }

        // Get any pending batches
        let mut pending = self.pending_batches.lock();
        while let Some(batch) = pending.pop_front() {
            batches.push(batch);
        }

        batches
    }

    /// Get batching statistics
    pub fn stats(&self) -> BatchStats {
        self.stats.lock().clone()
    }

    /// Check if the batcher has pending events
    pub fn has_pending(&self) -> bool {
        !self.current_batch.lock().is_empty() || !self.pending_batches.lock().is_empty()
    }

    /// Get the number of pending events
    pub fn pending_count(&self) -> usize {
        let current = self.current_batch.lock().len();
        let pending: usize = self.pending_batches.lock().iter().map(|b| b.len()).sum();
        current + pending
    }

    /// Stop the batcher
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Check if the batcher is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Enable or disable batching
    pub fn set_enabled(&mut self, enabled: bool) {
        self.config.enabled = enabled;
    }

    /// Update batch size limit
    pub fn set_max_batch_size(&mut self, size: usize) {
        self.config.max_batch_size = size;
    }

    /// Update batch timeout
    pub fn set_batch_timeout(&mut self, timeout_ms: u64) {
        self.config.batch_timeout_ms = timeout_ms;
    }
}

impl Default for MessageBatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Async message batcher that can be used with channels
pub struct AsyncMessageBatcher {
    /// Inner batcher
    batcher: MessageBatcher,
    /// Channel sender for outgoing batches
    sender: Option<mpsc::Sender<EventBatch>>,
}

impl AsyncMessageBatcher {
    /// Create a new async batcher
    pub fn new() -> Self {
        Self {
            batcher: MessageBatcher::new(),
            sender: None,
        }
    }

    /// Create with configuration
    pub fn with_config(config: BatchConfig) -> Self {
        Self {
            batcher: MessageBatcher::with_config(config),
            sender: None,
        }
    }

    /// Set the output channel
    pub fn set_sender(&mut self, sender: mpsc::Sender<EventBatch>) {
        self.sender = Some(sender);
    }

    /// Add an event and potentially send a batch
    pub async fn add_event(&self, method: &str, params: Value) -> Result<(), CdpError> {
        if let Some(batch) = self.batcher.add_event(method, params) {
            if let Some(ref sender) = self.sender {
                sender
                    .send(batch)
                    .await
                    .map_err(|e| CdpError::internal_error(format!("Failed to send batch: {}", e)))?;
            }
        }
        Ok(())
    }

    /// Flush and send all batches
    pub async fn flush_all(&self) -> Result<(), CdpError> {
        for batch in self.batcher.flush_all() {
            if let Some(ref sender) = self.sender {
                sender
                    .send(batch)
                    .await
                    .map_err(|e| CdpError::internal_error(format!("Failed to send batch: {}", e)))?;
            }
        }
        Ok(())
    }

    /// Get stats
    pub fn stats(&self) -> BatchStats {
        self.batcher.stats()
    }
}

impl Default for AsyncMessageBatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_config_default() {
        let config = BatchConfig::default();
        assert_eq!(config.max_batch_size, DEFAULT_MAX_BATCH_SIZE);
        assert_eq!(config.batch_timeout_ms, DEFAULT_BATCH_TIMEOUT_MS);
        assert!(config.enabled);
    }

    #[test]
    fn test_batched_event_new() {
        let event = BatchedEvent::new("Network.requestWillBeSent", json!({"requestId": "1"}));
        assert_eq!(event.method, "Network.requestWillBeSent");
        assert!(event.timestamp > 0);
    }

    #[test]
    fn test_batched_event_domain() {
        let event = BatchedEvent::new("Network.requestWillBeSent", json!({}));
        assert_eq!(event.domain(), Some("Network"));

        let event2 = BatchedEvent::new("invalidMethod", json!({}));
        assert_eq!(event2.domain(), Some("invalidMethod"));
    }

    #[test]
    fn test_event_batch_new() {
        let batch = EventBatch::new(1);
        assert!(batch.is_empty());
        assert_eq!(batch.len(), 0);
        assert_eq!(batch.sequence, 1);
        assert!(batch.sent_at.is_none());
    }

    #[test]
    fn test_event_batch_add_event() {
        let mut batch = EventBatch::new(1);
        let event = BatchedEvent::new("Test.event", json!({}));

        batch.add_event(event);
        assert_eq!(batch.len(), 1);
        assert!(!batch.is_empty());
    }

    #[test]
    fn test_event_batch_mark_sent() {
        let mut batch = EventBatch::new(1);
        assert!(batch.sent_at.is_none());

        batch.mark_sent();
        assert!(batch.sent_at.is_some());
    }

    #[test]
    fn test_event_batch_to_json() {
        let mut batch = EventBatch::new(1);
        batch.add_event(BatchedEvent::new("Test.event", json!({"key": "value"})));

        let json = batch.to_json();
        assert_eq!(json["method"], "CDP.batchedEvents");
        assert_eq!(json["params"]["eventCount"], 1);
        assert_eq!(json["params"]["batchSequence"], 1);
    }

    #[test]
    fn test_message_batcher_new() {
        let batcher = MessageBatcher::new();
        assert!(batcher.config().enabled);
        assert!(!batcher.has_pending());
    }

    #[test]
    fn test_message_batcher_disabled() {
        let config = BatchConfig {
            enabled: false,
            ..Default::default()
        };
        let batcher = MessageBatcher::with_config(config);

        // When disabled, should return batch immediately
        let batch = batcher.add_event("Test.event", json!({}));
        assert!(batch.is_some());

        let batch = batch.unwrap();
        assert_eq!(batch.len(), 1);
    }

    #[test]
    fn test_message_batcher_batching() {
        let config = BatchConfig {
            max_batch_size: 3,
            enabled: true,
            ..Default::default()
        };
        let batcher = MessageBatcher::with_config(config);

        // Add events without reaching limit
        let batch1 = batcher.add_event("Test.event1", json!({}));
        assert!(batch1.is_none());

        let batch2 = batcher.add_event("Test.event2", json!({}));
        assert!(batch2.is_none());

        // Third event should trigger flush
        let batch3 = batcher.add_event("Test.event3", json!({}));
        assert!(batch3.is_some());

        let batch = batch3.unwrap();
        assert_eq!(batch.len(), 3);
    }

    #[test]
    fn test_message_batcher_flush_current() {
        let batcher = MessageBatcher::new();

        batcher.add_event("Test.event1", json!({}));
        batcher.add_event("Test.event2", json!({}));

        assert!(batcher.has_pending());

        let batch = batcher.flush_current_batch();
        assert!(batch.is_some());
        assert_eq!(batch.unwrap().len(), 2);

        assert!(!batcher.has_pending());
    }

    #[test]
    fn test_message_batcher_flush_empty() {
        let batcher = MessageBatcher::new();

        let batch = batcher.flush_current_batch();
        assert!(batch.is_none());
    }

    #[test]
    fn test_message_batcher_flush_all() {
        let batcher = MessageBatcher::new();

        batcher.add_event("Test.event1", json!({}));
        batcher.add_event("Test.event2", json!({}));

        let batches = batcher.flush_all();
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].len(), 2);
    }

    #[test]
    fn test_message_batcher_stats() {
        let config = BatchConfig {
            max_batch_size: 2,
            enabled: true,
            ..Default::default()
        };
        let batcher = MessageBatcher::with_config(config);

        batcher.add_event("Test.event1", json!({}));
        batcher.add_event("Test.event2", json!({})); // Triggers flush

        let stats = batcher.stats();
        assert_eq!(stats.total_events, 2);
        assert_eq!(stats.batches_sent, 1);
        assert_eq!(stats.size_triggered, 1);
    }

    #[test]
    fn test_message_batcher_pending_count() {
        let batcher = MessageBatcher::new();

        assert_eq!(batcher.pending_count(), 0);

        batcher.add_event("Test.event1", json!({}));
        assert_eq!(batcher.pending_count(), 1);

        batcher.add_event("Test.event2", json!({}));
        assert_eq!(batcher.pending_count(), 2);

        batcher.flush_all();
        assert_eq!(batcher.pending_count(), 0);
    }

    #[test]
    fn test_message_batcher_stop() {
        let batcher = MessageBatcher::new();
        assert!(batcher.is_running());

        batcher.stop();
        assert!(!batcher.is_running());
    }

    #[test]
    fn test_message_batcher_set_enabled() {
        let mut batcher = MessageBatcher::new();
        assert!(batcher.config().enabled);

        batcher.set_enabled(false);
        assert!(!batcher.config.enabled);
    }

    #[test]
    fn test_message_batcher_set_max_batch_size() {
        let mut batcher = MessageBatcher::new();

        batcher.set_max_batch_size(100);
        assert_eq!(batcher.config.max_batch_size, 100);
    }

    #[test]
    fn test_message_batcher_set_batch_timeout() {
        let mut batcher = MessageBatcher::new();

        batcher.set_batch_timeout(200);
        assert_eq!(batcher.config.batch_timeout_ms, 200);
    }

    #[test]
    fn test_batch_stats_update_avg() {
        let mut stats = BatchStats::default();

        stats.update_avg(10);
        assert_eq!(stats.avg_batch_size, 10.0);

        stats.batches_sent = 1;
        stats.total_events = 10;
        stats.update_avg(20);
        assert_eq!(stats.avg_batch_size, 15.0); // (10 + 20) / 2
    }

    #[tokio::test]
    async fn test_async_message_batcher() {
        let batcher = AsyncMessageBatcher::new();

        // Without sender, should still work
        let result = batcher.add_event("Test.event", json!({})).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_async_message_batcher_with_channel() {
        let config = BatchConfig {
            max_batch_size: 2,
            enabled: true,
            ..Default::default()
        };
        let mut batcher = AsyncMessageBatcher::with_config(config);
        let (tx, mut rx) = mpsc::channel(10);
        batcher.set_sender(tx);

        // Add events
        batcher.add_event("Test.event1", json!({})).await.unwrap();
        batcher.add_event("Test.event2", json!({})).await.unwrap(); // Should trigger send

        // Receive batch
        let batch = rx.recv().await;
        assert!(batch.is_some());
        assert_eq!(batch.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_async_message_batcher_flush_all() {
        let mut batcher = AsyncMessageBatcher::new();
        let (tx, mut rx) = mpsc::channel(10);
        batcher.set_sender(tx);

        batcher.add_event("Test.event1", json!({})).await.unwrap();
        batcher.add_event("Test.event2", json!({})).await.unwrap();

        batcher.flush_all().await.unwrap();

        let batch = rx.recv().await;
        assert!(batch.is_some());
        assert_eq!(batch.unwrap().len(), 2);
    }
}
