// Unit tests for rate limiting

use usboverssh::rate_limit::{TokenBucket, RateLimiter, SimpleRateLimiter};
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
