

//! Peer Banning and Misbehavior Tracking System (Issue #5)
//!
//! Tracks peer misbehavior, assigns scores, and implements automatic banning
//! to prevent persistent attacks from malicious peers.

use std::{
    collections::HashMap,
    net::IpAddr,
    time::{Duration, SystemTime},
};

use serde::{Deserialize, Serialize};

/// Misbehavior point values
pub const POINTS_INVALID_BLOCK: u32 = 100; // Instant ban
pub const POINTS_INVALID_TRANSACTION: u32 = 10;
pub const POINTS_PROTOCOL_VIOLATION: u32 = 50;
pub const POINTS_RATE_LIMIT_EXCEEDED: u32 = 1;
pub const POINTS_CONNECTION_ABUSE: u32 = 20;

/// Ban threshold - accumulate 100 points for automatic ban
pub const BAN_THRESHOLD: u32 = 100;

/// Default ban duration (24 hours)
pub const DEFAULT_BAN_DURATION: Duration = Duration::from_secs(86400);

/// Maximum ban duration (30 days)
pub const MAX_BAN_DURATION: Duration = Duration::from_secs(86400 * 30);

/// Misbehavior score decay period (1 hour)
pub const MISBEHAVIOR_DECAY_PERIOD: Duration = Duration::from_secs(3600);

/// Peer ban manager
pub struct PeerBanManager {
    /// Banned IPs with ban information
    banned_ips: HashMap<IpAddr, BanInfo>,
    /// Misbehavior scores for peers (not yet banned)
    misbehavior_scores: HashMap<IpAddr, MisbehaviorInfo>,
    /// Ban count history (preserved after unban for escalation)
    ban_history: HashMap<IpAddr, u32>,
    /// Configuration
    config: BanConfig,
}

/// Ban information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BanInfo {
    /// Reason for ban
    pub reason: BanReason,
    /// When the peer was banned
    pub banned_at: SystemTime,
    /// Duration of the ban
    pub ban_duration: Duration,
    /// Number of times this IP has been banned (for escalation)
    pub ban_count: u32,
    /// Original misbehavior score that triggered the ban
    pub trigger_score: u32,
}

/// Misbehavior tracking information
#[derive(Debug, Clone)]
struct MisbehaviorInfo {
    /// Current misbehavior score
    score: u32,
    /// Last time misbehavior was recorded
    last_updated: SystemTime,
    /// History of recent misbehaviors
    recent_offenses: Vec<(SystemTime, BanReason, u32)>,
}

/// Reasons for banning a peer
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BanReason {
    /// Sent an invalid block
    InvalidBlock,
    /// Sent an invalid transaction
    InvalidTransaction,
    /// Protocol violation (malformed messages, etc.)
    ProtocolViolation,
    /// Exceeded rate limits repeatedly
    RateLimitExceeded,
    /// Connection abuse (slowloris, etc.)
    ConnectionAbuse,
    /// Manual ban by node operator
    ManualBan,
    /// Multiple accumulated offenses
    AccumulatedMisbehavior,
}

impl std::fmt::Display for BanReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BanReason::InvalidBlock => write!(f, "Invalid block"),
            BanReason::InvalidTransaction => write!(f, "Invalid transaction"),
            BanReason::ProtocolViolation => write!(f, "Protocol violation"),
            BanReason::RateLimitExceeded => write!(f, "Rate limit exceeded"),
            BanReason::ConnectionAbuse => write!(f, "Connection abuse"),
            BanReason::ManualBan => write!(f, "Manual ban"),
            BanReason::AccumulatedMisbehavior => write!(f, "Accumulated misbehavior"),
        }
    }
}

/// Ban manager configuration
#[derive(Debug, Clone)]
pub struct BanConfig {
    /// Ban threshold (points required for automatic ban)
    pub ban_threshold: u32,
    /// Default ban duration
    pub default_ban_duration: Duration,
    /// Enable ban duration escalation for repeat offenders
    pub enable_escalation: bool,
    /// Misbehavior score decay period
    pub decay_period: Duration,
    /// Enable automatic unbanning after ban duration expires
    pub auto_unban: bool,
}

