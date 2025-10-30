//! Blockchain data structures and operations for BTPC
//!
//! This module provides Bitcoin-compatible blockchain structures with quantum-resistant signatures.

use std::fmt;

pub mod block;
pub mod chain;
pub mod genesis;
pub mod merkle;
pub mod reorg;
pub mod transaction;
pub mod utxo;

pub use block::{Block, BlockError, BlockHeader};
pub use chain::{BlockChain, ChainError};
pub use genesis::{GenesisConfig, GenesisCreator, GenesisError, WellKnownGenesis};
pub use merkle::{calculate_merkle_root, MerkleError, MerkleTree};
pub use reorg::{calculate_chain_work, compare_chains, find_fork_point, ChainComparison};
pub use transaction::{
    OutPoint, Transaction, TransactionError, TransactionInput, TransactionOutput,
};
pub use utxo::{UTXOError, UTXOSet, UTXO};

use crate::{crypto::Hash, Network};

/// Blockchain constants for BTPC
pub mod constants {
    /// Maximum block size in bytes (Bitcoin-compatible)
    pub const MAX_BLOCK_SIZE: usize = 1_000_000; // 1MB

    /// Maximum transaction size in bytes
    pub const MAX_TRANSACTION_SIZE: usize = 100_000; // 100KB

    /// Maximum number of inputs per transaction
    pub const MAX_TRANSACTION_INPUTS: usize = 1000;

    /// Maximum number of outputs per transaction
    pub const MAX_TRANSACTION_OUTPUTS: usize = 1000;

    /// Coinbase maturity (blocks before coinbase can be spent)
    pub const COINBASE_MATURITY: u32 = 100;

    /// Maximum value for a single output (prevents overflow)
    pub const MAX_OUTPUT_VALUE: u64 = 21_000_000 * 100_000_000; // 21M BTPC in satoshis

    /// Minimum transaction fee (satoshis)
    pub const MIN_TRANSACTION_FEE: u64 = 1000; // 0.00001 BTPC

    /// Blocks per year (assuming 10-minute blocks)
    pub const BLOCKS_PER_YEAR: u32 = 365 * 24 * 6; // 52,560

    /// Years until tail emission
    pub const DECAY_YEARS: u32 = 24;

    /// Initial block reward (32.375 BTPC in satoshis)
    pub const INITIAL_BLOCK_REWARD: u64 = 3_237_500_000;

    /// Tail emission reward (0.5 BTPC in satoshis)
    pub const TAIL_EMISSION_REWARD: u64 = 50_000_000;
}

/// Error types for blockchain operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockchainError {
    /// Block-related error
    Block(BlockError),
    /// Transaction-related error
    Transaction(TransactionError),
    /// UTXO-related error
    UTXO(UTXOError),
    /// Chain-related error
    Chain(ChainError),
    /// Merkle tree error
    Merkle(MerkleError),
    /// Invalid block height
    InvalidHeight,
    /// Invalid timestamp
    InvalidTimestamp,
    /// Insufficient funds
    InsufficientFunds,
    /// Double spending attempt
    DoubleSpend,
    /// Invalid network
    InvalidNetwork,
    /// Serialization error
    Serialization(String),
}

impl fmt::Display for BlockchainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlockchainError::Block(e) => write!(f, "Block error: {}", e),
            BlockchainError::Transaction(e) => write!(f, "Transaction error: {}", e),
            BlockchainError::UTXO(e) => write!(f, "UTXO error: {}", e),
            BlockchainError::Chain(e) => write!(f, "Chain error: {}", e),
            BlockchainError::Merkle(e) => write!(f, "Merkle error: {}", e),
            BlockchainError::InvalidHeight => write!(f, "Invalid block height"),
            BlockchainError::InvalidTimestamp => write!(f, "Invalid timestamp"),
            BlockchainError::InsufficientFunds => write!(f, "Insufficient funds"),
            BlockchainError::DoubleSpend => write!(f, "Double spending attempt"),
            BlockchainError::InvalidNetwork => write!(f, "Invalid network"),
            BlockchainError::Serialization(msg) => write!(f, "Serialization error: {}", msg),
        }
    }
}

impl std::error::Error for BlockchainError {}

impl From<BlockError> for BlockchainError {
    fn from(err: BlockError) -> Self {
        BlockchainError::Block(err)
    }
}

