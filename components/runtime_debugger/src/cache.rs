//! Remote Object Caching (FEAT-042)
//!
//! Implements caching for RemoteObject references with:
//! - LRU (Least Recently Used) eviction policy
//! - Object group management
//! - Configurable cache size
//! - Thread-safe access

use cdp_types::domains::runtime::{RemoteObject, RemoteObjectId};
use parking_lot::RwLock;
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::debug;

/// Default maximum cache size
const DEFAULT_MAX_SIZE: usize = 1000;

/// Default TTL for cached objects (5 minutes)
const DEFAULT_TTL_SECS: u64 = 300;

/// Cache entry containing a remote object and metadata
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// The cached remote object
    pub object: RemoteObject,
    /// The raw value (for property access)
    pub value: Value,
    /// Object group this entry belongs to
    pub group: Option<String>,
    /// When this entry was created
    pub created_at: Instant,
    /// When this entry was last accessed
    pub last_accessed: Instant,
    /// Access count for statistics
    pub access_count: u64,
}

impl CacheEntry {
    /// Create a new cache entry
    pub fn new(object: RemoteObject, value: Value, group: Option<String>) -> Self {
        let now = Instant::now();
        Self {
            object,
            value,
            group,
            created_at: now,
            last_accessed: now,
            access_count: 1,
        }
    }

    /// Check if the entry has expired
    pub fn is_expired(&self, ttl: Duration) -> bool {
        self.created_at.elapsed() > ttl
    }

    /// Update last accessed time and increment counter
    pub fn touch(&mut self) {
        self.last_accessed = Instant::now();
        self.access_count += 1;
    }
}

/// Configuration for the remote object cache
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum number of entries in the cache
    pub max_size: usize,
    /// Time-to-live for cached entries
    pub ttl: Duration,
    /// Whether to track access statistics
    pub track_stats: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_size: DEFAULT_MAX_SIZE,
            ttl: Duration::from_secs(DEFAULT_TTL_SECS),
            track_stats: true,
        }
    }
}

/// Statistics about cache usage
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Number of cache hits
    pub hits: u64,
    /// Number of cache misses
    pub misses: u64,
    /// Number of entries evicted
    pub evictions: u64,
    /// Number of entries expired
    pub expirations: u64,
    /// Current cache size
    pub size: usize,
}

impl CacheStats {
    /// Calculate hit rate as a percentage
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            (self.hits as f64 / total as f64) * 100.0
        }
    }
}

/// Remote object cache with LRU eviction
pub struct RemoteObjectCache {
    /// Map from object ID to cache entry
    entries: Arc<RwLock<HashMap<String, CacheEntry>>>,
    /// LRU order tracking (most recent at back)
    lru_order: Arc<RwLock<VecDeque<String>>>,
    /// Object groups (group name -> object IDs)
    groups: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// Cache configuration
    config: CacheConfig,
    /// Cache statistics
    stats: Arc<RwLock<CacheStats>>,
    /// Atomic counters for stats
    hits: Arc<AtomicU64>,
    misses: Arc<AtomicU64>,
}

impl RemoteObjectCache {
    /// Create a new cache with default configuration
    pub fn new() -> Self {
        Self::with_config(CacheConfig::default())
    }

    /// Create a new cache with custom configuration
    pub fn with_config(config: CacheConfig) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            lru_order: Arc::new(RwLock::new(VecDeque::new())),
            groups: Arc::new(RwLock::new(HashMap::new())),
            config,
            stats: Arc::new(RwLock::new(CacheStats::default())),
            hits: Arc::new(AtomicU64::new(0)),
            misses: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Get the cache configuration
    pub fn config(&self) -> &CacheConfig {
        &self.config
    }

    /// Insert an object into the cache
    pub fn insert(&self, object: RemoteObject, value: Value, group: Option<String>) -> Option<RemoteObjectId> {
        let object_id = object.object_id.clone()?;
        let id_str = object_id.0.clone();

        debug!("Caching object: {}", id_str);

        // Create cache entry
        let entry = CacheEntry::new(object, value, group.clone());

        // Evict if necessary
        self.evict_if_needed();

        // Insert entry
        {
            let mut entries = self.entries.write();
            entries.insert(id_str.clone(), entry);
        }

        // Update LRU order
        {
            let mut lru = self.lru_order.write();
            // Remove if already present (to re-add at end)
            lru.retain(|id| id != &id_str);
            lru.push_back(id_str.clone());
        }

        // Update group mapping
        if let Some(group_name) = group {
            let mut groups = self.groups.write();
            groups
                .entry(group_name)
                .or_insert_with(Vec::new)
                .push(id_str);
        }

        // Update stats
        {
            let mut stats = self.stats.write();
            stats.size = self.entries.read().len();
        }

        Some(object_id)
    }

