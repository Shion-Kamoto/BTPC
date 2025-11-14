//! Genesis block creation and validation
//!
//! Utilities for creating and managing genesis blocks for different networks.

use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::{
    blockchain::{
        Block, BlockHeader, OutPoint, Transaction, TransactionInput as TxInput,
        TransactionOutput as TxOutput,
    },
    consensus::DifficultyTarget,
    crypto::{Hash, Script},
};

/// Genesis block configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisConfig {
    /// Timestamp for the genesis block
    pub timestamp: u64,
    /// Genesis message/coinbase message
    pub message: String,
    /// Initial difficulty target
    pub difficulty_target: DifficultyTarget,
    /// Genesis reward amount (in base units)
    pub reward: u64,
    /// Genesis recipient public key hash
    pub recipient_pubkey_hash: Vec<u8>,
    /// Network name
    pub network: String,
}

impl GenesisConfig {
    /// Create mainnet genesis configuration
    pub fn mainnet() -> Self {
        GenesisConfig {
            timestamp: 1735689600, // 2025-01-01 00:00:00 UTC
            message: "Times 03/Jan/2009 Chancellor on brink of second bailout for banks/ 22/Nov/2025 Security for the future - beyond the financial reset".to_string(),
            difficulty_target: DifficultyTarget::minimum_for_network(crate::Network::Mainnet),
            reward: 5_000_000_000, // 50 BTPC (with 8 decimal places)
            recipient_pubkey_hash: vec![0u8; 20], // Burn address
            network: "mainnet".to_string(),
        }
    }

    /// Create testnet genesis configuration
    pub fn testnet() -> Self {
        GenesisConfig {
            timestamp: 1735689600, // Same as mainnet for deterministic testing
            message: "BTPC Testnet Genesis Block - Post-Quantum Bitcoin".to_string(),
            difficulty_target: DifficultyTarget::minimum_for_network(crate::Network::Testnet),
            reward: 5_000_000_000,                // 50 BTPC
            recipient_pubkey_hash: vec![0u8; 20], // Burn address
            network: "testnet".to_string(),
        }
    }

    /// Create regtest genesis configuration
    pub fn regtest() -> Self {
        GenesisConfig {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_else(|_| std::time::Duration::from_secs(0))
                .as_secs(),
            message: "BTPC Regtest Genesis Block".to_string(),
            difficulty_target: DifficultyTarget::minimum_for_network(crate::Network::Regtest),
            reward: 5_000_000_000,                // 50 BTPC
            recipient_pubkey_hash: vec![0u8; 20], // Burn address
            network: "regtest".to_string(),
        }
    }

    /// Create custom genesis configuration
    pub fn custom(
        timestamp: u64,
        message: String,
        difficulty_target: DifficultyTarget,
        reward: u64,
        recipient_pubkey_hash: Vec<u8>,
        network: String,
    ) -> Self {
        GenesisConfig {
            timestamp,
            message,
            difficulty_target,
            reward,
            recipient_pubkey_hash,
            network,
        }
    }
}

/// Genesis block creator
pub struct GenesisCreator {
    config: GenesisConfig,
}

impl GenesisCreator {
    /// Create a new genesis creator with the given configuration
    pub fn new(config: GenesisConfig) -> Self {
        GenesisCreator { config }
    }

    /// Create the genesis block
    pub fn create_genesis_block(&self) -> Block {
        let coinbase_tx = self.create_coinbase_transaction();
        let merkle_root = Self::calculate_merkle_root(&[&coinbase_tx]);

        let header = BlockHeader {
            version: 1,
            prev_hash: Hash::zero(), // Genesis has no previous block
            merkle_root,
            timestamp: self.config.timestamp,
            bits: self.config.difficulty_target.bits,
            nonce: 0, // Will be set during mining
        };

        Block {
            header,
            transactions: vec![coinbase_tx],
        }
    }

    /// Create the coinbase transaction for genesis block
    fn create_coinbase_transaction(&self) -> Transaction {
        // Genesis coinbase input (special case)
        let coinbase_input = TxInput {
            previous_output: OutPoint {
                txid: Hash::zero(),
                vout: 0xFFFFFFFF, // Special value for coinbase
            },
            script_sig: self.create_coinbase_script(),
            sequence: 0xFFFFFFFF,
        };

        // Genesis output to recipient
        let genesis_output = TxOutput {
            value: self.config.reward,
            script_pubkey: self.create_genesis_output_script(),
        };

        Transaction {
            version: 1,
            inputs: vec![coinbase_input],
            outputs: vec![genesis_output],
            lock_time: 0,
            fork_id: 0, // Genesis defaults to mainnet (Issue #6)
        }
    }

