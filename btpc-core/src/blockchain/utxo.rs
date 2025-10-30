//! UTXO (Unspent Transaction Output) management for BTPC
//!
//! This module provides efficient UTXO set management with fast lookups and validation.

use std::{
    collections::{HashMap, HashSet},
    fmt,
};

use serde::{Deserialize, Serialize};

use crate::blockchain::{
    constants::COINBASE_MATURITY,
    transaction::{OutPoint, TransactionOutput},
};

/// An unspent transaction output
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UTXO {
    /// Reference to the output
    pub outpoint: OutPoint,
    /// The transaction output data
    pub output: TransactionOutput,
    /// Block height where this UTXO was created
    pub height: u32,
    /// Whether this is a coinbase transaction
    pub is_coinbase: bool,
}

impl UTXO {
    /// Create a new UTXO
    pub fn new(
        outpoint: OutPoint,
        output: TransactionOutput,
        height: u32,
        is_coinbase: bool,
    ) -> Self {
        UTXO {
            outpoint,
            output,
            height,
            is_coinbase,
        }
    }

    /// Check if this UTXO can be spent at the given height
    pub fn can_spend_at_height(&self, current_height: u32) -> bool {
        if !self.is_coinbase {
            return true; // Regular UTXOs can be spent immediately
        }

        // Coinbase UTXOs need to mature
        current_height >= self.height + COINBASE_MATURITY
    }

    /// Get the value of this UTXO
    pub fn value(&self) -> u64 {
        self.output.value
    }

    /// Create a test UTXO for development
    #[cfg(test)]
    pub fn create_test_utxo(value: u64, is_coinbase: bool) -> Self {
        use crate::crypto::{Hash, Script};

        let outpoint = OutPoint::new(Hash::random(), 0);
        let output = TransactionOutput {
            value,
            script_pubkey: Script::pay_to_pubkey_hash(Hash::random()),
        };

        UTXO::new(outpoint, output, 100, is_coinbase)
    }

    /// Create a test UTXO with specific ID
    #[cfg(test)]
    pub fn create_test_utxo_with_id(id: u64, value: u64, is_coinbase: bool) -> Self {
        use crate::crypto::{Hash, Script};

        let outpoint = OutPoint::new(Hash::from_int(id), 0);
        let output = TransactionOutput {
            value,
            script_pubkey: Script::pay_to_pubkey_hash(Hash::from_int(id)),
        };

        UTXO::new(outpoint, output, 100, is_coinbase)
    }
}

/// UTXO set for tracking unspent outputs
#[derive(Debug, Clone)]
pub struct UTXOSet {
    /// Map from outpoint to UTXO
    utxos: HashMap<OutPoint, UTXO>,
    /// Track spent outputs for validation
    spent_outputs: HashSet<OutPoint>,
    /// Current block height for maturity checks
    current_height: u32,
}

impl UTXOSet {
    /// Create a new empty UTXO set
    pub fn new() -> Self {
        UTXOSet {
            utxos: HashMap::new(),
            spent_outputs: HashSet::new(),
            current_height: 0,
        }
    }

    /// Add a UTXO to the set
    pub fn add_utxo(&mut self, utxo: UTXO) -> Result<(), UTXOError> {
        if self.utxos.contains_key(&utxo.outpoint) {
            return Err(UTXOError::UTXOAlreadyExists);
        }

        if self.spent_outputs.contains(&utxo.outpoint) {
            return Err(UTXOError::UTXOAlreadySpent);
        }

        self.utxos.insert(utxo.outpoint, utxo);
        Ok(())
    }

    /// Get a UTXO by outpoint
    pub fn get_utxo(&self, outpoint: &OutPoint) -> Option<&UTXO> {
        self.utxos.get(outpoint)
    }

    /// Remove a UTXO when it's spent
    pub fn spend_utxo(&mut self, outpoint: &OutPoint) -> Result<UTXO, UTXOError> {
        let utxo = self.utxos.remove(outpoint).ok_or(UTXOError::UTXONotFound)?;

        self.spent_outputs.insert(*outpoint);
        Ok(utxo)
    }

