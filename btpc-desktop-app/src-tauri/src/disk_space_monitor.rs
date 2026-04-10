//! Disk Space Monitoring Module (FR-058)
//!
//! Monitors available disk space and enforces thresholds to prevent
//! data loss during blockchain sync and mining operations.
//!
//! Thresholds:
//! - Warning: < 10GB available → emit warning notification
//! - Pause Sync: < 5GB available → pause blockchain sync
//! - Prevent Mining: < 2GB available → block mining start

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use sysinfo::Disks;
use tokio::sync::RwLock;

/// Disk space thresholds in bytes (FR-058)
pub const THRESHOLD_WARNING_BYTES: u64 = 10 * 1024 * 1024 * 1024; // 10 GB
pub const THRESHOLD_PAUSE_SYNC_BYTES: u64 = 5 * 1024 * 1024 * 1024; // 5 GB
pub const THRESHOLD_PREVENT_MINING_BYTES: u64 = 2 * 1024 * 1024 * 1024; // 2 GB

/// Default monitoring interval in seconds (FR-058f)
pub const DEFAULT_MONITOR_INTERVAL_SECS: u64 = 60;

/// Estimated full blockchain size (for display purposes)
pub const ESTIMATED_FULL_BLOCKCHAIN_GB: u64 = 100;

/// Disk space information for a partition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskSpaceInfo {
    /// Mount point or partition name
    pub partition: String,
    /// Total capacity in bytes
    pub total_bytes: u64,
    /// Available space in bytes
    pub available_bytes: u64,
    /// Used space in bytes
    pub used_bytes: u64,
    /// Usage percentage (0.0 - 100.0)
    pub usage_percent: f64,
    /// File system type (ext4, ntfs, apfs, etc.)
    pub filesystem: String,
}

impl DiskSpaceInfo {
    /// Get available space in gigabytes
    pub fn available_gb(&self) -> f64 {
        self.available_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
    }

    /// Get total space in gigabytes
    pub fn total_gb(&self) -> f64 {
        self.total_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
    }

    /// Check if below warning threshold (< 10GB)
    pub fn is_warning(&self) -> bool {
        self.available_bytes < THRESHOLD_WARNING_BYTES
    }

    /// Check if should pause sync (< 5GB)
    pub fn should_pause_sync(&self) -> bool {
        self.available_bytes < THRESHOLD_PAUSE_SYNC_BYTES
    }

    /// Check if should prevent mining (< 2GB)
    pub fn should_prevent_mining(&self) -> bool {
        self.available_bytes < THRESHOLD_PREVENT_MINING_BYTES
    }
}

/// Disk space alert levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiskSpaceAlertLevel {
    /// Space is sufficient (> 10GB)
    Normal,
    /// Warning: space below 10GB
    Warning,
    /// Critical: space below 5GB, sync paused
    SyncPaused,
    /// Severe: space below 2GB, mining prevented
    MiningPrevented,
}

impl DiskSpaceAlertLevel {
    /// Get alert level from available bytes
    pub fn from_available_bytes(available: u64) -> Self {
        if available < THRESHOLD_PREVENT_MINING_BYTES {
            DiskSpaceAlertLevel::MiningPrevented
        } else if available < THRESHOLD_PAUSE_SYNC_BYTES {
            DiskSpaceAlertLevel::SyncPaused
        } else if available < THRESHOLD_WARNING_BYTES {
            DiskSpaceAlertLevel::Warning
        } else {
            DiskSpaceAlertLevel::Normal
        }
    }

    /// Get human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            DiskSpaceAlertLevel::Normal => "Disk space is sufficient",
            DiskSpaceAlertLevel::Warning => "Low disk space warning (< 10GB)",
            DiskSpaceAlertLevel::SyncPaused => "Very low disk space - sync paused (< 5GB)",
            DiskSpaceAlertLevel::MiningPrevented => "Critical disk space - mining disabled (< 2GB)",
        }
    }
}

/// Disk space alert event for frontend notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskSpaceAlert {
    /// Alert severity level
    pub level: DiskSpaceAlertLevel,
    /// Affected partition path
    pub partition: String,
    /// Available space in bytes
    pub available_bytes: u64,
    /// Available space formatted (e.g., "4.2 GB")
    pub available_formatted: String,
    /// Human-readable message
    pub message: String,
    /// Timestamp of alert
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Storage breakdown by database component (FR-058e)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageBreakdown {
    /// Total database size in bytes
    pub total_bytes: u64,
    /// Blocks column family size
    pub blocks_bytes: u64,
    /// Transactions column family size
    pub transactions_bytes: u64,
    /// UTXOs column family size
    pub utxos_bytes: u64,
    /// Wallets column family size
    pub wallets_bytes: u64,
    /// Metadata and other files
    pub other_bytes: u64,
}