    /// Create the coinbase script with genesis message
    fn create_coinbase_script(&self) -> Script {
        let mut script_data = Vec::new();

        // Add timestamp (4 bytes)
        script_data.extend_from_slice(&self.config.timestamp.to_le_bytes());

        // Add difficulty target (64 bytes, but use first 32 for compatibility)
        script_data.extend_from_slice(&self.config.difficulty_target.target[..32]);

        // Add message length and message
        let message_bytes = self.config.message.as_bytes();
        script_data.push(message_bytes.len() as u8);
        script_data.extend_from_slice(message_bytes);

        Script::from_bytes(script_data)
    }

    /// Create the output script for genesis block
    fn create_genesis_output_script(&self) -> Script {
        // Create a simple P2PKH-style script for the genesis output
        let mut script_data = Vec::new();

        // OP_DUP OP_HASH160 <pubkey_hash> OP_EQUALVERIFY OP_CHECKSIG
        script_data.push(0x76); // OP_DUP
        script_data.push(0xa9); // OP_HASH160
        script_data.push(self.config.recipient_pubkey_hash.len() as u8);
        script_data.extend_from_slice(&self.config.recipient_pubkey_hash);
        script_data.push(0x88); // OP_EQUALVERIFY
        script_data.push(0xac); // OP_CHECKSIG

        Script::from_bytes(script_data)
    }

    /// Calculate merkle root for transactions
    fn calculate_merkle_root(transactions: &[&Transaction]) -> Hash {
        if transactions.is_empty() {
            return Hash::zero();
        }

        if transactions.len() == 1 {
            return transactions[0].hash();
        }

        let mut hashes: Vec<Hash> = transactions.iter().map(|tx| tx.hash()).collect();

        while hashes.len() > 1 {
            let mut next_level = Vec::new();

            for chunk in hashes.chunks(2) {
                let left = chunk[0];
                let right = if chunk.len() > 1 { chunk[1] } else { chunk[0] };

                // Combine left and right hashes
                let mut combined = Vec::new();
                combined.extend_from_slice(left.as_bytes());
                combined.extend_from_slice(right.as_bytes());

                next_level.push(Hash::hash(&combined));
            }

            hashes = next_level;
        }

        hashes[0]
    }

    /// Mine the genesis block (find valid nonce)
    pub fn mine_genesis_block(&self) -> Block {
        let mut block = self.create_genesis_block();

        println!("Mining genesis block for {}...", self.config.network);
        println!("Target: {}", self.config.difficulty_target);

        let start_time = SystemTime::now();
        let mut attempts = 0u64;

        loop {
            block.header.nonce += 1;
            attempts += 1;

            let block_hash = block.header.hash();

            if self.config.difficulty_target.validates_hash(&block_hash) {
                let elapsed = start_time.elapsed().unwrap_or_default();
                println!("Found genesis block!");
                println!("Hash: {}", hex::encode(block_hash.as_bytes()));
                println!("Nonce: {}", block.header.nonce);
                println!("Attempts: {}", attempts);
                println!("Time: {:.2}s", elapsed.as_secs_f64());
                return block;
            }

            // Progress update every million attempts
            if attempts % 1_000_000 == 0 {
                println!("Tried {} nonces...", attempts);
            }
        }
    }

    /// Validate a genesis block
    pub fn validate_genesis_block(&self, block: &Block) -> Result<(), GenesisError> {
        // Check that it's actually a genesis block
        if block.header.prev_hash != Hash::zero() {
            return Err(GenesisError::NotGenesisBlock);
        }

        // Check timestamp
        if block.header.timestamp != self.config.timestamp {
            return Err(GenesisError::InvalidTimestamp);
        }

        // Check difficulty target
        if block.header.bits != self.config.difficulty_target.bits {
            return Err(GenesisError::InvalidDifficulty);
        }

        // Check that we have exactly one transaction (coinbase)
        if block.transactions.len() != 1 {
            return Err(GenesisError::InvalidTransactionCount);
        }

        // Validate coinbase transaction
        self.validate_coinbase_transaction(&block.transactions[0])?;

        // Check merkle root
        let calculated_merkle_root =
            Self::calculate_merkle_root(&block.transactions.iter().collect::<Vec<_>>());
        if block.header.merkle_root != calculated_merkle_root {
            return Err(GenesisError::InvalidMerkleRoot);
        }

        // Check proof of work
        let block_hash = block.header.hash();
        if !self.config.difficulty_target.validates_hash(&block_hash) {
            return Err(GenesisError::InvalidProofOfWork);
        }

        Ok(())
    }

