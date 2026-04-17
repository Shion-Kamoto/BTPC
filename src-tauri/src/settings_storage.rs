//! Persistent Settings Storage using RocksDB
//!
//! This module provides persistent storage for application settings using RocksDB.
//! Settings are stored as key-value pairs in the CF_METADATA column family.
//!
//! # Features
//! - Persistent storage across app restarts
//! - Type-safe settings with JSON serialization
//! - Support for network configuration, preferences, and user settings
//!
//! # Usage
//! ```rust,ignore
//! let settings = SettingsStorage::open("~/.btpc/settings")?;
//! settings.save_setting("network", "regtest")?;
//! let network = settings.load_setting("network")?;
//! ```

use anyhow::{anyhow, Result};
use rocksdb::{Options, DB};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

/// Column family name for settings storage
#[allow(dead_code)]
const CF_SETTINGS: &str = "settings";

/// Settings storage backed by RocksDB
#[allow(dead_code)] // Reserved for persistent settings
pub struct SettingsStorage {
    db: Arc<DB>,
}

#[allow(dead_code)]
impl SettingsStorage {
    /// Open or create a new settings storage database
    ///
    /// # Arguments
    /// * `path` - Path to the RocksDB database directory
    ///
    /// # Returns
    /// * `Ok(SettingsStorage)` - Successfully opened database
    /// * `Err(anyhow::Error)` - Failed to open database
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        // Define column families
        let cfs = vec![CF_SETTINGS];

        let db = DB::open_cf(&opts, path, &cfs)
            .map_err(|e| anyhow!("Failed to open settings RocksDB: {}", e))?;

