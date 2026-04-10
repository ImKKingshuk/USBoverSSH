//! Device Pooling and Reservation System
//!
//! Provides shared device pools with reservation system for multi-user access.

use crate::config::PoolConfig;
use crate::error::{Error, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Reservation status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReservationStatus {
    /// Reservation is active
    Active,
    /// Reservation has expired
    Expired,
    /// Reservation was released
    Released,
}

/// Device reservation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reservation {
    /// Unique reservation ID
    pub id: Uuid,
    /// Device bus ID
    pub device_bus_id: String,
    /// User/session identifier
    pub user_id: String,
    /// Pool name
    pub pool_name: String,
    /// Reservation creation time
    pub created_at: DateTime<Utc>,
    /// Reservation expiration time
    pub expires_at: DateTime<Utc>,
    /// Current status
    pub status: ReservationStatus,
}

impl Reservation {
    /// Check if reservation is expired
    pub fn is_expired(&self) -> bool {
        self.status == ReservationStatus::Expired || Utc::now() > self.expires_at
    }

    /// Mark reservation as released
    pub fn release(&mut self) {
        self.status = ReservationStatus::Released;
    }

    /// Mark reservation as expired
    pub fn expire(&mut self) {
        self.status = ReservationStatus::Expired;
    }
}

/// Device pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    /// Maximum concurrent reservations per pool
    pub max_reservations: usize,
    /// Default reservation timeout in seconds
    pub default_timeout_seconds: u64,
    /// Persistence file path (optional)
    pub persistence_path: Option<String>,
    /// Auto-cleanup interval in seconds
    pub cleanup_interval_seconds: u64,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_reservations: 10,
            default_timeout_seconds: 1800, // 30 minutes
            persistence_path: None,
            cleanup_interval_seconds: 300, // 5 minutes
        }
    }
}

/// Device pool
#[derive(Debug, Clone)]
pub struct DevicePool {
    /// Pool name
    name: String,
    /// Reservations by device bus ID
    reservations: Arc<RwLock<HashMap<String, Reservation>>>,
    /// Pool configuration
    config: PoolConfig,
    /// Reservation queue (first-come-first-served)
    queue: Arc<RwLock<Vec<QueueEntry>>>,
}

/// Queue entry for pending reservations
#[derive(Debug, Clone)]
struct QueueEntry {
    device_bus_id: String,
    user_id: String,
    #[allow(dead_code)]
    requested_at: DateTime<Utc>,
}