    /// Validate the coinbase transaction
    fn validate_coinbase_transaction(&self, tx: &Transaction) -> Result<(), GenesisError> {
        // Check version
        if tx.version != 1 {
            return Err(GenesisError::InvalidCoinbaseVersion);
        }

        // Check inputs
        if tx.inputs.len() != 1 {
            return Err(GenesisError::InvalidCoinbaseInputs);
        }

        let coinbase_input = &tx.inputs[0];

        // Check coinbase input format
        if coinbase_input.previous_output.txid != Hash::zero()
            || coinbase_input.previous_output.vout != 0xFFFFFFFF
        {
            return Err(GenesisError::InvalidCoinbaseInput);
        }

        // Check outputs
        if tx.outputs.len() != 1 {
            return Err(GenesisError::InvalidCoinbaseOutputs);
        }

        let output = &tx.outputs[0];

        // Check reward amount
        if output.value != self.config.reward {
            return Err(GenesisError::InvalidReward);
        }

        Ok(())
    }

    /// Get the expected genesis block hash for this configuration
    pub fn expected_hash(&self) -> Hash {
        self.create_genesis_block().hash()
    }
}

/// Genesis block errors
#[derive(Debug, thiserror::Error)]
pub enum GenesisError {
    #[error("Not a genesis block")]
    NotGenesisBlock,
    #[error("Invalid timestamp")]
    InvalidTimestamp,
    #[error("Invalid difficulty")]
    InvalidDifficulty,
    #[error("Invalid transaction count")]
    InvalidTransactionCount,
    #[error("Invalid merkle root")]
    InvalidMerkleRoot,
    #[error("Invalid proof of work")]
    InvalidProofOfWork,
    #[error("Invalid coinbase version")]
    InvalidCoinbaseVersion,
    #[error("Invalid coinbase inputs")]
    InvalidCoinbaseInputs,
    #[error("Invalid coinbase input format")]
    InvalidCoinbaseInput,
    #[error("Invalid coinbase outputs")]
    InvalidCoinbaseOutputs,
    #[error("Invalid reward amount")]
    InvalidReward,
}

/// Well-known genesis blocks for different networks
pub struct WellKnownGenesis;

impl WellKnownGenesis {
    /// Get mainnet genesis block hash
    pub fn mainnet_hash() -> Hash {
        Hash::from_hex("060fc7adbfa428aa9e222798cf26fdd83f4b30f2cb6c95a331b69d7d93c11f58ce69e7840395dcd52b4420ac4c07994817d3d14da8e81f1ecf3296ac2723d2d7")
            .expect("Invalid mainnet genesis hash")
    }

    /// Get testnet genesis block hash
    pub fn testnet_hash() -> Hash {
        Hash::from_hex("66f93816446e9aae8eebd6a26c4bc9b74f161c54871a59d4722d39baf194df3ec91605384f048c76c6524089ce7f2029e89557a0c482db56aa44aaf58028ad6c")
            .expect("Invalid testnet genesis hash")
    }

    /// Create mainnet genesis block
    pub fn mainnet_block() -> Block {
        let creator = GenesisCreator::new(GenesisConfig::mainnet());
        creator.create_genesis_block()
    }

    /// Create testnet genesis block
    pub fn testnet_block() -> Block {
        let creator = GenesisCreator::new(GenesisConfig::testnet());
        creator.create_genesis_block()
    }

    /// Create regtest genesis block
    pub fn regtest_block() -> Block {
        let creator = GenesisCreator::new(GenesisConfig::regtest());
        creator.create_genesis_block()
    }

    /// Validate a block is the correct genesis for the given network
    pub fn validate_network_genesis(network: &str, block: &Block) -> Result<(), GenesisError> {
        let config = match network {
            "mainnet" => GenesisConfig::mainnet(),
            "testnet" => GenesisConfig::testnet(),
            "regtest" => GenesisConfig::regtest(),
            _ => return Err(GenesisError::NotGenesisBlock),
        };

        let creator = GenesisCreator::new(config);
        creator.validate_genesis_block(block)
    }
}