        Ok(Self { db: Arc::new(db) })
    }

    /// Save a setting value
    ///
    /// # Arguments
    /// * `key` - Setting key (e.g., "network", "rpc_port")
    /// * `value` - Setting value (JSON-serializable)
    ///
    /// # Returns
    /// * `Ok(())` - Successfully saved setting
    /// * `Err(anyhow::Error)` - Failed to save setting
    pub fn save_setting(&self, key: &str, value: &str) -> Result<()> {
        let cf = self
            .db
            .cf_handle(CF_SETTINGS)
            .ok_or_else(|| anyhow!("Settings column family not found"))?;

        self.db
            .put_cf(&cf, key.as_bytes(), value.as_bytes())
            .map_err(|e| anyhow!("Failed to save setting '{}': {}", key, e))?;

        // FIX 2025-12-03: Explicit flush to ensure data is persisted to disk
        // Without this, data might be lost if app is killed abruptly (e.g., dev restart)
        self.db
            .flush()
            .map_err(|e| anyhow!("Failed to flush settings to disk: {}", e))?;

        println!("💾 Saved setting: {} = {} (flushed to disk)", key, value);
        Ok(())
    }

    /// Load a setting value
    ///
    /// # Arguments
    /// * `key` - Setting key to retrieve
    ///
    /// # Returns
    /// * `Ok(Some(String))` - Setting value found
    /// * `Ok(None)` - Setting not found
    /// * `Err(anyhow::Error)` - Failed to load setting
    pub fn load_setting(&self, key: &str) -> Result<Option<String>> {
        let cf = self
            .db
            .cf_handle(CF_SETTINGS)
            .ok_or_else(|| anyhow!("Settings column family not found"))?;

        match self.db.get_cf(&cf, key.as_bytes()) {
            Ok(Some(value)) => {
                let value_str = String::from_utf8(value)
                    .map_err(|e| anyhow!("Failed to decode setting '{}': {}", key, e))?;
                Ok(Some(value_str))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(anyhow!("Failed to load setting '{}': {}", key, e)),
        }
    }

    /// Load all settings as a HashMap
    ///
    /// # Returns
    /// * `Ok(HashMap<String, String>)` - All settings
    /// * `Err(anyhow::Error)` - Failed to load settings
    pub fn load_all_settings(&self) -> Result<HashMap<String, String>> {
        let cf = self
            .db
            .cf_handle(CF_SETTINGS)
            .ok_or_else(|| anyhow!("Settings column family not found"))?;

        let mut settings = HashMap::new();

        let iter = self.db.iterator_cf(&cf, rocksdb::IteratorMode::Start);
        for item in iter {
            match item {
                Ok((key, value)) => {
                    let key_str = String::from_utf8(key.to_vec())
                        .map_err(|e| anyhow!("Failed to decode setting key: {}", e))?;
                    let value_str = String::from_utf8(value.to_vec())
                        .map_err(|e| anyhow!("Failed to decode setting value: {}", e))?;
                    settings.insert(key_str, value_str);
                }
                Err(e) => {
                    eprintln!("⚠️ Failed to read setting entry: {}", e);
                    continue;
                }
            }
        }

        println!("📖 Loaded {} settings from database", settings.len());
        Ok(settings)
    }

    /// Delete a setting
    ///
    /// # Arguments
    /// * `key` - Setting key to delete
    ///
    /// # Returns
    /// * `Ok(())` - Successfully deleted setting
    /// * `Err(anyhow::Error)` - Failed to delete setting
    pub fn delete_setting(&self, key: &str) -> Result<()> {
        let cf = self
            .db
            .cf_handle(CF_SETTINGS)
            .ok_or_else(|| anyhow!("Settings column family not found"))?;

        self.db
            .delete_cf(&cf, key.as_bytes())
            .map_err(|e| anyhow!("Failed to delete setting '{}': {}", key, e))?;

        println!("🗑️  Deleted setting: {}", key);
        Ok(())
    }

    /// Check if a setting exists
    ///
    /// # Arguments
    /// * `key` - Setting key to check
    ///
    /// # Returns
    /// * `Ok(true)` - Setting exists
    /// * `Ok(false)` - Setting does not exist
    pub fn has_setting(&self, key: &str) -> Result<bool> {
        let cf = self
            .db
            .cf_handle(CF_SETTINGS)
            .ok_or_else(|| anyhow!("Settings column family not found"))?;

        match self.db.get_cf(&cf, key.as_bytes()) {
            Ok(Some(_)) => Ok(true),
            Ok(None) => Ok(false),
            Err(e) => Err(anyhow!("Failed to check setting '{}': {}", key, e)),
        }
    }

    /// Get count of stored settings
    ///
    /// # Returns
    /// * `Ok(usize)` - Number of settings
    pub fn count_settings(&self) -> Result<usize> {
        let settings = self.load_all_settings()?;
        Ok(settings.len())
    }
}

/// Type-safe wrapper for common settings
#[allow(dead_code)] // Reserved for structured settings API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// Active network (mainnet, testnet, regtest)
    pub network: Option<String>,
    /// RPC server port
    pub rpc_port: Option<u16>,
    /// P2P listen port
    pub p2p_port: Option<u16>,
    /// Theme preference (light, dark, auto)
    pub theme: Option<String>,
    /// Auto-start blockchain sync
    pub auto_sync: Option<bool>,
    /// Mining threads count
    pub mining_threads: Option<u32>,
}

#[allow(dead_code)]
impl AppSettings {
    /// Load settings from storage
    pub fn load(storage: &SettingsStorage) -> Result<Self> {
        Ok(Self {
            network: storage.load_setting("network")?,
            rpc_port: storage
                .load_setting("rpc_port")?
                .and_then(|s| s.parse().ok()),
            p2p_port: storage
                .load_setting("p2p_port")?
                .and_then(|s| s.parse().ok()),
            theme: storage.load_setting("theme")?,
            auto_sync: storage
                .load_setting("auto_sync")?
                .and_then(|s| s.parse().ok()),
            mining_threads: storage
                .load_setting("mining_threads")?
                .and_then(|s| s.parse().ok()),
        })
    }