impl From<TransactionError> for BlockchainError {
    fn from(err: TransactionError) -> Self {
        BlockchainError::Transaction(err)
    }
}

impl From<UTXOError> for BlockchainError {
    fn from(err: UTXOError) -> Self {
        BlockchainError::UTXO(err)
    }
}

impl From<ChainError> for BlockchainError {
    fn from(err: ChainError) -> Self {
        BlockchainError::Chain(err)
    }
}

impl From<MerkleError> for BlockchainError {
    fn from(err: MerkleError) -> Self {
        BlockchainError::Merkle(err)
    }
}

/// Result type for blockchain operations
pub type BlockchainResult<T> = Result<T, BlockchainError>;

/// Network parameters for different BTPC networks
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkParams {
    /// Network type
    pub network: Network,
    /// Genesis block hash
    pub genesis_hash: Hash,
    /// Default port for this network
    pub default_port: u16,
    /// Network magic bytes
    pub magic_bytes: [u8; 4],
    /// Minimum difficulty target
    pub min_difficulty_target: u32,
    /// Maximum difficulty target
    pub max_difficulty_target: u32,
}

impl NetworkParams {
    /// Get network parameters for mainnet
    pub fn mainnet() -> Self {
        NetworkParams {
            network: Network::Mainnet,
            genesis_hash: Hash::zero(), // Will be set after genesis creation
            default_port: 8333,
            magic_bytes: [0x42, 0x54, 0x00, 0x01], // "BT" + version
            min_difficulty_target: 0x1d00ffff,
            max_difficulty_target: 0x207fffff,
        }
    }

    /// Get network parameters for testnet
    pub fn testnet() -> Self {
        NetworkParams {
            network: Network::Testnet,
            genesis_hash: Hash::zero(), // Will be set after genesis creation
            default_port: 18333,
            magic_bytes: [0x42, 0x54, 0x01, 0x01], // "BT" + testnet version
            min_difficulty_target: 0x1d0fffff, // Consistent with DifficultyTarget::minimum_for_network
            max_difficulty_target: 0x207fffff,
        }
    }

    /// Get network parameters for regtest
    pub fn regtest() -> Self {
        NetworkParams {
            network: Network::Regtest,
            genesis_hash: Hash::zero(), // Will be set after genesis creation
            default_port: 18444,
            magic_bytes: [0x42, 0x54, 0x02, 0x01], // "BT" + regtest version
            min_difficulty_target: 0x1d0fffff, // Consistent with DifficultyTarget::minimum_for_network
            max_difficulty_target: 0x207fffff,
        }
    }

    /// Get parameters for a specific network
    pub fn for_network(network: Network) -> Self {
        match network {
            Network::Mainnet => Self::mainnet(),
            Network::Testnet => Self::testnet(),
            Network::Regtest => Self::regtest(),
        }
    }
}

/// Calculate block reward based on height using linear decay
pub fn calculate_block_reward(height: u32) -> u64 {
    use constants::*;

    if height == 0 {
        return INITIAL_BLOCK_REWARD;
    }

    let total_decay_blocks = DECAY_YEARS * BLOCKS_PER_YEAR;

    if height >= total_decay_blocks {
        // Tail emission phase
        return TAIL_EMISSION_REWARD;
    }

    // Linear decay phase
    let total_decrease = INITIAL_BLOCK_REWARD - TAIL_EMISSION_REWARD;

    // Ensure the last block gives exactly TAIL_EMISSION_REWARD
    if height == total_decay_blocks - 1 {
        return TAIL_EMISSION_REWARD;
    }

    let decrease_per_block = total_decrease / total_decay_blocks as u64;
    let current_decrease = (height as u64) * decrease_per_block;

    INITIAL_BLOCK_REWARD - current_decrease
}

