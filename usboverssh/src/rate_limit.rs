//! Rate Limiting Module
//!
//! Provides token bucket rate limiting for preventing DoS attacks.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

/// Token bucket rate limiter
#[derive(Debug)]
pub struct TokenBucket {
    /// Maximum number of tokens (capacity)
    capacity: u64,
    /// Current number of tokens
    tokens: u64,
    /// Token refill rate (tokens per second)
    refill_rate: u64,
    /// Last refill time
    last_refill: Instant,
}

impl TokenBucket {
    /// Create new token bucket
    pub fn new(capacity: u64, refill_rate: u64) -> Self {
        Self {
            capacity,
            tokens: capacity,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    /// Try to consume a token
    /// Returns true if token was consumed, false if rate limited
    pub fn try_consume(&mut self) -> bool {
        self.refill();

        if self.tokens > 0 {
            self.tokens -= 1;
            true
        } else {
            false
        }
    }

    /// Try to consume multiple tokens
    /// Returns true if all tokens were consumed, false if rate limited
    pub fn try_consume_n(&mut self, count: u64) -> bool {
        self.refill();

        if self.tokens >= count {
            self.tokens -= count;
            true
        } else {
            false
        }
    }

    /// Refill tokens based on elapsed time
    fn refill(&mut self) {
        let elapsed = self.last_refill.elapsed();
        let elapsed_secs = elapsed.as_secs_f64();

        if elapsed_secs > 0.0 {
            let tokens_to_add = (elapsed_secs * self.refill_rate as f64) as u64;
            self.tokens = (self.tokens + tokens_to_add).min(self.capacity);
            self.last_refill = Instant::now();
        }
    }

    /// Get current token count
    pub fn available_tokens(&self) -> u64 {
        self.tokens
    }

    /// Reset the bucket (for testing)
    pub fn reset(&mut self) {
        self.tokens = self.capacity;
        self.last_refill = Instant::now();
    }
}

/// Rate limiter for multiple clients
#[derive(Debug)]
pub struct RateLimiter {
    /// Token buckets per client (identified by key)
    buckets: Arc<Mutex<HashMap<String, TokenBucket>>>,
    /// Default capacity for new buckets
    default_capacity: u64,
    /// Default refill rate for new buckets
    default_refill_rate: u64,
    /// Maximum number of clients to track
    max_clients: usize,
}

impl RateLimiter {
    /// Create new rate limiter
    pub fn new(default_capacity: u64, default_refill_rate: u64, max_clients: usize) -> Self {
        Self {
            buckets: Arc::new(Mutex::new(HashMap::new())),
            default_capacity,
            default_refill_rate,
            max_clients,
        }
    }

    /// Create with sensible defaults
    pub fn with_defaults() -> Self {
        Self::new(10, 1, 1000) // 10 tokens capacity, 1 token/sec refill, max 1000 clients
    }

    /// Check if a client is allowed to proceed
    /// Returns true if allowed, false if rate limited
    pub async fn check(&self, client_id: &str) -> bool {
        let mut buckets = self.buckets.lock().await;

        // Get or create bucket for this client
        if !buckets.contains_key(client_id) {
            // Prune old buckets if we're at max capacity
            if buckets.len() >= self.max_clients {
                // Simple FIFO: remove first key
                if let Some(key) = buckets.keys().next().cloned() {
                    buckets.remove(&key);
                }
            }

            buckets.insert(
                client_id.to_string(),
                TokenBucket::new(self.default_capacity, self.default_refill_rate),
            );
        }

        if let Some(bucket) = buckets.get_mut(client_id) {
            bucket.try_consume()
        } else {
            false
        }
    }

    /// Check if a client can consume multiple tokens
    pub async fn check_n(&self, client_id: &str, count: u64) -> bool {
        let mut buckets = self.buckets.lock().await;

        if !buckets.contains_key(client_id) {
            if buckets.len() >= self.max_clients {
                if let Some(key) = buckets.keys().next().cloned() {
                    buckets.remove(&key);
                }
            }

            buckets.insert(
                client_id.to_string(),
                TokenBucket::new(self.default_capacity, self.default_refill_rate),
            );
        }

        if let Some(bucket) = buckets.get_mut(client_id) {
            bucket.try_consume_n(count)
        } else {
            false
        }
    }

    /// Get available tokens for a client
    pub async fn available_tokens(&self, client_id: &str) -> u64 {
        let buckets = self.buckets.lock().await;
        buckets
            .get(client_id)
            .map(|b| b.available_tokens())
            .unwrap_or(0)
    }

    /// Remove a client's bucket
    pub async fn remove_client(&self, client_id: &str) {
        let mut buckets = self.buckets.lock().await;
        buckets.remove(client_id);
    }

    /// Clear all buckets
    pub async fn clear(&self) {
        let mut buckets = self.buckets.lock().await;
        buckets.clear();
    }

    /// Get number of tracked clients
    pub async fn client_count(&self) -> usize {
        let buckets = self.buckets.lock().await;
        buckets.len()
    }
}

/// Simple rate limiter (single bucket)
#[derive(Debug)]
pub struct SimpleRateLimiter {
    bucket: Arc<Mutex<TokenBucket>>,
}

impl SimpleRateLimiter {
    /// Create new simple rate limiter
    pub fn new(capacity: u64, refill_rate: u64) -> Self {
        Self {
            bucket: Arc::new(Mutex::new(TokenBucket::new(capacity, refill_rate))),
        }
    }

    /// Check if allowed (single global bucket)
    pub async fn check(&self) -> bool {
        let mut bucket = self.bucket.lock().await;
        bucket.try_consume()
    }

    /// Check if allowed with multiple tokens
    pub async fn check_n(&self, count: u64) -> bool {
        let mut bucket = self.bucket.lock().await;
        bucket.try_consume_n(count)
    }

    /// Get available tokens
    pub async fn available_tokens(&self) -> u64 {
        let bucket = self.bucket.lock().await;
        bucket.available_tokens()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_token_bucket_basic() {
        let mut bucket = TokenBucket::new(10, 1);

        // Should be able to consume 10 tokens
        for _ in 0..10 {
            assert!(bucket.try_consume());
        }

        // 11th should fail
        assert!(!bucket.try_consume());

        // Wait for refill
        thread::sleep(Duration::from_millis(1100));

        // Should have 1 token now
        assert!(bucket.try_consume());
        assert!(!bucket.try_consume());
    }

    #[test]
    fn test_token_bucket_multiple() {
        let mut bucket = TokenBucket::new(10, 5);

        // Consume 5 tokens
        assert!(bucket.try_consume_n(5));
        assert_eq!(bucket.available_tokens(), 5);

        // Try to consume 6, should fail
        assert!(!bucket.try_consume_n(6));
    }

    #[tokio::test]
    async fn test_rate_limiter_single_client() {
        let limiter = RateLimiter::new(5, 1, 10);

        // Consume all tokens
        for _ in 0..5 {
            assert!(limiter.check("client1").await);
        }

        // Should be rate limited
        assert!(!limiter.check("client1").await);

        // Different client should work
        assert!(limiter.check("client2").await);
    }

    #[tokio::test]
    async fn test_rate_limiter_max_clients() {
        let limiter = RateLimiter::new(5, 1, 2);

        limiter.check("client1").await;
        limiter.check("client2").await;

        // Third client should evict first
        limiter.check("client3").await;

        // First client should have been evicted and start fresh
        assert!(limiter.check("client1").await);
    }

    #[tokio::test]
    async fn test_simple_rate_limiter() {
        let limiter = SimpleRateLimiter::new(5, 1);

        for _ in 0..5 {
            assert!(limiter.check().await);
        }

        assert!(!limiter.check().await);
    }
}
