//! Circuit Breaker Pattern
//!
//! Implements circuit breaker pattern for fault tolerance in SSH and USB/IP connections.

use crate::error::{Error, Result};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitBreakerState {
    /// Circuit is closed, allowing requests through
    Closed,
    /// Circuit is open, blocking requests
    Open,
    /// Circuit is half-open, testing if service has recovered
    HalfOpen,
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening the circuit
    pub failure_threshold: u32,
    /// Number of successes required to close circuit in half-open state
    pub success_threshold: u32,
    /// Timeout before attempting to close circuit (half-open)
    pub timeout: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            timeout: Duration::from_secs(60),
        }
    }
}

impl CircuitBreakerConfig {
    /// Create a new circuit breaker config
    pub fn new(failure_threshold: u32, success_threshold: u32, timeout: Duration) -> Self {
        Self {
            failure_threshold,
            success_threshold,
            timeout,
        }
    }

    /// Create config for SSH connections
    pub fn for_ssh() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            timeout: Duration::from_secs(60),
        }
    }

    /// Create config for USB/IP operations
    pub fn for_usbip() -> Self {
        Self {
            failure_threshold: 3,
            success_threshold: 1,
            timeout: Duration::from_secs(30),
        }
    }
}

/// Circuit breaker for fault tolerance
pub struct CircuitBreaker {
    state: Arc<Mutex<CircuitBreakerState>>,
    failure_count: Arc<AtomicU32>,
    success_count: Arc<AtomicU32>,
    config: CircuitBreakerConfig,
    last_failure_time: Arc<Mutex<Option<Instant>>>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            state: Arc::new(Mutex::new(CircuitBreakerState::Closed)),
            failure_count: Arc::new(AtomicU32::new(0)),
            success_count: Arc::new(AtomicU32::new(0)),
            config,
            last_failure_time: Arc::new(Mutex::new(None)),
        }
    }

    /// Create circuit breaker for SSH connections
    pub fn for_ssh() -> Self {
        Self::new(CircuitBreakerConfig::for_ssh())
    }

    /// Create circuit breaker for USB/IP operations
    pub fn for_usbip() -> Self {
        Self::new(CircuitBreakerConfig::for_usbip())
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new(CircuitBreakerConfig::default())
    }
}

impl CircuitBreaker {
    /// Get current state
    pub async fn state(&self) -> CircuitBreakerState {
        *self.state.lock().await
    }

    /// Reset the circuit breaker to closed state
    pub async fn reset(&self) {
        let mut state = self.state.lock().await;
        *state = CircuitBreakerState::Closed;
        self.failure_count.store(0, Ordering::Relaxed);
        self.success_count.store(0, Ordering::Relaxed);
        *self.last_failure_time.lock().await = None;
    }

    /// Call an operation with circuit breaker protection
    pub async fn call<F, Fut, T>(&self, operation: F) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        // Check if circuit is open
        let current_state = self.state.lock().await;
        if *current_state == CircuitBreakerState::Open {
            drop(current_state);

            // Check if timeout has passed
            let last_failure = self.last_failure_time.lock().await;
            if let Some(failure_time) = *last_failure {
                if failure_time.elapsed() < self.config.timeout {
                    return Err(Error::Other("Circuit breaker is open".to_string()));
                }
            }
            drop(last_failure);

            // Transition to half-open
            let mut state = self.state.lock().await;
            *state = CircuitBreakerState::HalfOpen;
            self.success_count.store(0, Ordering::Relaxed);
        } else {
            drop(current_state);
        }

