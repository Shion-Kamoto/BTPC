//! Network State Tracking Module
//!
//! This module tracks the current state of the BTPC blockchain network including:
//! - Best block hash and height
//! - Total chain work
//! - UTXO set hash for validation
//! - Total supply calculation
//! - Network hashrate estimation

use crate::blockchain::Block;
use crate::storage::BlockchainDB;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

/// Helper for serializing/deserializing [u8; 64] arrays
mod serde_array {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(array: &[u8; 64], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        array[..].serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 64], D::Error>
    where
        D: Deserializer<'de>,
    {
        let vec: Vec<u8> = Vec::deserialize(deserializer)?;
        if vec.len() != 64 {
            return Err(serde::de::Error::invalid_length(vec.len(), &"64"));
        }
        let mut array = [0u8; 64];
        array.copy_from_slice(&vec);
        Ok(array)
    }
}

/// Network state information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkState {
    /// Hash of the best (tip) block
    #[serde(with = "serde_array")]
    pub best_block_hash: [u8; 64],
    /// Current blockchain height
    pub height: u64,
    /// Cumulative proof-of-work (total chain work)
    pub total_work: u128,
    /// Hash of the UTXO set for validation
    #[serde(with = "serde_array")]
    pub utxo_set_hash: [u8; 64],
    /// Total supply in satoshis (sum of all UTXOs)
    pub total_supply: u64,
    /// Estimated network hashrate (hashes/second)
    pub network_hashrate: f64,
    /// Timestamp of last state update
    pub last_updated: u64,
}

impl NetworkState {
    /// Create a new network state for genesis
    pub fn genesis(genesis_block: &Block) -> Self {
        Self {
            best_block_hash: *genesis_block.hash().as_bytes(),
            height: 0,
            total_work: 0,
            utxo_set_hash: [0u8; 64], // Will be computed from genesis UTXO
            total_supply: 0, // Genesis block creates initial supply
            network_hashrate: 0.0,
            last_updated: genesis_block.header.timestamp,
        }
    }

    /// Update state with a new block
    pub fn update_with_block(&mut self, block: &Block, utxo_set_hash: [u8; 64], total_supply: u64) {
        self.best_block_hash = *block.hash().as_bytes();
        self.height += 1;
        self.total_work += self.calculate_block_work(block);
        self.utxo_set_hash = utxo_set_hash;
        self.total_supply = total_supply;
        self.last_updated = block.header.timestamp;
    }

    /// Calculate proof-of-work for a block based on difficulty target
    fn calculate_block_work(&self, block: &Block) -> u128 {
        // Work = 2^256 / (target + 1)
        // For simplicity, we use the difficulty bits as a proxy
        // In a real implementation, this would compute the actual work from the target
        let difficulty_bits = block.header.bits;
        2u128.pow(difficulty_bits)
    }

    /// Calculate network hashrate from recent blocks
    pub fn calculate_hashrate(&mut self, recent_blocks: &[Block]) {
        if recent_blocks.len() < 2 {
            self.network_hashrate = 0.0;
            return;
        }

        // Calculate average time between blocks
        let first_time = recent_blocks[0].header.timestamp;
        let last_time = recent_blocks[recent_blocks.len() - 1].header.timestamp;
        let time_span = (last_time - first_time) as f64;

        if time_span == 0.0 {
            self.network_hashrate = 0.0;
            return;
        }

        // Estimate hashrate based on difficulty and block time
        // hashrate = difficulty * 2^32 / time_span
        let avg_difficulty = recent_blocks
            .iter()
            .map(|b| b.header.bits as f64)
            .sum::<f64>() / recent_blocks.len() as f64;

        self.network_hashrate = (avg_difficulty * 2f64.powi(32)) / time_span;
    }
}

/// Network state manager with thread-safe access
pub struct NetworkStateManager {
    state: Arc<RwLock<NetworkState>>,
    db: Arc<BlockchainDB>,
}

impl NetworkStateManager {
    /// Create a new network state manager
    pub fn new(db: Arc<BlockchainDB>, initial_state: NetworkState) -> Self {
        Self {
            state: Arc::new(RwLock::new(initial_state)),
            db,
        }
    }

