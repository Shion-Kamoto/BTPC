// Block and transaction storage implementation
use std::sync::Arc;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    blockchain::{Block, BlockHeader},
    crypto::Hash,
    storage::database::Database,
};

/// Blockchain database for storing blocks and transactions
#[derive(Debug)]
pub struct BlockchainDb {
    db: Arc<Database>,
}

/// Blockchain database interface
pub trait BlockchainDatabase {
    /// Store a block
    fn store_block(&mut self, block: &crate::blockchain::Block) -> Result<(), BlockchainDbError>;
    /// Get a block by hash
    fn get_block(
        &self,
        hash: &crate::crypto::Hash,
    ) -> Result<Option<crate::blockchain::Block>, BlockchainDbError>;
    /// Get block header by hash
    fn get_header(
        &self,
        hash: &crate::crypto::Hash,
    ) -> Result<Option<crate::blockchain::BlockHeader>, BlockchainDbError>;
    /// Get the tip of the chain (latest block)
    fn get_chain_tip(&self) -> Result<Option<crate::blockchain::Block>, BlockchainDbError>;
    /// Get the height of a block by its hash
    fn get_block_height(&self, hash: &crate::crypto::Hash) -> Result<u32, BlockchainDbError>;
    /// Check if a transaction exists in any block
    fn has_transaction(&self, txid: &crate::crypto::Hash) -> Result<bool, BlockchainDbError>;
    /// Store transaction ID mapping to block (for duplicate detection)
    fn store_transaction(&mut self, txid: &crate::crypto::Hash, block_hash: &crate::crypto::Hash) -> Result<(), BlockchainDbError>;
}

impl BlockchainDb {
    /// Create a new blockchain database with the given underlying database
    pub fn new(db: Arc<Database>) -> Self {
        BlockchainDb { db }
    }

    /// Encode a block hash as a block key
    fn encode_block_key(block_hash: &Hash) -> Vec<u8> {
        let mut key = Vec::with_capacity(6 + 32); // "block:" + hash (first 32 bytes)
        key.extend_from_slice(b"block:");
        // Use only the first 32 bytes of the 64-byte hash for storage efficiency
        key.extend_from_slice(&block_hash.as_bytes()[..32]);
        key
    }

    /// Encode a block hash as a header key
    fn encode_header_key(block_hash: &Hash) -> Vec<u8> {
        let mut key = Vec::with_capacity(7 + 32); // "header:" + hash (first 32 bytes)
        key.extend_from_slice(b"header:");
        // Use only the first 32 bytes of the 64-byte hash for storage efficiency
        key.extend_from_slice(&block_hash.as_bytes()[..32]);
        key
    }

    /// Store block statistics and metadata
    pub fn get_stats(&self) -> BlockchainStats {
        let stats = self.db.get_statistics();
        BlockchainStats {
            total_blocks: 0,       // Would need to count block entries
            total_transactions: 0, // Would need to iterate all blocks
            db_size: stats.total_size,
        }
    }

    /// Iterate over all blocks
    pub fn iter_blocks(
        &self,
    ) -> impl Iterator<Item = Result<(Hash, Block), BlockchainDbError>> + '_ {
        self.db.iter_prefix(b"block:").filter_map(|result| {
            match result {
                Ok((key, value)) => {
                    // Extract block hash from key and deserialize block from value
                    match Self::decode_block_entry(&key, &value) {
                        Ok((hash, block)) => Some(Ok((hash, block))),
                        Err(e) => Some(Err(e)),
                    }
                }
                Err(e) => Some(Err(BlockchainDbError::DatabaseError(e.to_string()))),
            }
        })
    }

    /// Decode a database entry into block hash and block
    fn decode_block_entry(key: &[u8], value: &[u8]) -> Result<(Hash, Block), BlockchainDbError> {
        if !key.starts_with(b"block:") || key.len() != 38 {
            // 6 + 32
            return Err(BlockchainDbError::InvalidBlockData(
                "Invalid key format".to_string(),
            ));
        }

        // Extract hash from key
        let hash_bytes: [u8; 32] = key[6..38]
            .try_into()
            .map_err(|_| BlockchainDbError::InvalidBlockData("Invalid hash length".to_string()))?;

        // Convert 32-byte hash to 64-byte hash by padding with zeros
        let mut full_hash_bytes = [0u8; 64];
        full_hash_bytes[0..32].copy_from_slice(&hash_bytes);

        let block_hash = Hash::from_bytes(full_hash_bytes);

        // Deserialize block from value
        let block: Block = serde_json::from_slice(value).map_err(|e| {
            BlockchainDbError::InvalidBlockData(format!("Deserialization failed: {}", e))
        })?;

        Ok((block_hash, block))
    }

    /// Decode a database entry into block hash and header
    fn decode_header_entry(
        key: &[u8],
        value: &[u8],
    ) -> Result<(Hash, BlockHeader), BlockchainDbError> {
        if !key.starts_with(b"header:") || key.len() != 39 {
            // 7 + 32
            return Err(BlockchainDbError::InvalidBlockData(
                "Invalid key format".to_string(),
            ));
        }

        // Extract hash from key
        let hash_bytes: [u8; 32] = key[7..39]
            .try_into()
            .map_err(|_| BlockchainDbError::InvalidBlockData("Invalid hash length".to_string()))?;

        // Convert 32-byte hash to 64-byte hash by padding with zeros
        let mut full_hash_bytes = [0u8; 64];
        full_hash_bytes[0..32].copy_from_slice(&hash_bytes);

        let block_hash = Hash::from_bytes(full_hash_bytes);

        // Deserialize header from value
        let header: BlockHeader = serde_json::from_slice(value).map_err(|e| {
            BlockchainDbError::InvalidBlockData(format!("Deserialization failed: {}", e))
        })?;

        Ok((block_hash, header))
    }

    /// Flush pending blockchain database writes to disk
    pub fn flush(&self) -> Result<(), BlockchainDbError> {
        self.db
            .flush()
            .map_err(|e| BlockchainDbError::DatabaseError(e.to_string()))
    }

    /// Compact the blockchain database
    pub fn compact(&self) -> Result<(), BlockchainDbError> {
        self.db
            .compact()
            .map_err(|e| BlockchainDbError::DatabaseError(e.to_string()))
    }
}