impl Default for BanConfig {
    fn default() -> Self {
        BanConfig {
            ban_threshold: BAN_THRESHOLD,
            default_ban_duration: DEFAULT_BAN_DURATION,
            enable_escalation: true,
            decay_period: MISBEHAVIOR_DECAY_PERIOD,
            auto_unban: true,
        }
    }
}

impl PeerBanManager {
    /// Create a new peer ban manager
    pub fn new(config: BanConfig) -> Self {
        PeerBanManager {
            banned_ips: HashMap::new(),
            misbehavior_scores: HashMap::new(),
            ban_history: HashMap::new(),
            config,
        }
    }

    /// Create with default configuration
    pub fn default() -> Self {
        Self::new(BanConfig::default())
    }

    /// Check if an IP is currently banned
    pub fn is_banned(&self, ip: &IpAddr) -> bool {
        if let Some(ban_info) = self.banned_ips.get(ip) {
            // Check if ban has expired
            if self.config.auto_unban {
                let elapsed = SystemTime::now()
                    .duration_since(ban_info.banned_at)
                    .unwrap_or(Duration::ZERO);

                if elapsed >= ban_info.ban_duration {
                    // Ban expired, should be removed
                    return false;
                }
            }
            true
        } else {
            false
        }
    }

    /// Get ban information for an IP
    pub fn get_ban_info(&self, ip: &IpAddr) -> Option<&BanInfo> {
        self.banned_ips.get(ip)
    }

    /// Add misbehavior points to a peer
    ///
    /// Returns `Some(BanReason)` if the peer should be banned
    pub fn add_misbehavior(&mut self, ip: IpAddr, points: u32, reason: BanReason) -> Option<BanReason> {
        // Don't add misbehavior to already banned IPs
        if self.is_banned(&ip) {
            return None;
        }

        let now = SystemTime::now();

        // Get or create misbehavior info
        let info = self.misbehavior_scores.entry(ip).or_insert(MisbehaviorInfo {
            score: 0,
            last_updated: now,
            recent_offenses: Vec::new(),
        });

        // Apply decay if needed
        if let Ok(elapsed) = now.duration_since(info.last_updated) {
            if elapsed >= self.config.decay_period {
                let decay_factor = (elapsed.as_secs() / self.config.decay_period.as_secs()) as u32;
                info.score = info.score.saturating_sub(decay_factor * 10); // Decay 10 points per period
            }
        }

        // Add new misbehavior points
        info.score += points;
        info.last_updated = now;
        info.recent_offenses.push((now, reason, points));

        // Keep only recent offenses (last 100)
        if info.recent_offenses.len() > 100 {
            info.recent_offenses.drain(0..50); // Remove oldest 50
        }

        // Check if ban threshold exceeded
        if info.score >= self.config.ban_threshold {
            // Clone recent offenses to avoid borrow checker issues
            let recent_offenses = info.recent_offenses.clone();
            let ban_reason = self.determine_ban_reason(&recent_offenses);
            self.ban_peer(ip, ban_reason, None);
            Some(ban_reason)
        } else {
            None
        }
    }

    /// Manually ban a peer
    pub fn ban_peer(&mut self, ip: IpAddr, reason: BanReason, duration: Option<Duration>) {
        let now = SystemTime::now();

        // Get current ban count from history (persists across unbans)
        let ban_count = self.ban_history
            .get(&ip)
            .map(|count| count + 1)
            .unwrap_or(1);

        // Calculate ban duration (with escalation if enabled)
        let ban_duration = duration.unwrap_or_else(|| {
            if self.config.enable_escalation && ban_count > 1 {
                // Escalate ban duration: 24h, 48h, 96h, 192h, ... up to 30 days
                let escalation_factor = 2_u32.pow((ban_count - 1).min(5));
                let escalated = self.config.default_ban_duration * escalation_factor;
                escalated.min(MAX_BAN_DURATION)
            } else {
                self.config.default_ban_duration
            }
        });

        // Get misbehavior score
        let trigger_score = self.misbehavior_scores
            .get(&ip)
            .map(|info| info.score)
            .unwrap_or(self.config.ban_threshold);

        let ban_info = BanInfo {
            reason,
            banned_at: now,
            ban_duration,
            ban_count,
            trigger_score,
        };

        self.banned_ips.insert(ip, ban_info);

        // Update ban history (preserve count for future bans)
        self.ban_history.insert(ip, ban_count);

        // Remove from misbehavior scores
        self.misbehavior_scores.remove(&ip);
    }

