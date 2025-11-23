use chrono::Utc;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::error::{ApiError, ApiResult};

/// Rate limit configuration for a virtual key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    pub requests_per_minute: Option<i32>,
    pub tokens_per_minute: Option<i32>,
}

/// Rate limit status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitStatus {
    pub limited: bool,
    pub requests_remaining: Option<i32>,
    pub tokens_remaining: Option<i32>,
    pub reset_at: Option<i64>,    // Unix timestamp
    pub retry_after: Option<i64>, // Seconds until reset
}

/// Rate limiter using sliding window algorithm with Redis
#[derive(Clone)]
pub struct RateLimiter {
    redis_client: Option<redis::aio::ConnectionManager>,
    window_size_seconds: i64,
}

impl RateLimiter {
    pub fn new(redis_client: Option<redis::aio::ConnectionManager>) -> Self {
        Self {
            redis_client,
            window_size_seconds: 60, // 1 minute window
        }
    }

    /// Check if a request is allowed and increment counters
    /// Returns RateLimitStatus with remaining capacity and reset time
    pub async fn check_and_increment(
        &self,
        key_id: &str,
        rate_limit: &RateLimit,
        tokens: i32,
    ) -> ApiResult<RateLimitStatus> {
        // If rate limiting is disabled (no Redis or no limits), allow all requests
        if self.redis_client.is_none() {
            return Ok(RateLimitStatus {
                limited: false,
                requests_remaining: None,
                tokens_remaining: None,
                reset_at: None,
                retry_after: None,
            });
        }

        // If no limits are configured for this key, allow the request
        if rate_limit.requests_per_minute.is_none() && rate_limit.tokens_per_minute.is_none() {
            return Ok(RateLimitStatus {
                limited: false,
                requests_remaining: None,
                tokens_remaining: None,
                reset_at: None,
                retry_after: None,
            });
        }

        let redis_conn = self.redis_client.as_ref().unwrap();
        let now = Utc::now().timestamp();
        let window_start = now - self.window_size_seconds;

        // Check and increment request count
        let requests_status = if let Some(rpm_limit) = rate_limit.requests_per_minute {
            self.check_and_increment_counter(
                redis_conn,
                &format!("ratelimit:{}:requests", key_id),
                rpm_limit,
                now,
                window_start,
                1,
            )
            .await?
        } else {
            CounterStatus {
                allowed: true,
                remaining: None,
                reset_at: now + self.window_size_seconds,
            }
        };

        // Check and increment token count
        let tokens_status = if let Some(tpm_limit) = rate_limit.tokens_per_minute {
            self.check_and_increment_counter(
                redis_conn,
                &format!("ratelimit:{}:tokens", key_id),
                tpm_limit,
                now,
                window_start,
                tokens,
            )
            .await?
        } else {
            CounterStatus {
                allowed: true,
                remaining: None,
                reset_at: now + self.window_size_seconds,
            }
        };

        // If either limit is exceeded, deny the request
        if !requests_status.allowed || !tokens_status.allowed {
            let reset_at = std::cmp::max(requests_status.reset_at, tokens_status.reset_at);
            let retry_after = reset_at - now;

            warn!(
                "Rate limit exceeded for key {}: requests_allowed={}, tokens_allowed={}",
                key_id, requests_status.allowed, tokens_status.allowed
            );

            return Ok(RateLimitStatus {
                limited: true,
                requests_remaining: requests_status.remaining,
                tokens_remaining: tokens_status.remaining,
                reset_at: Some(reset_at),
                retry_after: Some(retry_after),
            });
        }

        // Request allowed
        debug!(
            "Rate limit check passed for key {}: requests_remaining={:?}, tokens_remaining={:?}",
            key_id, requests_status.remaining, tokens_status.remaining
        );

        Ok(RateLimitStatus {
            limited: false,
            requests_remaining: requests_status.remaining,
            tokens_remaining: tokens_status.remaining,
            reset_at: Some(requests_status.reset_at),
            retry_after: None,
        })
    }

    /// Get current rate limit status without incrementing
    pub async fn get_status(
        &self,
        key_id: &str,
        rate_limit: &RateLimit,
    ) -> ApiResult<RateLimitStatus> {
        if self.redis_client.is_none() {
            return Ok(RateLimitStatus {
                limited: false,
                requests_remaining: None,
                tokens_remaining: None,
                reset_at: None,
                retry_after: None,
            });
        }

        if rate_limit.requests_per_minute.is_none() && rate_limit.tokens_per_minute.is_none() {
            return Ok(RateLimitStatus {
                limited: false,
                requests_remaining: None,
                tokens_remaining: None,
                reset_at: None,
                retry_after: None,
            });
        }

        let redis_conn = self.redis_client.as_ref().unwrap();
        let now = Utc::now().timestamp();
        let window_start = now - self.window_size_seconds;

        // Get request count
        let requests_remaining = if let Some(rpm_limit) = rate_limit.requests_per_minute {
            let current = self
                .get_counter_value(
                    redis_conn,
                    &format!("ratelimit:{}:requests", key_id),
                    window_start,
                )
                .await?;
            Some(std::cmp::max(0, rpm_limit - current))
        } else {
            None
        };

        // Get token count
        let tokens_remaining = if let Some(tpm_limit) = rate_limit.tokens_per_minute {
            let current = self
                .get_counter_value(
                    redis_conn,
                    &format!("ratelimit:{}:tokens", key_id),
                    window_start,
                )
                .await?;
            Some(std::cmp::max(0, tpm_limit - current))
        } else {
            None
        };

        Ok(RateLimitStatus {
            limited: false,
            requests_remaining,
            tokens_remaining,
            reset_at: Some(now + self.window_size_seconds),
            retry_after: None,
        })
    }

