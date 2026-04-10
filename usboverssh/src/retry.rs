//! Retry Logic with Exponential Backoff
//!
//! Provides configurable retry mechanisms with exponential backoff and jitter.

use crate::error::{Error, Result};
use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Initial delay before first retry
    pub initial_delay: Duration,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Multiplier for exponential backoff
    pub multiplier: f64,
    /// Add jitter to prevent thundering herd
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            multiplier: 2.0,
            jitter: true,
        }
    }
}

impl RetryConfig {
    /// Create a new retry config with custom values
    pub fn new(
        max_attempts: u32,
        initial_delay: Duration,
        max_delay: Duration,
        multiplier: f64,
        jitter: bool,
    ) -> Self {
        Self {
            max_attempts,
            initial_delay,
            max_delay,
            multiplier,
            jitter,
        }
    }

    /// Create a retry config for SSH connections
    pub fn for_ssh() -> Self {
        Self {
            max_attempts: 5,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
            multiplier: 2.0,
            jitter: true,
        }
    }

    /// Create a retry config for USB/IP operations
    pub fn for_usbip() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(10),
            multiplier: 1.5,
            jitter: true,
        }
    }

    /// Calculate delay for a given attempt number
    fn calculate_delay(&self, attempt: u32) -> Duration {
        let delay_ms = self.initial_delay.as_millis() as f64
            * self.multiplier.powi(attempt as i32 - 1);

        let delay = Duration::from_millis(delay_ms as u64).min(self.max_delay);

        if self.jitter {
            // Add random jitter up to 25% of the delay
            let jitter_ms = (delay.as_millis() as f64 * 0.25) as u64;
            let random_jitter = fastrand::u64(0..=jitter_ms);
            let jitter_sign = if fastrand::bool() { 1 } else { -1 };
            let jittered_ms = delay.as_millis() as i64 + (random_jitter as i64 * jitter_sign);
            Duration::from_millis(jittered_ms.max(0) as u64)
        } else {
            delay
        }
    }
}

/// Retry an async operation with exponential backoff
pub async fn retry_with_backoff<F, Fut, T, E>(
    config: RetryConfig,
    operation: F,
) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::error::Error + Send + Sync + 'static,
{
    let mut last_error = None;

    for attempt in 1..=config.max_attempts {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                last_error = Some(Error::Other(e.to_string()));

                if attempt < config.max_attempts {
                    let delay = config.calculate_delay(attempt);
                    tracing::warn!(
                        "Attempt {}/{} failed, retrying after {:?}",
                        attempt,
                        config.max_attempts,
                        delay
                    );
                    sleep(delay).await;
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| {
        Error::Other(format!(
            "Operation failed after {} attempts",
            config.max_attempts
        ))
    }))
}

/// Retry an async operation with a custom error handler
pub async fn retry_with_backoff_handler<F, Fut, T, E, H>(
    config: RetryConfig,
    operation: F,
    handler: H,
) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::error::Error + Send + Sync + 'static,
    H: Fn(&E, u32) -> bool,
{
    let mut last_error = None;

    for attempt in 1..=config.max_attempts {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                last_error = Some(Error::Other(e.to_string()));

                // Check if handler says we should continue retrying
                if !handler(&e, attempt) {
                    break;
                }

                if attempt < config.max_attempts {
                    let delay = config.calculate_delay(attempt);
                    tracing::warn!(
                        "Attempt {}/{} failed, retrying after {:?}",
                        attempt,
                        config.max_attempts,
                        delay
                    );
                    sleep(delay).await;
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| {
        Error::Other(format!(
            "Operation failed after {} attempts",
            config.max_attempts
        ))
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.initial_delay, Duration::from_millis(100));
        assert_eq!(config.max_delay, Duration::from_secs(30));
        assert_eq!(config.multiplier, 2.0);
        assert!(config.jitter);
    }

    #[test]
    fn test_retry_config_for_ssh() {
        let config = RetryConfig::for_ssh();
        assert_eq!(config.max_attempts, 5);
        assert_eq!(config.initial_delay, Duration::from_secs(1));
        assert_eq!(config.max_delay, Duration::from_secs(30));
    }

    #[test]
    fn test_retry_config_for_usbip() {
        let config = RetryConfig::for_usbip();
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.initial_delay, Duration::from_millis(500));
        assert_eq!(config.max_delay, Duration::from_secs(10));
    }

    #[test]
    fn test_calculate_delay() {
        let config = RetryConfig {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            multiplier: 2.0,
            jitter: false,
        };

        // Attempt 1: initial delay
        let delay1 = config.calculate_delay(1);
        assert_eq!(delay1, Duration::from_millis(100));

        // Attempt 2: initial * multiplier
        let delay2 = config.calculate_delay(2);
        assert_eq!(delay2, Duration::from_millis(200));

        // Attempt 3: initial * multiplier^2
        let delay3 = config.calculate_delay(3);
        assert_eq!(delay3, Duration::from_millis(400));
    }

    #[test]
    fn test_calculate_delay_with_max() {
        let config = RetryConfig {
            max_attempts: 10,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_millis(300),
            multiplier: 10.0,
            jitter: false,
        };

        // Should cap at max_delay
        let delay = config.calculate_delay(5);
        assert_eq!(delay, Duration::from_millis(300));
    }

    #[tokio::test]
    async fn test_retry_success_on_first_attempt() {
        let config = RetryConfig::default();
        let mut attempt_count = 0;

        let result = retry_with_backoff(config, || {
            attempt_count += 1;
            async { Ok::<_, std::io::Error>(42) }
        })
        .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempt_count, 1);
    }

    #[tokio::test]
    async fn test_retry_success_after_retry() {
        let config = RetryConfig {
            max_attempts: 3,
            initial_delay: Duration::from_millis(10),
            max_delay: Duration::from_millis(100),
            multiplier: 1.0,
            jitter: false,
        };
        let mut attempt_count = 0;

        let result = retry_with_backoff(config, || {
            attempt_count += 1;
            async {
                if attempt_count < 2 {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::ConnectionRefused,
                        "connection refused",
                    ))
                } else {
                    Ok::<_, std::io::Error>(42)
                }
            }
        })
        .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempt_count, 2);
    }

    #[tokio::test]
    async fn test_retry_exhausts_attempts() {
        let config = RetryConfig {
            max_attempts: 2,
            initial_delay: Duration::from_millis(10),
            max_delay: Duration::from_millis(100),
            multiplier: 1.0,
            jitter: false,
        };
        let mut attempt_count = 0;

        let result = retry_with_backoff(config, || {
            attempt_count += 1;
            async {
                Err::<(), std::io::Error>(std::io::Error::new(
                    std::io::ErrorKind::ConnectionRefused,
                    "connection refused",
                ))
            }
        })
        .await;

        assert!(result.is_err());
        assert_eq!(attempt_count, 2);
    }
}
