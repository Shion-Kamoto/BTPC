//! Memory pool (mempool) for unconfirmed transactions (Issue #14)
//!
//! The mempool holds transactions that have been validated but not yet included
//! in a block. It provides:
//! - Transaction size and fee validation
//! - Memory limits to prevent DoS
//! - Double-spend detection
//! - Priority ordering by fee

use std::collections::{HashMap, HashSet};

use thiserror::Error;

use crate::{
    blockchain::{OutPoint, Transaction},
    crypto::Hash,
};

/// Memory pool configuration
#[derive(Debug, Clone)]
pub struct MempoolConfig {
    /// Maximum number of transactions in mempool
    pub max_transactions: usize,
    /// Maximum total size of all transactions in bytes
    pub max_size_bytes: usize,
    /// Minimum fee per byte (in smallest units)
    pub min_fee_per_byte: u64,
    /// Maximum transaction size in bytes
    pub max_transaction_size: usize,
}

impl Default for MempoolConfig {
    fn default() -> Self {
        MempoolConfig {
            max_transactions: 5000,        // 5000 transactions
            max_size_bytes: 300_000_000,   // 300 MB
            min_fee_per_byte: 1,           // 1 satoshi per byte minimum
            max_transaction_size: 100_000, // 100 KB per transaction
        }
    }
}

/// Transaction entry in mempool with metadata
#[derive(Debug, Clone)]
pub struct MempoolEntry {
    /// The transaction
    pub transaction: Transaction,
    /// Transaction ID (hash)
    pub txid: Hash,
    /// Transaction size in bytes
    pub size: usize,
    /// Transaction fee (input value - output value)
    pub fee: u64,
    /// Fee per byte
    pub fee_per_byte: f64,
    /// When transaction was added (Unix timestamp)
    pub added_time: u64,
}

/// Memory pool for unconfirmed transactions
pub struct Mempool {
    /// Configuration
    config: MempoolConfig,
    /// Transactions by ID
    transactions: HashMap<Hash, MempoolEntry>,
    /// UTXOs being spent (for double-spend detection)
    spent_outpoints: HashSet<OutPoint>,
    /// Current total size in bytes
    total_size: usize,
}

impl Mempool {
    /// Create new mempool with default configuration
    pub fn new() -> Self {
        Self::with_config(MempoolConfig::default())
    }

    /// Create new mempool with custom configuration
    pub fn with_config(config: MempoolConfig) -> Self {
        Mempool {
            config,
            transactions: HashMap::new(),
            spent_outpoints: HashSet::new(),
            total_size: 0,
        }
    }

    /// Add transaction to mempool with validation (Issue #14)
    ///
    /// Validates:
    /// - Transaction size limits
    /// - Minimum fee requirements
    /// - No double-spending (inputs not already in mempool)
    /// - Memory limits
    ///
    /// Returns the transaction ID if successful.
    pub fn add_transaction(
        &mut self,
        transaction: Transaction,
        fee: u64,
    ) -> Result<Hash, MempoolError> {
        // Calculate transaction size
        let tx_size = transaction.size();
        let txid = transaction.hash();

        // Check if already in mempool
        if self.transactions.contains_key(&txid) {
            return Err(MempoolError::DuplicateTransaction(txid));
        }

        // Validate transaction size (Issue #14: prevent oversized transactions)
        if tx_size > self.config.max_transaction_size {
            return Err(MempoolError::TransactionTooLarge {
                size: tx_size,
                max: self.config.max_transaction_size,
            });
        }

        // Check mempool capacity (Issue #14: memory limits)
        if self.transactions.len() >= self.config.max_transactions {
            return Err(MempoolError::MempoolFull {
                current: self.transactions.len(),
                max: self.config.max_transactions,
            });
        }

        // Check total size limit (Issue #14: prevent memory exhaustion)
        if self.total_size + tx_size > self.config.max_size_bytes {
            return Err(MempoolError::MempoolSizeLimitExceeded {
                current: self.total_size,
                additional: tx_size,
                max: self.config.max_size_bytes,
            });
        }

        // Validate minimum fee (Issue #14: fee requirements)
        let fee_per_byte = fee as f64 / tx_size as f64;
        if fee_per_byte < self.config.min_fee_per_byte as f64 {
            return Err(MempoolError::InsufficientFee {
                fee_per_byte,
                min_required: self.config.min_fee_per_byte as f64,
            });
        }

        // Check for double-spending (Issue #14: prevent conflicting transactions)
        for input in &transaction.inputs {
            if self.spent_outpoints.contains(&input.previous_output) {
                return Err(MempoolError::DoubleSpend(input.previous_output));
            }
        }

        // Validate basic transaction structure
        transaction
            .validate_structure()
            .map_err(|e| MempoolError::InvalidTransaction(format!("{}", e)))?;

        // Add transaction to mempool
        let entry = MempoolEntry {
            transaction: transaction.clone(),
            txid,
            size: tx_size,
            fee,
            fee_per_byte,
            added_time: Self::current_time(),
        };

        self.transactions.insert(txid, entry);

        // Mark UTXOs as spent
        for input in &transaction.inputs {
            self.spent_outpoints.insert(input.previous_output);
        }

        self.total_size += tx_size;

        Ok(txid)
    }