    /// Sliding window counter implementation using Redis sorted sets
    /// Returns whether the request is allowed and remaining capacity
    async fn check_and_increment_counter(
        &self,
        redis_conn: &redis::aio::ConnectionManager,
        key: &str,
        limit: i32,
        now: i64,
        window_start: i64,
        increment: i32,
    ) -> ApiResult<CounterStatus> {
        let mut conn = redis_conn.clone();

        // Use Redis pipeline for atomic operations
        let pipe = redis::pipe()
            // Remove old entries outside the window
            .zrembyscore(key, "-inf", window_start)
            // Count current entries in the window
            .zcount(key, window_start, "+inf")
            // Add new entry with current timestamp as score
            // Use unique member by appending microseconds to avoid collisions
            .zadd(
                key,
                format!("{}:{}", now, Utc::now().timestamp_subsec_micros()),
                now,
            )
            // Set expiration to window size + buffer
            .expire(key, self.window_size_seconds + 10)
            .clone();

        let results: Vec<i32> = pipe
            .query_async(&mut conn)
            .await
            .map_err(|e| ApiError::RateLimitError(format!("Redis error: {}", e)))?;

        // Results: [removed_count, current_count, zadd_result, expire_result]
        let current_count = results.get(1).copied().unwrap_or(0);

        // Check if adding this request would exceed the limit
        // We check current_count (before increment) + increment <= limit
        if current_count + increment > limit {
            // Calculate reset time (start of next window)
            let reset_at = now + self.window_size_seconds;

            return Ok(CounterStatus {
                allowed: false,
                remaining: Some(std::cmp::max(0, limit - current_count)),
                reset_at,
            });
        }

        // Increment the counter by adding 'increment' entries
        // For tokens, we add multiple entries to represent token usage
        if increment > 1 {
            let mut pipe = redis::pipe();
            for i in 1..increment {
                pipe.zadd(
                    key,
                    format!("{}:{}:{}", now, Utc::now().timestamp_subsec_micros(), i),
                    now,
                );
            }
            let _: () = pipe
                .query_async(&mut conn)
                .await
                .map_err(|e| ApiError::RateLimitError(format!("Redis error: {}", e)))?;
        }

        let new_count = current_count + increment;
        let remaining = limit - new_count;

        Ok(CounterStatus {
            allowed: true,
            remaining: Some(std::cmp::max(0, remaining)),
            reset_at: now + self.window_size_seconds,
        })
    }

    /// Get the current counter value without incrementing
    async fn get_counter_value(
        &self,
        redis_conn: &redis::aio::ConnectionManager,
        key: &str,
        window_start: i64,
    ) -> ApiResult<i32> {
        let mut conn = redis_conn.clone();

        let count: i32 = conn
            .zcount(key, window_start, "+inf")
            .await
            .map_err(|e| ApiError::RateLimitError(format!("Redis error: {}", e)))?;

        Ok(count)
    }

    /// Reset rate limits for a key (for testing or admin operations)
    pub async fn reset(&self, key_id: &str) -> ApiResult<()> {
        if let Some(redis_conn) = &self.redis_client {
            let mut conn = redis_conn.clone();
            let _: () = conn
                .del(&[
                    format!("ratelimit:{}:requests", key_id),
                    format!("ratelimit:{}:tokens", key_id),
                ])
                .await
                .map_err(|e| ApiError::RateLimitError(format!("Redis error: {}", e)))?;

            debug!("Rate limits reset for key {}", key_id);
        }
        Ok(())
    }
}

#[derive(Debug)]
struct CounterStatus {
    allowed: bool,
    remaining: Option<i32>,
    reset_at: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_status_serialization() {
        let status = RateLimitStatus {
            limited: true,
            requests_remaining: Some(10),
            tokens_remaining: Some(1000),
            reset_at: Some(1234567890),
            retry_after: Some(30),
        };

        let json = serde_json::to_string(&status).unwrap();
        let deserialized: RateLimitStatus = serde_json::from_str(&json).unwrap();

        assert_eq!(status.limited, deserialized.limited);
        assert_eq!(status.requests_remaining, deserialized.requests_remaining);
        assert_eq!(status.tokens_remaining, deserialized.tokens_remaining);
    }

    #[test]
    fn test_rate_limit_no_limits() {
        let rate_limit = RateLimit {
            requests_per_minute: None,
            tokens_per_minute: None,
        };

        assert!(rate_limit.requests_per_minute.is_none());
        assert!(rate_limit.tokens_per_minute.is_none());
    }
}
