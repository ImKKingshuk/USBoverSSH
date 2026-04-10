//! Connection Pool for SSH Sessions
//!
//! Implements SSH connection pooling with keep-alive and health checks.

use crate::config::HostConfig;
use crate::error::{Error, Result};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

/// Pooled connection with metadata
pub struct PooledConnection {
    /// Connection key (user@host:port)
    pub key: String,
    /// Creation timestamp
    pub created_at: Instant,
    /// Last used timestamp
    pub last_used: Instant,
}

/// Connection pool configuration
#[derive(Debug, Clone)]
pub struct ConnectionPoolConfig {
    /// Maximum number of connections per host
    pub max_size_per_host: usize,
    /// Idle timeout before connection is closed
    pub idle_timeout: Duration,
    /// Maximum connection lifetime
    pub max_lifetime: Duration,
    /// Health check interval
    pub health_check_interval: Duration,
}

impl Default for ConnectionPoolConfig {
    fn default() -> Self {
        Self {
            max_size_per_host: 5,
            idle_timeout: Duration::from_secs(300), // 5 minutes
            max_lifetime: Duration::from_secs(3600), // 1 hour
            health_check_interval: Duration::from_secs(60), // 1 minute
        }
    }
}

impl ConnectionPoolConfig {
    /// Create a new connection pool config
    pub fn new(
        max_size_per_host: usize,
        idle_timeout: Duration,
        max_lifetime: Duration,
        health_check_interval: Duration,
    ) -> Self {
        Self {
            max_size_per_host,
            idle_timeout,
            max_lifetime,
            health_check_interval,
        }
    }
}

/// Connection pool for SSH sessions
pub struct ConnectionPool {
    /// Map of host key to connection metadata
    pool: Arc<Mutex<HashMap<String, PooledConnection>>>,
    /// Configuration
    config: ConnectionPoolConfig,
    /// Cleanup task handle
    cleanup_task: Option<JoinHandle<()>>,
}

impl ConnectionPool {
    /// Create a new connection pool
    pub fn new(config: ConnectionPoolConfig) -> Self {
        let pool = Arc::new(Mutex::new(HashMap::new()));
        let pool_clone = Arc::clone(&pool);
        let config_clone = config.clone();

        // Start cleanup task
        let cleanup_task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(config_clone.health_check_interval);
            loop {
                interval.tick().await;
                Self::cleanup_idle_connections(&pool_clone, &config_clone).await;
            }
        });

        Self {
            pool,
            config,
            cleanup_task: Some(cleanup_task),
        }
    }

    /// Create connection pool with default config
    pub fn new_default() -> Self {
        Self::new(ConnectionPoolConfig::default())
    }
}

impl Default for ConnectionPool {
    fn default() -> Self {
        Self::new(ConnectionPoolConfig::default())
    }
}

impl ConnectionPool {
    /// Generate connection key from host config
    pub fn key_from_host(host: &HostConfig) -> String {
        format!("{}@{}:{}", host.user, host.hostname, host.port)
    }

    /// Get a connection (placeholder - would return actual SSH session)
    pub async fn get_connection(&self, host: &HostConfig) -> Result<()> {
        let key = Self::key_from_host(host);
        let mut pool = self.pool.lock().await;

        // Check if connection exists and is valid
        if let Some(conn) = pool.get(&key) {
            // Check if connection is still valid
            let now = Instant::now();
            if conn.created_at.elapsed() < self.config.max_lifetime
                && conn.last_used.elapsed() < self.config.idle_timeout
            {
                // Update last used time
                if let Some(conn) = pool.get_mut(&key) {
                    conn.last_used = now;
                }
                return Ok(());
            }
        }

        // Check pool size limit
        let host_connections: Vec<_> = pool
            .iter()
            .filter(|(k, _)| k.starts_with(&format!("{}@{}", host.user, host.hostname)))
            .collect();

        if host_connections.len() >= self.config.max_size_per_host {
            return Err(Error::Other("Connection pool full for this host".to_string()));
        }

        // Create new connection (placeholder)
        let now = Instant::now();
        pool.insert(
            key,
            PooledConnection {
                key: Self::key_from_host(host),
                created_at: now,
                last_used: now,
            },
        );

        Ok(())
    }

    /// Return a connection to the pool
    pub async fn return_connection(&self, host: &HostConfig) {
        let key = Self::key_from_host(host);
        let mut pool = self.pool.lock().await;

        if let Some(conn) = pool.get_mut(&key) {
            conn.last_used = Instant::now();
        }
    }

    /// Cleanup idle and expired connections
    async fn cleanup_idle_connections(
        pool: &Arc<Mutex<HashMap<String, PooledConnection>>>,
        config: &ConnectionPoolConfig,
    ) {
        let mut pool = pool.lock().await;
        pool.retain(|_, conn| {
            conn.created_at.elapsed() < config.max_lifetime
                && conn.last_used.elapsed() < config.idle_timeout
        });
    }

