//! Rate limiting for peer connections (Issue #1 - HIGH)
//!
//! Implements token bucket rate limiting to prevent DoS attacks via message flooding.
//! Each peer has separate rate limits for:
//! - Message count per second
//! - Bandwidth (bytes) per second

use std::collections::HashMap;
use std::time::{Duration, Instant};
use thiserror::Error;

use crate::network::protocol::MessageType;

/// Maximum accepted P2P frame size (32 MiB). Frames larger than this
/// are treated as instant-ban by [`PeerRateLimiter::record_message_type`].
pub const MAX_FRAME_BYTES: usize = 32 * 1024 * 1024;

/// Per-message-type quota (003-testnet-p2p-hardening, FR-008).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MessageTypeQuota {
    /// Allowed messages of this type per rolling minute.
    pub per_minute: u32,
    /// Ban score added each time a peer sends a message of this type
    /// after the quota is exhausted (unless the overage rule says
    /// otherwise).
    pub ban_score_per_overage: u32,
}

impl MessageTypeQuota {
    /// Return the default quota for `mt`. Values come from
    /// `specs/003-testnet-p2p-hardening/contracts/p2p-wire.md §6`.
    pub fn default_for(mt: MessageType) -> Self {
        match mt {
            MessageType::Addr => MessageTypeQuota {
                per_minute: 5,
                ban_score_per_overage: 20,
            },
            MessageType::Inv => MessageTypeQuota {
                per_minute: 100,
                // `inv` uses +1 per excess message (not a flat overage).
                ban_score_per_overage: 1,
            },
            MessageType::GetData => MessageTypeQuota {
                per_minute: 100,
                ban_score_per_overage: 1,
            },
            MessageType::GetHeaders => MessageTypeQuota {
                per_minute: 30,
                ban_score_per_overage: 5,
            },
            MessageType::Headers => MessageTypeQuota {
                per_minute: 200,
                ban_score_per_overage: 1,
            },
            MessageType::Block => MessageTypeQuota {
                per_minute: 200,
                ban_score_per_overage: 1,
            },
            MessageType::Tx => MessageTypeQuota {
                per_minute: 2_000,
                ban_score_per_overage: 1,
            },
            MessageType::Ping | MessageType::Pong => MessageTypeQuota {
                per_minute: 60,
                ban_score_per_overage: 5,
            },
            MessageType::GetAddr => MessageTypeQuota {
                per_minute: 2,
                ban_score_per_overage: 20,
            },
            MessageType::Version | MessageType::VerAck => MessageTypeQuota {
                per_minute: 2,
                ban_score_per_overage: 50,
            },
            MessageType::Unknown => MessageTypeQuota {
                per_minute: 0,
                ban_score_per_overage: 10,
            },
        }
    }
}

/// Outcome returned by the per-message-type rate limiter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitOutcome {
    /// Within limits.
    Ok,
    /// Quota exceeded — apply `ban_score_delta` to the peer.
    Exceeded {
        message_type: MessageType,
        ban_score_delta: u32,
    },
    /// Violation severe enough to demand an immediate ban.
    InstantBan { reason: &'static str },
}

/// Per-message-type sliding counter.
#[derive(Debug, Clone)]
struct TypeCounter {
    window_start: Instant,
    count: u32,
}

impl TypeCounter {
    fn new() -> Self {
        TypeCounter {
            window_start: Instant::now(),
            count: 0,
        }
    }

    fn refresh_if_expired(&mut self, window: Duration) {
        if self.window_start.elapsed() >= window {
            self.window_start = Instant::now();
            self.count = 0;
        }
    }
}

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
            messages_per_second: 100.0,  // 100 messages/sec
            bytes_per_second: 5_000_000, // 5 MB/sec
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
    /// Per-message-type sliding counters (003-testnet-p2p-hardening).
    type_counters: HashMap<MessageType, TypeCounter>,
    /// Per-type-quota window (60s by default).
    type_window: Duration,
}

impl PeerRateLimiter {
    /// Create new rate limiter with the given configuration.
    ///
    /// Signature changed in 003-testnet-p2p-hardening — callers that
    /// want the previous default-config constructor can use
    /// [`PeerRateLimiter::default`].
    pub fn new(config: RateLimiterConfig) -> Self {
        Self::with_config(config)
    }

    /// Create new rate limiter with custom configuration
    pub fn with_config(config: RateLimiterConfig) -> Self {
        PeerRateLimiter {
            config,
            message_count: 0,
            byte_count: 0,
            window_start: Instant::now(),
            type_counters: HashMap::new(),
            type_window: Duration::from_secs(60),
        }
    }

    /// Record a typed message against the per-message-type quota.
    /// Returns a [`RateLimitOutcome`] — callers are responsible for
    /// applying the ban score and disconnecting the peer if required.
    pub fn record_message_type(&mut self, mt: MessageType, size: usize) -> RateLimitOutcome {
        if size > MAX_FRAME_BYTES {
            return RateLimitOutcome::InstantBan {
                reason: "oversized frame > 32 MB",
            };
        }

        let window = self.type_window;
        let counter = self
            .type_counters
            .entry(mt)
            .or_insert_with(TypeCounter::new);
        counter.refresh_if_expired(window);

        let quota = MessageTypeQuota::default_for(mt);
        counter.count = counter.count.saturating_add(1);

        if counter.count <= quota.per_minute {
            RateLimitOutcome::Ok
        } else {
            // `inv`/`headers`/`getdata`/etc. use +1 per excess message;
            // `addr`/`getaddr` use a flat overage penalty.
            let excess = counter.count - quota.per_minute;
            let ban_score_delta = match mt {
                MessageType::Inv
                | MessageType::GetData
                | MessageType::Headers
                | MessageType::Block
                | MessageType::Tx => excess,
                _ => quota.ban_score_per_overage,
            };
            RateLimitOutcome::Exceeded {
                message_type: mt,
                ban_score_delta,
            }
        }
    }