    /// Get an object from the cache
    pub fn get(&self, object_id: &RemoteObjectId) -> Option<CacheEntry> {
        let id_str = &object_id.0;

        // Check for expired entries first
        {
            let entries = self.entries.read();
            if let Some(entry) = entries.get(id_str) {
                if entry.is_expired(self.config.ttl) {
                    drop(entries);
                    self.remove(object_id);
                    self.misses.fetch_add(1, Ordering::Relaxed);
                    {
                        let mut stats = self.stats.write();
                        stats.expirations += 1;
                        stats.misses = self.misses.load(Ordering::Relaxed);
                    }
                    return None;
                }
            }
        }

        // Get and touch the entry
        let mut entries = self.entries.write();
        if let Some(entry) = entries.get_mut(id_str) {
            entry.touch();

            // Update LRU order
            {
                let mut lru = self.lru_order.write();
                lru.retain(|id| id != id_str);
                lru.push_back(id_str.clone());
            }

            self.hits.fetch_add(1, Ordering::Relaxed);
            {
                let mut stats = self.stats.write();
                stats.hits = self.hits.load(Ordering::Relaxed);
            }

            Some(entry.clone())
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
            {
                let mut stats = self.stats.write();
                stats.misses = self.misses.load(Ordering::Relaxed);
            }
            None
        }
    }

    /// Check if an object is in the cache
    pub fn contains(&self, object_id: &RemoteObjectId) -> bool {
        let entries = self.entries.read();
        if let Some(entry) = entries.get(&object_id.0) {
            !entry.is_expired(self.config.ttl)
        } else {
            false
        }
    }

    /// Remove an object from the cache
    pub fn remove(&self, object_id: &RemoteObjectId) -> Option<CacheEntry> {
        let id_str = &object_id.0;

        debug!("Removing cached object: {}", id_str);

        // Remove from entries
        let entry = {
            let mut entries = self.entries.write();
            entries.remove(id_str)
        };

        if let Some(ref e) = entry {
            // Remove from LRU order
            {
                let mut lru = self.lru_order.write();
                lru.retain(|id| id != id_str);
            }

            // Remove from group
            if let Some(ref group_name) = e.group {
                let mut groups = self.groups.write();
                if let Some(group_ids) = groups.get_mut(group_name) {
                    group_ids.retain(|id| id != id_str);
                    if group_ids.is_empty() {
                        groups.remove(group_name);
                    }
                }
            }

            // Update stats
            {
                let mut stats = self.stats.write();
                stats.size = self.entries.read().len();
            }
        }

        entry
    }

    /// Release all objects in a group
    pub fn release_group(&self, group_name: &str) -> Vec<CacheEntry> {
        debug!("Releasing object group: {}", group_name);

        let object_ids: Vec<String> = {
            let groups = self.groups.read();
            groups.get(group_name).cloned().unwrap_or_default()
        };

        let mut released = Vec::new();

        for id_str in object_ids {
            let object_id = RemoteObjectId(id_str);
            if let Some(entry) = self.remove(&object_id) {
                released.push(entry);
            }
        }

        // Remove the group
        {
            let mut groups = self.groups.write();
            groups.remove(group_name);
        }

        released
    }