    /// Get pool statistics
    pub async fn stats(&self) -> PoolStats {
        let pool = self.pool.lock().await;
        PoolStats {
            total_connections: pool.len(),
            idle_connections: pool
                .values()
                .filter(|c| c.last_used.elapsed() > Duration::from_secs(30))
                .count(),
        }
    }

    /// Close all connections
    pub async fn close_all(&self) {
        let mut pool = self.pool.lock().await;
        pool.clear();
    }
}

impl Drop for ConnectionPool {
    fn drop(&mut self) {
        // Abort cleanup task
        if let Some(task) = self.cleanup_task.take() {
            task.abort();
        }
    }
}

/// Connection pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    /// Total number of connections
    pub total_connections: usize,
    /// Number of idle connections (> 30s unused)
    pub idle_connections: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_pool_config_default() {
        let config = ConnectionPoolConfig::default();
        assert_eq!(config.max_size_per_host, 5);
        assert_eq!(config.idle_timeout, Duration::from_secs(300));
        assert_eq!(config.max_lifetime, Duration::from_secs(3600));
        assert_eq!(config.health_check_interval, Duration::from_secs(60));
    }

    #[test]
    fn test_key_from_host() {
        let host = HostConfig {
            hostname: "example.com".to_string(),
            port: 2222,
            user: "testuser".to_string(),
            identity_file: None,
            device_filters: vec![],
            description: None,
        };

        let key = ConnectionPool::key_from_host(&host);
        assert_eq!(key, "testuser@example.com:2222");
    }

    #[tokio::test]
    async fn test_connection_pool_get_connection() {
        let config = ConnectionPoolConfig {
            max_size_per_host: 5,
            idle_timeout: Duration::from_secs(300),
            max_lifetime: Duration::from_secs(3600),
            health_check_interval: Duration::from_secs(1),
        };
        let pool = ConnectionPool::new(config);

        let host = HostConfig {
            hostname: "example.com".to_string(),
            port: 2222,
            user: "testuser".to_string(),
            identity_file: None,
            device_filters: vec![],
            description: None,
        };

        // Get connection
        let result = pool.get_connection(&host).await;
        assert!(result.is_ok());

        // Get stats
        let stats = pool.stats().await;
        assert_eq!(stats.total_connections, 1);
    }

    #[tokio::test]
    async fn test_connection_pool_reuse_connection() {
        let config = ConnectionPoolConfig {
            max_size_per_host: 5,
            idle_timeout: Duration::from_secs(300),
            max_lifetime: Duration::from_secs(3600),
            health_check_interval: Duration::from_secs(1),
        };
        let pool = ConnectionPool::new(config);

        let host = HostConfig {
            hostname: "example.com".to_string(),
            port: 2222,
            user: "testuser".to_string(),
            identity_file: None,
            device_filters: vec![],
            description: None,
        };

        // Get connection
        pool.get_connection(&host).await.unwrap();

        // Return connection
        pool.return_connection(&host).await;

        // Get same connection again
        let result = pool.get_connection(&host).await;
        assert!(result.is_ok());

        // Should still be 1 connection (reused)
        let stats = pool.stats().await;
        assert_eq!(stats.total_connections, 1);
    }

    #[tokio::test]
    async fn test_connection_pool_size_limit() {
        let config = ConnectionPoolConfig {
            max_size_per_host: 2,
            idle_timeout: Duration::from_secs(300),
            max_lifetime: Duration::from_secs(3600),
            health_check_interval: Duration::from_secs(1),
        };
        let pool = ConnectionPool::new(config);

        let host1 = HostConfig {
            hostname: "example.com".to_string(),
            port: 2222,
            user: "testuser".to_string(),
            identity_file: None,
            device_filters: vec![],
            description: None,
        };

        let host2 = HostConfig {
            hostname: "example.com".to_string(),
            port: 2223,
            user: "testuser".to_string(),
            identity_file: None,
            device_filters: vec![],
            description: None,
        };

        // Get 2 connections
        pool.get_connection(&host1).await.unwrap();
        pool.get_connection(&host2).await.unwrap();

        // Third connection should fail (pool full for this host)
        let host3 = HostConfig {
            hostname: "example.com".to_string(),
            port: 2224,
            user: "testuser".to_string(),
            identity_file: None,
            device_filters: vec![],
            description: None,
        };

        let result = pool.get_connection(&host3).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_connection_pool_close_all() {
        let config = ConnectionPoolConfig {
            max_size_per_host: 5,
            idle_timeout: Duration::from_secs(300),
            max_lifetime: Duration::from_secs(3600),
            health_check_interval: Duration::from_secs(1),
        };
        let pool = ConnectionPool::new(config);

        let host = HostConfig {
            hostname: "example.com".to_string(),
            port: 2222,
            user: "testuser".to_string(),
            identity_file: None,
            device_filters: vec![],
            description: None,
        };

        pool.get_connection(&host).await.unwrap();

        let stats = pool.stats().await;
        assert_eq!(stats.total_connections, 1);

        // Close all
        pool.close_all().await;

        let stats = pool.stats().await;
        assert_eq!(stats.total_connections, 0);
    }
}
