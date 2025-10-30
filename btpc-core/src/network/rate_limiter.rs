//! Rate limiting for peer connections (Issue #1 - HIGH)
//!
//! Implements token bucket rate limiting to prevent DoS attacks via message flooding.
//! Each peer has separate rate limits for:
//! - Message count per second
//! - Bandwidth (bytes) per second

use std::time::{Duration, Instant};
use thiserror::Error;

/// Rate limiter configuration
#[derive(Debug, Clone)]
pub struct RateLimiterConfig {
    /// Maximum messages per second
    pub messages_per_second: f64,
    /// Maximum bytes per second
    pub bytes_per_second: usize,
    /// Window duration for rate calculation
    pub window_duration: Duration,
}

impl Default for RateLimiterConfig {
    fn default() -> Self {
        RateLimiterConfig {
            messages_per_second: 100.0,       // 100 messages/sec
            bytes_per_second: 5_000_000,      // 5 MB/sec
            window_duration: Duration::from_secs(1),
        }
    }
}

/// Token bucket rate limiter for a single peer
pub struct PeerRateLimiter {
    config: RateLimiterConfig,
    /// Message count in current window
    message_count: u32,
    /// Byte count in current window
    byte_count: usize,
    /// When the current window started
    window_start: Instant,
}

impl PeerRateLimiter {
    /// Create new rate limiter with default configuration
    pub fn new() -> Self {
        Self::with_config(RateLimiterConfig::default())
    }

    /// Create new rate limiter with custom configuration
    pub fn with_config(config: RateLimiterConfig) -> Self {
        PeerRateLimiter {
            config,
            message_count: 0,
            byte_count: 0,
            window_start: Instant::now(),
        }
    }

    /// Check if a message can be accepted, and record it if so
    ///
    /// Returns Ok(()) if within limits, Err if rate limit exceeded.
    /// If Ok, the message is recorded and counts toward the limit.
    pub fn check_and_record(&mut self, message_size: usize) -> Result<(), RateLimitError> {
        self.refresh_window_if_needed();

        // Check message rate limit
        if self.message_count as f64 >= self.config.messages_per_second {
            return Err(RateLimitError::MessageRateExceeded {
                current: self.message_count,
                limit: self.config.messages_per_second as u32,
            });
        }

        // Check bandwidth limit
        if self.byte_count + message_size > self.config.bytes_per_second {
            return Err(RateLimitError::BandwidthExceeded {
                current: self.byte_count,
                additional: message_size,
                limit: self.config.bytes_per_second,
            });
        }

        // Record the message
        self.message_count += 1;
        self.byte_count += message_size;

        Ok(())
    }

    /// Check if a message would be accepted without recording it
    pub fn check_only(&self, message_size: usize) -> Result<(), RateLimitError> {
        let mut clone = self.clone();
        clone.refresh_window_if_needed();
        clone.check_and_record(message_size)
    }

    /// Refresh the rate limiting window if duration has elapsed
    fn refresh_window_if_needed(&mut self) {
        let elapsed = self.window_start.elapsed();

        if elapsed >= self.config.window_duration {
            // Reset counters for new window
            self.message_count = 0;
            self.byte_count = 0;
            self.window_start = Instant::now();
        }
    }

    /// Get current message rate (messages per second in current window)
    pub fn current_message_rate(&self) -> f64 {
        let elapsed = self.window_start.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            self.message_count as f64 / elapsed
        } else {
            0.0
        }
    }

    /// Get current bandwidth usage (bytes per second in current window)
    pub fn current_bandwidth(&self) -> f64 {
        let elapsed = self.window_start.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            self.byte_count as f64 / elapsed
        } else {
            0.0
        }
    }

    /// Get statistics for monitoring
    pub fn stats(&self) -> RateLimiterStats {
        RateLimiterStats {
            message_count: self.message_count,
            byte_count: self.byte_count,
            window_elapsed: self.window_start.elapsed(),
            message_rate: self.current_message_rate(),
            bandwidth: self.current_bandwidth(),
        }
    }

    /// Reset rate limiter (clear all counters)
    pub fn reset(&mut self) {
        self.message_count = 0;
        self.byte_count = 0;
        self.window_start = Instant::now();
    }
}

impl Default for PeerRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for PeerRateLimiter {
    fn clone(&self) -> Self {
        PeerRateLimiter {
            config: self.config.clone(),
            message_count: self.message_count,
            byte_count: self.byte_count,
            window_start: self.window_start,
        }
    }
}