    /// Clear the entire cache
    pub fn clear(&self) {
        debug!("Clearing cache");

        {
            let mut entries = self.entries.write();
            entries.clear();
        }
        {
            let mut lru = self.lru_order.write();
            lru.clear();
        }
        {
            let mut groups = self.groups.write();
            groups.clear();
        }
        {
            let mut stats = self.stats.write();
            stats.size = 0;
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let mut stats = self.stats.read().clone();
        stats.hits = self.hits.load(Ordering::Relaxed);
        stats.misses = self.misses.load(Ordering::Relaxed);
        stats.size = self.entries.read().len();
        stats
    }

    /// Get the current cache size
    pub fn len(&self) -> usize {
        self.entries.read().len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.entries.read().is_empty()
    }

    /// Get all object IDs in a group
    pub fn get_group(&self, group_name: &str) -> Vec<RemoteObjectId> {
        let groups = self.groups.read();
        groups
            .get(group_name)
            .map(|ids| ids.iter().map(|id| RemoteObjectId(id.clone())).collect())
            .unwrap_or_default()
    }

    /// Get all group names
    pub fn get_group_names(&self) -> Vec<String> {
        self.groups.read().keys().cloned().collect()
    }

    /// Evict entries if cache is at capacity
    fn evict_if_needed(&self) {
        let current_size = self.entries.read().len();

        if current_size >= self.config.max_size {
            // First, try to evict expired entries
            self.evict_expired();

            // If still at capacity, evict LRU entries
            let current_size = self.entries.read().len();
            if current_size >= self.config.max_size {
                self.evict_lru();
            }
        }
    }

    /// Evict expired entries
    fn evict_expired(&self) {
        let expired_ids: Vec<String> = {
            let entries = self.entries.read();
            entries
                .iter()
                .filter(|(_, entry)| entry.is_expired(self.config.ttl))
                .map(|(id, _)| id.clone())
                .collect()
        };

        for id_str in expired_ids {
            let object_id = RemoteObjectId(id_str);
            self.remove(&object_id);
            {
                let mut stats = self.stats.write();
                stats.expirations += 1;
            }
        }
    }

    /// Evict the least recently used entry
    fn evict_lru(&self) {
        let oldest_id: Option<String> = {
            let lru = self.lru_order.read();
            lru.front().cloned()
        };

        if let Some(id_str) = oldest_id {
            debug!("Evicting LRU entry: {}", id_str);
            let object_id = RemoteObjectId(id_str);
            self.remove(&object_id);
            {
                let mut stats = self.stats.write();
                stats.evictions += 1;
            }
        }
    }

    /// Perform garbage collection (remove expired entries)
    pub fn gc(&self) -> usize {
        let initial_size = self.entries.read().len();
        self.evict_expired();
        let final_size = self.entries.read().len();
        initial_size - final_size
    }
}

impl Default for RemoteObjectCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cdp_types::domains::runtime::RemoteObjectType;
    use serde_json::json;

    fn make_remote_object(id: &str) -> RemoteObject {
        RemoteObject {
            object_type: RemoteObjectType::Object,
            subtype: None,
            class_name: Some("Object".to_string()),
            value: None,
            unserializable_value: None,
            description: Some("Test object".to_string()),
            object_id: Some(RemoteObjectId(id.to_string())),
            preview: None,
        }
    }

