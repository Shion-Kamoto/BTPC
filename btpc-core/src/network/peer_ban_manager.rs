//! Peer Banning and Misbehavior Tracking System (Issue #5)
//!
//! Tracks peer misbehavior, assigns scores, and implements automatic banning
//! to prevent persistent attacks from malicious peers.
//!
//! 003-testnet-p2p-hardening (T025, FR-010): ban records can now be
//! persisted into a dedicated RocksDB column family via [`BanStorage`]
//! so that a process restart cannot wash away an active ban.

use std::{
    collections::HashMap,
    net::IpAddr,
    path::Path,
    sync::{Arc, RwLock},
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
    /// Interior state (RwLock for interior mutability — 003-testnet-p2p-hardening).
    inner: RwLock<BanManagerInner>,
    /// Optional RocksDB-backed persistence layer.
    storage: Option<BanStorage>,
    /// Configuration
    config: BanConfig,
}

struct BanManagerInner {
    /// Banned IPs with ban information
    banned_ips: HashMap<IpAddr, BanInfo>,
    /// Misbehavior scores for peers (not yet banned)
    misbehavior_scores: HashMap<IpAddr, MisbehaviorInfo>,
    /// Ban count history (preserved after unban for escalation)
    ban_history: HashMap<IpAddr, u32>,
}

impl BanManagerInner {
    fn new() -> Self {
        BanManagerInner {
            banned_ips: HashMap::new(),
            misbehavior_scores: HashMap::new(),
            ban_history: HashMap::new(),
        }
    }
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

impl BanInfo {
    /// Returns true if this ban was recorded with the given reason.
    /// Small helper used by storage round-trip tests (T094 RED).
    pub fn reason_is(&self, reason: BanReason) -> bool {
        self.reason == reason
    }
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
    /// Spam (repeated unwanted messages; 003-testnet-p2p-hardening).
    Spam,
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
            BanReason::Spam => write!(f, "Spam"),
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

// ============================================================================
// BanStorage — RocksDB persistence layer (T025, FR-010)
// ============================================================================

/// RocksDB-backed ban store.
///
/// Opens a standalone RocksDB instance at a caller-supplied path and
/// maps `IpAddr → BanInfo` using bincode serialisation. The handle is
/// cheaply cloneable (internally `Arc<rocksdb::DB>`), so multiple
/// `PeerBanManager` instances can share one underlying store — this is
/// what the round-trip reload test depends on.
#[derive(Clone)]
pub struct BanStorage {
    db: Arc<rocksdb::DB>,
}

/// Errors produced by [`BanStorage`].
#[derive(Debug, thiserror::Error)]
pub enum BanStorageError {
    #[error("rocksdb error: {0}")]
    RocksDb(String),
    #[error("codec error: {0}")]
    Codec(String),
}

impl From<rocksdb::Error> for BanStorageError {
    fn from(e: rocksdb::Error) -> Self {
        BanStorageError::RocksDb(e.to_string())
    }
}

impl BanStorage {
    /// Open (or create) a ban store at `path`.
    pub fn open_at(path: impl AsRef<Path>) -> Result<Self, BanStorageError> {
        let mut opts = rocksdb::Options::default();
        opts.create_if_missing(true);
        let db = rocksdb::DB::open(&opts, path.as_ref())?;
        Ok(BanStorage { db: Arc::new(db) })
    }

    /// Directly fetch a `BanInfo` for `ip`, bypassing the manager cache.
    ///
    /// Used by the persistence round-trip tests to assert that
    /// `ban_peer` writes through synchronously (no shutdown required).
    pub fn raw_get(&self, ip: &IpAddr) -> Result<Option<BanInfo>, BanStorageError> {
        match self.db.get(ip_key(ip))? {
            Some(bytes) => bincode::deserialize(&bytes)
                .map(Some)
                .map_err(|e| BanStorageError::Codec(e.to_string())),
            None => Ok(None),
        }
    }

    fn put(&self, ip: &IpAddr, info: &BanInfo) -> Result<(), BanStorageError> {
        let bytes = bincode::serialize(info).map_err(|e| BanStorageError::Codec(e.to_string()))?;
        self.db.put(ip_key(ip), bytes)?;
        Ok(())
    }