/// Calculate total supply up to a given height
pub fn calculate_total_supply(height: u32) -> u64 {
    use constants::*;

    if height == 0 {
        return INITIAL_BLOCK_REWARD;
    }

    let total_decay_blocks = DECAY_YEARS * BLOCKS_PER_YEAR;

    if height < total_decay_blocks {
        // During linear decay phase (not including tail emission start)
        let mut total = 0u64;
        for h in 0..=height {
            total += calculate_block_reward(h);
        }
        total
    } else {
        // Calculate decay phase total + tail emission
        let mut decay_total = 0u64;
        for h in 0..total_decay_blocks {
            decay_total += calculate_block_reward(h);
        }

        let tail_blocks = height - total_decay_blocks + 1; // Include the boundary block
        let tail_total = (tail_blocks as u64) * TAIL_EMISSION_REWARD;

        decay_total + tail_total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_params() {
        let mainnet = NetworkParams::mainnet();
        let testnet = NetworkParams::testnet();
        let regtest = NetworkParams::regtest();

        // Different networks should have different magic bytes
        assert_ne!(mainnet.magic_bytes, testnet.magic_bytes);
        assert_ne!(testnet.magic_bytes, regtest.magic_bytes);

        // Different ports
        assert_ne!(mainnet.default_port, testnet.default_port);
        assert_ne!(testnet.default_port, regtest.default_port);

        // Regtest should have easiest difficulty
        assert!(regtest.min_difficulty_target > mainnet.min_difficulty_target);
    }

    #[test]
    fn test_block_reward_calculation() {
        use constants::*;

        // Genesis block
        assert_eq!(calculate_block_reward(0), INITIAL_BLOCK_REWARD);

        // After 1 year
        let year_1_height = BLOCKS_PER_YEAR;
        let year_1_reward = calculate_block_reward(year_1_height);
        assert!(year_1_reward < INITIAL_BLOCK_REWARD);
        assert!(year_1_reward > TAIL_EMISSION_REWARD);

        // At tail emission
        let tail_height = DECAY_YEARS * BLOCKS_PER_YEAR;
        assert_eq!(calculate_block_reward(tail_height), TAIL_EMISSION_REWARD);

        // After tail emission
        assert_eq!(
            calculate_block_reward(tail_height + 1000),
            TAIL_EMISSION_REWARD
        );
    }

    #[test]
    fn test_linear_decay_properties() {
        use constants::*;

        // Reward should decrease monotonically
        let mut previous_reward = INITIAL_BLOCK_REWARD;
        for height in 1..(DECAY_YEARS * BLOCKS_PER_YEAR) {
            let current_reward = calculate_block_reward(height);
            assert!(
                current_reward <= previous_reward,
                "Reward should decrease monotonically at height {}",
                height
            );
            previous_reward = current_reward;
        }

        // Should reach exactly tail emission at the right height
        let tail_height = DECAY_YEARS * BLOCKS_PER_YEAR;
        assert_eq!(calculate_block_reward(tail_height), TAIL_EMISSION_REWARD);
    }

    #[test]
    fn test_total_supply_calculation() {
        use constants::*;

        // Genesis
        assert_eq!(calculate_total_supply(0), INITIAL_BLOCK_REWARD);

        // After some blocks
        let height_100 = 100;
        let supply_100 = calculate_total_supply(height_100);
        assert!(supply_100 > INITIAL_BLOCK_REWARD);

        // Supply should always increase
        let supply_200 = calculate_total_supply(200);
        assert!(supply_200 > supply_100);

        // After tail emission starts
        let tail_height = DECAY_YEARS * BLOCKS_PER_YEAR;
        let tail_supply = calculate_total_supply(tail_height);
        let tail_supply_plus_1000 = calculate_total_supply(tail_height + 1000);

        let expected_increase = 1000 * TAIL_EMISSION_REWARD;
        assert_eq!(tail_supply_plus_1000 - tail_supply, expected_increase);
    }

    #[test]
    fn test_constants_validity() {
        use constants::*;

        // Sanity checks on constants
        assert!(MAX_BLOCK_SIZE > 0);
        assert!(MAX_TRANSACTION_SIZE < MAX_BLOCK_SIZE);
        assert!(COINBASE_MATURITY > 0);
        assert!(INITIAL_BLOCK_REWARD > TAIL_EMISSION_REWARD);
        assert!(TAIL_EMISSION_REWARD > 0);
        assert!(BLOCKS_PER_YEAR > 0);
        assert!(DECAY_YEARS > 0);
    }

    #[test]
    fn test_error_conversions() {
        // Test that error types convert correctly
        let block_error = BlockError::InvalidHash;
        let blockchain_error: BlockchainError = block_error.into();

        match blockchain_error {
            BlockchainError::Block(BlockError::InvalidHash) => (),
            _ => panic!("Error conversion failed"),
        }
    }
}
