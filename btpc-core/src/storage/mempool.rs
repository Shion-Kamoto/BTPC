// Memory pool for pending transactions
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use anyhow::Result;
use thiserror::Error;

use crate::blockchain::Transaction;
use crate::crypto::Hash;

/// Memory pool for storing pending transactions
#[derive(Debug, Clone)]
pub struct Mempool {
    /// Transactions indexed by hash
    transactions: Arc<RwLock<HashMap<Hash, Transaction>>>,
    /// Maximum mempool size in bytes
    max_size: usize,
    /// Current size in bytes
    current_size: Arc<RwLock<usize>>,
}

impl Mempool {
    /// Create a new mempool with default size (100MB)
    pub fn new() -> Result<Self> {
        Self::with_max_size(100 * 1024 * 1024)
    }

    /// Create a new mempool with specified maximum size
    pub fn with_max_size(max_size: usize) -> Result<Self> {
        Ok(Mempool {
            transactions: Arc::new(RwLock::new(HashMap::new())),
            max_size,
            current_size: Arc::new(RwLock::new(0)),
        })
    }

    /// Add a transaction to the mempool
    pub fn add_transaction(&self, tx: Transaction) -> Result<(), MempoolError> {
        let tx_hash = tx.hash();
        let tx_size = tx.size();

        let mut transactions = self.transactions.write().unwrap();
        let mut current_size = self.current_size.write().unwrap();

        // Check if transaction already exists
        if transactions.contains_key(&tx_hash) {
            return Err(MempoolError::TransactionExists);
        }

        // Check if mempool is full
        if *current_size + tx_size > self.max_size {
            return Err(MempoolError::MempoolFull);
        }

        // Add transaction
        transactions.insert(tx_hash, tx);
        *current_size += tx_size;

        Ok(())
    }

    /// Get a transaction by hash
    pub fn get_transaction(&self, hash: &Hash) -> Option<Transaction> {
        let transactions = self.transactions.read().unwrap();
        transactions.get(hash).cloned()
    }

    /// Remove a transaction from the mempool
    pub fn remove_transaction(&self, hash: &Hash) -> Result<(), MempoolError> {
        let mut transactions = self.transactions.write().unwrap();
        let mut current_size = self.current_size.write().unwrap();

        if let Some(tx) = transactions.remove(hash) {
            *current_size = current_size.saturating_sub(tx.size());
            Ok(())
        } else {
            Err(MempoolError::TransactionNotFound)
        }
    }

    /// Remove multiple transactions (e.g., after block confirmation)
    pub fn remove_transactions(&self, hashes: &[Hash]) {
        let mut transactions = self.transactions.write().unwrap();
        let mut current_size = self.current_size.write().unwrap();

        for hash in hashes {
            if let Some(tx) = transactions.remove(hash) {
                *current_size = current_size.saturating_sub(tx.size());
            }
        }
    }

    /// Get all transactions in the mempool
    pub fn get_all_transactions(&self) -> Vec<Transaction> {
        let transactions = self.transactions.read().unwrap();
        transactions.values().cloned().collect()
    }

    /// Get transaction hashes for inventory announcements
    pub fn get_transaction_hashes(&self) -> Vec<Hash> {
        let transactions = self.transactions.read().unwrap();
        transactions.keys().cloned().collect()
    }

    /// Check if mempool contains a transaction
    pub fn contains(&self, hash: &Hash) -> bool {
        let transactions = self.transactions.read().unwrap();
        transactions.contains_key(hash)
    }

    /// Clear all transactions from the mempool
    pub fn clear(&self) {
        let mut transactions = self.transactions.write().unwrap();
        let mut current_size = self.current_size.write().unwrap();
        transactions.clear();
        *current_size = 0;
    }

    /// Get statistics about the mempool
    pub fn get_statistics(&self) -> MempoolStats {
        let transactions = self.transactions.read().unwrap();
        let current_size = self.current_size.read().unwrap();

        MempoolStats {
            transaction_count: transactions.len(),
            total_size: *current_size,
            total_fees: 0, // TODO: Calculate fees when fee system is implemented
        }
    }
}

impl Default for Mempool {
    fn default() -> Self {
        Self::new().expect("Failed to create mempool")
    }
}

/// Mempool statistics
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MempoolStats {
    pub transaction_count: usize,
    pub total_size: usize,
    pub total_fees: u64,
}

/// Mempool errors
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum MempoolError {
    #[error("Transaction already exists")]
    TransactionExists,
    #[error("Transaction not found")]
    TransactionNotFound,
    #[error("Invalid transaction")]
    InvalidTransaction,
    #[error("Mempool full")]
    MempoolFull,
}