    /// Get current network state
    pub fn get_state(&self) -> Result<NetworkState> {
        let state = self.state.read()
            .map_err(|e| anyhow!("Lock poisoned: {}", e))?;
        Ok(state.clone())
    }

    /// Update state on new block
    pub fn on_new_block(&self, block: &Block, utxo_set_hash: [u8; 64], total_supply: u64) -> Result<()> {
        let mut state = self.state.write()
            .map_err(|e| anyhow!("Lock poisoned: {}", e))?;
        state.update_with_block(block, utxo_set_hash, total_supply);

        // Persist to database
        self.save_state(&state)?;

        Ok(())
    }

    /// Recalculate network hashrate
    pub fn update_hashrate(&self, recent_blocks: &[Block]) -> Result<()> {
        let mut state = self.state.write()
            .map_err(|e| anyhow!("Lock poisoned: {}", e))?;
        state.calculate_hashrate(recent_blocks);
        self.save_state(&state)?;
        Ok(())
    }

    /// Save state to database
    fn save_state(&self, state: &NetworkState) -> Result<()> {
        let state_bytes = bincode::serialize(state)
            .map_err(|e| anyhow!("Failed to serialize network state: {}", e))?;

        self.db.put_metadata(b"network_state", &state_bytes)
            .map_err(|e| anyhow!("Failed to save network state: {}", e))?;

        Ok(())
    }

    /// Load state from database
    pub fn load_state(&self) -> Result<NetworkState> {
        let state_bytes = self.db.get_metadata(b"network_state")?
            .ok_or_else(|| anyhow!("Network state not found in database"))?;

        let state: NetworkState = bincode::deserialize(&state_bytes)
            .map_err(|e| anyhow!("Failed to deserialize network state: {}", e))?;

        Ok(state)
    }

    /// Initialize or restore state from database
    pub fn initialize(db: Arc<BlockchainDB>, genesis_block: &Block) -> Result<Self> {
        let manager = Self::new(
            Arc::clone(&db),
            NetworkState::genesis(genesis_block),
        );

        // Try to load existing state
        if let Ok(stored_state) = manager.load_state() {
            *manager.state.write()
                .map_err(|e| anyhow!("Lock poisoned: {}", e))? = stored_state;
        } else {
            // Save genesis state
            let state = manager.state.read()
                .map_err(|e| anyhow!("Lock poisoned: {}", e))?;
            manager.save_state(&state)?;
        }

        Ok(manager)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blockchain::{BlockHeader, Transaction};

    fn create_test_block(height: u64, timestamp: u64) -> Block {
        use crate::crypto::Hash;
        let header = BlockHeader {
            version: 1,
            prev_hash: Hash::zero(),
            merkle_root: Hash::zero(),
            timestamp,
            bits: 20,
            nonce: 0,
        };

        Block {
            header,
            transactions: vec![],
        }
    }

    #[test]
    fn test_genesis_state() {
        let block = create_test_block(0, 1000000);
        let state = NetworkState::genesis(&block);

        assert_eq!(state.height, 0);
        assert_eq!(state.total_work, 0);
        assert_eq!(state.last_updated, 1000000);
    }

    #[test]
    fn test_update_with_block() {
        let genesis = create_test_block(0, 1000000);
        let mut state = NetworkState::genesis(&genesis);

        let block = create_test_block(1, 1000600);
        let utxo_hash = [1u8; 64];
        let total_supply = 3237500000; // 32.375 BTPC in satoshis

        state.update_with_block(&block, utxo_hash, total_supply);

        assert_eq!(state.height, 1);
        assert_eq!(state.total_supply, 3237500000);
        assert_eq!(state.utxo_set_hash, utxo_hash);
    }

    #[test]
    fn test_hashrate_calculation() {
        let genesis = create_test_block(0, 1000000);
        let mut state = NetworkState::genesis(&genesis);

        let blocks = vec![
            create_test_block(0, 1000000),
            create_test_block(1, 1000600),  // 600 seconds later
            create_test_block(2, 1001200),  // Another 600 seconds
        ];

        state.calculate_hashrate(&blocks);
        assert!(state.network_hashrate > 0.0);
    }
}