    /// Remove transaction from mempool (e.g., when included in block)
    pub fn remove_transaction(&mut self, txid: &Hash) -> Option<MempoolEntry> {
        if let Some(entry) = self.transactions.remove(txid) {
            // Remove spent outpoints
            for input in &entry.transaction.inputs {
                self.spent_outpoints.remove(&input.previous_output);
            }

            self.total_size -= entry.size;

            Some(entry)
        } else {
            None
        }
    }

    /// Get transaction by ID
    pub fn get_transaction(&self, txid: &Hash) -> Option<&MempoolEntry> {
        self.transactions.get(txid)
    }

    /// Check if transaction is in mempool
    pub fn contains(&self, txid: &Hash) -> bool {
        self.transactions.contains_key(txid)
    }

    /// Get number of transactions in mempool
    pub fn len(&self) -> usize {
        self.transactions.len()
    }

    /// Check if mempool is empty
    pub fn is_empty(&self) -> bool {
        self.transactions.is_empty()
    }

    /// Get total size of all transactions in bytes
    pub fn total_size(&self) -> usize {
        self.total_size
    }

    /// Get transactions ordered by fee (highest first) for block creation
    pub fn get_transactions_by_fee(&self, limit: usize) -> Vec<&MempoolEntry> {
        let mut entries: Vec<&MempoolEntry> = self.transactions.values().collect();

        // Sort by fee_per_byte descending (highest fee first)
        entries.sort_by(|a, b| {
            b.fee_per_byte
                .partial_cmp(&a.fee_per_byte)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        entries.into_iter().take(limit).collect()
    }

    /// Clear all transactions from mempool
    pub fn clear(&mut self) {
        self.transactions.clear();
        self.spent_outpoints.clear();
        self.total_size = 0;
    }

    /// Get current Unix timestamp
    fn current_time() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    /// Remove expired transactions (older than specified seconds)
    pub fn remove_expired(&mut self, max_age_seconds: u64) -> Vec<Hash> {
        let current_time = Self::current_time();
        let mut removed = Vec::new();

        // Find expired transactions
        let expired_txids: Vec<Hash> = self
            .transactions
            .iter()
            .filter(|(_, entry)| current_time - entry.added_time > max_age_seconds)
            .map(|(txid, _)| *txid)
            .collect();

        // Remove them
        for txid in expired_txids {
            if self.remove_transaction(&txid).is_some() {
                removed.push(txid);
            }
        }

        removed
    }

    /// Get mempool statistics
    pub fn stats(&self) -> MempoolStats {
        MempoolStats {
            transaction_count: self.transactions.len(),
            total_size_bytes: self.total_size,
            avg_fee_per_byte: self.calculate_avg_fee_per_byte(),
        }
    }

    /// Calculate average fee per byte across all transactions
    fn calculate_avg_fee_per_byte(&self) -> f64 {
        if self.transactions.is_empty() {
            return 0.0;
        }

        let total_fee_per_byte: f64 = self
            .transactions
            .values()
            .map(|entry| entry.fee_per_byte)
            .sum();

        total_fee_per_byte / self.transactions.len() as f64
    }
}

impl Default for Mempool {
    fn default() -> Self {
        Self::new()
    }
}

/// Mempool statistics
#[derive(Debug, Clone)]
pub struct MempoolStats {
    /// Number of transactions
    pub transaction_count: usize,
    /// Total size in bytes
    pub total_size_bytes: usize,
    /// Average fee per byte
    pub avg_fee_per_byte: f64,
}

/// Mempool errors
#[derive(Error, Debug, Clone)]
pub enum MempoolError {
    #[error("Duplicate transaction: {0}")]
    DuplicateTransaction(Hash),
    #[error("Transaction too large: {size} bytes exceeds max {max} bytes")]
    TransactionTooLarge { size: usize, max: usize },
    #[error("Mempool full: {current} transactions (max {max})")]
    MempoolFull { current: usize, max: usize },
    #[error("Mempool size limit exceeded: {current} + {additional} bytes > {max} bytes")]
    MempoolSizeLimitExceeded {
        current: usize,
        additional: usize,
        max: usize,
    },
    #[error("Insufficient fee: {fee_per_byte} sat/byte < {min_required} sat/byte minimum")]
    InsufficientFee {
        fee_per_byte: f64,
        min_required: f64,
    },
    #[error("Double spend detected: {0:?}")]
    DoubleSpend(OutPoint),
    #[error("Invalid transaction: {0}")]
    InvalidTransaction(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        blockchain::{Transaction, TransactionInput, TransactionOutput},
        crypto::Script,
    };

    fn create_test_transaction(fee: u64) -> (Transaction, u64) {
        let tx = Transaction::create_test_transfer(1000, Hash::random());
        (tx, fee)
    }

    #[test]
    fn test_mempool_creation() {
        let mempool = Mempool::new();
        assert_eq!(mempool.len(), 0);
        assert!(mempool.is_empty());
    }

    #[test]
    fn test_add_transaction() {
        let mut mempool = Mempool::new();
        let (tx, fee) = create_test_transaction(1000);

        let result = mempool.add_transaction(tx, fee);
        assert!(result.is_ok());
        assert_eq!(mempool.len(), 1);
    }

    #[test]
    fn test_duplicate_transaction_rejection() {
        let mut mempool = Mempool::new();
        let (tx, fee) = create_test_transaction(1000);

        mempool.add_transaction(tx.clone(), fee).unwrap();
        let result = mempool.add_transaction(tx, fee);

        assert!(matches!(
            result,
            Err(MempoolError::DuplicateTransaction(_))
        ));
    }

    #[test]
    fn test_insufficient_fee_rejection() {
        let mut mempool = Mempool::new();
        let (tx, _) = create_test_transaction(1000);

        // Try to add with zero fee (below minimum)
        let result = mempool.add_transaction(tx, 0);

        assert!(matches!(result, Err(MempoolError::InsufficientFee { .. })));
    }

    #[test]
    fn test_mempool_size_limit() {
        let config = MempoolConfig {
            max_transactions: 2,
            ..Default::default()
        };
        let mut mempool = Mempool::with_config(config);

        let (tx1, fee) = create_test_transaction(1000);
        let (tx2, fee) = create_test_transaction(1000);
        let (tx3, fee) = create_test_transaction(1000);

        mempool.add_transaction(tx1, fee).unwrap();
        mempool.add_transaction(tx2, fee).unwrap();

        let result = mempool.add_transaction(tx3, fee);
        assert!(matches!(result, Err(MempoolError::MempoolFull { .. })));
    }

    #[test]
    fn test_remove_transaction() {
        let mut mempool = Mempool::new();
        let (tx, fee) = create_test_transaction(1000);
        let txid = tx.hash();

        mempool.add_transaction(tx, fee).unwrap();
        assert_eq!(mempool.len(), 1);

        let removed = mempool.remove_transaction(&txid);
        assert!(removed.is_some());
        assert_eq!(mempool.len(), 0);
    }

    #[test]
    fn test_get_transactions_by_fee() {
        let mut mempool = Mempool::new();

        let (tx1, _) = create_test_transaction(1000);
        let (tx2, _) = create_test_transaction(2000);
        let (tx3, _) = create_test_transaction(500);

        mempool.add_transaction(tx1, 1000).unwrap();
        mempool.add_transaction(tx2, 2000).unwrap();
        mempool.add_transaction(tx3, 500).unwrap();

        let ordered = mempool.get_transactions_by_fee(3);
        assert_eq!(ordered.len(), 3);

        // Should be ordered by fee per byte (highest first)
        assert!(ordered[0].fee_per_byte >= ordered[1].fee_per_byte);
        assert!(ordered[1].fee_per_byte >= ordered[2].fee_per_byte);
    }

    #[test]
    fn test_mempool_stats() {
        let mut mempool = Mempool::new();
        let (tx, fee) = create_test_transaction(1000);

        mempool.add_transaction(tx, fee).unwrap();

        let stats = mempool.stats();
        assert_eq!(stats.transaction_count, 1);
        assert!(stats.total_size_bytes > 0);
        assert!(stats.avg_fee_per_byte > 0.0);
    }
}