//! Blockchain chain management for BTPC
//!
//! Manages the chain of blocks and provides validation and query operations.

use std::{collections::HashMap, fmt};

use crate::{
    blockchain::{utxo::UTXOSet, Block, BlockError, NetworkParams},
    crypto::Hash,
    Network,
};

/// Blockchain state and management
#[derive(Debug, Clone)]
pub struct BlockChain {
    /// Network parameters
    network_params: NetworkParams,
    /// Chain of blocks (height -> block)
    blocks: HashMap<u32, Block>,
    /// Block hash to height mapping
    hash_to_height: HashMap<Hash, u32>,
    /// Current chain tip height
    tip_height: u32,
    /// Current chain tip hash
    tip_hash: Hash,
    /// Genesis block hash
    genesis_hash: Hash,
}

impl BlockChain {
    /// Create a new blockchain for the given network
    pub fn new_for_network(network: Network) -> Result<Self, ChainError> {
        let network_params = NetworkParams::for_network(network);

        Ok(BlockChain {
            network_params,
            blocks: HashMap::new(),
            hash_to_height: HashMap::new(),
            tip_height: 0,
            tip_hash: Hash::zero(),
            genesis_hash: Hash::zero(),
        })
    }

    /// Initialize blockchain with genesis block
    pub fn initialize_with_genesis(&mut self, genesis: Block) -> Result<(), ChainError> {
        if !self.blocks.is_empty() {
            return Err(ChainError::AlreadyInitialized);
        }

        // Validate genesis block
        genesis.validate_structure().map_err(ChainError::Block)?;

        // Genesis block should have zero previous hash
        if genesis.header.prev_hash != Hash::zero() {
            return Err(ChainError::InvalidGenesis);
        }

        let genesis_hash = genesis.hash();

        // Add genesis to chain
        self.blocks.insert(0, genesis);
        self.hash_to_height.insert(genesis_hash, 0);
        self.tip_height = 0;
        self.tip_hash = genesis_hash;
        self.genesis_hash = genesis_hash;

        Ok(())
    }

    /// Add a new block to the chain
    pub fn add_block(&mut self, block: Block) -> Result<(), ChainError> {
        // Validate block structure
        block.validate_structure().map_err(ChainError::Block)?;

        let block_hash = block.hash();
        let expected_height = self.tip_height + 1;

        // Check that block connects to current tip
        if block.header.prev_hash != self.tip_hash {
            return Err(ChainError::DoesNotConnect);
        }

        // Check for duplicate block
        if self.hash_to_height.contains_key(&block_hash) {
            return Err(ChainError::DuplicateBlock);
        }

        // Add block to chain
        self.blocks.insert(expected_height, block);
        self.hash_to_height.insert(block_hash, expected_height);
        self.tip_height = expected_height;
        self.tip_hash = block_hash;

        Ok(())
    }

    /// Get block by height
    pub fn get_block_at_height(&self, height: u32) -> Option<&Block> {
        self.blocks.get(&height)
    }

    /// Get block by hash
    pub fn get_block_by_hash(&self, hash: &Hash) -> Option<&Block> {
        if let Some(&height) = self.hash_to_height.get(hash) {
            self.blocks.get(&height)
        } else {
            None
        }
    }

    /// Get current chain height
    pub fn height(&self) -> u32 {
        self.tip_height
    }

    /// Get current best hash
    pub fn best_hash(&self) -> Hash {
        self.tip_hash
    }

    /// Get genesis hash
    pub fn genesis_hash(&self) -> Hash {
        self.genesis_hash
    }

    /// Get network
    pub fn network(&self) -> Network {
        self.network_params.network
    }

