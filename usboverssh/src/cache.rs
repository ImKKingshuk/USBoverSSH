//! Device List Caching
//!
//! Provides in-memory caching for device lists with TTL-based expiration.

use crate::device::DeviceInfo;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Cache entry with expiration time
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    /// Cached device list
    devices: Vec<DeviceInfo>,
    /// Expiration timestamp
    expires_at: DateTime<Utc>,
    /// Cache hit count
    hits: u64,
}

/// Device list cache with TTL
#[derive(Debug, Clone)]
pub struct DeviceListCache {
    /// Cache entries keyed by cache key
    entries: Arc<RwLock<HashMap<String, CacheEntry>>>,
    /// Default TTL in seconds
    default_ttl_seconds: u64,
    /// Total cache hits
    total_hits: Arc<std::sync::atomic::AtomicU64>,
    /// Total cache misses
    total_misses: Arc<std::sync::atomic::AtomicU64>,
}

impl DeviceListCache {
    /// Create new cache with default TTL
    pub fn new(default_ttl_seconds: u64) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            default_ttl_seconds,
            total_hits: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            total_misses: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Get cached device list for key
    pub async fn get(&self, key: &str) -> Option<Vec<DeviceInfo>> {
        let entries = self.entries.read().await;
        if let Some(entry) = entries.get(key) {
            if entry.expires_at > Utc::now() {
                self.total_hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                return Some(entry.devices.clone());
            }
        }
        self.total_misses.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        None
    }

    /// Set cached device list for key with TTL
    pub async fn set(&self, key: String, devices: Vec<DeviceInfo>, ttl_seconds: Option<u64>) {
        let ttl = ttl_seconds.unwrap_or(self.default_ttl_seconds);
        let expires_at = Utc::now() + chrono::Duration::seconds(ttl as i64);
        
        let entry = CacheEntry {
            devices,
            expires_at,
            hits: 0,
        };

        let mut entries = self.entries.write().await;
        entries.insert(key, entry);
    }

    /// Invalidate specific cache entry
    pub async fn invalidate(&self, key: &str) {
        let mut entries = self.entries.write().await;
        entries.remove(key);
    }

    /// Clear all cache entries
    pub async fn clear(&self) {
        let mut entries = self.entries.write().await;
        entries.clear();
    }

    /// Remove expired entries
    pub async fn cleanup_expired(&self) -> usize {
        let mut entries = self.entries.write().await;
        let now = Utc::now();
        let before = entries.len();
        entries.retain(|_, entry| entry.expires_at > now);
        before - entries.len()
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        CacheStats {
            total_hits: self.total_hits.load(std::sync::atomic::Ordering::Relaxed),
            total_misses: self.total_misses.load(std::sync::atomic::Ordering::Relaxed),
            entry_count: self.entries.read().await.len(),
        }
    }

