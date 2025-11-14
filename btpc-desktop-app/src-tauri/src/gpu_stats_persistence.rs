//! GPU Statistics Persistence Module (Feature 012: T022)
//!
//! Provides atomic save/load of per-GPU lifetime statistics to JSON file.
//! Location: ~/.btpc/data/mining_stats_per_gpu.json
//!
//! Design:
//! - Atomic writes via temp file + rename
//! - Graceful degradation if file missing or corrupt
//! - Constitution Article V compliance (structured logging)

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Persistent GPU statistics (stored in JSON)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistentGpuStats {
    pub device_index: u32,
    pub lifetime_blocks_found: u64,
    pub total_hashes: u64,
    pub total_uptime: u64,  // seconds
    pub first_seen: String,  // ISO 8601 timestamp
    pub last_updated: String,  // ISO 8601 timestamp
}

/// GPU stats file structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuStatsFile {
    pub version: String,
    pub last_updated: Option<String>,
    pub gpus: HashMap<u32, PersistentGpuStats>,
}

impl Default for GpuStatsFile {
    fn default() -> Self {
        GpuStatsFile {
            version: "1.0.0".to_string(),
            last_updated: None,
            gpus: HashMap::new(),
        }
    }
}

/// GPU stats persistence manager
pub struct GpuStatsPersistence {
    file_path: PathBuf,
}

impl GpuStatsPersistence {
    /// Create new persistence manager
    ///
    /// # Arguments
    /// * `data_dir` - Data directory path (typically ~/.btpc/data)
    pub fn new(data_dir: PathBuf) -> Self {
        let file_path = data_dir.join("mining_stats_per_gpu.json");
        GpuStatsPersistence { file_path }
    }

    /// Load GPU stats from disk
    ///
    /// Returns empty GpuStatsFile if file doesn't exist or is corrupt.
    /// Graceful degradation - never returns error.
    pub fn load(&self) -> GpuStatsFile {
        match self.load_internal() {
            Ok(stats) => stats,
            Err(e) => {
                eprintln!("Warning: Failed to load GPU stats ({}), using empty state", e);
                GpuStatsFile::default()
            }
        }
    }

    /// Internal load with error propagation
    fn load_internal(&self) -> Result<GpuStatsFile> {
        if !self.file_path.exists() {
            return Ok(GpuStatsFile::default());
        }

        let contents = fs::read_to_string(&self.file_path)
            .context("Failed to read GPU stats file")?;

        let stats: GpuStatsFile = serde_json::from_str(&contents)
            .context("Failed to parse GPU stats JSON")?;

        Ok(stats)
    }

    /// Save GPU stats to disk (atomic write)
    ///
    /// Uses temp file + rename for atomicity.
    /// Constitution Article V: Atomic persistence with structured logging.
    ///
    /// # Arguments
    /// * `stats` - GPU stats to save
    ///
    /// # Errors
    /// Returns error if write or rename fails
    pub fn save(&self, stats: &GpuStatsFile) -> Result<()> {
        // Create parent directory if needed
        if let Some(parent) = self.file_path.parent() {
            fs::create_dir_all(parent)
                .context("Failed to create GPU stats directory")?;
        }

        // Write to temp file
        let temp_path = self.file_path.with_extension("tmp");
        let json = serde_json::to_string_pretty(stats)
            .context("Failed to serialize GPU stats")?;

        fs::write(&temp_path, json)
            .context("Failed to write GPU stats temp file")?;

        // Atomic rename
        fs::rename(&temp_path, &self.file_path)
            .context("Failed to rename GPU stats temp file")?;

        Ok(())
    }

    /// Update stats for a specific GPU
    ///
    /// Loads existing stats, updates the specified GPU, and saves atomically.
    ///
    /// # Arguments
    /// * `device_index` - GPU device index
    /// * `blocks_found` - Lifetime blocks found (cumulative)
    /// * `total_hashes` - Lifetime total hashes (cumulative)
    /// * `total_uptime` - Lifetime uptime in seconds (cumulative)
    pub fn update_gpu_stats(
        &self,
        device_index: u32,
        blocks_found: u64,
        total_hashes: u64,
        total_uptime: u64,
    ) -> Result<()> {
        let mut stats_file = self.load();

        let now = chrono::Utc::now().to_rfc3339();

        // Get or create stats entry for this GPU
        let entry = stats_file.gpus.entry(device_index).or_insert_with(|| {
            PersistentGpuStats {
                device_index,
                lifetime_blocks_found: 0,
                total_hashes: 0,
                total_uptime: 0,
                first_seen: now.clone(),
                last_updated: now.clone(),
            }
        });

        // Update stats
        entry.lifetime_blocks_found = blocks_found;
        entry.total_hashes = total_hashes;
        entry.total_uptime = total_uptime;
        entry.last_updated = now.clone();

        // Update file metadata
        stats_file.last_updated = Some(now);

        // Save atomically
        self.save(&stats_file)?;

        Ok(())
    }

