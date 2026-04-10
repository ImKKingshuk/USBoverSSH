// Integration tests for SSH tunneling

use usboverssh::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitBreakerState};
use usboverssh::connection_pool::{ConnectionPool, ConnectionPoolConfig};
use usboverssh::retry::{RetryConfig, retry_with_backoff};
use usboverssh::error::Error;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

#[tokio::test]
async fn test_retry_with_backoff_success() {
    let config = RetryConfig::default();
    let attempts = Arc::new(Mutex::new(0));

    let result = retry_with_backoff(config, || {
        let count = Arc::clone(&attempts);
        async move {
            *count.lock().await += 1;
            let current = *count.lock().await;
            if current <= 2 {
                Err(Error::Other("connection failed".to_string()))
            } else {
                Ok::<_, Error>(42)
            }
        }
    })
    .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);
    assert_eq!(*attempts.lock().await, 3);
}

#[tokio::test]
async fn test_retry_with_backoff_exhausted() {
    let config = RetryConfig {
        max_attempts: 2,
        initial_delay: Duration::from_millis(10),
        max_delay: Duration::from_millis(100),
        multiplier: 1.0,
    };

    let result = retry_with_backoff(config, || async {
        Err::<(), Error>(Error::Other("connection failed".to_string()))
    })
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_circuit_breaker_closed_to_open() {
    let config = CircuitBreakerConfig {
        failure_threshold: 3,
        success_threshold: 1,
        timeout: Duration::from_secs(10),
    };
    let cb = CircuitBreaker::new(config);

    // Fail until threshold
    for _ in 0..3 {
        let _ = cb.call(|| async {
            Err::<(), Error>(Error::Other("connection failed".to_string()))
        })
        .await;
    }

    assert_eq!(cb.state().await, CircuitBreakerState::Open);
}

#[tokio::test]
async fn test_circuit_breaker_blocks_when_open() {
    let config = CircuitBreakerConfig {
        failure_threshold: 2,
        success_threshold: 1,
        timeout: Duration::from_secs(10),
    };
    let cb = CircuitBreaker::new(config);

    // Fail until threshold
    for _ in 0..2 {
        let _ = cb.call(|| async {
            Err::<(), Error>(Error::Other("connection failed".to_string()))
        })
        .await;
    }

    // Should block when open
    let result = cb.call(|| async {
        Err::<(), Error>(Error::Other("connection failed".to_string()))
    })
    .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Circuit breaker is open"));
}

#[tokio::test]
async fn test_circuit_breaker_reset() {
    let config = CircuitBreakerConfig {
        failure_threshold: 2,
        success_threshold: 1,
        timeout: Duration::from_secs(10),
    };
    let cb = CircuitBreaker::new(config);

    // Fail until threshold
    for _ in 0..2 {
        let _ = cb.call(|| async {
            Err::<(), Error>(Error::Other("connection failed".to_string()))
        })
        .await;
    }

    assert_eq!(cb.state().await, CircuitBreakerState::Open);

    // Reset
    cb.reset().await;

    assert_eq!(cb.state().await, CircuitBreakerState::Closed);
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

    let host = usboverssh::config::HostConfig {
        hostname: "example.com".to_string(),
        port: 2222,
        user: "testuser".to_string(),
        identity_file: None,
        device_filters: vec![],
        description: None,
    };

    let result = pool.get_connection(&host).await;
    assert!(result.is_ok());

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

    let host1 = usboverssh::config::HostConfig {
        hostname: "example.com".to_string(),
        port: 2222,
        user: "testuser".to_string(),
        identity_file: None,
        device_filters: vec![],
        description: None,
    };

    let host2 = usboverssh::config::HostConfig {
        hostname: "example.com".to_string(),
        port: 2223,
        user: "testuser".to_string(),
        identity_file: None,
        device_filters: vec![],
        description: None,
    };

    pool.get_connection(&host1).await.unwrap();
    pool.get_connection(&host2).await.unwrap();

    let host3 = usboverssh::config::HostConfig {
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
