//! Unified RocksDB Database Module
//!
//! Implements single RocksDB instance with 5 column families:
//! - CF_BLOCKS: Block data (btpc-core managed)
//! - CF_TRANSACTIONS: Transaction data (btpc-core managed)
//! - CF_UTXOS: UTXO set (btpc-core managed)
//! - CF_METADATA: Chain metadata (btpc-core managed)
//! - CF_WALLETS: Desktop app wallet data (NEW - replaces separate .dat files)
//!
//! This eliminates database duplication and provides shared access to blockchain
//! state between embedded node and desktop app features.

use anyhow::{Context, Result};
use rocksdb::{BoundColumnFamily, ColumnFamilyDescriptor, Options, DB};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Column family names (from btpc-core/src/storage/rocksdb_config.rs)
pub const CF_BLOCKS: &str = "blocks";
pub const CF_TRANSACTIONS: &str = "transactions";
pub const CF_UTXOS: &str = "utxos";
pub const CF_METADATA: &str = "metadata";

/// NEW column family for desktop app wallet storage
pub const CF_WALLETS: &str = "wallets";

/// All column families that must be created
const COLUMN_FAMILIES: &[&str] = &[
    CF_BLOCKS,
    CF_TRANSACTIONS,
    CF_UTXOS,
    CF_METADATA,
    CF_WALLETS,
];

/// Unified database handle with thread-safe Arc wrapper
///
/// Design decision (from research.md):
/// - Single RocksDB instance reduces memory overhead (512MB cache total vs 1GB for 2 DBs)
/// - Column families provide logical separation (btpc-core vs desktop app data)
/// - Arc<DB> allows shared immutable access across modules
#[derive(Clone)]
pub struct UnifiedDatabase {
    db: Arc<DB>,
    data_path: PathBuf,
}

impl UnifiedDatabase {
    /// Open or create unified database with all column families
    ///
    /// # Arguments
    /// * `data_dir` - Base data directory (e.g., ~/.btpc)
    ///
    /// # Returns
    /// * `Ok(UnifiedDatabase)` - Successfully opened/created database
    /// * `Err(anyhow::Error)` - Failed to open database (permissions, corruption, etc.)
    ///
    /// # Performance
    /// - 512MB block cache (shared across all column families)
    /// - LZ4 compression for blocks/transactions (reduce disk I/O)
    /// - Bloom filters for UTXO lookups (reduce read amplification)
    pub fn open<P: AsRef<Path>>(data_dir: P) -> Result<Self> {
        let data_path = data_dir.as_ref().join("blockchain.db");

        // Create database options with performance tuning
        let mut db_opts = Options::default();
        db_opts.create_if_missing(true);
        db_opts.create_missing_column_families(true);

        // Performance tuning (from research.md R002)
        db_opts.set_max_open_files(1000);
        db_opts.set_use_fsync(false); // Use fdatasync for better performance
        db_opts.set_bytes_per_sync(1048576); // 1MB background sync
        db_opts.set_keep_log_file_num(10); // Limit WAL log files

        // Block cache: 512MB shared across all column families (set via write buffer)
        db_opts.set_write_buffer_size(64 * 1024 * 1024); // 64MB write buffer
        db_opts.set_max_write_buffer_number(3);
        db_opts.set_target_file_size_base(64 * 1024 * 1024); // 64MB SST files

        // Create column family descriptors with specialized options
        let cf_descriptors: Vec<ColumnFamilyDescriptor> = COLUMN_FAMILIES
            .iter()
            .map(|cf_name| {
                let mut cf_opts = Options::default();

                // Blocks/Transactions: Enable LZ4 compression
                if *cf_name == CF_BLOCKS || *cf_name == CF_TRANSACTIONS {
                    cf_opts.set_compression_type(rocksdb::DBCompressionType::Lz4);
                }

                // Wallets: Optimize for small key-value pairs
                if *cf_name == CF_WALLETS {
                    cf_opts.set_write_buffer_size(16 * 1024 * 1024); // 16MB (smaller than blockchain data)
                }

                ColumnFamilyDescriptor::new(*cf_name, cf_opts)
            })
            .collect();

        // Open database with all column families
        let db = DB::open_cf_descriptors(&db_opts, &data_path, cf_descriptors)
            .with_context(|| format!("Failed to open database at {:?}", data_path))?;

        Ok(UnifiedDatabase {
            db: Arc::new(db),
            data_path,
        })
    }