    /// Manually unban a peer
    pub fn unban_peer(&mut self, ip: &IpAddr) -> bool {
        self.banned_ips.remove(ip).is_some()
    }

    /// Clean up expired bans
    pub fn cleanup_expired_bans(&mut self) -> usize {
        if !self.config.auto_unban {
            return 0;
        }

        let now = SystemTime::now();
        let mut removed = 0;

        self.banned_ips.retain(|_, ban_info| {
            let elapsed = now.duration_since(ban_info.banned_at)
                .unwrap_or(Duration::ZERO);

            if elapsed >= ban_info.ban_duration {
                removed += 1;
                false // Remove expired ban
            } else {
                true // Keep active ban
            }
        });

        removed
    }

    /// Get current misbehavior score for an IP
    pub fn get_misbehavior_score(&self, ip: &IpAddr) -> u32 {
        self.misbehavior_scores
            .get(ip)
            .map(|info| info.score)
            .unwrap_or(0)
    }

    /// Get statistics
    pub fn stats(&self) -> BanStats {
        BanStats {
            banned_count: self.banned_ips.len(),
            misbehaving_count: self.misbehavior_scores.len(),
            total_tracked: self.banned_ips.len() + self.misbehavior_scores.len(),
        }
    }

    /// List all banned IPs
    pub fn list_bans(&self) -> Vec<(IpAddr, &BanInfo)> {
        self.banned_ips.iter().map(|(ip, info)| (*ip, info)).collect()
    }

    /// Clear all bans and misbehavior scores (for testing)
    pub fn clear_all(&mut self) {
        self.banned_ips.clear();
        self.misbehavior_scores.clear();
        self.ban_history.clear();
    }

    /// Determine ban reason from recent offenses
    fn determine_ban_reason(&self, offenses: &[(SystemTime, BanReason, u32)]) -> BanReason {
        if offenses.is_empty() {
            return BanReason::AccumulatedMisbehavior;
        }

        // Find the most severe recent offense
        let mut max_points = 0;
        let mut reason = BanReason::AccumulatedMisbehavior;

        for (_, offense_reason, points) in offenses.iter().rev().take(10) {
            if *points > max_points {
                max_points = *points;
                reason = *offense_reason;
            }
        }

        // If multiple small offenses, use accumulated misbehavior
        if max_points < POINTS_PROTOCOL_VIOLATION {
            BanReason::AccumulatedMisbehavior
        } else {
            reason
        }
    }
}

/// Ban statistics
#[derive(Debug, Clone)]
pub struct BanStats {
    /// Number of currently banned IPs
    pub banned_count: usize,
    /// Number of IPs with misbehavior scores (not yet banned)
    pub misbehaving_count: usize,
    /// Total IPs being tracked
    pub total_tracked: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ban_manager_creation() {
        let manager = PeerBanManager::default();
        let stats = manager.stats();

        assert_eq!(stats.banned_count, 0);
        assert_eq!(stats.misbehaving_count, 0);
    }

    #[test]
    fn test_instant_ban_invalid_block() {
        let mut manager = PeerBanManager::default();
        let ip: IpAddr = "10.0.0.1".parse().unwrap();

        // Invalid block should instant ban (100 points)
        let result = manager.add_misbehavior(ip, POINTS_INVALID_BLOCK, BanReason::InvalidBlock);

        assert!(result.is_some());
        assert_eq!(result.unwrap(), BanReason::InvalidBlock);
        assert!(manager.is_banned(&ip));
    }

    #[test]
    fn test_accumulated_misbehavior() {
        let mut manager = PeerBanManager::default();
        let ip: IpAddr = "10.0.0.2".parse().unwrap();

        // Add multiple small offenses
        for _ in 0..5 {
            let result = manager.add_misbehavior(ip, POINTS_INVALID_TRANSACTION, BanReason::InvalidTransaction);
            assert!(result.is_none()); // Not banned yet
        }

        assert!(!manager.is_banned(&ip));
        assert_eq!(manager.get_misbehavior_score(&ip), 50);

        // Add more to reach threshold
        for _ in 0..5 {
            manager.add_misbehavior(ip, POINTS_INVALID_TRANSACTION, BanReason::InvalidTransaction);
        }

        assert!(manager.is_banned(&ip));
        assert_eq!(manager.get_misbehavior_score(&ip), 0); // Cleared after ban
    }