    fn delete(&self, ip: &IpAddr) -> Result<(), BanStorageError> {
        self.db.delete(ip_key(ip))?;
        Ok(())
    }

    fn iter_all(&self) -> Vec<(IpAddr, BanInfo)> {
        let mut out = Vec::new();
        for (k, v) in self.db.iterator(rocksdb::IteratorMode::Start).flatten() {
            if let (Some(ip), Ok(info)) = (parse_ip_key(&k), bincode::deserialize::<BanInfo>(&v)) {
                out.push((ip, info));
            }
        }
        out
    }
}

fn ip_key(ip: &IpAddr) -> Vec<u8> {
    ip.to_string().into_bytes()
}

fn parse_ip_key(bytes: &[u8]) -> Option<IpAddr> {
    std::str::from_utf8(bytes).ok()?.parse().ok()
}

// ============================================================================
// PeerBanManager
// ============================================================================

impl PeerBanManager {
    /// Create a new peer ban manager with the given configuration.
    pub fn new(config: BanConfig) -> Self {
        PeerBanManager {
            inner: RwLock::new(BanManagerInner::new()),
            storage: None,
            config,
        }
    }

    /// Create with default configuration.
    pub fn default() -> Self {
        Self::new(BanConfig::default())
    }

    /// Create a manager backed by a RocksDB [`BanStorage`]. On construction
    /// the manager rehydrates its in-memory cache from disk, pruning any
    /// entries whose ban has already expired.
    pub fn with_storage(storage: BanStorage) -> Self {
        let mgr = PeerBanManager {
            inner: RwLock::new(BanManagerInner::new()),
            storage: Some(storage.clone()),
            config: BanConfig::default(),
        };
        let now = SystemTime::now();
        let mut guard = mgr.inner.write().expect("ban inner lock");
        for (ip, info) in storage.iter_all() {
            let elapsed = now.duration_since(info.banned_at).unwrap_or(Duration::ZERO);
            if elapsed >= info.ban_duration {
                // Expired — purge from disk lazily.
                let _ = storage.delete(&ip);
                continue;
            }
            guard.ban_history.insert(ip, info.ban_count);
            guard.banned_ips.insert(ip, info);
        }
        drop(guard);
        mgr
    }

    /// Check if an IP is currently banned.
    pub fn is_banned(&self, ip: &IpAddr) -> bool {
        let guard = self.inner.read().expect("ban inner lock");
        if let Some(ban_info) = guard.banned_ips.get(ip) {
            if self.config.auto_unban {
                let elapsed = SystemTime::now()
                    .duration_since(ban_info.banned_at)
                    .unwrap_or(Duration::ZERO);
                if elapsed >= ban_info.ban_duration {
                    return false;
                }
            }
            true
        } else {
            false
        }
    }

    /// Get a cloned snapshot of the ban info for an IP.
    pub fn get_ban_info(&self, ip: &IpAddr) -> Option<BanInfo> {
        self.inner
            .read()
            .expect("ban inner lock")
            .banned_ips
            .get(ip)
            .cloned()
    }

    /// Add misbehavior points to a peer.
    ///
    /// Returns `Some(BanReason)` if the peer crossed the ban threshold.
    pub fn add_misbehavior(&self, ip: IpAddr, points: u32, reason: BanReason) -> Option<BanReason> {
        if self.is_banned(&ip) {
            return None;
        }

        let mut guard = self.inner.write().expect("ban inner lock");
        let now = SystemTime::now();
        let info = guard
            .misbehavior_scores
            .entry(ip)
            .or_insert(MisbehaviorInfo {
                score: 0,
                last_updated: now,
                recent_offenses: Vec::new(),
            });

        if let Ok(elapsed) = now.duration_since(info.last_updated) {
            if elapsed >= self.config.decay_period {
                let decay_factor = (elapsed.as_secs() / self.config.decay_period.as_secs()) as u32;
                info.score = info.score.saturating_sub(decay_factor * 10);
            }
        }

        info.score += points;
        info.last_updated = now;
        info.recent_offenses.push((now, reason, points));

        if info.recent_offenses.len() > 100 {
            info.recent_offenses.drain(0..50);
        }

        if info.score >= self.config.ban_threshold {
            let recent_offenses = info.recent_offenses.clone();
            let ban_reason = Self::determine_ban_reason_static(&recent_offenses);
            drop(guard);
            self.ban_peer(ip, ban_reason, None);
            Some(ban_reason)
        } else {
            None
        }
    }

