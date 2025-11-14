//! Block structures and operations for BTPC
//!
//! BTPC uses Bitcoin-compatible block structure with SHA-512 proof-of-work.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{
    blockchain::{
        constants::MAX_BLOCK_SIZE, merkle::calculate_merkle_root, transaction::Transaction,
    },
    crypto::Hash,
    Network,
};

/// A BTPC block
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    /// Block header
    pub header: BlockHeader,
    /// Block transactions
    pub transactions: Vec<Transaction>,
}

impl Block {
    /// Create a new block
    pub fn new(header: BlockHeader, transactions: Vec<Transaction>) -> Self {
        Block {
            header,
            transactions,
        }
    }

    /// Calculate the block hash
    pub fn hash(&self) -> Hash {
        self.header.hash()
    }

    /// Get block size in bytes
    pub fn size(&self) -> usize {
        self.serialize().len()
    }

    /// Check if block exceeds size limits
    pub fn is_oversized(&self) -> bool {
        self.size() > MAX_BLOCK_SIZE
    }

    /// Validate basic block structure
    pub fn validate_structure(&self) -> Result<(), BlockError> {
        // Check size
        if self.is_oversized() {
            return Err(BlockError::BlockTooLarge);
        }

        // Must have at least one transaction (coinbase)
        if self.transactions.is_empty() {
            return Err(BlockError::NoTransactions);
        }

        // First transaction must be coinbase
        if !self.transactions[0].is_coinbase() {
            return Err(BlockError::NoCoinbase);
        }

        // Only first transaction can be coinbase
        for (i, tx) in self.transactions.iter().enumerate() {
            if i > 0 && tx.is_coinbase() {
                return Err(BlockError::MultipleCoinbase);
            }
        }

        // Validate all transactions
        for tx in &self.transactions {
            tx.validate_structure().map_err(BlockError::Transaction)?;
        }

        // Verify merkle root
        let calculated_merkle =
            calculate_merkle_root(&self.transactions).map_err(BlockError::Merkle)?;

        if self.header.merkle_root != calculated_merkle {
            return Err(BlockError::InvalidMerkleRoot);
        }

        // Validate header
        self.header.validate()?;

        Ok(())
    }

    /// Get the coinbase transaction
    pub fn coinbase_transaction(&self) -> Option<&Transaction> {
        if !self.transactions.is_empty() && self.transactions[0].is_coinbase() {
            Some(&self.transactions[0])
        } else {
            None
        }
    }

    /// Calculate total transaction fees in the block
    pub fn calculate_fees(&self, utxo_set: &crate::blockchain::UTXOSet) -> Result<u64, BlockError> {
        let mut total_fees = 0u64;

        for tx in &self.transactions[1..] {
            // Skip coinbase
            let fee = tx
                .calculate_fee(utxo_set)
                .map_err(BlockError::Transaction)?;
            total_fees = total_fees.checked_add(fee).ok_or(BlockError::FeeOverflow)?;
        }

        Ok(total_fees)
    }

    /// Serialize block to bytes
    pub fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Header
        bytes.extend_from_slice(&self.header.serialize());

        // Transaction count
        Block::write_varint(&mut bytes, self.transactions.len() as u64);

        // Transactions
        for tx in &self.transactions {
            bytes.extend_from_slice(&tx.serialize());
        }