impl StorageBreakdown {
    /// Format as human-readable string
    pub fn format_summary(&self) -> String {
        format!(
            "Total: {:.2} GB (Blocks: {:.2} GB, Transactions: {:.2} GB, UTXOs: {:.2} GB, Wallets: {:.2} MB)",
            self.total_bytes as f64 / (1024.0 * 1024.0 * 1024.0),
            self.blocks_bytes as f64 / (1024.0 * 1024.0 * 1024.0),
            self.transactions_bytes as f64 / (1024.0 * 1024.0 * 1024.0),
            self.utxos_bytes as f64 / (1024.0 * 1024.0 * 1024.0),
            self.wallets_bytes as f64 / (1024.0 * 1024.0),
        )
    }
}

/// Disk space monitor state
pub struct DiskSpaceMonitor {
    /// Path being monitored (usually ~/.btpc or data directory)
    monitored_path: String,
    /// Last known disk space info
    last_info: RwLock<Option<DiskSpaceInfo>>,
    /// Last check timestamp
    last_check: AtomicU64,
    /// Current alert level
    current_alert_level: RwLock<DiskSpaceAlertLevel>,
    /// Whether monitoring is active (reserved for background monitoring)
    #[allow(dead_code)]
    is_monitoring: AtomicBool,
    /// Whether sync is paused due to low space
    sync_paused: AtomicBool,
    /// Whether mining is prevented due to low space
    mining_prevented: AtomicBool,
}

impl DiskSpaceMonitor {
    /// Create a new disk space monitor for the given path
    pub fn new(monitored_path: impl Into<String>) -> Self {
        Self {
            monitored_path: monitored_path.into(),
            last_info: RwLock::new(None),
            last_check: AtomicU64::new(0),
            current_alert_level: RwLock::new(DiskSpaceAlertLevel::Normal),
            is_monitoring: AtomicBool::new(false),
            sync_paused: AtomicBool::new(false),
            mining_prevented: AtomicBool::new(false),
        }
    }

    /// Get disk space information for a specific path
    pub fn get_disk_info(path: &Path) -> Result<DiskSpaceInfo, String> {
        let disks = Disks::new_with_refreshed_list();

        // Find the disk that contains the path
        let mut best_match: Option<&sysinfo::Disk> = None;
        let mut best_match_len = 0;

        for disk in disks.list() {
            let mount_point = disk.mount_point();
            if path.starts_with(mount_point) {
                let mount_len = mount_point.to_string_lossy().len();
                if mount_len > best_match_len {
                    best_match = Some(disk);
                    best_match_len = mount_len;
                }
            }
        }

        match best_match {
            Some(disk) => {
                let total = disk.total_space();
                let available = disk.available_space();
                let used = total.saturating_sub(available);
                let usage_percent = if total > 0 {
                    (used as f64 / total as f64) * 100.0
                } else {
                    0.0
                };

                Ok(DiskSpaceInfo {
                    partition: disk.mount_point().to_string_lossy().to_string(),
                    total_bytes: total,
                    available_bytes: available,
                    used_bytes: used,
                    usage_percent,
                    filesystem: disk.file_system().to_string_lossy().to_string(),
                })
            }
            None => Err(format!(
                "Could not find disk information for path: {}",
                path.display()
            )),
        }
    }

    /// Get all disk partitions on the system
    pub fn get_all_partitions() -> Vec<DiskSpaceInfo> {
        let disks = Disks::new_with_refreshed_list();

        disks
            .list()
            .iter()
            .map(|disk| {
                let total = disk.total_space();
                let available = disk.available_space();
                let used = total.saturating_sub(available);
                let usage_percent = if total > 0 {
                    (used as f64 / total as f64) * 100.0
                } else {
                    0.0
                };

                DiskSpaceInfo {
                    partition: disk.mount_point().to_string_lossy().to_string(),
                    total_bytes: total,
                    available_bytes: available,
                    used_bytes: used,
                    usage_percent,
                    filesystem: disk.file_system().to_string_lossy().to_string(),
                }
            })
            .collect()
    }

    /// Check disk space and return current info
    pub async fn check(&self) -> Result<DiskSpaceInfo, String> {
        let path = Path::new(&self.monitored_path);
        let info = Self::get_disk_info(path)?;

        // Update cached info
        {
            let mut last_info = self.last_info.write().await;
            *last_info = Some(info.clone());
        }

        // Update last check timestamp
        self.last_check.store(
            chrono::Utc::now().timestamp() as u64,
            Ordering::SeqCst,
        );

        // Update alert level and flags
        let new_level = DiskSpaceAlertLevel::from_available_bytes(info.available_bytes);
        {
            let mut level = self.current_alert_level.write().await;
            *level = new_level;
        }

        self.sync_paused
            .store(info.should_pause_sync(), Ordering::SeqCst);
        self.mining_prevented
            .store(info.should_prevent_mining(), Ordering::SeqCst);

        Ok(info)
    }

    /// Check thresholds and return any alerts that should be emitted
    pub async fn check_thresholds(&self) -> Vec<DiskSpaceAlert> {
        let mut alerts = Vec::new();

        let info = match self.check().await {
            Ok(info) => info,
            Err(_) => return alerts,
        };

        let level = DiskSpaceAlertLevel::from_available_bytes(info.available_bytes);

        if level != DiskSpaceAlertLevel::Normal {
            alerts.push(DiskSpaceAlert {
                level,
                partition: info.partition.clone(),
                available_bytes: info.available_bytes,
                available_formatted: format!("{:.2} GB", info.available_gb()),
                message: level.description().to_string(),
                timestamp: chrono::Utc::now(),
            });
        }

        alerts
    }