    #[test]
    fn test_cache_new() {
        let cache = RemoteObjectCache::new();
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_with_config() {
        let config = CacheConfig {
            max_size: 100,
            ttl: Duration::from_secs(60),
            track_stats: true,
        };
        let cache = RemoteObjectCache::with_config(config);
        assert_eq!(cache.config().max_size, 100);
    }

    #[test]
    fn test_cache_insert_and_get() {
        let cache = RemoteObjectCache::new();
        let obj = make_remote_object("obj-1");
        let value = json!({"test": true});

        let result = cache.insert(obj.clone(), value.clone(), None);
        assert!(result.is_some());

        let object_id = RemoteObjectId("obj-1".to_string());
        let entry = cache.get(&object_id);
        assert!(entry.is_some());

        let entry = entry.unwrap();
        assert_eq!(entry.object.object_id, obj.object_id);
        assert_eq!(entry.value, value);
    }

    #[test]
    fn test_cache_contains() {
        let cache = RemoteObjectCache::new();
        let obj = make_remote_object("obj-1");

        cache.insert(obj, json!({}), None);

        assert!(cache.contains(&RemoteObjectId("obj-1".to_string())));
        assert!(!cache.contains(&RemoteObjectId("obj-2".to_string())));
    }

    #[test]
    fn test_cache_remove() {
        let cache = RemoteObjectCache::new();
        let obj = make_remote_object("obj-1");

        cache.insert(obj, json!({}), None);
        assert_eq!(cache.len(), 1);

        let removed = cache.remove(&RemoteObjectId("obj-1".to_string()));
        assert!(removed.is_some());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_cache_clear() {
        let cache = RemoteObjectCache::new();

        for i in 0..5 {
            let obj = make_remote_object(&format!("obj-{}", i));
            cache.insert(obj, json!({}), None);
        }

        assert_eq!(cache.len(), 5);

        cache.clear();
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_groups() {
        let cache = RemoteObjectCache::new();

        let obj1 = make_remote_object("obj-1");
        let obj2 = make_remote_object("obj-2");
        let obj3 = make_remote_object("obj-3");

        cache.insert(obj1, json!({}), Some("group-a".to_string()));
        cache.insert(obj2, json!({}), Some("group-a".to_string()));
        cache.insert(obj3, json!({}), Some("group-b".to_string()));

        let group_a = cache.get_group("group-a");
        assert_eq!(group_a.len(), 2);

        let group_b = cache.get_group("group-b");
        assert_eq!(group_b.len(), 1);

        let group_names = cache.get_group_names();
        assert_eq!(group_names.len(), 2);
    }

    #[test]
    fn test_cache_release_group() {
        let cache = RemoteObjectCache::new();

        let obj1 = make_remote_object("obj-1");
        let obj2 = make_remote_object("obj-2");
        let obj3 = make_remote_object("obj-3");

        cache.insert(obj1, json!({}), Some("group-a".to_string()));
        cache.insert(obj2, json!({}), Some("group-a".to_string()));
        cache.insert(obj3, json!({}), Some("group-b".to_string()));

        assert_eq!(cache.len(), 3);

        let released = cache.release_group("group-a");
        assert_eq!(released.len(), 2);
        assert_eq!(cache.len(), 1);

        // Group should be removed
        assert!(cache.get_group("group-a").is_empty());
    }

    #[test]
    fn test_cache_lru_eviction() {
        let config = CacheConfig {
            max_size: 3,
            ttl: Duration::from_secs(300),
            track_stats: true,
        };
        let cache = RemoteObjectCache::with_config(config);

        // Insert 3 objects (at capacity)
        for i in 0..3 {
            let obj = make_remote_object(&format!("obj-{}", i));
            cache.insert(obj, json!({}), None);
        }

        assert_eq!(cache.len(), 3);

        // Access obj-0 to make it recently used
        cache.get(&RemoteObjectId("obj-0".to_string()));

        // Insert 4th object - should evict obj-1 (least recently used)
        let obj4 = make_remote_object("obj-3");
        cache.insert(obj4, json!({}), None);

        assert_eq!(cache.len(), 3);
        // obj-1 should be evicted (was LRU before obj-0 was accessed)
        assert!(!cache.contains(&RemoteObjectId("obj-1".to_string())));
        assert!(cache.contains(&RemoteObjectId("obj-0".to_string())));
        assert!(cache.contains(&RemoteObjectId("obj-2".to_string())));
        assert!(cache.contains(&RemoteObjectId("obj-3".to_string())));
    }

    #[test]
    fn test_cache_stats() {
        let cache = RemoteObjectCache::new();

        let obj = make_remote_object("obj-1");
        cache.insert(obj, json!({}), None);

        // Hit
        cache.get(&RemoteObjectId("obj-1".to_string()));

        // Miss
        cache.get(&RemoteObjectId("obj-nonexistent".to_string()));

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.size, 1);
        assert_eq!(stats.hit_rate(), 50.0);
    }

    #[test]
    fn test_cache_entry_touch() {
        let obj = make_remote_object("obj-1");
        let mut entry = CacheEntry::new(obj, json!({}), None);

        let initial_access = entry.last_accessed;
        std::thread::sleep(std::time::Duration::from_millis(10));

        entry.touch();

        assert!(entry.last_accessed > initial_access);
        assert_eq!(entry.access_count, 2);
    }

    #[test]
    fn test_cache_entry_expired() {
        let obj = make_remote_object("obj-1");
        let entry = CacheEntry::new(obj, json!({}), None);

        assert!(!entry.is_expired(Duration::from_secs(300)));
        assert!(!entry.is_expired(Duration::from_secs(1)));

        // Can't easily test true expiration without waiting, but the logic is tested
    }

    #[test]
    fn test_cache_stats_hit_rate() {
        let stats = CacheStats {
            hits: 75,
            misses: 25,
            ..Default::default()
        };
        assert_eq!(stats.hit_rate(), 75.0);

        let empty_stats = CacheStats::default();
        assert_eq!(empty_stats.hit_rate(), 0.0);
    }

    #[test]
    fn test_cache_gc() {
        // This is tricky to test without mocking time, but we can test the method exists
        let cache = RemoteObjectCache::new();

        let obj = make_remote_object("obj-1");
        cache.insert(obj, json!({}), None);

        let removed = cache.gc();
        // No entries should be removed since TTL is 5 minutes
        assert_eq!(removed, 0);
    }

    #[test]
    fn test_insert_without_object_id() {
        let cache = RemoteObjectCache::new();
        let obj = RemoteObject {
            object_type: RemoteObjectType::Number,
            subtype: None,
            class_name: None,
            value: Some(json!(42)),
            unserializable_value: None,
            description: Some("42".to_string()),
            object_id: None, // No object ID
            preview: None,
        };

        let result = cache.insert(obj, json!(42), None);
        assert!(result.is_none());
        assert_eq!(cache.len(), 0);
    }
}