/// Rate limiter statistics
#[derive(Debug, Clone)]
pub struct RateLimiterStats {
    /// Messages in current window
    pub message_count: u32,
    /// Bytes in current window
    pub byte_count: usize,
    /// Time elapsed in current window
    pub window_elapsed: Duration,
    /// Current message rate (msg/sec)
    pub message_rate: f64,
    /// Current bandwidth (bytes/sec)
    pub bandwidth: f64,
}

/// Rate limiting errors
#[derive(Error, Debug, Clone, PartialEq)]
pub enum RateLimitError {
    #[error("Message rate exceeded: {current} messages in window (limit: {limit}/sec)")]
    MessageRateExceeded { current: u32, limit: u32 },

    #[error("Bandwidth exceeded: {current} + {additional} bytes > {limit} bytes/sec")]
    BandwidthExceeded {
        current: usize,
        additional: usize,
        limit: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_rate_limiter_creation() {
        let limiter = PeerRateLimiter::new();
        assert_eq!(limiter.message_count, 0);
        assert_eq!(limiter.byte_count, 0);
    }

    #[test]
    fn test_message_rate_limit() {
        let config = RateLimiterConfig {
            messages_per_second: 10.0,
            bytes_per_second: 1_000_000,
            window_duration: Duration::from_secs(1),
        };

        let mut limiter = PeerRateLimiter::with_config(config);

        // Should accept first 10 messages
        for i in 0..10 {
            assert!(
                limiter.check_and_record(100).is_ok(),
                "Message {} should be accepted",
                i
            );
        }

        // 11th message should be rejected
        assert!(matches!(
            limiter.check_and_record(100),
            Err(RateLimitError::MessageRateExceeded { .. })
        ));
    }

    #[test]
    fn test_bandwidth_limit() {
        let config = RateLimiterConfig {
            messages_per_second: 1000.0,
            bytes_per_second: 1000, // Only 1KB/sec
            window_duration: Duration::from_secs(1),
        };

        let mut limiter = PeerRateLimiter::with_config(config);

        // Should accept messages totaling <1000 bytes
        assert!(limiter.check_and_record(500).is_ok());
        assert!(limiter.check_and_record(400).is_ok());

        // This would exceed 1000 bytes total
        assert!(matches!(
            limiter.check_and_record(200),
            Err(RateLimitError::BandwidthExceeded { .. })
        ));
    }

    #[test]
    fn test_window_refresh() {
        let config = RateLimiterConfig {
            messages_per_second: 5.0,
            bytes_per_second: 1_000_000,
            window_duration: Duration::from_millis(100), // 100ms window
        };

        let mut limiter = PeerRateLimiter::with_config(config);

        // Fill up the limit
        for _ in 0..5 {
            assert!(limiter.check_and_record(100).is_ok());
        }

        // Should be at limit
        assert!(limiter.check_and_record(100).is_err());

        // Wait for window to refresh
        thread::sleep(Duration::from_millis(150));

        // Should accept messages again after window refresh
        assert!(limiter.check_and_record(100).is_ok());
    }

    #[test]
    fn test_stats() {
        let mut limiter = PeerRateLimiter::new();

        limiter.check_and_record(1000).unwrap();
        limiter.check_and_record(2000).unwrap();

        let stats = limiter.stats();
        assert_eq!(stats.message_count, 2);
        assert_eq!(stats.byte_count, 3000);
    }

    #[test]
    fn test_reset() {
        let mut limiter = PeerRateLimiter::new();

        limiter.check_and_record(1000).unwrap();
        limiter.check_and_record(1000).unwrap();

        assert_eq!(limiter.message_count, 2);
        assert_eq!(limiter.byte_count, 2000);

        limiter.reset();

        assert_eq!(limiter.message_count, 0);
        assert_eq!(limiter.byte_count, 0);
    }

    #[test]
    fn test_check_only_doesnt_record() {
        let mut limiter = PeerRateLimiter::new();

        // check_only should not affect counters
        assert!(limiter.check_only(1000).is_ok());
        assert_eq!(limiter.message_count, 0);
        assert_eq!(limiter.byte_count, 0);

        // check_and_record should affect counters
        assert!(limiter.check_and_record(1000).is_ok());
        assert_eq!(limiter.message_count, 1);
        assert_eq!(limiter.byte_count, 1000);
    }
}