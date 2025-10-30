//! Storage layer for BTPC blockchain data
//!
//! Provides persistent storage using RocksDB with efficient indexing and retrieval.

use std::{fmt, path::Path, sync::Arc};

use serde::{Deserialize, Serialize};

pub mod blockchain_db;
pub mod database;
pub mod mempool;
pub mod rocksdb_config;
pub mod utxo_db;

pub use blockchain_db::{BlockchainDatabase, BlockchainDb, BlockchainDbError};
pub use database::{Database, DatabaseConfig, DatabaseError};
pub use mempool::{Mempool, MempoolError};
pub use rocksdb_config::BlockchainDB;
pub use utxo_db::{UTXODatabase, UTXODbError, UtxoDb};

use crate::crypto::Hash;

/// Storage subsystem constants
pub mod constants {
    /// Default cache size for RocksDB (128MB)
    pub const DEFAULT_CACHE_SIZE: usize = 128 * 1024 * 1024;

    /// Default write buffer size (64MB)
    pub const DEFAULT_WRITE_BUFFER_SIZE: usize = 64 * 1024 * 1024;

    /// Maximum open files for RocksDB
    pub const DEFAULT_MAX_OPEN_FILES: i32 = 1000;

    /// Target file size for compaction (64MB)
    pub const DEFAULT_TARGET_FILE_SIZE: u64 = 64 * 1024 * 1024;

    /// Bloom filter bits per key
    pub const DEFAULT_BLOOM_FILTER_BITS: i32 = 10;

    /// Block cache size (64MB)
    pub const DEFAULT_BLOCK_CACHE_SIZE: usize = 64 * 1024 * 1024;

    /// Maximum mempool size (50MB)
    pub const DEFAULT_MEMPOOL_SIZE: usize = 50 * 1024 * 1024;

    /// Maximum mempool transactions
    pub const DEFAULT_MAX_MEMPOOL_TXS: usize = 10000;
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Database directory
    pub data_dir: String,
    /// Cache size in bytes
    pub cache_size: usize,
    /// Write buffer size in bytes
    pub write_buffer_size: usize,
    /// Maximum open files
    pub max_open_files: i32,
    /// Enable compression
    pub enable_compression: bool,
    /// Enable statistics
    pub enable_statistics: bool,
    /// Bloom filter configuration
    pub bloom_filter_bits: i32,
    /// Mempool configuration
    pub mempool_config: MempoolConfig,
}

impl StorageConfig {
    /// Create default storage configuration
    pub fn default(data_dir: &str) -> Self {
        StorageConfig {
            data_dir: data_dir.to_string(),
            cache_size: constants::DEFAULT_CACHE_SIZE,
            write_buffer_size: constants::DEFAULT_WRITE_BUFFER_SIZE,
            max_open_files: constants::DEFAULT_MAX_OPEN_FILES,
            enable_compression: true,
            enable_statistics: true,
            bloom_filter_bits: constants::DEFAULT_BLOOM_FILTER_BITS,
            mempool_config: MempoolConfig::default(),
        }
    }

    /// Create configuration for testing (smaller sizes)
    pub fn test(data_dir: &str) -> Self {
        StorageConfig {
            data_dir: data_dir.to_string(),
            cache_size: 16 * 1024 * 1024,       // 16MB
            write_buffer_size: 8 * 1024 * 1024, // 8MB
            max_open_files: 100,
            enable_compression: false, // Faster for tests
            enable_statistics: false,
            bloom_filter_bits: 10,
            mempool_config: MempoolConfig::test(),
        }
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), StorageError> {
        if self.data_dir.is_empty() {
            return Err(StorageError::InvalidConfig(
                "Empty data directory".to_string(),
            ));
        }

        if self.cache_size == 0 {
            return Err(StorageError::InvalidConfig(
                "Cache size cannot be zero".to_string(),
            ));
        }

        if self.write_buffer_size == 0 {
            return Err(StorageError::InvalidConfig(
                "Write buffer size cannot be zero".to_string(),
            ));
        }

        if self.max_open_files <= 0 {
            return Err(StorageError::InvalidConfig(
                "Max open files must be positive".to_string(),
            ));
        }

        self.mempool_config.validate()?;

        Ok(())
    }
}

/// Mempool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MempoolConfig {
    /// Maximum mempool size in bytes
    pub max_size: usize,
    /// Maximum number of transactions
    pub max_transactions: usize,
    /// Minimum transaction fee (satoshis per byte)
    pub min_fee_rate: u64,
    /// Maximum transaction age in seconds
    pub max_age: u64,
    /// Enable fee replacement (RBF)
    pub enable_rbf: bool,
}