    #[test]
    fn test_ban_escalation() {
        let mut manager = PeerBanManager::default();
        let ip: IpAddr = "10.0.0.3".parse().unwrap();

        // First ban
        manager.ban_peer(ip, BanReason::ProtocolViolation, None);
        let ban_info = manager.get_ban_info(&ip).unwrap();
        assert_eq!(ban_info.ban_count, 1);
        assert_eq!(ban_info.ban_duration, DEFAULT_BAN_DURATION);

        // Unban
        manager.unban_peer(&ip);

        // Second ban (should be escalated)
        manager.ban_peer(ip, BanReason::ProtocolViolation, None);
        let ban_info = manager.get_ban_info(&ip).unwrap();
        assert_eq!(ban_info.ban_count, 2);
        assert_eq!(ban_info.ban_duration, DEFAULT_BAN_DURATION * 2); // Doubled
    }

    #[test]
    fn test_manual_ban() {
        let mut manager = PeerBanManager::default();
        let ip: IpAddr = "10.0.0.4".parse().unwrap();

        manager.ban_peer(ip, BanReason::ManualBan, Some(Duration::from_secs(3600)));

        assert!(manager.is_banned(&ip));
        let ban_info = manager.get_ban_info(&ip).unwrap();
        assert_eq!(ban_info.reason, BanReason::ManualBan);
        assert_eq!(ban_info.ban_duration, Duration::from_secs(3600));
    }

    #[test]
    fn test_unban() {
        let mut manager = PeerBanManager::default();
        let ip: IpAddr = "10.0.0.5".parse().unwrap();

        manager.ban_peer(ip, BanReason::RateLimitExceeded, None);
        assert!(manager.is_banned(&ip));

        let unbanned = manager.unban_peer(&ip);
        assert!(unbanned);
        assert!(!manager.is_banned(&ip));
    }

    #[test]
    fn test_ban_expiry() {
        let config = BanConfig {
            ban_threshold: BAN_THRESHOLD,
            default_ban_duration: Duration::from_secs(1), // 1 second for testing
            enable_escalation: false,
            decay_period: MISBEHAVIOR_DECAY_PERIOD,
            auto_unban: true,
        };

        let mut manager = PeerBanManager::new(config);
        let ip: IpAddr = "10.0.0.6".parse().unwrap();

        manager.ban_peer(ip, BanReason::ProtocolViolation, Some(Duration::from_secs(1)));
        assert!(manager.is_banned(&ip));

        // Wait for ban to expire
        std::thread::sleep(Duration::from_millis(1100));

        // Should no longer be banned
        assert!(!manager.is_banned(&ip));

        // Cleanup should remove it
        let removed = manager.cleanup_expired_bans();
        assert_eq!(removed, 1);
    }

    #[test]
    fn test_stats() {
        let mut manager = PeerBanManager::default();
        let ip1: IpAddr = "10.0.0.7".parse().unwrap();
        let ip2: IpAddr = "10.0.0.8".parse().unwrap();
        let ip3: IpAddr = "10.0.0.9".parse().unwrap();

        // Ban one
        manager.ban_peer(ip1, BanReason::InvalidBlock, None);

        // Add misbehavior to two others
        manager.add_misbehavior(ip2, POINTS_INVALID_TRANSACTION, BanReason::InvalidTransaction);
        manager.add_misbehavior(ip3, POINTS_RATE_LIMIT_EXCEEDED, BanReason::RateLimitExceeded);

        let stats = manager.stats();
        assert_eq!(stats.banned_count, 1);
        assert_eq!(stats.misbehaving_count, 2);
        assert_eq!(stats.total_tracked, 3);
    }

    #[test]
    fn test_list_bans() {
        let mut manager = PeerBanManager::default();
        let ip1: IpAddr = "10.0.0.10".parse().unwrap();
        let ip2: IpAddr = "10.0.0.11".parse().unwrap();

        manager.ban_peer(ip1, BanReason::InvalidBlock, None);
        manager.ban_peer(ip2, BanReason::ProtocolViolation, None);

        let bans = manager.list_bans();
        assert_eq!(bans.len(), 2);
    }
}