    /// Record an unknown / unparseable command frame. Always returns
    /// an `Exceeded` outcome — unknown commands are never allowed.
    pub fn record_unknown_command(&mut self, _command: &str, _size: usize) -> RateLimitOutcome {
        let quota = MessageTypeQuota::default_for(MessageType::Unknown);
        RateLimitOutcome::Exceeded {
            message_type: MessageType::Unknown,
            ban_score_delta: quota.ban_score_per_overage,
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
        Self::with_config(RateLimiterConfig::default())
    }
}

impl Clone for PeerRateLimiter {
    fn clone(&self) -> Self {
        PeerRateLimiter {
            config: self.config.clone(),
            message_count: self.message_count,
            byte_count: self.byte_count,
            window_start: self.window_start,
            type_counters: self.type_counters.clone(),
            type_window: self.type_window,
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
        let limiter = PeerRateLimiter::with_config(RateLimiterConfig::default());
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
        let mut limiter = PeerRateLimiter::with_config(RateLimiterConfig::default());

        limiter.check_and_record(1000).unwrap();
        limiter.check_and_record(2000).unwrap();

        let stats = limiter.stats();
        assert_eq!(stats.message_count, 2);
        assert_eq!(stats.byte_count, 3000);
    }

    #[test]
    fn test_reset() {
        let mut limiter = PeerRateLimiter::with_config(RateLimiterConfig::default());

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
        let mut limiter = PeerRateLimiter::with_config(RateLimiterConfig::default());

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

// ============================================================================
// 003-testnet-p2p-hardening — RED-phase tests for T095 (Phase 2.5)
// ============================================================================
//
// Covers FR-008 per-message-type rate limiting. Today `PeerRateLimiter` only
// tracks aggregate count + byte windows (lines 33-38 of this file); GREEN
// impl in T026/T027 adds a `HashMap<MessageType, Counter>` dimension.
// Referenced symbols (`MessageTypeQuota`, `record_message_type`, etc.)
// do not yet exist.

#[cfg(test)]
mod red_phase_per_type_tests {
    use super::*;
    use crate::network::protocol::MessageType;
    use crate::network::rate_limiter::{MessageTypeQuota, RateLimitOutcome};

    fn mk_limiter() -> PeerRateLimiter {
        PeerRateLimiter::new(RateLimiterConfig::default())
    }

    #[test]
    fn addr_quota_is_five_per_minute_then_plus_twenty_ban_score() {
        let mut limiter = mk_limiter();
        // First 5 `addr` messages in a minute are free.
        for _ in 0..5 {
            let outcome = limiter.record_message_type(MessageType::Addr, 128);
            assert_eq!(outcome, RateLimitOutcome::Ok);
        }
        // 6th one exceeds the per-type quota.
        let outcome = limiter.record_message_type(MessageType::Addr, 128);
        assert_eq!(
            outcome,
            RateLimitOutcome::Exceeded {
                message_type: MessageType::Addr,
                ban_score_delta: 20,
            }
        );
    }

    #[test]
    fn inv_quota_is_one_hundred_per_minute_plus_one_per_excess() {
        let mut limiter = mk_limiter();
        for _ in 0..100 {
            assert_eq!(
                limiter.record_message_type(MessageType::Inv, 64),
                RateLimitOutcome::Ok
            );
        }
        // Next 3 exceed by 1, 2, 3 respectively.
        for expected_delta in 1..=3 {
            let outcome = limiter.record_message_type(MessageType::Inv, 64);
            assert_eq!(
                outcome,
                RateLimitOutcome::Exceeded {
                    message_type: MessageType::Inv,
                    ban_score_delta: expected_delta,
                }
            );
        }
    }

    #[test]
    fn unknown_command_adds_ten_ban_score_per_hit() {
        let mut limiter = mk_limiter();
        let outcome = limiter.record_unknown_command("garbage", 64);
        assert_eq!(
            outcome,
            RateLimitOutcome::Exceeded {
                message_type: MessageType::Unknown,
                ban_score_delta: 10,
            }
        );
    }

    #[test]
    fn oversized_frame_over_thirty_two_mb_is_instant_ban() {
        let mut limiter = mk_limiter();
        let outcome = limiter.record_message_type(MessageType::Block, 33 * 1024 * 1024);
        assert_eq!(
            outcome,
            RateLimitOutcome::InstantBan {
                reason: "oversized frame > 32 MB",
            },
            "frames > 32 MB must be instant-ban (ddos protection)"
        );
    }

    #[test]
    fn quota_lookup_returns_defaults_for_untuned_types() {
        // Looking up a message-type quota that wasn't explicitly configured
        // should return a safe default, not panic.
        let q = MessageTypeQuota::default_for(MessageType::Tx);
        assert!(q.per_minute > 0, "defaults must be populated");
    }
}