    /// Get stats for a specific GPU
    pub fn get_gpu_stats(&self, device_index: u32) -> Option<PersistentGpuStats> {
        let stats_file = self.load();
        stats_file.gpus.get(&device_index).cloned()
    }

    /// Get all GPU stats
    pub fn get_all_stats(&self) -> HashMap<u32, PersistentGpuStats> {
        let stats_file = self.load();
        stats_file.gpus
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_persistence_new() {
        let temp_dir = TempDir::new().unwrap();
        let persistence = GpuStatsPersistence::new(temp_dir.path().to_path_buf());
        assert!(persistence.file_path.ends_with("mining_stats_per_gpu.json"));
    }

    #[test]
    fn test_load_empty_file() {
        let temp_dir = TempDir::new().unwrap();
        let persistence = GpuStatsPersistence::new(temp_dir.path().to_path_buf());

        let stats = persistence.load();
        assert_eq!(stats.version, "1.0.0");
        assert!(stats.gpus.is_empty());
    }

    #[test]
    fn test_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let persistence = GpuStatsPersistence::new(temp_dir.path().to_path_buf());

        // Update stats for GPU 0
        persistence.update_gpu_stats(0, 42, 1000000, 3600).unwrap();

        // Load and verify
        let loaded = persistence.get_gpu_stats(0).unwrap();
        assert_eq!(loaded.device_index, 0);
        assert_eq!(loaded.lifetime_blocks_found, 42);
        assert_eq!(loaded.total_hashes, 1000000);
        assert_eq!(loaded.total_uptime, 3600);
    }

    #[test]
    fn test_atomic_write() {
        let temp_dir = TempDir::new().unwrap();
        let persistence = GpuStatsPersistence::new(temp_dir.path().to_path_buf());

        // Write stats
        persistence.update_gpu_stats(0, 10, 50000, 600).unwrap();

        // Verify temp file was cleaned up
        let temp_path = persistence.file_path.with_extension("tmp");
        assert!(!temp_path.exists(), "Temp file should be cleaned up after atomic rename");

        // Verify main file exists
        assert!(persistence.file_path.exists(), "Main file should exist after save");
    }

    #[test]
    fn test_update_existing_gpu() {
        let temp_dir = TempDir::new().unwrap();
        let persistence = GpuStatsPersistence::new(temp_dir.path().to_path_buf());

        // First update
        persistence.update_gpu_stats(0, 10, 50000, 600).unwrap();

        // Second update (should overwrite)
        persistence.update_gpu_stats(0, 20, 100000, 1200).unwrap();

        // Verify latest values
        let loaded = persistence.get_gpu_stats(0).unwrap();
        assert_eq!(loaded.lifetime_blocks_found, 20);
        assert_eq!(loaded.total_hashes, 100000);
        assert_eq!(loaded.total_uptime, 1200);
    }

    #[test]
    fn test_multiple_gpus() {
        let temp_dir = TempDir::new().unwrap();
        let persistence = GpuStatsPersistence::new(temp_dir.path().to_path_buf());

        // Update multiple GPUs
        persistence.update_gpu_stats(0, 10, 50000, 600).unwrap();
        persistence.update_gpu_stats(1, 15, 75000, 900).unwrap();
        persistence.update_gpu_stats(2, 5, 25000, 300).unwrap();

        // Verify all stats
        let all_stats = persistence.get_all_stats();
        assert_eq!(all_stats.len(), 3);
        assert_eq!(all_stats.get(&0).unwrap().lifetime_blocks_found, 10);
        assert_eq!(all_stats.get(&1).unwrap().lifetime_blocks_found, 15);
        assert_eq!(all_stats.get(&2).unwrap().lifetime_blocks_found, 5);
    }
}