    /// Manually ban a peer.
    pub fn ban_peer(&self, ip: IpAddr, reason: BanReason, duration: Option<Duration>) {
        let now = SystemTime::now();
        let mut guard = self.inner.write().expect("ban inner lock");

        let ban_count = guard
            .ban_history
            .get(&ip)
            .map(|count| count + 1)
            .unwrap_or(1);

        let ban_duration = duration.unwrap_or_else(|| {
            if self.config.enable_escalation && ban_count > 1 {
                let escalation_factor = 2_u32.pow((ban_count - 1).min(5));
                let escalated = self.config.default_ban_duration * escalation_factor;
                escalated.min(MAX_BAN_DURATION)
            } else {
                self.config.default_ban_duration
            }
        });

        let trigger_score = guard
            .misbehavior_scores
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

        guard.banned_ips.insert(ip, ban_info.clone());
        guard.ban_history.insert(ip, ban_count);
        guard.misbehavior_scores.remove(&ip);
        drop(guard);

        if let Some(storage) = &self.storage {
            let _ = storage.put(&ip, &ban_info);
        }
    }

    /// Manually unban a peer.
    pub fn unban_peer(&self, ip: &IpAddr) -> bool {
        let removed = self
            .inner
            .write()
            .expect("ban inner lock")
            .banned_ips
            .remove(ip)
            .is_some();
        if removed {
            if let Some(storage) = &self.storage {
                let _ = storage.delete(ip);
            }
        }
        removed
    }

    /// Clean up expired bans (returns the number removed).
    pub fn cleanup_expired_bans(&self) -> usize {
        if !self.config.auto_unban {
            return 0;
        }
        let now = SystemTime::now();
        let mut removed_ips: Vec<IpAddr> = Vec::new();
        let mut guard = self.inner.write().expect("ban inner lock");
        guard.banned_ips.retain(|ip, ban_info| {
            let elapsed = now
                .duration_since(ban_info.banned_at)
                .unwrap_or(Duration::ZERO);
            if elapsed >= ban_info.ban_duration {
                removed_ips.push(*ip);
                false
            } else {
                true
            }
        });
        drop(guard);

        if let Some(storage) = &self.storage {
            for ip in &removed_ips {
                let _ = storage.delete(ip);
            }
        }
        removed_ips.len()
    }

    /// Get current misbehavior score for an IP.
    pub fn get_misbehavior_score(&self, ip: &IpAddr) -> u32 {
        self.inner
            .read()
            .expect("ban inner lock")
            .misbehavior_scores
            .get(ip)
            .map(|info| info.score)
            .unwrap_or(0)
    }

    /// Get statistics.
    pub fn stats(&self) -> BanStats {
        let guard = self.inner.read().expect("ban inner lock");
        BanStats {
            banned_count: guard.banned_ips.len(),
            misbehaving_count: guard.misbehavior_scores.len(),
            total_tracked: guard.banned_ips.len() + guard.misbehavior_scores.len(),
        }
    }

    /// List all banned IPs (snapshot — cloned).
    pub fn list_bans(&self) -> Vec<(IpAddr, BanInfo)> {
        // Skip expired entries even if cleanup_expired_bans hasn't been
        // called yet; otherwise a reopen-then-list sequence would still
        // surface ghosts of bans that are already past their deadline.
        let now = SystemTime::now();
        self.inner
            .read()
            .expect("ban inner lock")
            .banned_ips
            .iter()
            .filter(|(_, info)| {
                if !self.config.auto_unban {
                    return true;
                }
                let elapsed = now.duration_since(info.banned_at).unwrap_or(Duration::ZERO);
                elapsed < info.ban_duration
            })
            .map(|(ip, info)| (*ip, info.clone()))
            .collect()
    }