    /// Check if a UTXO can be spent at current height
    pub fn can_spend_utxo(&self, outpoint: &OutPoint, current_height: u32) -> bool {
        if let Some(utxo) = self.get_utxo(outpoint) {
            utxo.can_spend_at_height(current_height)
        } else {
            false
        }
    }

    /// Update current height for maturity checks
    pub fn set_current_height(&mut self, height: u32) {
        self.current_height = height;
    }

    /// Get current height
    pub fn current_height(&self) -> u32 {
        self.current_height
    }

    /// Get total number of UTXOs
    pub fn len(&self) -> usize {
        self.utxos.len()
    }

    /// Check if UTXO set is empty
    pub fn is_empty(&self) -> bool {
        self.utxos.is_empty()
    }

    /// Get statistics about the UTXO set
    pub fn get_statistics(&self) -> UTXOStatistics {
        let mut total_value = 0u64;
        let mut coinbase_count = 0;
        let mut regular_count = 0;

        for utxo in self.utxos.values() {
            total_value += utxo.value();
            if utxo.is_coinbase {
                coinbase_count += 1;
            } else {
                regular_count += 1;
            }
        }

        UTXOStatistics {
            total_count: self.utxos.len(),
            total_value,
            coinbase_count,
            regular_count,
        }
    }

    /// Estimate memory usage in bytes
    pub fn memory_usage(&self) -> usize {
        // Rough estimate of memory usage
        let utxo_size = std::mem::size_of::<UTXO>() + std::mem::size_of::<OutPoint>();
        let spent_size = std::mem::size_of::<OutPoint>();

        (self.utxos.len() * utxo_size) + (self.spent_outputs.len() * spent_size)
    }

    /// Add multiple UTXOs in batch
    pub fn add_utxos_batch(&mut self, utxos: &[UTXO]) -> Result<(), UTXOError> {
        // Validate all UTXOs first
        for utxo in utxos {
            if self.utxos.contains_key(&utxo.outpoint) {
                return Err(UTXOError::UTXOAlreadyExists);
            }
            if self.spent_outputs.contains(&utxo.outpoint) {
                return Err(UTXOError::UTXOAlreadySpent);
            }
        }

        // Add all UTXOs
        for utxo in utxos {
            self.utxos.insert(utxo.outpoint, utxo.clone());
        }

        Ok(())
    }

    /// Spend multiple UTXOs in batch
    pub fn spend_utxos_batch(&mut self, outpoints: &[OutPoint]) -> Result<Vec<UTXO>, UTXOError> {
        let mut spent_utxos = Vec::new();

        // Validate all outpoints first
        for outpoint in outpoints {
            if !self.utxos.contains_key(outpoint) {
                return Err(UTXOError::UTXONotFound);
            }
        }

        // Spend all UTXOs
        for outpoint in outpoints {
            let utxo = self.utxos.remove(outpoint).unwrap();
            self.spent_outputs.insert(*outpoint);
            spent_utxos.push(utxo);
        }

        Ok(spent_utxos)
    }

    /// Create a checkpoint for rollback
    pub fn create_checkpoint(&self) -> UTXOCheckpoint {
        UTXOCheckpoint {
            utxos: self.utxos.clone(),
            spent_outputs: self.spent_outputs.clone(),
            current_height: self.current_height,
        }
    }

    /// Rollback to a checkpoint
    pub fn rollback_to_checkpoint(&mut self, checkpoint: UTXOCheckpoint) -> Result<(), UTXOError> {
        self.utxos = checkpoint.utxos;
        self.spent_outputs = checkpoint.spent_outputs;
        self.current_height = checkpoint.current_height;
        Ok(())
    }

    /// Apply a block to the UTXO set
    pub fn apply_block(&mut self, block: &crate::blockchain::Block) -> Result<(), UTXOError> {
        let block_height = self.current_height + 1;

        // Remove spent UTXOs from inputs
        for tx in &block.transactions {
            if !tx.is_coinbase() {
                for input in &tx.inputs {
                    self.spend_utxo(&input.previous_output)?;
                }
            }
        }

        // Add new UTXOs from outputs
        for tx in &block.transactions {
            let is_coinbase = tx.is_coinbase();
            for (vout, output) in tx.outputs.iter().enumerate() {
                let outpoint = OutPoint::new(tx.hash(), vout as u32);
                let utxo = UTXO::new(outpoint, output.clone(), block_height, is_coinbase);
                self.add_utxo(utxo)?;
            }
        }

        self.current_height = block_height;
        Ok(())
    }