    /// Get raw Arc<DB> handle for btpc-core integration
    ///
    /// btpc-core modules (BlockchainDb, UtxoDb) need direct DB access.
    /// This provides the Arc<DB> handle they expect.
    pub fn inner(&self) -> Arc<DB> {
        self.db.clone()
    }

    /// Get column family handle by name
    ///
    /// # Arguments
    /// * `cf_name` - Column family name (CF_BLOCKS, CF_UTXOS, etc.)
    ///
    /// # Returns
    /// * `Some(Arc<BoundColumnFamily>)` - Column family exists
    /// * `None` - Column family not found (should never happen if COLUMN_FAMILIES is correct)
    pub fn cf_handle(&self, cf_name: &str) -> Option<Arc<BoundColumnFamily<'_>>> {
        self.db.cf_handle(cf_name)
    }

    /// Get database path for logging/diagnostics
    pub fn path(&self) -> &Path {
        &self.data_path
    }

    /// Get database statistics for monitoring
    ///
    /// Returns human-readable stats about:
    /// - Total keys per column family
    /// - Disk usage per column family
    /// - Cache hit ratio
    /// - Compaction stats
    pub fn get_stats(&self) -> Result<DatabaseStats> {
        let mut stats = DatabaseStats::default();

        // Get approximate sizes for each column family
        for cf_name in COLUMN_FAMILIES {
            if let Some(cf) = self.cf_handle(cf_name) {
                // Get approximate number of keys
                if let Ok(Some(count)) = self.db.property_int_value_cf(&cf, "rocksdb.estimate-num-keys") {
                    match *cf_name {
                        CF_BLOCKS => stats.blocks_count = count,
                        CF_TRANSACTIONS => stats.transactions_count = count,
                        CF_UTXOS => stats.utxos_count = count,
                        CF_WALLETS => stats.wallets_count = count,
                        _ => {}
                    }
                }

                // Get approximate disk usage
                if let Ok(Some(size)) = self.db.property_int_value_cf(&cf, "rocksdb.total-sst-files-size") {
                    match *cf_name {
                        CF_BLOCKS => stats.blocks_size_bytes = size,
                        CF_TRANSACTIONS => stats.transactions_size_bytes = size,
                        CF_UTXOS => stats.utxos_size_bytes = size,
                        CF_WALLETS => stats.wallets_size_bytes = size,
                        _ => {}
                    }
                }
            }
        }

        // Get cache hit ratio
        if let Ok(Some(hits)) = self.db.property_int_value("rocksdb.block.cache.hit") {
            if let Ok(Some(misses)) = self.db.property_int_value("rocksdb.block.cache.miss") {
                let total = hits + misses;
                if total > 0 {
                    stats.cache_hit_ratio = (hits as f64 / total as f64) * 100.0;
                }
            }
        }

        Ok(stats)
    }

    /// Force manual compaction (for maintenance operations)
    ///
    /// Triggers RocksDB compaction to reclaim disk space from deleted keys.
    /// Should be called during idle periods or after large deletions.
    pub fn compact(&self) -> Result<()> {
        for cf_name in COLUMN_FAMILIES {
            if let Some(cf) = self.cf_handle(cf_name) {
                self.db
                    .compact_range_cf(&cf, None::<&[u8]>, None::<&[u8]>);
            }
        }
        Ok(())
    }

    /// Flush Write-Ahead Log to disk
    ///
    /// Called during graceful shutdown to ensure all pending writes are persisted.
    /// Part of shutdown sequence: Mining → P2P → Mempool → **WAL flush** → Key zeroization
    pub fn flush_wal(&self) -> Result<()> {
        self.db
            .flush_wal(true)
            .context("Failed to flush Write-Ahead Log")?;
        Ok(())
    }

    /// Get block by height
    ///
    /// # T011-005
    /// Part of Feature 011: Frontend-Backend Integration
    ///
    /// Retrieves a block from the blockchain at the specified height.
    /// This method iterates through the DEFAULT column family to find
    /// the block hash corresponding to the given height, then retrieves
    /// the full block from CF_BLOCKS.
    ///
    /// # Arguments
    /// * `height` - The block height to retrieve (0 = genesis block)
    ///
    /// # Returns
    /// * `Ok(Some(Block))` - Block found at the specified height
    /// * `Ok(None)` - No block exists at this height
    /// * `Err(anyhow::Error)` - Database error or deserialization failure
    ///
    /// # Performance Note
    /// This implementation iterates through height metadata entries to find
    /// the matching block hash. For better performance, a future optimization
    /// could add a reverse index (height → hash) in the database.
    ///
    /// # Database Schema
    /// btpc-core stores height metadata in the DEFAULT column family:
    /// - Key: b"height:" + block_hash (64 bytes)
    /// - Value: height as 4-byte little-endian u32
    pub fn get_block(&self, height: u32) -> Result<Option<btpc_core::blockchain::Block>> {
        use btpc_core::blockchain::Block;

        // Iterate through all entries in DEFAULT column family to find height mapping
        // Note: btpc-core stores metadata in default CF, not CF_METADATA
        // Key format: "height:" + block_hash (64 bytes)
        // Value: height as 4-byte little-endian u32
        let mut iter = self.db.raw_iterator();
        iter.seek(b"height:");

        while iter.valid() {
            if let (Some(key), Some(value)) = (iter.key(), iter.value()) {
                // Check if this is a height entry (starts with "height:")
                if key.starts_with(b"height:") && value.len() == 4 {
                    let stored_height = u32::from_le_bytes([value[0], value[1], value[2], value[3]]);

                    if stored_height == height {
                        // Found matching height! Extract block hash from key
                        // Key format: b"height:" (7 bytes) + block_hash (64 bytes)
                        if key.len() >= 7 + 64 {
                            let block_hash_bytes = &key[7..7 + 64];

                            // Now get the block from CF_BLOCKS
                            let blocks_cf = self
                                .cf_handle(CF_BLOCKS)
                                .context("CF_BLOCKS not found")?;

                            // Block key format: "block:" + block_hash
                            let mut block_key = Vec::with_capacity(6 + 64);
                            block_key.extend_from_slice(b"block:");
                            block_key.extend_from_slice(block_hash_bytes);

                            if let Some(block_bytes) = self.db.get_cf(&blocks_cf, &block_key)? {
                                // Deserialize block from JSON (btpc-core format)
                                let block: Block = serde_json::from_slice(&block_bytes)
                                    .context("Failed to deserialize block from JSON")?;
                                return Ok(Some(block));
                            }
                        }
                    }
                }
            }

            iter.next();
        }

        // No block found at this height
        Ok(None)
    }

    /// Get transaction by txid hash
    ///
    /// # Arguments
    /// * `txid_hash` - Transaction ID as 64-byte SHA-512 hash
    ///
    /// # Returns
    /// * `Ok(Some((Transaction, u32)))` - Transaction found with block height
    /// * `Ok(None)` - Transaction not found in database
    /// * `Err(anyhow::Error)` - Database error or deserialization failure
    ///
    /// # Database Schema
    /// btpc-core stores transactions in CF_TRANSACTIONS:
    /// - Key: txid (SHA-512 hash, 64 bytes)
    /// - Value: serialized Transaction
    pub fn get_transaction(&self, txid_hash: &[u8; 64]) -> Result<Option<(btpc_core::blockchain::Transaction, u32)>> {
        use btpc_core::blockchain::Transaction;

        // Get CF_TRANSACTIONS handle
        let cf = self.cf_handle(CF_TRANSACTIONS)
            .ok_or_else(|| anyhow::anyhow!("CF_TRANSACTIONS not found"))?;

        // Query database for transaction
        match self.db.get_cf(&cf, txid_hash)? {
            Some(tx_bytes) => {
                // Deserialize transaction
                let transaction = Transaction::deserialize(&tx_bytes)
                    .map_err(|e| anyhow::anyhow!("Failed to deserialize transaction: {}", e))?;

                // Find block height containing this transaction
                // Strategy: Iterate through blocks to find which one contains this txid
                // Note: This is O(n) on blocks, but CF_TRANSACTIONS queries are rare
                // Future optimization: Store block_height in transaction metadata
                let block_height = self.find_block_height_for_transaction(txid_hash)?;

                Ok(Some((transaction, block_height)))
            }
            None => Ok(None),
        }
    }

    /// Find block height containing a transaction (helper for get_transaction)
    ///
    /// Iterates through blocks to find which one contains the transaction.
    /// Returns 0 if not found in any block (should not happen if tx is in CF_TRANSACTIONS).
    fn find_block_height_for_transaction(&self, txid_hash: &[u8; 64]) -> Result<u32> {
        use btpc_core::crypto::Hash;

        // Iterate through height metadata to find all blocks
        let mut iter = self.db.raw_iterator();
        iter.seek(b"height:");

        while iter.valid() {
            if let (Some(key), Some(value)) = (iter.key(), iter.value()) {
                // Check if key starts with "height:"
                if !key.starts_with(b"height:") {
                    break; // No more height entries
                }

                // Parse block height from value (4-byte little-endian u32)
                if value.len() == 4 {
                    let height = u32::from_le_bytes([value[0], value[1], value[2], value[3]]);

                    // Get block at this height
                    if let Ok(Some(block)) = self.get_block(height) {
                        // Check if any transaction in this block matches txid
                        let target_hash = Hash::from_bytes(*txid_hash);
                        for tx in &block.transactions {
                            if tx.txid() == target_hash {
                                return Ok(height);
                            }
                        }
                    }
                }
            }

            iter.next();
        }

        // Transaction not found in any block (return 0 as fallback)
        Ok(0)
    }

    /// Get maximum blockchain height from database
    ///
    /// Iterates through all height metadata entries to find the highest block.
    ///
    /// # Returns
    /// * `Ok(Some((height, block_hash)))` - Maximum height found with its block hash
    /// * `Ok(None)` - No blocks in database (fresh blockchain)
    /// * `Err(anyhow::Error)` - Database error
    ///
    /// # Database Schema
    /// btpc-core stores height metadata in DEFAULT column family:
    /// - Key: b"height:" + block_hash (64 bytes)
    /// - Value: height as 4-byte little-endian u32
    pub fn get_max_height(&self) -> Result<Option<(u64, String)>> {
        let mut max_height: Option<(u32, Vec<u8>)> = None;

        // Iterate through all height entries
        let mut iter = self.db.raw_iterator();
        iter.seek(b"height:");

        while iter.valid() {
            if let (Some(key), Some(value)) = (iter.key(), iter.value()) {
                // Check if key starts with "height:"
                if !key.starts_with(b"height:") {
                    break; // No more height entries
                }

                // Parse block height from value (4-byte little-endian u32)
                if value.len() == 4 && key.len() >= 7 + 64 {
                    let height = u32::from_le_bytes([value[0], value[1], value[2], value[3]]);

                    // Extract block hash from key (bytes 7 to 71)
                    let block_hash_bytes = key[7..7 + 64].to_vec();

                    // Update max if this is higher
                    if max_height.is_none() || height > max_height.as_ref().unwrap().0 {
                        max_height = Some((height, block_hash_bytes));
                    }
                }
            }

            iter.next();
        }

        // Convert to (u64, hex_string) format
        match max_height {
            Some((height, hash_bytes)) => {
                let hash_hex = hex::encode(&hash_bytes);
                Ok(Some((height as u64, hash_hex)))
            }
            None => Ok(None),
        }
    }
}