    /// Generate cache key from parameters
    pub fn generate_key(host: &str, filter: Option<&str>) -> String {
        match filter {
            Some(f) => format!("{}:{}", host, f),
            None => host.to_string(),
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// Total cache hits
    pub total_hits: u64,
    /// Total cache misses
    pub total_misses: u64,
    /// Current entry count
    pub entry_count: usize,
}

impl CacheStats {
    /// Calculate hit rate
    pub fn hit_rate(&self) -> f64 {
        let total = self.total_hits + self.total_misses;
        if total == 0 {
            0.0
        } else {
            self.total_hits as f64 / total as f64
        }
    }
}

impl Default for DeviceListCache {
    fn default() -> Self {
        Self::new(30) // 30 seconds default TTL
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::{DeviceClass, DeviceSpeed};

    fn create_mock_device(bus_id: &str) -> DeviceInfo {
        DeviceInfo {
            bus_id: bus_id.to_string(),
            vendor_id: 0x1234,
            product_id: 0x5678,
            device_class: DeviceClass::Hid,
            bus_num: 1,
            dev_num: 1,
            speed: DeviceSpeed::High,
            manufacturer: Some("Test".to_string()),
            product: Some("Device".to_string()),
            serial: None,
            num_configurations: 1,
            is_attached: false,
            is_bound: false,
        }
    }

    #[tokio::test]
    async fn test_cache_set_and_get() {
        let cache = DeviceListCache::new(30);
        let devices = vec![create_mock_device("1-1")];
        
        cache.set("test_key".to_string(), devices.clone(), None).await;
        let retrieved = cache.get("test_key").await;
        
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_cache_ttl_expiration() {
        let cache = DeviceListCache::new(1); // 1 second TTL
        let devices = vec![create_mock_device("1-1")];
        
        cache.set("test_key".to_string(), devices, None).await;
        
        // Should be available immediately
        assert!(cache.get("test_key").await.is_some());
        
        // Wait for expiration
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        
        // Should be expired
        assert!(cache.get("test_key").await.is_none());
    }

    #[tokio::test]
    async fn test_cache_invalidate() {
        let cache = DeviceListCache::new(30);
        let devices = vec![create_mock_device("1-1")];
        
        cache.set("test_key".to_string(), devices, None).await;
        assert!(cache.get("test_key").await.is_some());
        
        cache.invalidate("test_key").await;
        assert!(cache.get("test_key").await.is_none());
    }

    #[tokio::test]
    async fn test_cache_clear() {
        let cache = DeviceListCache::new(30);
        
        cache.set("key1".to_string(), vec![create_mock_device("1-1")], None).await;
        cache.set("key2".to_string(), vec![create_mock_device("2-1")], None).await;
        
        assert!(cache.get("key1").await.is_some());
        assert!(cache.get("key2").await.is_some());
        
        cache.clear().await;
        
        assert!(cache.get("key1").await.is_none());
        assert!(cache.get("key2").await.is_none());
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let cache = DeviceListCache::new(30);
        let devices = vec![create_mock_device("1-1")];
        
        cache.set("test_key".to_string(), devices, None).await;
        
        // Hit
        cache.get("test_key").await;
        
        // Miss
        cache.get("nonexistent").await;
        
        let stats = cache.stats().await;
        assert_eq!(stats.total_hits, 1);
        assert_eq!(stats.total_misses, 1);
        assert_eq!(stats.entry_count, 1);
        assert_eq!(stats.hit_rate(), 0.5);
    }

    #[tokio::test]
    async fn test_cache_cleanup_expired() {
        let cache = DeviceListCache::new(1);
        
        cache.set("key1".to_string(), vec![create_mock_device("1-1")], Some(1)).await;
        cache.set("key2".to_string(), vec![create_mock_device("2-1")], Some(100)).await;
        
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        
        let removed = cache.cleanup_expired().await;
        assert_eq!(removed, 1);
        assert_eq!(cache.stats().await.entry_count, 1);
    }

    #[tokio::test]
    async fn test_cache_key_generation() {
        assert_eq!(DeviceListCache::generate_key("host1", None), "host1");
        assert_eq!(DeviceListCache::generate_key("host1", Some("vid:1234")), "host1:vid:1234");
    }

    #[tokio::test]
    async fn test_cache_concurrent_access() {
        let cache = Arc::new(DeviceListCache::new(30));
        let mut handles = vec![];
        
        for i in 0..10 {
            let cache_clone = Arc::clone(&cache);
            let handle = tokio::spawn(async move {
                let key = format!("key_{}", i);
                cache_clone.set(key.clone(), vec![create_mock_device(&format!("{}-1", i))], None).await;
                cache_clone.get(&key).await
            });
            handles.push(handle);
        }
        
        let results: Vec<_> = futures::future::join_all(handles).await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();
        
        assert_eq!(results.len(), 10);
        assert!(results.iter().all(|r| r.is_some()));
    }
}