    /// Check if blockchain is empty
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    /// Get block count
    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }

    /// Get current difficulty target
    pub fn current_difficulty(&self) -> crate::consensus::DifficultyTarget {
        if let Some(tip_block) = self.get_block_at_height(self.tip_height) {
            crate::consensus::DifficultyTarget::from_bits(tip_block.header.bits)
        } else {
            // Default difficulty for empty chain
            crate::consensus::DifficultyTarget::from_bits(self.network_params.max_difficulty_target)
        }
    }

    /// Calculate total work in the chain
    pub fn total_work(&self) -> f64 {
        let mut total = 0.0;
        for height in 0..=self.tip_height {
            if let Some(block) = self.get_block_at_height(height) {
                let target = crate::consensus::DifficultyTarget::from_bits(block.header.bits);
                // Use work_integer() for consensus-critical calculation
                total += target.work_integer() as f64;
            }
        }
        total
    }

    /// Get chain statistics
    pub fn get_statistics(&self) -> ChainStatistics {
        let mut total_transactions = 0;
        let mut total_size = 0;

        for block in self.blocks.values() {
            total_transactions += block.transactions.len();
            total_size += block.size();
        }

        ChainStatistics {
            height: self.tip_height,
            block_count: self.blocks.len(),
            total_transactions,
            total_size,
            total_work: self.total_work(),
        }
    }

    /// Validate the entire chain
    pub fn validate_chain(&self) -> Result<(), ChainError> {
        if self.is_empty() {
            return Ok(()); // Empty chain is valid
        }

        // Validate genesis
        let genesis = self
            .get_block_at_height(0)
            .ok_or(ChainError::MissingGenesis)?;

        if genesis.header.prev_hash != Hash::zero() {
            return Err(ChainError::InvalidGenesis);
        }

        // Validate each block and chain connections
        for height in 0..=self.tip_height {
            let block = self
                .get_block_at_height(height)
                .ok_or(ChainError::MissingBlock)?;

            // Validate block structure
            block.validate_structure().map_err(ChainError::Block)?;

            // Check chain connection (except genesis)
            if height > 0 {
                let prev_block = self
                    .get_block_at_height(height - 1)
                    .ok_or(ChainError::MissingBlock)?;

                if block.header.prev_hash != prev_block.hash() {
                    return Err(ChainError::BrokenChain);
                }
            }
        }

        Ok(())
    }

    /// Check if a block hash exists in the chain
    pub fn contains_block(&self, hash: &Hash) -> bool {
        self.hash_to_height.contains_key(hash)
    }

    /// Get the height of a block by hash
    pub fn get_height_by_hash(&self, hash: &Hash) -> Option<u32> {
        self.hash_to_height.get(hash).copied()
    }

    /// Get blocks in a range (inclusive)
    pub fn get_blocks_range(&self, start_height: u32, end_height: u32) -> Vec<&Block> {
        let mut blocks = Vec::new();
        for height in start_height..=end_height.min(self.tip_height) {
            if let Some(block) = self.get_block_at_height(height) {
                blocks.push(block);
            }
        }
        blocks
    }

    /// Find the common ancestor of two block hashes
    pub fn find_common_ancestor(&self, hash1: &Hash, hash2: &Hash) -> Option<(Hash, u32)> {
        let height1 = self.get_height_by_hash(hash1)?;
        let height2 = self.get_height_by_hash(hash2)?;

        // Start from the lower height
        let mut current_height = height1.min(height2);

        while current_height > 0 {
            let block1 = self.get_block_at_height(current_height)?;
            let block2 = self.get_block_at_height(current_height)?;

            // If we're at different heights, we need to trace back
            if height1 != height2 {
                // This is simplified - full implementation would trace back properly
                current_height -= 1;
                continue;
            }

            if block1.hash() == block2.hash() {
                return Some((block1.hash(), current_height));
            }

            current_height -= 1;
        }

        // Genesis is always a common ancestor
        if current_height == 0 {
            let genesis = self.get_block_at_height(0)?;
            Some((genesis.hash(), 0))
        } else {
            None
        }
    }

    /// Build UTXO set from the current chain
    pub fn build_utxo_set(&self) -> Result<UTXOSet, ChainError> {
        let mut utxo_set = UTXOSet::new();

        // Apply all blocks in order
        for height in 0..=self.tip_height {
            let block = self
                .get_block_at_height(height)
                .ok_or(ChainError::MissingBlock)?;

            utxo_set.apply_block(block).map_err(ChainError::UTXO)?;
        }

        Ok(utxo_set)
    }

    /// Create test blockchain with blocks
    #[cfg(test)]
    pub fn create_test_blockchain_with_blocks(count: u32) -> Self {
        let mut chain = BlockChain::new_for_network(Network::Regtest)
            .expect("Test blockchain creation should not fail");

        // Add genesis
        let genesis = Block::create_genesis_block();
        chain.initialize_with_genesis(genesis)
            .expect("Test genesis initialization should not fail");

        // Add additional blocks
        for i in 1..count {
            let mut block = Block::create_test_block();
            block.header.prev_hash = chain.best_hash();
            block.header.timestamp = 1735344000 + (i as u64 * 600); // 10 minute intervals

            // Recalculate merkle root
            block.header.merkle_root =
                crate::blockchain::calculate_merkle_root(&block.transactions)
                    .expect("Test merkle root calculation should not fail");

            chain.add_block(block)
                .expect("Test block addition should not fail");
        }

        chain
    }
}