    /// Apply genesis block to initialize UTXO set
    pub fn apply_genesis_block(
        &mut self,
        genesis: &crate::blockchain::Block,
    ) -> Result<(), UTXOError> {
        if !self.utxos.is_empty() {
            return Err(UTXOError::NotEmpty);
        }

        self.current_height = 0;
        self.apply_block(genesis)
    }

    /// Create test UTXO set for development
    #[cfg(test)]
    pub fn create_test_set() -> Self {
        let mut set = UTXOSet::new();

        // Add some test UTXOs
        for i in 0..10 {
            let utxo = UTXO::create_test_utxo_with_id(i, 1000000, i == 0);
            set.add_utxo(utxo).unwrap();
        }

        set
    }
}

impl Default for UTXOSet {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about the UTXO set
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UTXOStatistics {
    /// Total number of UTXOs
    pub total_count: usize,
    /// Total value of all UTXOs
    pub total_value: u64,
    /// Number of coinbase UTXOs
    pub coinbase_count: usize,
    /// Number of regular UTXOs
    pub regular_count: usize,
}

/// Checkpoint for UTXO set rollback
#[derive(Debug, Clone)]
pub struct UTXOCheckpoint {
    utxos: HashMap<OutPoint, UTXO>,
    spent_outputs: HashSet<OutPoint>,
    current_height: u32,
}

/// Error types for UTXO operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UTXOError {
    /// UTXO not found
    UTXONotFound,
    /// UTXO already exists
    UTXOAlreadyExists,
    /// UTXO already spent
    UTXOAlreadySpent,
    /// UTXO not mature (coinbase)
    UTXONotMature,
    /// UTXO set not empty when expected
    NotEmpty,
    /// Invalid block height
    InvalidHeight,
    /// Value overflow
    ValueOverflow,
}

impl fmt::Display for UTXOError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UTXOError::UTXONotFound => write!(f, "UTXO not found"),
            UTXOError::UTXOAlreadyExists => write!(f, "UTXO already exists"),
            UTXOError::UTXOAlreadySpent => write!(f, "UTXO already spent"),
            UTXOError::UTXONotMature => write!(f, "UTXO not mature (coinbase)"),
            UTXOError::NotEmpty => write!(f, "UTXO set not empty"),
            UTXOError::InvalidHeight => write!(f, "Invalid block height"),
            UTXOError::ValueOverflow => write!(f, "Value overflow"),
        }
    }
}