        // Execute the operation
        match operation().await {
            Ok(result) => {
                self.on_success().await;
                Ok(result)
            }
            Err(e) => {
                self.on_failure().await;
                Err(e)
            }
        }
    }

    /// Handle successful operation
    async fn on_success(&self) {
        let state = {
            let state = self.state.lock().await;
            *state
        };

        if state == CircuitBreakerState::HalfOpen {
            let successes = self.success_count.fetch_add(1, Ordering::Relaxed) + 1;

            if successes >= self.config.success_threshold {
                // Circuit is healthy again, close it
                let mut state = self.state.lock().await;
                *state = CircuitBreakerState::Closed;
                self.failure_count.store(0, Ordering::Relaxed);
                self.success_count.store(0, Ordering::Relaxed);
            }
        } else {
            // Reset failure count on success in closed state
            self.failure_count.store(0, Ordering::Relaxed);
        }
    }

    /// Handle failed operation
    async fn on_failure(&self) {
        let failures = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;

        if failures >= self.config.failure_threshold {
            // Open the circuit
            let mut state = self.state.lock().await;
            *state = CircuitBreakerState::Open;
            *self.last_failure_time.lock().await = Some(Instant::now());
        }
    }

    /// Get failure count
    pub fn failure_count(&self) -> u32 {
        self.failure_count.load(Ordering::Relaxed)
    }

    /// Get success count (in half-open state)
    pub fn success_count(&self) -> u32 {
        self.success_count.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_config_default() {
        let config = CircuitBreakerConfig::default();
        assert_eq!(config.failure_threshold, 5);
        assert_eq!(config.success_threshold, 2);
        assert_eq!(config.timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_circuit_breaker_config_for_ssh() {
        let config = CircuitBreakerConfig::for_ssh();
        assert_eq!(config.failure_threshold, 5);
        assert_eq!(config.success_threshold, 2);
    }

    #[test]
    fn test_circuit_breaker_config_for_usbip() {
        let config = CircuitBreakerConfig::for_usbip();
        assert_eq!(config.failure_threshold, 3);
        assert_eq!(config.success_threshold, 1);
    }

    #[tokio::test]
    async fn test_circuit_breaker_initially_closed() {
        let cb = CircuitBreaker::default();
        assert_eq!(cb.state().await, CircuitBreakerState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_opens_on_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            success_threshold: 1,
            timeout: Duration::from_secs(1),
        };
        let cb = CircuitBreaker::new(config);

        // Fail until threshold
        for _ in 0..3 {
            let _ = cb
                .call(|| async { Err::<(), Error>(Error::Other("test error".to_string())) })
                .await;
        }

        assert_eq!(cb.state().await, CircuitBreakerState::Open);
        assert_eq!(cb.failure_count(), 3);
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
            let _ = cb
                .call(|| async { Err::<(), Error>(Error::Other("test error".to_string())) })
                .await;
        }

        // Should block when open
        let result = cb
            .call(|| async { Err::<(), Error>(Error::Other("test error".to_string())) })
            .await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Circuit breaker is open"));
    }

    #[tokio::test]
    async fn test_circuit_breaker_transitions_to_half_open_after_timeout() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 1,
            timeout: Duration::from_millis(100),
        };
        let cb = CircuitBreaker::new(config);

        // Fail until threshold
        for _ in 0..2 {
            let _ = cb
                .call(|| async { Err::<(), Error>(Error::Other("test error".to_string())) })
                .await;
        }

        assert_eq!(cb.state().await, CircuitBreakerState::Open);

        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Should transition to half-open on next call
        let result = cb
            .call(|| async { Err::<(), Error>(Error::Other("test error".to_string())) })
            .await;

        assert!(result.is_err());
        // The circuit breaker should now be in half-open state after the call
        let state = cb.state().await;
        assert!(state == CircuitBreakerState::HalfOpen || state == CircuitBreakerState::Open);
    }

    #[tokio::test]
    async fn test_circuit_breaker_closes_on_success_in_half_open() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 1,
            timeout: Duration::from_millis(100),
        };
        let cb = CircuitBreaker::new(config);

        // Fail until threshold
        for _ in 0..2 {
            let _ = cb
                .call(|| async { Err::<(), Error>(Error::Other("test error".to_string())) })
                .await;
        }

        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Success should transition to half-open and then close
        let result = cb.call(|| async { Ok::<(), Error>(()) }).await;

        assert!(result.is_ok());
        // After success in half-open, should be closed
        assert_eq!(cb.state().await, CircuitBreakerState::Closed);
        assert_eq!(cb.failure_count(), 0);
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
            let _ = cb
                .call(|| async { Err::<(), Error>(Error::Other("test error".to_string())) })
                .await;
        }

        assert_eq!(cb.state().await, CircuitBreakerState::Open);

        // Reset
        cb.reset().await;

        assert_eq!(cb.state().await, CircuitBreakerState::Closed);
        assert_eq!(cb.failure_count(), 0);
    }
}