    /// Save settings to storage
    pub fn save(&self, storage: &SettingsStorage) -> Result<()> {
        if let Some(ref network) = self.network {
            storage.save_setting("network", network)?;
        }
        if let Some(rpc_port) = self.rpc_port {
            storage.save_setting("rpc_port", &rpc_port.to_string())?;
        }
        if let Some(p2p_port) = self.p2p_port {
            storage.save_setting("p2p_port", &p2p_port.to_string())?;
        }
        if let Some(ref theme) = self.theme {
            storage.save_setting("theme", theme)?;
        }
        if let Some(auto_sync) = self.auto_sync {
            storage.save_setting("auto_sync", &auto_sync.to_string())?;
        }
        if let Some(mining_threads) = self.mining_threads {
            storage.save_setting("mining_threads", &mining_threads.to_string())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_save_and_load_setting() {
        let temp_dir = TempDir::new().unwrap();
        let storage = SettingsStorage::open(temp_dir.path()).unwrap();

        storage.save_setting("test_key", "test_value").unwrap();
        let loaded = storage.load_setting("test_key").unwrap();

        assert_eq!(loaded, Some("test_value".to_string()));
    }

    #[test]
    fn test_load_nonexistent_setting() {
        let temp_dir = TempDir::new().unwrap();
        let storage = SettingsStorage::open(temp_dir.path()).unwrap();

        let loaded = storage.load_setting("nonexistent").unwrap();
        assert_eq!(loaded, None);
    }

    #[test]
    fn test_load_all_settings() {
        let temp_dir = TempDir::new().unwrap();
        let storage = SettingsStorage::open(temp_dir.path()).unwrap();

        storage.save_setting("key1", "value1").unwrap();
        storage.save_setting("key2", "value2").unwrap();
        storage.save_setting("key3", "value3").unwrap();

        let all_settings = storage.load_all_settings().unwrap();
        assert_eq!(all_settings.len(), 3);
        assert_eq!(all_settings.get("key1"), Some(&"value1".to_string()));
        assert_eq!(all_settings.get("key2"), Some(&"value2".to_string()));
        assert_eq!(all_settings.get("key3"), Some(&"value3".to_string()));
    }

    #[test]
    fn test_delete_setting() {
        let temp_dir = TempDir::new().unwrap();
        let storage = SettingsStorage::open(temp_dir.path()).unwrap();

        storage.save_setting("test_key", "test_value").unwrap();
        assert_eq!(
            storage.load_setting("test_key").unwrap(),
            Some("test_value".to_string())
        );

        storage.delete_setting("test_key").unwrap();
        assert_eq!(storage.load_setting("test_key").unwrap(), None);
    }

    #[test]
    fn test_has_setting() {
        let temp_dir = TempDir::new().unwrap();
        let storage = SettingsStorage::open(temp_dir.path()).unwrap();

        assert!(!storage.has_setting("test_key").unwrap());

        storage.save_setting("test_key", "test_value").unwrap();
        assert!(storage.has_setting("test_key").unwrap());
    }

    #[test]
    fn test_app_settings_round_trip() {
        let temp_dir = TempDir::new().unwrap();
        let storage = SettingsStorage::open(temp_dir.path()).unwrap();

        let settings = AppSettings {
            network: Some("regtest".to_string()),
            rpc_port: Some(18360),
            p2p_port: Some(18361),
            theme: Some("dark".to_string()),
            auto_sync: Some(true),
            mining_threads: Some(4),
        };

        settings.save(&storage).unwrap();

        let loaded = AppSettings::load(&storage).unwrap();
        assert_eq!(loaded.network, Some("regtest".to_string()));
        assert_eq!(loaded.rpc_port, Some(18360));
        assert_eq!(loaded.p2p_port, Some(18361));
        assert_eq!(loaded.theme, Some("dark".to_string()));
        assert_eq!(loaded.auto_sync, Some(true));
        assert_eq!(loaded.mining_threads, Some(4));
    }
}