impl DevicePool {
    /// Create new device pool
    pub fn new(name: String, config: PoolConfig) -> Self {
        Self {
            name,
            reservations: Arc::new(RwLock::new(HashMap::new())),
            config,
            queue: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Reserve a device
    pub async fn reserve_device(
        &self,
        device_bus_id: String,
        user_id: String,
        timeout_seconds: Option<u64>,
    ) -> Result<Uuid> {
        let timeout = timeout_seconds.unwrap_or(self.config.default_timeout_seconds);
        let expires_at = Utc::now() + chrono::Duration::seconds(timeout as i64);

        // Check if device is already reserved
        let reservations = self.reservations.read().await;
        if let Some(existing) = reservations.get(&device_bus_id) {
            if !existing.is_expired() {
                return Err(Error::Pool(format!(
                    "Device {} is already reserved by {} until {}",
                    device_bus_id, existing.user_id, existing.expires_at
                )));
            }
        }
        drop(reservations);

        // Check pool capacity
        let reservations = self.reservations.read().await;
        let active_count = reservations.values().filter(|r| !r.is_expired()).count();
        if active_count >= self.config.max_reservations {
            drop(reservations);
            
            // Add to queue
            let mut queue = self.queue.write().await;
            queue.push(QueueEntry {
                device_bus_id: device_bus_id.clone(),
                user_id: user_id.clone(),
                requested_at: Utc::now(),
            });
            
            return Err(Error::Pool(format!(
                "Pool {} is at capacity ({} reservations). Device {} added to queue.",
                self.name, self.config.max_reservations, device_bus_id
            )));
        }
        drop(reservations);

        // Create reservation
        let reservation_id = Uuid::new_v4();
        let reservation = Reservation {
            id: reservation_id,
            device_bus_id: device_bus_id.clone(),
            user_id,
            pool_name: self.name.clone(),
            created_at: Utc::now(),
            expires_at,
            status: ReservationStatus::Active,
        };

        // Add to reservations
        let mut reservations = self.reservations.write().await;
        reservations.insert(device_bus_id, reservation);

        Ok(reservation_id)
    }

    /// Release a device reservation
    pub async fn release_reservation(&self, reservation_id: Uuid) -> Result<()> {
        let mut reservations = self.reservations.write().await;
        
        // Find reservation by ID
        let mut found_device_id = None;
        for (device_id, reservation) in reservations.iter_mut() {
            if reservation.id == reservation_id {
                reservation.release();
                found_device_id = Some(device_id.clone());
                break;
            }
        }

        if let Some(device_id) = found_device_id {
            reservations.remove(&device_id);
            
            // Process queue for this device
            self.process_queue_for_device(&device_id).await;
            
            Ok(())
        } else {
            Err(Error::Pool(format!("Reservation {} not found", reservation_id)))
        }
    }

    /// Release reservation by device bus ID
    pub async fn release_by_device(&self, device_bus_id: &str) -> Result<()> {
        let mut reservations = self.reservations.write().await;
        
        if let Some(mut reservation) = reservations.remove(device_bus_id) {
            reservation.release();
            self.process_queue_for_device(device_bus_id).await;
            Ok(())
        } else {
            Err(Error::Pool(format!("Device {} is not reserved", device_bus_id)))
        }
    }

    /// Get reservation for a device
    pub async fn get_reservation(&self, device_bus_id: &str) -> Option<Reservation> {
        let reservations = self.reservations.read().await;
        reservations.get(device_bus_id).cloned()
    }

    /// Get pool status
    pub async fn get_status(&self) -> PoolStatus {
        let reservations = self.reservations.read().await;
        let queue = self.queue.read().await;

        let active_reservations: Vec<_> = reservations
            .values()
            .filter(|r| !r.is_expired())
            .cloned()
            .collect();

        let queue_length = queue.len();

        PoolStatus {
            name: self.name.clone(),
            active_reservations,
            queue_length,
            max_reservations: self.config.max_reservations,
        }
    }

    /// Cleanup expired reservations
    pub async fn cleanup_expired(&self) -> usize {
        let mut reservations = self.reservations.write().await;
        let before = reservations.len();
        let mut expired_devices = Vec::new();

        reservations.retain(|device_id, reservation| {
            if reservation.is_expired() {
                expired_devices.push(device_id.clone());
                false
            } else {
                true
            }
        });

        // Process queue for expired devices
        for device_id in expired_devices {
            drop(reservations);
            self.process_queue_for_device(&device_id).await;
            reservations = self.reservations.write().await;
        }

        before - reservations.len()
    }

    /// Process queue for a specific device
    async fn process_queue_for_device(&self, device_bus_id: &str) {
        let mut queue = self.queue.write().await;
        let mut to_reserve = Vec::new();

        // Find queue entries for this device
        queue.retain(|entry| {
            if entry.device_bus_id == device_bus_id {
                to_reserve.push(entry.clone());
                false
            } else {
                true
            }
        });

        drop(queue);

        // Try to reserve for queued users
        for entry in to_reserve {
            if let Err(_) = self
                .reserve_device(entry.device_bus_id.clone(), entry.user_id.clone(), None)
                .await
            {
                // If still can't reserve, put back in queue
                let mut queue = self.queue.write().await;
                queue.push(entry);
            }
        }
    }

    /// Save pool state to file
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let reservations = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.reservations.read().await.clone()
            })
        });

        let state = PoolState {
            name: self.name.clone(),
            reservations: reservations.values().cloned().collect(),
        };

        let json = serde_json::to_string_pretty(&state)
            .map_err(|e| Error::Pool(format!("Failed to serialize pool state: {}", e)))?;
        fs::write(path, json)
            .map_err(|e| Error::Pool(format!("Failed to write pool state: {}", e)))?;
        Ok(())
    }

    /// Load pool state from file
    pub fn load_from_file(path: &Path) -> Result<PoolState> {
        let json = fs::read_to_string(path)
            .map_err(|e| Error::Pool(format!("Failed to read pool state: {}", e)))?;
        let state: PoolState = serde_json::from_str(&json)
            .map_err(|e| Error::Pool(format!("Failed to deserialize pool state: {}", e)))?;
        Ok(state)
    }
}