    /// Clear all bans and misbehavior scores (for testing).
    pub fn clear_all(&self) {
        let mut guard = self.inner.write().expect("ban inner lock");
        guard.banned_ips.clear();
        guard.misbehavior_scores.clear();
        guard.ban_history.clear();
    }

    fn determine_ban_reason_static(offenses: &[(SystemTime, BanReason, u32)]) -> BanReason {
        if offenses.is_empty() {
            return BanReason::AccumulatedMisbehavior;
        }

        let mut max_points = 0;
        let mut reason = BanReason::AccumulatedMisbehavior;
        for (_, offense_reason, points) in offenses.iter().rev().take(10) {
            if *points > max_points {
                max_points = *points;
                reason = *offense_reason;
            }
        }

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
        let manager = PeerBanManager::default();
        let ip: IpAddr = "10.0.0.1".parse().unwrap();

        let result = manager.add_misbehavior(ip, POINTS_INVALID_BLOCK, BanReason::InvalidBlock);

        assert!(result.is_some());
        assert_eq!(result.unwrap(), BanReason::InvalidBlock);
        assert!(manager.is_banned(&ip));
    }

    #[test]
    fn test_accumulated_misbehavior() {
        let manager = PeerBanManager::default();
        let ip: IpAddr = "10.0.0.2".parse().unwrap();

        for _ in 0..5 {
            let result = manager.add_misbehavior(
                ip,
                POINTS_INVALID_TRANSACTION,
                BanReason::InvalidTransaction,
            );
            assert!(result.is_none());
        }

        assert!(!manager.is_banned(&ip));
        assert_eq!(manager.get_misbehavior_score(&ip), 50);

        for _ in 0..5 {
            manager.add_misbehavior(
                ip,
                POINTS_INVALID_TRANSACTION,
                BanReason::InvalidTransaction,
            );
        }

        assert!(manager.is_banned(&ip));
        assert_eq!(manager.get_misbehavior_score(&ip), 0);
    }

    #[test]
    fn test_ban_escalation() {
        let manager = PeerBanManager::default();
        let ip: IpAddr = "10.0.0.3".parse().unwrap();

        manager.ban_peer(ip, BanReason::ProtocolViolation, None);
        let ban_info = manager.get_ban_info(&ip).unwrap();
        assert_eq!(ban_info.ban_count, 1);
        assert_eq!(ban_info.ban_duration, DEFAULT_BAN_DURATION);

        manager.unban_peer(&ip);

        manager.ban_peer(ip, BanReason::ProtocolViolation, None);
        let ban_info = manager.get_ban_info(&ip).unwrap();
        assert_eq!(ban_info.ban_count, 2);
        assert_eq!(ban_info.ban_duration, DEFAULT_BAN_DURATION * 2);
    }

    #[test]
    fn test_manual_ban() {
        let manager = PeerBanManager::default();
        let ip: IpAddr = "10.0.0.4".parse().unwrap();

        manager.ban_peer(ip, BanReason::ManualBan, Some(Duration::from_secs(3600)));

        assert!(manager.is_banned(&ip));
        let ban_info = manager.get_ban_info(&ip).unwrap();
        assert_eq!(ban_info.reason, BanReason::ManualBan);
        assert_eq!(ban_info.ban_duration, Duration::from_secs(3600));
    }

    #[test]
    fn test_unban() {
        let manager = PeerBanManager::default();
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
            default_ban_duration: Duration::from_secs(1),
            enable_escalation: false,
            decay_period: MISBEHAVIOR_DECAY_PERIOD,
            auto_unban: true,
        };

        let manager = PeerBanManager::new(config);
        let ip: IpAddr = "10.0.0.6".parse().unwrap();

        manager.ban_peer(
            ip,
            BanReason::ProtocolViolation,
            Some(Duration::from_secs(1)),
        );
        assert!(manager.is_banned(&ip));

        std::thread::sleep(Duration::from_millis(1100));