impl Default for MempoolConfig {
    fn default() -> Self {
        MempoolConfig {
            max_size: constants::DEFAULT_MEMPOOL_SIZE,
            max_transactions: constants::DEFAULT_MAX_MEMPOOL_TXS,
            min_fee_rate: 1000,    // 1000 satoshis per byte
            max_age: 24 * 60 * 60, // 24 hours
            enable_rbf: true,
        }
    }
}

impl MempoolConfig {

    /// Create test configuration
    pub fn test() -> Self {
        MempoolConfig {
            max_size: 1024 * 1024, // 1MB
            max_transactions: 100,
            min_fee_rate: 100, // Lower for testing
            max_age: 60 * 60,  // 1 hour
            enable_rbf: true,
        }
    }

    /// Validate mempool configuration
    pub fn validate(&self) -> Result<(), StorageError> {
        if self.max_size == 0 {
            return Err(StorageError::InvalidConfig(
                "Mempool max size cannot be zero".to_string(),
            ));
        }

        if self.max_transactions == 0 {
            return Err(StorageError::InvalidConfig(
                "Max transactions cannot be zero".to_string(),
            ));
        }

        if self.max_age == 0 {
            return Err(StorageError::InvalidConfig(
                "Max age cannot be zero".to_string(),
            ));
        }

        Ok(())
    }
}

/// Storage subsystem manager
pub struct StorageManager {
    /// Database instance
    database: Arc<Database>,
    /// Blockchain database
    blockchain_db: Box<dyn BlockchainDatabase>,
    /// UTXO database
    utxo_db: Box<dyn UTXODatabase>,
    /// Transaction mempool
    mempool: Mempool,
    /// Configuration
    config: StorageConfig,
}

impl fmt::Debug for StorageManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StorageManager")
            .field("database", &self.database)
            .field("mempool", &self.mempool)
            .field("config", &self.config)
            .finish()
    }
}

impl StorageManager {
    /// Create a new storage manager
    pub fn new(config: StorageConfig) -> Result<Self, StorageError> {
        config.validate()?;

        // Create data directory if it doesn't exist
        std::fs::create_dir_all(&config.data_dir)
            .map_err(|e| StorageError::IoError(e.to_string()))?;

        // Initialize database
        let db_config = DatabaseConfig::from_storage_config(&config);
        let database = Arc::new(Database::open(&config.data_dir, db_config)?);

        // Initialize sub-databases
        let blockchain_db =
            Box::new(BlockchainDb::new(database.clone())) as Box<dyn BlockchainDatabase>;
        let utxo_db = Box::new(UtxoDb::new(database.clone())) as Box<dyn UTXODatabase>;
        let mempool = Mempool::new().map_err(|e| StorageError::IoError(e.to_string()))?;

        Ok(StorageManager {
            database,
            blockchain_db,
            utxo_db,
            mempool,
            config,
        })
    }

    /// Get blockchain database
    pub fn blockchain_db(&self) -> &dyn BlockchainDatabase {
        self.blockchain_db.as_ref()
    }

    /// Get mutable blockchain database
    pub fn blockchain_db_mut(&mut self) -> &mut dyn BlockchainDatabase {
        self.blockchain_db.as_mut()
    }

    /// Get UTXO database
    pub fn utxo_db(&self) -> &dyn UTXODatabase {
        self.utxo_db.as_ref()
    }

    /// Get mutable UTXO database
    pub fn utxo_db_mut(&mut self) -> &mut dyn UTXODatabase {
        self.utxo_db.as_mut()
    }

    /// Get mempool
    pub fn mempool(&self) -> &Mempool {
        &self.mempool
    }

    /// Get mutable mempool
    pub fn mempool_mut(&mut self) -> &mut Mempool {
        &mut self.mempool
    }

    /// Get storage statistics
    pub fn get_statistics(&self) -> StorageStatistics {
        let db_stats = self.database.get_statistics();
        let mempool_stats = self.mempool.get_statistics();

        StorageStatistics {
            database_size: db_stats.total_size,
            blockchain_entries: db_stats.total_keys,
            utxo_count: 0, // TODO: Implement UTXO count method
            mempool_size: mempool_stats.total_size,
            mempool_transactions: mempool_stats.transaction_count,
            cache_hit_rate: db_stats.cache_hit_rate,
        }
    }

    /// Perform database compaction
    pub fn compact(&self) -> Result<(), StorageError> {
        self.database.compact()?;
        Ok(())
    }

    /// Create database backup
    pub fn backup<P: AsRef<Path>>(&self, backup_path: P) -> Result<(), StorageError> {
        self.database.backup(backup_path)?;
        Ok(())
    }

    /// Flush all pending writes
    pub fn flush(&self) -> Result<(), StorageError> {
        self.database.flush()?;
        Ok(())
    }

