// RocksDB configuration for BTPC blockchain
// Implements multi-column family architecture with performance optimizations

use rocksdb::{Options, DB, ColumnFamilyDescriptor, Cache, SliceTransform};
use std::path::Path;
use std::sync::Arc;
use anyhow::Result;

/// Column family names for blockchain data segregation
pub const CF_BLOCKS: &str = "blocks";
pub const CF_TRANSACTIONS: &str = "transactions";
pub const CF_UTXOS: &str = "utxos";
pub const CF_METADATA: &str = "metadata";

/// BlockchainDB wraps RocksDB with optimized configuration
/// Implements multi-column family architecture per research.md
pub struct BlockchainDB {
    db: Arc<DB>,
}

impl BlockchainDB {
    /// Create new BlockchainDB with optimized configuration
    ///
    /// Configuration per research.md:
    /// - Universal compaction strategy for write-heavy workloads
    /// - Large block cache (50-70% RAM) using ClockCache
    /// - Prefix bloom filters for UTXO lookups
    /// - Multi-threaded compaction and writes
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut db_opts = Options::default();
        db_opts.create_if_missing(true);
        db_opts.create_missing_column_families(true);

        // Universal compaction strategy reduces write amplification
        db_opts.set_compaction_style(rocksdb::DBCompactionStyle::Universal);

        // Calculate cache size as 60% of available RAM (mid-range of 50-70%)
        let available_mem = get_available_memory_mb();
        let cache_size = (available_mem as f64 * 0.6 * 1024.0 * 1024.0) as usize;
        let cache = Cache::new_lru_cache(cache_size);

        // Block-based table options with cache
        let mut block_opts = rocksdb::BlockBasedOptions::default();
        block_opts.set_block_cache(&cache);
        block_opts.set_bloom_filter(10.0, false); // 10 bits per key
        db_opts.set_block_based_table_factory(&block_opts);

        // Multi-threaded configuration
        db_opts.increase_parallelism(num_cpus::get() as i32);
        db_opts.set_max_background_jobs(4);

        // Write buffer configuration
        db_opts.set_write_buffer_size(64 * 1024 * 1024); // 64MB
        db_opts.set_max_write_buffer_number(3);

        // Create column family descriptors with specific optimizations
        let cf_blocks = create_blocks_cf();
        let cf_transactions = create_transactions_cf();
        let cf_utxos = create_utxos_cf();
        let cf_metadata = create_metadata_cf();

        let cfs = vec![cf_blocks, cf_transactions, cf_utxos, cf_metadata];

        let db = DB::open_cf_descriptors(&db_opts, path, cfs)?;

        Ok(Self {
            db: Arc::new(db),
        })
    }

    /// Get reference to the underlying RocksDB instance
    pub fn db(&self) -> &DB {
        &self.db
    }

    /// Get data from blocks column family
    pub fn get_block(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let cf = self.db.cf_handle(CF_BLOCKS)
            .ok_or_else(|| anyhow::anyhow!("Blocks CF not found"))?;
        Ok(self.db.get_cf(&cf, key)?)
    }

    /// Put data into blocks column family
    pub fn put_block(&self, key: &[u8], value: &[u8]) -> Result<()> {
        let cf = self.db.cf_handle(CF_BLOCKS)
            .ok_or_else(|| anyhow::anyhow!("Blocks CF not found"))?;
        self.db.put_cf(&cf, key, value)?;
        Ok(())
    }

    /// Delete from blocks column family
    pub fn delete_block(&self, key: &[u8]) -> Result<()> {
        let cf = self.db.cf_handle(CF_BLOCKS)
            .ok_or_else(|| anyhow::anyhow!("Blocks CF not found"))?;
        self.db.delete_cf(&cf, key)?;
        Ok(())
    }

    /// Get data from transactions column family
    pub fn get_transaction(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let cf = self.db.cf_handle(CF_TRANSACTIONS)
            .ok_or_else(|| anyhow::anyhow!("Transactions CF not found"))?;
        Ok(self.db.get_cf(&cf, key)?)
    }

    /// Put data into transactions column family
    pub fn put_transaction(&self, key: &[u8], value: &[u8]) -> Result<()> {
        let cf = self.db.cf_handle(CF_TRANSACTIONS)
            .ok_or_else(|| anyhow::anyhow!("Transactions CF not found"))?;
        self.db.put_cf(&cf, key, value)?;
        Ok(())
    }

    /// Get data from UTXOs column family
    pub fn get_utxo(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let cf = self.db.cf_handle(CF_UTXOS)
            .ok_or_else(|| anyhow::anyhow!("Transactions CF not found"))?;
        Ok(self.db.get_cf(&cf, key)?)
    }

    /// Put data into UTXOs column family
    pub fn put_utxo(&self, key: &[u8], value: &[u8]) -> Result<()> {
        let cf = self.db.cf_handle(CF_UTXOS)
            .ok_or_else(|| anyhow::anyhow!("UTXOs CF not found"))?;
        self.db.put_cf(&cf, key, value)?;
        Ok(())
    }

    /// Delete from UTXOs column family (when UTXO is spent)
    pub fn delete_utxo(&self, key: &[u8]) -> Result<()> {
        let cf = self.db.cf_handle(CF_UTXOS)
            .ok_or_else(|| anyhow::anyhow!("UTXOs CF not found"))?;
        self.db.delete_cf(&cf, key)?;
        Ok(())
    }

    /// Get metadata
    pub fn get_metadata(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let cf = self.db.cf_handle(CF_METADATA)
            .ok_or_else(|| anyhow::anyhow!("Metadata CF not found"))?;
        Ok(self.db.get_cf(&cf, key)?)
    }

    /// Put metadata
    pub fn put_metadata(&self, key: &[u8], value: &[u8]) -> Result<()> {
        let cf = self.db.cf_handle(CF_METADATA)
            .ok_or_else(|| anyhow::anyhow!("Metadata CF not found"))?;
        self.db.put_cf(&cf, key, value)?;
        Ok(())
    }
}