/// Utility functions for genesis block operations
pub mod utils {
    use super::*;

    /// Create a minimal valid genesis block for testing
    pub fn create_test_genesis() -> Block {
        let config = GenesisConfig::custom(
            1234567890,
            "Test Genesis Block".to_string(),
            DifficultyTarget::minimum_for_network(crate::Network::Regtest),
            1_000_000_000, // 10 BTPC
            vec![0u8; 20],
            "test".to_string(),
        );

        let creator = GenesisCreator::new(config);
        creator.create_genesis_block()
    }

    /// Export genesis block to JSON
    pub fn export_genesis_json(block: &Block) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(block)
    }

    /// Import genesis block from JSON
    pub fn import_genesis_json(json: &str) -> Result<Block, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Calculate the size of a genesis block in bytes
    pub fn genesis_block_size(block: &Block) -> usize {
        // Approximate serialized size
        serde_json::to_vec(block).map(|v| v.len()).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genesis_config_creation() {
        let mainnet_config = GenesisConfig::mainnet();
        assert_eq!(mainnet_config.network, "mainnet");
        assert_eq!(mainnet_config.reward, 5_000_000_000);

        let testnet_config = GenesisConfig::testnet();
        assert_eq!(testnet_config.network, "testnet");
    }

    #[test]
    fn test_genesis_block_creation() {
        let config = GenesisConfig::testnet();
        let creator = GenesisCreator::new(config);
        let genesis_block = creator.create_genesis_block();

        // Basic checks
        assert_eq!(genesis_block.header.prev_hash, Hash::zero());
        assert_eq!(genesis_block.transactions.len(), 1);
        assert_eq!(genesis_block.transactions[0].inputs.len(), 1);
        assert_eq!(genesis_block.transactions[0].outputs.len(), 1);
    }

    #[test]
    fn test_coinbase_transaction() {
        let config = GenesisConfig::testnet();
        let creator = GenesisCreator::new(config.clone());
        let genesis_block = creator.create_genesis_block();
        let coinbase_tx = &genesis_block.transactions[0];

        // Check coinbase input
        assert_eq!(coinbase_tx.inputs[0].previous_output.txid, Hash::zero());
        assert_eq!(coinbase_tx.inputs[0].previous_output.vout, 0xFFFFFFFF);

        // Check output value
        assert_eq!(coinbase_tx.outputs[0].value, config.reward);
    }

    #[test]
    fn test_genesis_validation() {
        let config = GenesisConfig::testnet();
        let creator = GenesisCreator::new(config);
        let genesis_block = creator.create_genesis_block();

        // Should validate successfully (except for proof of work since not mined)
        match creator.validate_genesis_block(&genesis_block) {
            Err(GenesisError::InvalidProofOfWork) => {} // Expected
            Err(e) => panic!("Unexpected validation error: {}", e),
            Ok(_) => panic!("Expected proof of work validation to fail"),
        }
    }

    #[test]
    fn test_merkle_root_calculation() {
        let config = GenesisConfig::testnet();
        let creator = GenesisCreator::new(config);
        let genesis_block = creator.create_genesis_block();

        let calculated_merkle_root = GenesisCreator::calculate_merkle_root(
            &genesis_block.transactions.iter().collect::<Vec<_>>(),
        );

        assert_eq!(genesis_block.header.merkle_root, calculated_merkle_root);
    }

    #[test]
    fn test_well_known_genesis() {
        let mainnet_block = WellKnownGenesis::mainnet_block();
        assert_eq!(mainnet_block.header.prev_hash, Hash::zero());

        let testnet_block = WellKnownGenesis::testnet_block();
        assert_eq!(testnet_block.header.prev_hash, Hash::zero());
    }

    #[test]
    fn test_genesis_utils() {
        let test_genesis = utils::create_test_genesis();
        assert_eq!(test_genesis.transactions.len(), 1);

        let json = utils::export_genesis_json(&test_genesis).unwrap();
        assert!(!json.is_empty());

        let imported_genesis = utils::import_genesis_json(&json).unwrap();
        assert_eq!(test_genesis.hash(), imported_genesis.hash());

        let size = utils::genesis_block_size(&test_genesis);
        assert!(size > 0);
    }
}