/// Chain statistics
#[derive(Debug, Clone, PartialEq)]
pub struct ChainStatistics {
    /// Current chain height
    pub height: u32,
    /// Total number of blocks
    pub block_count: usize,
    /// Total number of transactions
    pub total_transactions: usize,
    /// Total chain size in bytes
    pub total_size: usize,
    /// Total proof-of-work
    pub total_work: f64,
}

/// Error types for blockchain operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainError {
    /// Block-related error
    Block(BlockError),
    /// UTXO-related error
    UTXO(crate::blockchain::UTXOError),
    /// Chain already initialized
    AlreadyInitialized,
    /// Invalid genesis block
    InvalidGenesis,
    /// Missing genesis block
    MissingGenesis,
    /// Missing block in chain
    MissingBlock,
    /// Block does not connect to chain
    DoesNotConnect,
    /// Duplicate block
    DuplicateBlock,
    /// Broken chain (missing connection)
    BrokenChain,
    /// Invalid chain height
    InvalidHeight,
    /// Chain reorganization needed
    ReorganizationNeeded,
}

impl fmt::Display for ChainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChainError::Block(e) => write!(f, "Block error: {}", e),
            ChainError::UTXO(e) => write!(f, "UTXO error: {}", e),
            ChainError::AlreadyInitialized => write!(f, "Blockchain already initialized"),
            ChainError::InvalidGenesis => write!(f, "Invalid genesis block"),
            ChainError::MissingGenesis => write!(f, "Missing genesis block"),
            ChainError::MissingBlock => write!(f, "Missing block in chain"),
            ChainError::DoesNotConnect => write!(f, "Block does not connect to chain"),
            ChainError::DuplicateBlock => write!(f, "Duplicate block"),
            ChainError::BrokenChain => write!(f, "Broken chain connection"),
            ChainError::InvalidHeight => write!(f, "Invalid chain height"),
            ChainError::ReorganizationNeeded => write!(f, "Chain reorganization needed"),
        }
    }
}

impl std::error::Error for ChainError {}

impl From<BlockError> for ChainError {
    fn from(err: BlockError) -> Self {
        ChainError::Block(err)
    }
}