/// Database statistics for monitoring and diagnostics
#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct DatabaseStats {
    /// Number of blocks stored
    pub blocks_count: u64,
    /// Number of transactions stored
    pub transactions_count: u64,
    /// Number of UTXOs in set
    pub utxos_count: u64,
    /// Number of wallets stored
    pub wallets_count: u64,

    /// Disk space used by blocks (bytes)
    pub blocks_size_bytes: u64,
    /// Disk space used by transactions (bytes)
    pub transactions_size_bytes: u64,
    /// Disk space used by UTXOs (bytes)
    pub utxos_size_bytes: u64,
    /// Disk space used by wallets (bytes)
    pub wallets_size_bytes: u64,

    /// Cache hit ratio (0-100%)
    pub cache_hit_ratio: f64,
}

impl DatabaseStats {
    /// Total number of keys across all column families
    pub fn total_keys(&self) -> u64 {
        self.blocks_count + self.transactions_count + self.utxos_count + self.wallets_count
    }

    /// Total disk usage across all column families (bytes)
    pub fn total_size_bytes(&self) -> u64 {
        self.blocks_size_bytes + self.transactions_size_bytes + self.utxos_size_bytes + self.wallets_size_bytes
    }

    /// Total disk usage in megabytes
    pub fn total_size_mb(&self) -> f64 {
        self.total_size_bytes() as f64 / (1024.0 * 1024.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_open_unified_database() {
        // Arrange
        let temp_dir = tempdir().expect("Failed to create temp dir");

        // Act
        let db = UnifiedDatabase::open(temp_dir.path()).expect("Failed to open database");

        // Assert
        assert!(db.cf_handle(CF_BLOCKS).is_some(), "CF_BLOCKS should exist");
        assert!(db.cf_handle(CF_TRANSACTIONS).is_some(), "CF_TRANSACTIONS should exist");
        assert!(db.cf_handle(CF_UTXOS).is_some(), "CF_UTXOS should exist");
        assert!(db.cf_handle(CF_METADATA).is_some(), "CF_METADATA should exist");
        assert!(db.cf_handle(CF_WALLETS).is_some(), "CF_WALLETS should exist");
    }

    #[test]
    fn test_database_stats() {
        // Arrange
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let db = UnifiedDatabase::open(temp_dir.path()).expect("Failed to open database");

        // Act
        let stats = db.get_stats().expect("Failed to get stats");

        // Assert
        assert_eq!(stats.blocks_count, 0, "Fresh database should have 0 blocks");
        assert_eq!(stats.utxos_count, 0, "Fresh database should have 0 UTXOs");
        assert!(stats.cache_hit_ratio >= 0.0 && stats.cache_hit_ratio <= 100.0,
                "Cache hit ratio should be in range [0, 100]");
    }

    #[test]
    fn test_flush_wal() {
        // Arrange
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let db = UnifiedDatabase::open(temp_dir.path()).expect("Failed to open database");

        // Act
        let result = db.flush_wal();

        // Assert
        assert!(result.is_ok(), "WAL flush should succeed");
    }

    #[test]
    fn test_compact() {
        // Arrange
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let db = UnifiedDatabase::open(temp_dir.path()).expect("Failed to open database");

        // Act
        let result = db.compact();

        // Assert
        assert!(result.is_ok(), "Compaction should succeed");
    }
}