    /// Get configuration
    pub fn config(&self) -> &StorageConfig {
        &self.config
    }

    /// Close storage (graceful shutdown)
    pub fn close(self) -> Result<(), StorageError> {
        // Flush pending writes
        self.flush()?;

        // Close database - try to get unique ownership
        match Arc::try_unwrap(self.database) {
            Ok(db) => db.close()?,
            Err(_) => {
                // Database is still shared, just flush
                // It will be closed when the last Arc is dropped
            }
        }

        Ok(())
    }
}

/// Storage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStatistics {
    /// Total database size in bytes
    pub database_size: u64,
    /// Number of blockchain entries
    pub blockchain_entries: u64,
    /// Number of UTXOs
    pub utxo_count: usize,
    /// Mempool size in bytes
    pub mempool_size: usize,
    /// Number of transactions in mempool
    pub mempool_transactions: usize,
    /// Cache hit rate (0.0 to 1.0)
    pub cache_hit_rate: f64,
}

/// Error types for storage operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageError {
    /// Database error
    Database(DatabaseError),
    /// Blockchain database error
    BlockchainDb(BlockchainDbError),
    /// UTXO database error
    UTXODb(UTXODbError),
    /// Mempool error
    Mempool(MempoolError),
    /// Invalid configuration
    InvalidConfig(String),
    /// I/O error
    IoError(String),
    /// Serialization error
    Serialization(String),
    /// Storage corrupted
    Corrupted(String),
    /// Not found
    NotFound,
    /// Already exists
    AlreadyExists,
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::Database(e) => write!(f, "Database error: {}", e),
            StorageError::BlockchainDb(e) => write!(f, "Blockchain database error: {}", e),
            StorageError::UTXODb(e) => write!(f, "UTXO database error: {}", e),
            StorageError::Mempool(e) => write!(f, "Mempool error: {}", e),
            StorageError::InvalidConfig(msg) => write!(f, "Invalid configuration: {}", msg),
            StorageError::IoError(msg) => write!(f, "I/O error: {}", msg),
            StorageError::Serialization(msg) => write!(f, "Serialization error: {}", msg),
            StorageError::Corrupted(msg) => write!(f, "Storage corrupted: {}", msg),
            StorageError::NotFound => write!(f, "Not found"),
            StorageError::AlreadyExists => write!(f, "Already exists"),
        }
    }
}

impl std::error::Error for StorageError {}

impl From<DatabaseError> for StorageError {
    fn from(err: DatabaseError) -> Self {
        StorageError::Database(err)
    }
}

impl From<BlockchainDbError> for StorageError {
    fn from(err: BlockchainDbError) -> Self {
        StorageError::BlockchainDb(err)
    }
}

impl From<UTXODbError> for StorageError {
    fn from(err: UTXODbError) -> Self {
        StorageError::UTXODb(err)
    }
}

impl From<MempoolError> for StorageError {
    fn from(err: MempoolError) -> Self {
        StorageError::Mempool(err)
    }
}

/// Result type for storage operations
pub type StorageResult<T> = Result<T, StorageError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_config_creation() {
        let config = StorageConfig::default("/tmp/btpc_test");

        assert_eq!(config.data_dir, "/tmp/btpc_test");
        assert!(config.cache_size > 0);
        assert!(config.write_buffer_size > 0);
        assert!(config.max_open_files > 0);
    }

    #[test]
    fn test_storage_config_validation() {
        let valid_config = StorageConfig::default("/tmp/btpc_test");
        assert!(valid_config.validate().is_ok());

        let mut invalid_config = valid_config.clone();
        invalid_config.data_dir = String::new();
        assert!(invalid_config.validate().is_err());

        invalid_config = valid_config.clone();
        invalid_config.cache_size = 0;
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_mempool_config() {
        let config = MempoolConfig::default();
        assert!(config.validate().is_ok());

        let test_config = MempoolConfig::test();
        assert!(test_config.validate().is_ok());
        assert!(test_config.max_size < config.max_size);
    }

    #[test]
    fn test_error_conversions() {
        let db_error = DatabaseError::InvalidKey;
        let storage_error: StorageError = db_error.into();

        match storage_error {
            StorageError::Database(DatabaseError::InvalidKey) => (),
            _ => panic!("Error conversion failed"),
        }
    }

    #[test]
    fn test_constants() {
        use constants::*;

        assert!(DEFAULT_CACHE_SIZE > 0);
        assert!(DEFAULT_WRITE_BUFFER_SIZE > 0);
        assert!(DEFAULT_MAX_OPEN_FILES > 0);
        assert!(DEFAULT_MEMPOOL_SIZE > 0);
        assert!(DEFAULT_MAX_MEMPOOL_TXS > 0);
    }
}