impl From<crate::blockchain::UTXOError> for ChainError {
    fn from(err: crate::blockchain::UTXOError) -> Self {
        ChainError::UTXO(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blockchain_creation() {
        let chain = BlockChain::new_for_network(Network::Regtest).unwrap();

        assert!(chain.is_empty());
        assert_eq!(chain.height(), 0);
        assert_eq!(chain.block_count(), 0);
        assert_eq!(chain.network(), Network::Regtest);
    }

    #[test]
    fn test_genesis_initialization() {
        let mut chain = BlockChain::new_for_network(Network::Regtest).unwrap();
        let genesis = Block::create_genesis_block();
        let genesis_hash = genesis.hash();

        chain.initialize_with_genesis(genesis).unwrap();

        assert!(!chain.is_empty());
        assert_eq!(chain.height(), 0);
        assert_eq!(chain.block_count(), 1);
        assert_eq!(chain.genesis_hash(), genesis_hash);
        assert_eq!(chain.best_hash(), genesis_hash);
    }

    #[test]
    fn test_block_addition() {
        let mut chain = BlockChain::new_for_network(Network::Regtest).unwrap();
        let genesis = Block::create_genesis_block();

        chain.initialize_with_genesis(genesis.clone()).unwrap();

        // Add second block
        let mut block2 = Block::create_test_block();
        block2.header.prev_hash = genesis.hash();
        block2.header.merkle_root =
            crate::blockchain::calculate_merkle_root(&block2.transactions).unwrap();

        chain.add_block(block2.clone()).unwrap();

        assert_eq!(chain.height(), 1);
        assert_eq!(chain.block_count(), 2);
        assert_eq!(chain.best_hash(), block2.hash());
    }

    #[test]
    fn test_block_retrieval() {
        let mut chain = BlockChain::new_for_network(Network::Regtest).unwrap();
        let genesis = Block::create_genesis_block();
        let genesis_hash = genesis.hash();

        chain.initialize_with_genesis(genesis.clone()).unwrap();

        // Test retrieval by height
        let retrieved_by_height = chain.get_block_at_height(0).unwrap();
        assert_eq!(retrieved_by_height.hash(), genesis_hash);

        // Test retrieval by hash
        let retrieved_by_hash = chain.get_block_by_hash(&genesis_hash).unwrap();
        assert_eq!(retrieved_by_hash.hash(), genesis_hash);

        // Test non-existent block
        assert!(chain.get_block_at_height(100).is_none());
        assert!(chain.get_block_by_hash(&Hash::random()).is_none());
    }

    #[test]
    fn test_chain_validation() {
        let mut chain = BlockChain::new_for_network(Network::Regtest).unwrap();
        let genesis = Block::create_genesis_block();

        chain.initialize_with_genesis(genesis.clone()).unwrap();

        // Valid chain should pass validation
        assert!(chain.validate_chain().is_ok());

        // Add valid block
        let mut block2 = Block::create_test_block();
        block2.header.prev_hash = genesis.hash();
        block2.header.merkle_root =
            crate::blockchain::calculate_merkle_root(&block2.transactions).unwrap();

        chain.add_block(block2).unwrap();
        assert!(chain.validate_chain().is_ok());
    }

    #[test]
    fn test_invalid_block_connection() {
        let mut chain = BlockChain::new_for_network(Network::Regtest).unwrap();
        let genesis = Block::create_genesis_block();

        chain.initialize_with_genesis(genesis).unwrap();

        // Try to add block that doesn't connect
        let mut invalid_block = Block::create_test_block();
        invalid_block.header.prev_hash = Hash::random(); // Wrong previous hash

        assert!(chain.add_block(invalid_block).is_err());
    }

    #[test]
    fn test_duplicate_block_prevention() {
        let mut chain = BlockChain::new_for_network(Network::Regtest).unwrap();
        let genesis = Block::create_genesis_block();

        chain.initialize_with_genesis(genesis.clone()).unwrap();

        let mut block2 = Block::create_test_block();
        block2.header.prev_hash = genesis.hash();
        block2.header.merkle_root =
            crate::blockchain::calculate_merkle_root(&block2.transactions).unwrap();

        // First addition should succeed
        assert!(chain.add_block(block2.clone()).is_ok());

        // Second addition should fail (duplicate)
        assert!(chain.add_block(block2).is_err());
    }

    #[test]
    fn test_chain_statistics() {
        let chain = BlockChain::create_test_blockchain_with_blocks(5);
        let stats = chain.get_statistics();

        assert_eq!(stats.height, 4); // 0-indexed
        assert_eq!(stats.block_count, 5);
        assert!(stats.total_transactions >= 5); // At least one coinbase per block
        assert!(stats.total_size > 0);
        assert!(stats.total_work > 0.0);
    }

    #[test]
    fn test_utxo_set_building() {
        let chain = BlockChain::create_test_blockchain_with_blocks(3);
        let utxo_set = chain.build_utxo_set().unwrap();

        // Should have UTXOs from coinbase transactions
        let stats = utxo_set.get_statistics();
        assert_eq!(stats.total_count, 3); // One coinbase per block
        assert_eq!(stats.coinbase_count, 3);
    }

    #[test]
    fn test_block_range_retrieval() {
        let chain = BlockChain::create_test_blockchain_with_blocks(10);

        let blocks = chain.get_blocks_range(3, 7);
        assert_eq!(blocks.len(), 5); // Heights 3, 4, 5, 6, 7

        // Test out of range
        let blocks = chain.get_blocks_range(15, 20);
        assert!(blocks.is_empty());
    }

    #[test]
    fn test_double_initialization_prevention() {
        let mut chain = BlockChain::new_for_network(Network::Regtest).unwrap();
        let genesis = Block::create_genesis_block();

        // First initialization should succeed
        assert!(chain.initialize_with_genesis(genesis.clone()).is_ok());

        // Second initialization should fail
        assert!(chain.initialize_with_genesis(genesis).is_err());
    }
}