        bytes
    }

    /// Deserialize block from bytes
    pub fn deserialize(bytes: &[u8]) -> Result<Self, BlockError> {
        let mut cursor = 0;

        // Header (144 bytes for BTPC SHA-512 header)
        if bytes.len() < 144 {
            return Err(BlockError::InvalidSerialization);
        }
        let header = BlockHeader::deserialize(&bytes[cursor..cursor + 144])?;
        cursor += 144;

        // Transaction count
        let (tx_count, varint_size) =
            Block::read_varint(&bytes[cursor..]).map_err(|_| BlockError::InvalidSerialization)?;
        cursor += varint_size;

        // Transactions
        let mut transactions = Vec::new();
        for _ in 0..tx_count {
            let tx = Transaction::deserialize(&bytes[cursor..]).map_err(BlockError::Transaction)?;
            cursor += tx.serialize().len();
            transactions.push(tx);
        }

        Ok(Block::new(header, transactions))
    }

    // Helper functions for variable-length integers (copied from Transaction)
    fn write_varint(bytes: &mut Vec<u8>, value: u64) {
        if value < 0xfd {
            bytes.push(value as u8);
        } else if value <= 0xffff {
            bytes.push(0xfd);
            bytes.extend_from_slice(&(value as u16).to_le_bytes());
        } else if value <= 0xffffffff {
            bytes.push(0xfe);
            bytes.extend_from_slice(&(value as u32).to_le_bytes());
        } else {
            bytes.push(0xff);
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }

    fn read_varint(bytes: &[u8]) -> Result<(u64, usize), BlockError> {
        if bytes.is_empty() {
            return Err(BlockError::InvalidSerialization);
        }

        match bytes[0] {
            0..=0xfc => Ok((bytes[0] as u64, 1)),
            0xfd => {
                if bytes.len() < 3 {
                    return Err(BlockError::InvalidSerialization);
                }
                let value = u16::from_le_bytes([bytes[1], bytes[2]]) as u64;
                Ok((value, 3))
            }
            0xfe => {
                if bytes.len() < 5 {
                    return Err(BlockError::InvalidSerialization);
                }
                let value = u32::from_le_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]) as u64;
                Ok((value, 5))
            }
            0xff => {
                if bytes.len() < 9 {
                    return Err(BlockError::InvalidSerialization);
                }
                let value = u64::from_le_bytes([
                    bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7], bytes[8],
                ]);
                Ok((value, 9))
            }
        }
    }

    /// Create a test block for development
    pub fn create_test_block() -> Self {
        use crate::blockchain::calculate_block_reward;

        let coinbase = Transaction::coinbase(calculate_block_reward(0), Hash::random());

        // Calculate correct merkle root for the coinbase transaction
        let merkle_root = calculate_merkle_root(&[coinbase.clone()])
            .expect("Merkle root calculation should not fail for test block with single coinbase transaction");

        let header = BlockHeader {
            version: 1,
            prev_hash: Hash::zero(),
            merkle_root,
            timestamp: 1735344000,
            bits: 0x207fffff,
            nonce: 12345,
        };

        Block::new(header, vec![coinbase])
    }

    /// Create test block with specific timestamp
    #[cfg(test)]
    pub fn create_test_block_with_timestamp(timestamp: u64) -> Self {
        let mut block = Self::create_test_block();
        block.header.timestamp = timestamp;
        block
    }

    /// Create genesis block for network
    #[cfg(test)]
    pub fn create_genesis_block() -> Self {
        let mut header = BlockHeader {
            version: 1,
            prev_hash: Hash::zero(),
            merkle_root: Hash::zero(), // Will be calculated
            timestamp: 1735344000,     // BTPC project start
            bits: 0x207fffff,          // Easy difficulty for regtest
            nonce: 0,
        };

        let mut coinbase =
            Transaction::coinbase(crate::blockchain::calculate_block_reward(0), Hash::random());

        // Add genesis message to coinbase
        coinbase.inputs[0].script_sig.push_data(
            b"BTPC Genesis Block - The Times 03/Jan/2009 Chancellor on brink of second bailout for banks/Security for the future - beyond the financial reset".to_vec()
        );

        // Calculate merkle root
        header.merkle_root = calculate_merkle_root(&[coinbase.clone()])
            .expect("Merkle root calculation should not fail for genesis block with single coinbase transaction");

        // Mine the block (find valid nonce)
        Self::mine_genesis_block(&mut header);

        Block::new(header, vec![coinbase])
    }

    #[cfg(test)]
    fn mine_genesis_block(header: &mut BlockHeader) {
        // Simple mining for regtest (very easy difficulty)
        let target = [0x7fu8; 64]; // Very high target for easy mining

        for nonce in 0..1_000_000 {
            header.nonce = nonce;
            let hash = header.hash();
            if hash.meets_target(&target) {
                return;
            }
        }

        panic!("Failed to mine genesis block");
    }
}

/// Block header structure (Bitcoin-compatible)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockHeader {
    /// Block version
    pub version: u32,
    /// Previous block hash
    pub prev_hash: Hash,
    /// Merkle root of transactions
    pub merkle_root: Hash,
    /// Block timestamp
    pub timestamp: u64,
    /// Difficulty target (compact format)
    pub bits: u32,
    /// Proof-of-work nonce
    pub nonce: u32,
}

impl BlockHeader {
    /// Create a new block header
    pub fn new(
        version: u32,
        prev_hash: Hash,
        merkle_root: Hash,
        timestamp: u64,
        bits: u32,
        nonce: u32,
    ) -> Self {
        BlockHeader {
            version,
            prev_hash,
            merkle_root,
            timestamp,
            bits,
            nonce,
        }
    }

    /// Calculate the block hash (double SHA-512)
    pub fn hash(&self) -> Hash {
        let serialized = self.serialize();
        Hash::double_sha512(&serialized)
    }

    /// Validate block header
    pub fn validate(&self) -> Result<(), BlockError> {
        // Check version
        if self.version == 0 {
            return Err(BlockError::InvalidVersion);
        }

        // Check timestamp (must be reasonable)
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_else(|_| std::time::Duration::from_secs(0))
            .as_secs();

        // Not too far in the future (2 hours)
        if self.timestamp > current_time + 7200 {
            return Err(BlockError::TimestampTooFuture);
        }

        // Not too far in the past (would be checked against median time in full validation)
        if self.timestamp < 1735344000 {
            return Err(BlockError::TimestampTooOld);
        }

        Ok(())
    }