    /// Check if sync should be paused due to low disk space
    pub fn is_sync_paused(&self) -> bool {
        self.sync_paused.load(Ordering::SeqCst)
    }

    /// Check if mining should be prevented due to low disk space
    pub fn is_mining_prevented(&self) -> bool {
        self.mining_prevented.load(Ordering::SeqCst)
    }

    /// Get current alert level
    pub async fn get_alert_level(&self) -> DiskSpaceAlertLevel {
        *self.current_alert_level.read().await
    }

    /// Get last cached disk info (may be stale)
    pub async fn get_last_info(&self) -> Option<DiskSpaceInfo> {
        self.last_info.read().await.clone()
    }

    /// Get estimated space required for full blockchain sync
    pub fn estimated_sync_space_gb() -> u64 {
        ESTIMATED_FULL_BLOCKCHAIN_GB
    }

    /// Format bytes as human-readable string
    pub fn format_bytes(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;
        const TB: u64 = GB * 1024;

        if bytes >= TB {
            format!("{:.2} TB", bytes as f64 / TB as f64)
        } else if bytes >= GB {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.2} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} bytes", bytes)
        }
    }
}

/// Error type for disk space operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiskSpaceError {
    /// Insufficient disk space for operation
    InsufficientSpace {
        required_bytes: u64,
        available_bytes: u64,
        operation: String,
    },
    /// Could not determine disk space
    QueryFailed { path: String, error: String },
    /// Sync paused due to low space
    SyncPaused { available_gb: f64 },
    /// Mining prevented due to low space
    MiningPrevented { available_gb: f64 },
}

impl std::fmt::Display for DiskSpaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiskSpaceError::InsufficientSpace {
                required_bytes,
                available_bytes,
                operation,
            } => {
                write!(
                    f,
                    "Insufficient disk space for {}: need {}, have {}",
                    operation,
                    DiskSpaceMonitor::format_bytes(*required_bytes),
                    DiskSpaceMonitor::format_bytes(*available_bytes)
                )
            }
            DiskSpaceError::QueryFailed { path, error } => {
                write!(f, "Failed to query disk space for {}: {}", path, error)
            }
            DiskSpaceError::SyncPaused { available_gb } => {
                write!(
                    f,
                    "Blockchain sync paused: only {:.2} GB available (need 5+ GB)",
                    available_gb
                )
            }
            DiskSpaceError::MiningPrevented { available_gb } => {
                write!(
                    f,
                    "Mining disabled: only {:.2} GB available (need 2+ GB)",
                    available_gb
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_level_from_bytes() {
        // Normal (> 10GB)
        assert_eq!(
            DiskSpaceAlertLevel::from_available_bytes(15 * 1024 * 1024 * 1024),
            DiskSpaceAlertLevel::Normal
        );

        // Warning (5-10GB)
        assert_eq!(
            DiskSpaceAlertLevel::from_available_bytes(7 * 1024 * 1024 * 1024),
            DiskSpaceAlertLevel::Warning
        );

        // Sync paused (2-5GB)
        assert_eq!(
            DiskSpaceAlertLevel::from_available_bytes(3 * 1024 * 1024 * 1024),
            DiskSpaceAlertLevel::SyncPaused
        );

        // Mining prevented (< 2GB)
        assert_eq!(
            DiskSpaceAlertLevel::from_available_bytes(1024 * 1024 * 1024),
            DiskSpaceAlertLevel::MiningPrevented
        );
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(DiskSpaceMonitor::format_bytes(500), "500 bytes");
        assert_eq!(DiskSpaceMonitor::format_bytes(1536), "1.50 KB");
        assert_eq!(DiskSpaceMonitor::format_bytes(1_500_000), "1.43 MB");
        assert_eq!(DiskSpaceMonitor::format_bytes(5_000_000_000), "4.66 GB");
        assert_eq!(DiskSpaceMonitor::format_bytes(2_000_000_000_000), "1.82 TB");
    }

    #[test]
    fn test_disk_space_info_thresholds() {
        let info = DiskSpaceInfo {
            partition: "/".to_string(),
            total_bytes: 100 * 1024 * 1024 * 1024,
            available_bytes: 3 * 1024 * 1024 * 1024, // 3GB
            used_bytes: 97 * 1024 * 1024 * 1024,
            usage_percent: 97.0,
            filesystem: "ext4".to_string(),
        };

        assert!(info.is_warning()); // < 10GB
        assert!(info.should_pause_sync()); // < 5GB
        assert!(!info.should_prevent_mining()); // > 2GB
    }

    #[tokio::test]
    async fn test_get_all_partitions() {
        let partitions = DiskSpaceMonitor::get_all_partitions();
        // Should return at least one partition on any system
        assert!(!partitions.is_empty());

        for partition in &partitions {
            assert!(partition.total_bytes > 0);
            assert!(partition.usage_percent >= 0.0);
            assert!(partition.usage_percent <= 100.0);
        }
    }
}