        assert!(!manager.is_banned(&ip));

        let removed = manager.cleanup_expired_bans();
        assert_eq!(removed, 1);
    }

    #[test]
    fn test_stats() {
        let manager = PeerBanManager::default();
        let ip1: IpAddr = "10.0.0.7".parse().unwrap();
        let ip2: IpAddr = "10.0.0.8".parse().unwrap();
        let ip3: IpAddr = "10.0.0.9".parse().unwrap();

        manager.ban_peer(ip1, BanReason::InvalidBlock, None);

        manager.add_misbehavior(
            ip2,
            POINTS_INVALID_TRANSACTION,
            BanReason::InvalidTransaction,
        );
        manager.add_misbehavior(
            ip3,
            POINTS_RATE_LIMIT_EXCEEDED,
            BanReason::RateLimitExceeded,
        );

        let stats = manager.stats();
        assert_eq!(stats.banned_count, 1);
        assert_eq!(stats.misbehaving_count, 2);
        assert_eq!(stats.total_tracked, 3);
    }

    #[test]
    fn test_list_bans() {
        let manager = PeerBanManager::default();
        let ip1: IpAddr = "10.0.0.10".parse().unwrap();
        let ip2: IpAddr = "10.0.0.11".parse().unwrap();

        manager.ban_peer(ip1, BanReason::InvalidBlock, None);
        manager.ban_peer(ip2, BanReason::ProtocolViolation, None);

        let bans = manager.list_bans();
        assert_eq!(bans.len(), 2);
    }
}

// ============================================================================
// 003-testnet-p2p-hardening — RED-phase tests for T094 (Phase 2.5)
// ============================================================================

#[cfg(test)]
mod red_phase_persistence_tests {
    use super::*;
    use crate::network::peer_ban_manager::{BanStorage, PeerBanManager};
    use std::net::{IpAddr, Ipv4Addr};
    use std::time::Duration;

    fn ipv4(a: u8, b: u8, c: u8, d: u8) -> IpAddr {
        IpAddr::V4(Ipv4Addr::new(a, b, c, d))
    }

    #[test]
    fn ban_persists_across_manager_reopen() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let storage = BanStorage::open_at(tmp.path()).expect("open bans cf");

        {
            let mgr = PeerBanManager::with_storage(storage.clone());
            mgr.ban_peer(ipv4(203, 0, 113, 7), BanReason::InvalidBlock, None);
            assert!(mgr.is_banned(&ipv4(203, 0, 113, 7)));
        }

        let mgr = PeerBanManager::with_storage(storage);
        assert!(
            mgr.is_banned(&ipv4(203, 0, 113, 7)),
            "ban must survive manager reopen (persisted to RocksDB CF_PEER_BANS)"
        );
    }

    #[test]
    fn expired_bans_are_pruned_lazily_on_load() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let storage = BanStorage::open_at(tmp.path()).expect("open bans cf");

        {
            let mgr = PeerBanManager::with_storage(storage.clone());
            mgr.ban_peer(
                ipv4(203, 0, 113, 8),
                BanReason::ProtocolViolation,
                Some(Duration::from_secs(1)),
            );
        }
        std::thread::sleep(Duration::from_secs(2));

        let mgr = PeerBanManager::with_storage(storage);
        assert!(
            !mgr.is_banned(&ipv4(203, 0, 113, 8)),
            "expired ban must be pruned on reload"
        );
        assert_eq!(
            mgr.list_bans().len(),
            0,
            "expired entry must not be listed after lazy pruning"
        );
    }

    #[test]
    fn save_happens_on_insert_not_on_shutdown() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let storage = BanStorage::open_at(tmp.path()).expect("open bans cf");

        let mgr = PeerBanManager::with_storage(storage.clone());
        mgr.ban_peer(ipv4(203, 0, 113, 9), BanReason::Spam, None);

        let raw = storage
            .raw_get(&ipv4(203, 0, 113, 9))
            .expect("cf read")
            .expect("entry exists in cf immediately after ban_peer");
        assert!(raw.reason_is(BanReason::Spam));
    }
}