/// Pool status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStatus {
    /// Pool name
    pub name: String,
    /// Active reservations
    pub active_reservations: Vec<Reservation>,
    /// Queue length
    pub queue_length: usize,
    /// Max reservations
    pub max_reservations: usize,
}

/// Pool state for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolState {
    pub name: String,
    pub reservations: Vec<Reservation>,
}

/// Pool manager for multiple pools
#[derive(Debug, Clone)]
pub struct PoolManager {
    /// All pools
    pools: Arc<RwLock<HashMap<String, DevicePool>>>,
    /// Global configuration
    config: PoolConfig,
}

impl PoolManager {
    /// Create new pool manager
    pub fn new(config: PoolConfig) -> Self {
        Self {
            pools: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Create or get a pool
    pub async fn get_or_create_pool(&self, name: String) -> DevicePool {
        let pools = self.pools.read().await;
        if let Some(pool) = pools.get(&name) {
            pool.clone()
        } else {
            drop(pools);
            let mut pools = self.pools.write().await;
            let pool = DevicePool::new(name.clone(), self.config.clone());
            pools.insert(name, pool.clone());
            pool
        }
    }

    /// Get all pool statuses
    pub async fn get_all_statuses(&self) -> Vec<PoolStatus> {
        let pools = self.pools.read().await;
        let mut statuses = Vec::new();
        for pool in pools.values() {
            statuses.push(pool.get_status().await);
        }
        statuses
    }

    /// Cleanup all pools
    pub async fn cleanup_all(&self) -> usize {
        let pools = self.pools.read().await;
        let mut total = 0;
        for pool in pools.values() {
            total += pool.cleanup_expired().await;
        }
        total
    }

    /// Save all pools to file
    pub async fn save_all_to_file(&self, path: &Path) -> Result<()> {
        let pools = self.pools.read().await;
        let states: Vec<PoolState> = pools
            .values()
            .map(|pool| {
                let reservations = tokio::task::block_in_place(|| {
                    let handle = tokio::runtime::Handle::current();
                    handle.block_on(async {
                        pool.reservations.read().await.values().cloned().collect()
                    })
                });
                PoolState {
                    name: pool.name.clone(),
                    reservations,
                }
            })
            .collect();

        let json = serde_json::to_string_pretty(&states)
            .map_err(|e| Error::Pool(format!("Failed to serialize pool states: {}", e)))?;
        fs::write(path, json)
            .map_err(|e| Error::Pool(format!("Failed to write pool states: {}", e)))?;
        Ok(())
    }
}

impl Default for PoolManager {
    fn default() -> Self {
        Self::new(PoolConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_reserve_device() {
        let pool = DevicePool::new("test_pool".to_string(), PoolConfig::default());
        let reservation_id = pool
            .reserve_device("1-1".to_string(), "user1".to_string(), None)
            .await
            .unwrap();

        assert!(pool.get_reservation("1-1").await.is_some());
        assert_eq!(
            pool.get_reservation("1-1").await.unwrap().id,
            reservation_id
        );
    }

    #[tokio::test]
    async fn test_reserve_conflict() {
        let pool = DevicePool::new("test_pool".to_string(), PoolConfig::default());
        pool.reserve_device("1-1".to_string(), "user1".to_string(), None)
            .await
            .unwrap();

        let result = pool
            .reserve_device("1-1".to_string(), "user2".to_string(), None)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_release_reservation() {
        let pool = DevicePool::new("test_pool".to_string(), PoolConfig::default());
        let reservation_id = pool
            .reserve_device("1-1".to_string(), "user1".to_string(), None)
            .await
            .unwrap();

        pool.release_reservation(reservation_id).await.unwrap();
        assert!(pool.get_reservation("1-1").await.is_none());
    }

    #[tokio::test]
    async fn test_release_by_device() {
        let pool = DevicePool::new("test_pool".to_string(), PoolConfig::default());
        pool.reserve_device("1-1".to_string(), "user1".to_string(), None)
            .await
            .unwrap();

        pool.release_by_device("1-1").await.unwrap();
        assert!(pool.get_reservation("1-1").await.is_none());
    }

    #[tokio::test]
    async fn test_pool_capacity() {
        let config = PoolConfig {
            max_reservations: 2,
            ..Default::default()
        };
        let pool = DevicePool::new("test_pool".to_string(), config);

        pool.reserve_device("1-1".to_string(), "user1".to_string(), None)
            .await
            .unwrap();
        pool.reserve_device("2-1".to_string(), "user2".to_string(), None)
            .await
            .unwrap();

        let result = pool
            .reserve_device("3-1".to_string(), "user3".to_string(), None)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_reservation_expiration() {
        let pool = DevicePool::new(
            "test_pool".to_string(),
            PoolConfig {
                default_timeout_seconds: 1,
                ..Default::default()
            },
        );

        pool.reserve_device("1-1".to_string(), "user1".to_string(), None)
            .await
            .unwrap();

        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Should be able to reserve again after expiration
        let result = pool
            .reserve_device("1-1".to_string(), "user2".to_string(), None)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_pool_status() {
        let pool = DevicePool::new("test_pool".to_string(), PoolConfig::default());
        pool.reserve_device("1-1".to_string(), "user1".to_string(), None)
            .await
            .unwrap();

        let status = pool.get_status().await;
        assert_eq!(status.name, "test_pool");
        assert_eq!(status.active_reservations.len(), 1);
    }

    #[tokio::test]
    async fn test_cleanup_expired() {
        let pool = DevicePool::new(
            "test_pool".to_string(),
            PoolConfig {
                default_timeout_seconds: 1,
                ..Default::default()
            },
        );

        pool.reserve_device("1-1".to_string(), "user1".to_string(), None)
            .await
            .unwrap();

        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        let removed = pool.cleanup_expired().await;
        assert_eq!(removed, 1);
        assert_eq!(pool.get_status().await.active_reservations.len(), 0);
    }

    #[tokio::test]
    async fn test_pool_manager() {
        let manager = PoolManager::new(PoolConfig::default());
        
        let pool1 = manager.get_or_create_pool("pool1".to_string()).await;
        let pool2 = manager.get_or_create_pool("pool2".to_string()).await;
        
        pool1.reserve_device("1-1".to_string(), "user1".to_string(), None).await.unwrap();
        pool2.reserve_device("2-1".to_string(), "user2".to_string(), None).await.unwrap();
        
        let statuses = manager.get_all_statuses().await;
        assert_eq!(statuses.len(), 2);
    }

    #[tokio::test]
    async fn test_concurrent_reservations() {
        let pool = Arc::new(DevicePool::new("test_pool".to_string(), PoolConfig::default()));
        let mut handles = vec![];
        
        for i in 0..5 {
            let pool_clone = Arc::clone(&pool);
            let handle = tokio::spawn(async move {
                pool_clone
                    .reserve_device(format!("{}-1", i), format!("user{}", i), None)
                    .await
            });
            handles.push(handle);
        }
        
        let results: Vec<_> = futures::future::join_all(handles).await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();
        
        assert_eq!(results.len(), 5);
        assert!(results.iter().all(|r| r.is_ok()));
    }
}