/// Create column family descriptor for blocks
fn create_blocks_cf() -> ColumnFamilyDescriptor {
    let mut opts = Options::default();
    opts.set_compaction_style(rocksdb::DBCompactionStyle::Universal);
    opts.set_write_buffer_size(128 * 1024 * 1024); // 128MB for blocks

    ColumnFamilyDescriptor::new(CF_BLOCKS, opts)
}

/// Create column family descriptor for transactions
fn create_transactions_cf() -> ColumnFamilyDescriptor {
    let mut opts = Options::default();
    opts.set_compaction_style(rocksdb::DBCompactionStyle::Universal);
    opts.set_write_buffer_size(64 * 1024 * 1024); // 64MB for transactions

    ColumnFamilyDescriptor::new(CF_TRANSACTIONS, opts)
}

/// Create column family descriptor for UTXOs with prefix bloom filter
fn create_utxos_cf() -> ColumnFamilyDescriptor {
    let mut opts = Options::default();
    opts.set_compaction_style(rocksdb::DBCompactionStyle::Universal);

    // Prefix bloom filter for efficient UTXO lookups
    // UTXO keys are typically transaction hash (64 bytes) + output index
    opts.set_prefix_extractor(SliceTransform::create_fixed_prefix(32));

    let mut block_opts = rocksdb::BlockBasedOptions::default();
    block_opts.set_bloom_filter(10.0, false);
    block_opts.set_whole_key_filtering(false); // Use prefix filtering
    opts.set_block_based_table_factory(&block_opts);

    opts.set_write_buffer_size(128 * 1024 * 1024); // 128MB for UTXO set

    ColumnFamilyDescriptor::new(CF_UTXOS, opts)
}

/// Create column family descriptor for metadata
fn create_metadata_cf() -> ColumnFamilyDescriptor {
    let mut opts = Options::default();
    opts.set_write_buffer_size(16 * 1024 * 1024); // 16MB for metadata

    ColumnFamilyDescriptor::new(CF_METADATA, opts)
}

/// Get available system memory in MB
/// Used to calculate appropriate cache size (50-70% of RAM)
fn get_available_memory_mb() -> usize {
    // For production, would query actual system memory
    // For now, assume 8GB available and use conservative estimate
    
    8192
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_rocksdb_open() {
        let temp_dir = TempDir::new().unwrap();
        let db = BlockchainDB::open(temp_dir.path()).unwrap();

        // Verify all column families exist
        assert!(db.db().cf_handle(CF_BLOCKS).is_some());
        assert!(db.db().cf_handle(CF_TRANSACTIONS).is_some());
        assert!(db.db().cf_handle(CF_UTXOS).is_some());
        assert!(db.db().cf_handle(CF_METADATA).is_some());
    }

    #[test]
    fn test_block_operations() {
        let temp_dir = TempDir::new().unwrap();
        let db = BlockchainDB::open(temp_dir.path()).unwrap();

        let key = b"block_hash_123";
        let value = b"block_data_123";

        // Put and get
        db.put_block(key, value).unwrap();
        let retrieved = db.get_block(key).unwrap();
        assert_eq!(retrieved, Some(value.to_vec()));

        // Delete
        db.delete_block(key).unwrap();
        let retrieved = db.get_block(key).unwrap();
        assert_eq!(retrieved, None);
    }

    #[test]
    fn test_utxo_operations() {
        let temp_dir = TempDir::new().unwrap();
        let db = BlockchainDB::open(temp_dir.path()).unwrap();

        let key = b"utxo_outpoint_456";
        let value = b"utxo_data_456";

        // Put and get
        db.put_utxo(key, value).unwrap();
        let retrieved = db.get_utxo(key).unwrap();
        assert_eq!(retrieved, Some(value.to_vec()));

        // Delete (spent)
        db.delete_utxo(key).unwrap();
        let retrieved = db.get_utxo(key).unwrap();
        assert_eq!(retrieved, None);
    }
}