    /// Serialize header to bytes (144 bytes for SHA-512 compatibility)
    /// Layout: version(4) + prev_hash(64) + merkle_root(64) + timestamp(4) + bits(4) + nonce(4) =
    /// 144 bytes
    pub fn serialize(&self) -> [u8; 144] {
        let mut bytes = [0u8; 144];
        let mut cursor = 0;

        // Version (4 bytes)
        bytes[cursor..cursor + 4].copy_from_slice(&self.version.to_le_bytes());
        cursor += 4;

        // Previous hash (64 bytes for SHA-512)
        bytes[cursor..cursor + 64].copy_from_slice(self.prev_hash.as_slice());
        cursor += 64;

        // Merkle root (64 bytes for SHA-512)
        bytes[cursor..cursor + 64].copy_from_slice(self.merkle_root.as_slice());
        cursor += 64;

        // Timestamp (4 bytes, Bitcoin uses 32-bit timestamp)
        bytes[cursor..cursor + 4].copy_from_slice(&(self.timestamp as u32).to_le_bytes());
        cursor += 4;

        // Bits (4 bytes)
        bytes[cursor..cursor + 4].copy_from_slice(&self.bits.to_le_bytes());
        cursor += 4;

        // Nonce (4 bytes)
        bytes[cursor..cursor + 4].copy_from_slice(&self.nonce.to_le_bytes());

        bytes
    }

    /// Deserialize header from bytes
    pub fn deserialize(bytes: &[u8]) -> Result<Self, BlockError> {
        if bytes.len() < 144 {
            return Err(BlockError::InvalidSerialization);
        }

        let mut cursor = 0;

        // Version
        let version = u32::from_le_bytes([
            bytes[cursor],
            bytes[cursor + 1],
            bytes[cursor + 2],
            bytes[cursor + 3],
        ]);
        cursor += 4;

        // Previous hash (full 64 bytes for SHA-512)
        let mut prev_hash_bytes = [0u8; 64];
        prev_hash_bytes.copy_from_slice(&bytes[cursor..cursor + 64]);
        let prev_hash = Hash::from_bytes(prev_hash_bytes);
        cursor += 64;

        // Merkle root (full 64 bytes for SHA-512)
        let mut merkle_bytes = [0u8; 64];
        merkle_bytes.copy_from_slice(&bytes[cursor..cursor + 64]);
        let merkle_root = Hash::from_bytes(merkle_bytes);
        cursor += 64;

        // Timestamp (expand 32-bit to 64-bit)
        let timestamp = u32::from_le_bytes([
            bytes[cursor],
            bytes[cursor + 1],
            bytes[cursor + 2],
            bytes[cursor + 3],
        ]) as u64;
        cursor += 4;

        // Bits
        let bits = u32::from_le_bytes([
            bytes[cursor],
            bytes[cursor + 1],
            bytes[cursor + 2],
            bytes[cursor + 3],
        ]);
        cursor += 4;

        // Nonce
        let nonce = u32::from_le_bytes([
            bytes[cursor],
            bytes[cursor + 1],
            bytes[cursor + 2],
            bytes[cursor + 3],
        ]);

        Ok(BlockHeader {
            version,
            prev_hash,
            merkle_root,
            timestamp,
            bits,
            nonce,
        })
    }

    /// Create genesis header for network
    pub fn genesis_for_network(network: Network) -> Self {
        let bits = match network {
            Network::Mainnet => 0x1d00ffff,
            Network::Testnet => 0x1d00ffff,
            Network::Regtest => 0x207fffff,
        };

        BlockHeader {
            version: 1,
            prev_hash: Hash::zero(),
            merkle_root: Hash::zero(), // Will be calculated
            timestamp: 1735344000,     // BTPC project start
            bits,
            nonce: 0,
        }
    }

    /// Create test header for development
    #[cfg(test)]
    pub fn create_test_header() -> Self {
        BlockHeader {
            version: 1,
            prev_hash: Hash::zero(),
            merkle_root: Hash::from_int(12345),
            timestamp: 1735344000,
            bits: 0x207fffff,
            nonce: 12345,
        }
    }
}