/// Blockchain statistics
#[derive(Debug, Clone)]
pub struct BlockchainStats {
    /// Total number of blocks stored
    pub total_blocks: u64,
    /// Total number of transactions across all blocks
    pub total_transactions: u64,
    /// Database size in bytes
    pub db_size: u64,
}

impl BlockchainDatabase for BlockchainDb {
    fn store_block(&mut self, block: &Block) -> Result<(), BlockchainDbError> {
        // Calculate block hash
        let block_hash = block.hash();

        // Serialize block and header
        let block_value = serde_json::to_vec(block).map_err(|e| {
            BlockchainDbError::InvalidBlockData(format!("Block serialization failed: {}", e))
        })?;
        let header_value = serde_json::to_vec(&block.header).map_err(|e| {
            BlockchainDbError::InvalidBlockData(format!("Header serialization failed: {}", e))
        })?;

        // Create keys
        let block_key = Self::encode_block_key(&block_hash);
        let header_key = Self::encode_header_key(&block_hash);

        // Calculate height: get previous tip height and add 1
        let height = if block.header.prev_hash.as_bytes() == &[0u8; 64] {
            // Genesis block
            0u32
        } else {
            // Get height of previous block and increment
            self.get_block_height(&block.header.prev_hash)
                .unwrap_or(0)
                .saturating_add(1)
        };

        // Store height metadata for this block
        let mut height_key = Vec::with_capacity(7 + 64); // "height:" is 7 bytes
        height_key.extend_from_slice(b"height:");
        height_key.extend_from_slice(block_hash.as_bytes());
        let height_value = height.to_le_bytes().to_vec();

        // Determine if we should update the chain tip
        // Update tip if: 1) This block extends current tip OR 2) This creates a heavier chain
        let should_update_tip = {
            // Get current tip height
            let current_tip_height = match self.db.get(b"meta:tip_height") {
                Ok(Some(bytes)) if bytes.len() == 4 => {
                    let height_bytes: [u8; 4] = bytes[0..4].try_into()
                        .expect("Slice length validated as 4 bytes by match guard above");
                    u32::from_le_bytes(height_bytes)
                }
                _ => 0, // No tip set yet, this is first block
            };

            // Update tip if new block has greater or equal height
            // (This ensures we follow the longest chain)
            height >= current_tip_height
        };

        // Prepare tip metadata (must be outside if block for lifetime)
        let tip_key = b"meta:chain_tip";
        let tip_value = block_hash.as_bytes().to_vec();
        let tip_height_key = b"meta:tip_height";
        let tip_height_value = height.to_le_bytes().to_vec();

        // Prepare key-value pairs
        let mut pairs = vec![
            (block_key.as_slice(), block_value.as_slice()),
            (header_key.as_slice(), header_value.as_slice()),
            (height_key.as_slice(), height_value.as_slice()),
        ];

        // Only update tip if this block is part of the longest chain
        if should_update_tip {
            pairs.push((tip_key.as_slice(), tip_value.as_slice()));
            pairs.push((tip_height_key, tip_height_value.as_slice()));
        }

        self.db
            .put_batch(&pairs)
            .map_err(|e| BlockchainDbError::DatabaseError(e.to_string()))?;

        // Flush to ensure metadata is persisted
        self.db
            .flush()
            .map_err(|e| BlockchainDbError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    fn get_block(&self, hash: &Hash) -> Result<Option<Block>, BlockchainDbError> {
        let key = Self::encode_block_key(hash);

        match self.db.get(&key) {
            Ok(Some(value)) => {
                let block: Block = serde_json::from_slice(&value).map_err(|e| {
                    BlockchainDbError::InvalidBlockData(format!(
                        "Block deserialization failed: {}",
                        e
                    ))
                })?;
                Ok(Some(block))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(BlockchainDbError::DatabaseError(e.to_string())),
        }
    }

    fn get_header(&self, hash: &Hash) -> Result<Option<BlockHeader>, BlockchainDbError> {
        let key = Self::encode_header_key(hash);

        match self.db.get(&key) {
            Ok(Some(value)) => {
                let header: BlockHeader = serde_json::from_slice(&value).map_err(|e| {
                    BlockchainDbError::InvalidBlockData(format!(
                        "Header deserialization failed: {}",
                        e
                    ))
                })?;
                Ok(Some(header))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(BlockchainDbError::DatabaseError(e.to_string())),
        }
    }

    fn get_chain_tip(&self) -> Result<Option<Block>, BlockchainDbError> {
        // Read chain tip metadata key
        let tip_key = b"meta:chain_tip";

        match self.db.get(tip_key) {
            Ok(Some(hash_bytes)) => {
                // Deserialize the 64-byte hash
                if hash_bytes.len() != 64 {
                    return Err(BlockchainDbError::InvalidBlockData(
                        format!("Invalid chain tip hash length: {}", hash_bytes.len())
                    ));
                }

                let mut hash_array = [0u8; 64];
                hash_array.copy_from_slice(&hash_bytes);
                let tip_hash = Hash::from_bytes(hash_array);

                // Fetch the block
                self.get_block(&tip_hash)
            }
            Ok(None) => Ok(None), // No chain tip set (empty blockchain)
            Err(e) => Err(BlockchainDbError::DatabaseError(e.to_string())),
        }
    }

    fn get_block_height(&self, block_hash: &Hash) -> Result<u32, BlockchainDbError> {
        let mut height_key = Vec::with_capacity(7 + 64);
        height_key.extend_from_slice(b"height:");
        height_key.extend_from_slice(block_hash.as_bytes());

        match self.db.get(&height_key) {
            Ok(Some(bytes)) => {
                if bytes.len() == 4 {
                    let height_bytes: [u8; 4] = bytes[0..4].try_into()
                        .map_err(|_| BlockchainDbError::InvalidBlockData("Invalid height bytes".to_string()))?;
                    Ok(u32::from_le_bytes(height_bytes))
                } else {
                    Err(BlockchainDbError::InvalidBlockData("Invalid height length".to_string()))
                }
            }
            Ok(None) => Err(BlockchainDbError::BlockNotFound),
            Err(e) => Err(BlockchainDbError::DatabaseError(e.to_string())),
        }
    }

    fn has_transaction(&self, txid: &Hash) -> Result<bool, BlockchainDbError> {
        // Create transaction key: "tx:" + txid (first 32 bytes)
        let mut tx_key = Vec::with_capacity(3 + 32);
        tx_key.extend_from_slice(b"tx:");
        tx_key.extend_from_slice(&txid.as_bytes()[..32]);

        match self.db.get(&tx_key) {
            Ok(Some(_)) => Ok(true),
            Ok(None) => Ok(false),
            Err(e) => Err(BlockchainDbError::DatabaseError(e.to_string())),
        }
    }

    fn store_transaction(&mut self, txid: &Hash, block_hash: &Hash) -> Result<(), BlockchainDbError> {
        // Create transaction key: "tx:" + txid (first 32 bytes)
        let mut tx_key = Vec::with_capacity(3 + 32);
        tx_key.extend_from_slice(b"tx:");
        tx_key.extend_from_slice(&txid.as_bytes()[..32]);

        // Store the block hash where this transaction appears
        let tx_value = block_hash.as_bytes().to_vec();

        self.db
            .put(&tx_key, &tx_value)
            .map_err(|e| BlockchainDbError::DatabaseError(e.to_string()))
    }
}

/// Blockchain database errors
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum BlockchainDbError {
    #[error("Database operation failed: {0}")]
    DatabaseError(String),
    #[error("Block not found")]
    BlockNotFound,
    #[error("Invalid block data: {0}")]
    InvalidBlockData(String),
}