impl std::error::Error for UTXOError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{Hash, Script};

    #[test]
    fn test_utxo_creation() {
        let outpoint = OutPoint::new(Hash::from_int(12345), 0);
        let output = TransactionOutput {
            value: 1000000,
            script_pubkey: Script::pay_to_pubkey_hash(Hash::from_int(54321)),
        };

        let utxo = UTXO::new(outpoint, output.clone(), 100, false);

        assert_eq!(utxo.outpoint, outpoint);
        assert_eq!(utxo.output, output);
        assert_eq!(utxo.height, 100);
        assert!(!utxo.is_coinbase);
        assert_eq!(utxo.value(), 1000000);
    }

    #[test]
    fn test_coinbase_maturity() {
        let outpoint = OutPoint::new(Hash::from_int(12345), 0);
        let output = TransactionOutput {
            value: 3237500000,
            script_pubkey: Script::pay_to_pubkey_hash(Hash::from_int(54321)),
        };

        let coinbase_utxo = UTXO::new(outpoint, output.clone(), 100, true);
        let regular_utxo = UTXO::new(outpoint, output, 100, false);

        // Regular UTXO can be spent immediately
        assert!(regular_utxo.can_spend_at_height(100));
        assert!(regular_utxo.can_spend_at_height(101));

        // Coinbase UTXO needs to mature
        assert!(!coinbase_utxo.can_spend_at_height(100));
        assert!(!coinbase_utxo.can_spend_at_height(199)); // 99 blocks later
        assert!(coinbase_utxo.can_spend_at_height(200)); // 100 blocks later
    }

    #[test]
    fn test_utxo_set_operations() {
        let mut utxo_set = UTXOSet::new();

        let utxo = UTXO::create_test_utxo(1000000, false);
        let outpoint = utxo.outpoint;

        // Add UTXO
        assert!(utxo_set.add_utxo(utxo.clone()).is_ok());
        assert_eq!(utxo_set.len(), 1);

        // Get UTXO
        let retrieved = utxo_set.get_utxo(&outpoint);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), &utxo);

        // Can't add duplicate
        assert!(utxo_set.add_utxo(utxo.clone()).is_err());

        // Spend UTXO
        let spent = utxo_set.spend_utxo(&outpoint);
        assert!(spent.is_ok());
        assert_eq!(spent.unwrap(), utxo);
        assert_eq!(utxo_set.len(), 0);

        // Can't spend again
        assert!(utxo_set.spend_utxo(&outpoint).is_err());
    }

    #[test]
    fn test_utxo_set_statistics() {
        let mut utxo_set = UTXOSet::new();

        // Add various UTXOs
        utxo_set
            .add_utxo(UTXO::create_test_utxo(1000000, false))
            .unwrap();
        utxo_set
            .add_utxo(UTXO::create_test_utxo(2000000, false))
            .unwrap();
        utxo_set
            .add_utxo(UTXO::create_test_utxo(3237500000, true))
            .unwrap();

        let stats = utxo_set.get_statistics();

        assert_eq!(stats.total_count, 3);
        assert_eq!(stats.total_value, 3240500000);
        assert_eq!(stats.coinbase_count, 1);
        assert_eq!(stats.regular_count, 2);
    }

    #[test]
    fn test_batch_operations() {
        let mut utxo_set = UTXOSet::new();

        // Create batch of UTXOs
        let utxos = vec![
            UTXO::create_test_utxo_with_id(1, 1000000, false),
            UTXO::create_test_utxo_with_id(2, 2000000, false),
            UTXO::create_test_utxo_with_id(3, 3000000, false),
        ];

        // Batch add
        assert!(utxo_set.add_utxos_batch(&utxos).is_ok());
        assert_eq!(utxo_set.len(), 3);

        // Batch spend
        let outpoints: Vec<OutPoint> = utxos.iter().map(|u| u.outpoint).collect();
        let spent = utxo_set.spend_utxos_batch(&outpoints).unwrap();

        assert_eq!(spent.len(), 3);
        assert_eq!(utxo_set.len(), 0);
    }

    #[test]
    fn test_checkpoint_rollback() {
        let mut utxo_set = UTXOSet::new();

        // Add initial UTXO
        let utxo1 = UTXO::create_test_utxo_with_id(1, 1000000, false);
        utxo_set.add_utxo(utxo1.clone()).unwrap();

        // Create checkpoint
        let checkpoint = utxo_set.create_checkpoint();

        // Make changes
        let utxo2 = UTXO::create_test_utxo_with_id(2, 2000000, false);
        utxo_set.add_utxo(utxo2).unwrap();
        utxo_set.spend_utxo(&utxo1.outpoint).unwrap();

        assert_eq!(utxo_set.len(), 1);

        // Rollback
        utxo_set.rollback_to_checkpoint(checkpoint).unwrap();

        assert_eq!(utxo_set.len(), 1);
        assert!(utxo_set.get_utxo(&utxo1.outpoint).is_some());
    }

    #[test]
    fn test_memory_usage_estimation() {
        let mut utxo_set = UTXOSet::new();

        let initial_usage = utxo_set.memory_usage();

        // Add UTXOs and check memory growth
        for i in 0..100 {
            let utxo = UTXO::create_test_utxo_with_id(i, 1000000, false);
            utxo_set.add_utxo(utxo).unwrap();
        }

        let usage_with_100 = utxo_set.memory_usage();
        assert!(usage_with_100 > initial_usage);

        // Memory usage should be reasonable
        assert!(usage_with_100 < 1024 * 1024); // Less than 1MB for 100 UTXOs
    }
}