/// Error types for block operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockError {
    /// Invalid block version
    InvalidVersion,
    /// Block too large
    BlockTooLarge,
    /// No transactions in block
    NoTransactions,
    /// No coinbase transaction
    NoCoinbase,
    /// Multiple coinbase transactions
    MultipleCoinbase,
    /// Invalid merkle root
    InvalidMerkleRoot,
    /// Invalid block hash
    InvalidHash,
    /// Timestamp too far in future
    TimestampTooFuture,
    /// Timestamp too old
    TimestampTooOld,
    /// Fee calculation overflow
    FeeOverflow,
    /// Invalid serialization
    InvalidSerialization,
    /// Transaction error
    Transaction(crate::blockchain::TransactionError),
    /// Merkle tree error
    Merkle(crate::blockchain::MerkleError),
}

impl fmt::Display for BlockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlockError::InvalidVersion => write!(f, "Invalid block version"),
            BlockError::BlockTooLarge => write!(f, "Block too large"),
            BlockError::NoTransactions => write!(f, "Block has no transactions"),
            BlockError::NoCoinbase => write!(f, "Block has no coinbase transaction"),
            BlockError::MultipleCoinbase => write!(f, "Block has multiple coinbase transactions"),
            BlockError::InvalidMerkleRoot => write!(f, "Invalid merkle root"),
            BlockError::InvalidHash => write!(f, "Invalid block hash"),
            BlockError::TimestampTooFuture => write!(f, "Block timestamp too far in future"),
            BlockError::TimestampTooOld => write!(f, "Block timestamp too old"),
            BlockError::FeeOverflow => write!(f, "Fee calculation overflow"),
            BlockError::InvalidSerialization => write!(f, "Invalid block serialization"),
            BlockError::Transaction(e) => write!(f, "Transaction error: {}", e),
            BlockError::Merkle(e) => write!(f, "Merkle error: {}", e),
        }
    }
}

impl std::error::Error for BlockError {}

impl From<crate::blockchain::TransactionError> for BlockError {
    fn from(err: crate::blockchain::TransactionError) -> Self {
        BlockError::Transaction(err)
    }
}

impl From<crate::blockchain::MerkleError> for BlockError {
    fn from(err: crate::blockchain::MerkleError) -> Self {
        BlockError::Merkle(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_creation() {
        let header = BlockHeader::create_test_header();
        let coinbase = Transaction::coinbase(3237500000, Hash::random());
        let block = Block::new(header.clone(), vec![coinbase.clone()]);

        assert_eq!(block.header, header);
        assert_eq!(block.transactions.len(), 1);
        assert_eq!(block.transactions[0], coinbase);
    }

    #[test]
    fn test_block_hash() {
        let block = Block::create_test_block();
        let hash1 = block.hash();
        let hash2 = block.hash();

        // Hash should be deterministic
        assert_eq!(hash1, hash2);

        // Different blocks should have different hashes
        let mut block2 = block.clone();
        block2.header.nonce = 54321;
        assert_ne!(block.hash(), block2.hash());
    }

    #[test]
    fn test_block_serialization() {
        let block = Block::create_test_block();
        let serialized = block.serialize();
        let deserialized = Block::deserialize(&serialized).unwrap();

        assert_eq!(block, deserialized);
    }

    #[test]
    fn test_block_validation() {
        let mut block = Block::create_test_block();

        // Valid block should pass
        assert!(block.validate_structure().is_ok());

        // Remove coinbase - should fail
        block.transactions.clear();
        assert!(block.validate_structure().is_err());

        // Add non-coinbase as first transaction - should fail
        block
            .transactions
            .push(Transaction::create_test_transfer(1000000, Hash::random()));
        assert!(block.validate_structure().is_err());
    }

    #[test]
    fn test_header_serialization() {
        let header = BlockHeader::create_test_header();
        let serialized = header.serialize();
        let deserialized = BlockHeader::deserialize(&serialized).unwrap();

        assert_eq!(header, deserialized);
    }

    #[test]
    fn test_genesis_block_creation() {
        let genesis = Block::create_genesis_block();

        assert!(genesis.validate_structure().is_ok());
        assert!(genesis.transactions[0].is_coinbase());
        assert_eq!(genesis.header.prev_hash, Hash::zero());
    }

    #[test]
    fn test_coinbase_detection() {
        let mut block = Block::create_test_block();

        // Should have coinbase
        assert!(block.coinbase_transaction().is_some());

        // Remove coinbase
        block.transactions.clear();
        assert!(block.coinbase_transaction().is_none());
    }

    #[test]
    fn test_header_validation() {
        let mut header = BlockHeader::create_test_header();

        // Valid header should pass
        assert!(header.validate().is_ok());

        // Zero version should fail
        header.version = 0;
        assert!(header.validate().is_err());

        // Future timestamp should fail
        header.version = 1;
        header.timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 10000; // Way in the future
        assert!(header.validate().is_err());
    }

    #[test]
    fn test_block_size_limits() {
        let block = Block::create_test_block();

        // Normal block should not be oversized
        assert!(!block.is_oversized());
        assert!(block.size() < MAX_BLOCK_SIZE);
    }
}
