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
    /// Minimum fee per kilobyte (in credits)
    pub min_fee_per_kb: u64,
    /// Maximum transaction size in bytes
    pub max_transaction_size: usize,
}

impl Default for MempoolConfig {
    fn default() -> Self {
        MempoolConfig {
            max_transactions: 5000,        // 5000 transactions
            max_size_bytes: 300_000_000,   // 300 MB
            min_fee_per_kb: 1,             // 1 crd/KB minimum
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

        // Validate minimum fee (crd/KB)
        let fee_per_kb = fee as f64 / tx_size as f64 * 1024.0;
        if fee_per_kb < self.config.min_fee_per_kb as f64 {
            return Err(MempoolError::InsufficientFee {
                fee_per_kb,
                min_required: self.config.min_fee_per_kb as f64,
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
        let fee_per_byte = fee as f64 / tx_size as f64;
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
            .unwrap_or_else(|_| std::time::Duration::from_secs(0))
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

    /// Get mempool statistics with enhanced fee information
    pub fn stats(&self) -> MempoolStats {
        let fee_rates = self.get_sorted_fee_rates();
        let histogram = self.calculate_fee_histogram(&fee_rates);

        MempoolStats {
            transaction_count: self.transactions.len(),
            total_size_bytes: self.total_size,
            avg_fee_per_byte: self.calculate_avg_fee_per_byte(),
            min_fee_per_byte: fee_rates.first().copied().unwrap_or(0.0),
            max_fee_per_byte: fee_rates.last().copied().unwrap_or(0.0),
            fee_rate_p25_crd_per_byte: self.calculate_percentile(&fee_rates, 25),
            fee_rate_p50_crd_per_byte: self.calculate_percentile(&fee_rates, 50),
            fee_rate_p75_crd_per_byte: self.calculate_percentile(&fee_rates, 75),
            fee_histogram: histogram,
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

    /// Get fee rates sorted in ascending order
    fn get_sorted_fee_rates(&self) -> Vec<f64> {
        let mut rates: Vec<f64> = self
            .transactions
            .values()
            .map(|entry| entry.fee_per_byte)
            .collect();
        rates.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        rates
    }

    /// Calculate percentile from sorted fee rates
    fn calculate_percentile(&self, sorted_rates: &[f64], percentile: u8) -> f64 {
        if sorted_rates.is_empty() {
            return 0.0;
        }

        let index =
            ((percentile as f64 / 100.0) * (sorted_rates.len() - 1) as f64).round() as usize;
        sorted_rates[index.min(sorted_rates.len() - 1)]
    }

    /// Calculate fee histogram with 10 buckets
    fn calculate_fee_histogram(&self, sorted_rates: &[f64]) -> Vec<FeeHistogramBucket> {
        const NUM_BUCKETS: usize = 10;

        if sorted_rates.is_empty() {
            return Vec::new();
        }

        let min_rate = sorted_rates.first().copied().unwrap_or(0.0);
        let max_rate = sorted_rates.last().copied().unwrap_or(1.0);
        let range = (max_rate - min_rate).max(0.001); // Avoid division by zero
        let bucket_size = range / NUM_BUCKETS as f64;

        let mut buckets: Vec<FeeHistogramBucket> = (0..NUM_BUCKETS)
            .map(|i| {
                let bucket_min = min_rate + (i as f64 * bucket_size);
                let bucket_max = if i == NUM_BUCKETS - 1 {
                    max_rate + 0.001 // Include max value
                } else {
                    min_rate + ((i + 1) as f64 * bucket_size)
                };
                FeeHistogramBucket {
                    min_fee_rate: bucket_min,
                    max_fee_rate: bucket_max,
                    tx_count: 0,
                    total_size: 0,
                }
            })
            .collect();

        // Populate buckets
        for entry in self.transactions.values() {
            let bucket_index = if bucket_size > 0.0 {
                ((entry.fee_per_byte - min_rate) / bucket_size).floor() as usize
            } else {
                0
            };
            let bucket_index = bucket_index.min(NUM_BUCKETS - 1);

            buckets[bucket_index].tx_count += 1;
            buckets[bucket_index].total_size += entry.size;
        }

        buckets
    }

    /// Get recommended fee rate for a given priority level
    ///
    /// Returns the fee rate in credits per byte that should be used
    /// to achieve the desired confirmation priority.
    pub fn get_fee_rate_for_priority(&self, priority: FeePriority) -> f64 {
        let stats = self.stats();

        match priority {
            FeePriority::High => {
                // 75th percentile + 10% buffer for next-block confirmation
                let rate = stats.fee_rate_p75_crd_per_byte * 1.1;
                rate.max(self.config.min_fee_per_kb as f64)
            }
            FeePriority::Medium => {
                // 50th percentile (median) for 3-6 block confirmation
                let rate = stats.fee_rate_p50_crd_per_byte;
                rate.max(self.config.min_fee_per_kb as f64)
            }
            FeePriority::Low => {
                // 25th percentile for eventual confirmation
                let rate = stats.fee_rate_p25_crd_per_byte;
                rate.max(self.config.min_fee_per_kb as f64)
            }
        }
    }

    /// Estimate fee for a transaction of given size at specified priority
    ///
    /// # Arguments
    /// * `tx_size` - Transaction size in bytes
    /// * `priority` - Desired confirmation priority
    ///
    /// # Returns
    /// Recommended fee in credits
    pub fn estimate_fee(&self, tx_size: usize, priority: FeePriority) -> u64 {
        let rate = self.get_fee_rate_for_priority(priority);
        (tx_size as f64 * rate).ceil() as u64
    }
}

impl Default for Mempool {
    fn default() -> Self {
        Self::new()
    }
}

/// Transaction priority level for fee estimation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeePriority {
    /// High priority - confirms in next 1-2 blocks (75th percentile)
    High,
    /// Medium priority - confirms in next 3-6 blocks (50th percentile)
    Medium,
    /// Low priority - confirms eventually (25th percentile)
    Low,
}

/// Fee histogram bucket
#[derive(Debug, Clone)]
pub struct FeeHistogramBucket {
    /// Minimum fee rate for this bucket (crd/byte)
    pub min_fee_rate: f64,
    /// Maximum fee rate for this bucket (crd/byte)
    pub max_fee_rate: f64,
    /// Number of transactions in this bucket
    pub tx_count: usize,
    /// Total size of transactions in this bucket (bytes)
    pub total_size: usize,
}

/// Mempool statistics with enhanced fee information
#[derive(Debug, Clone)]
pub struct MempoolStats {
    /// Number of transactions
    pub transaction_count: usize,
    /// Total size in bytes
    pub total_size_bytes: usize,
    /// Average fee per byte
    pub avg_fee_per_byte: f64,
    /// Minimum fee per byte in mempool
    pub min_fee_per_byte: f64,
    /// Maximum fee per byte in mempool
    pub max_fee_per_byte: f64,
    /// 25th percentile fee rate (low priority)
    pub fee_rate_p25_crd_per_byte: f64,
    /// 50th percentile fee rate (medium priority / median)
    pub fee_rate_p50_crd_per_byte: f64,
    /// 75th percentile fee rate (high priority)
    pub fee_rate_p75_crd_per_byte: f64,
    /// Fee histogram (10 buckets)
    pub fee_histogram: Vec<FeeHistogramBucket>,
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
    #[error("Insufficient fee: {fee_per_kb} crd/KB < {min_required} crd/KB minimum")]
    InsufficientFee { fee_per_kb: f64, min_required: f64 },
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

        assert!(matches!(result, Err(MempoolError::DuplicateTransaction(_))));
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
        // New stats fields
        assert!(stats.min_fee_per_byte > 0.0);
        assert!(stats.max_fee_per_byte > 0.0);
        assert!(stats.fee_rate_p50_crd_per_byte > 0.0);
    }

    #[test]
    fn test_fee_percentiles() {
        let mut mempool = Mempool::new();

        // Add transactions with different fee rates
        // Test tx is ~118 bytes, so need at least 118 credits per tx for 1 crd/byte
        for i in 1..=10 {
            let (tx, _) = create_test_transaction(i * 100);
            let tx_size = tx.size();
            // Fee = i * tx_size to get i crd/byte fee rate
            mempool.add_transaction(tx, i * tx_size as u64).unwrap();
        }

        let stats = mempool.stats();
        assert_eq!(stats.transaction_count, 10);

        // Verify percentiles are ordered correctly
        assert!(stats.fee_rate_p25_crd_per_byte <= stats.fee_rate_p50_crd_per_byte);
        assert!(stats.fee_rate_p50_crd_per_byte <= stats.fee_rate_p75_crd_per_byte);
        assert!(stats.fee_rate_p25_crd_per_byte >= stats.min_fee_per_byte);
        assert!(stats.fee_rate_p75_crd_per_byte <= stats.max_fee_per_byte);
    }

    #[test]
    fn test_fee_histogram() {
        let mut mempool = Mempool::new();

        // Add transactions with varying fee rates
        // Test tx is ~118 bytes, so need at least 118 credits per tx for 1 crd/byte
        for i in 1..=20 {
            let (tx, _) = create_test_transaction(i * 50);
            let tx_size = tx.size();
            // Fee = i * tx_size to get i crd/byte fee rate
            mempool.add_transaction(tx, i * tx_size as u64).unwrap();
        }

        let stats = mempool.stats();

        // Histogram should have 10 buckets
        assert_eq!(stats.fee_histogram.len(), 10);

        // Total transactions in histogram should match mempool size
        let histogram_total: usize = stats.fee_histogram.iter().map(|b| b.tx_count).sum();
        assert_eq!(histogram_total, 20);

        // Each bucket should have ordered min/max rates
        for bucket in &stats.fee_histogram {
            assert!(bucket.min_fee_rate <= bucket.max_fee_rate);
        }
    }

    #[test]
    fn test_fee_priority_estimation() {
        let mut mempool = Mempool::new();

        // Add transactions with different fee rates
        // Test tx is ~118 bytes, so need at least 118 credits per tx for 1 crd/byte
        for i in 1..=100 {
            let (tx, _) = create_test_transaction(i * 10);
            let tx_size = tx.size();
            // Fee = i * tx_size to get i crd/byte fee rate
            mempool.add_transaction(tx, i * tx_size as u64).unwrap();
        }

        // Test priority levels
        let high_rate = mempool.get_fee_rate_for_priority(FeePriority::High);
        let medium_rate = mempool.get_fee_rate_for_priority(FeePriority::Medium);
        let low_rate = mempool.get_fee_rate_for_priority(FeePriority::Low);

        // Higher priority should have higher fee rate
        assert!(high_rate >= medium_rate);
        assert!(medium_rate >= low_rate);

        // All rates should be at least minimum
        let min_rate = mempool.config.min_fee_per_kb as f64;
        assert!(high_rate >= min_rate);
        assert!(medium_rate >= min_rate);
        assert!(low_rate >= min_rate);
    }

    #[test]
    fn test_estimate_fee() {
        let mut mempool = Mempool::new();

        // Add transactions with varying fee rates
        // Test tx is ~118 bytes, so need at least 118 credits per tx for 1 crd/byte
        for i in 1..=10 {
            let (tx, _) = create_test_transaction(i * 100);
            let tx_size = tx.size();
            // Fee = i * tx_size to get i crd/byte fee rate
            mempool.add_transaction(tx, i * tx_size as u64).unwrap();
        }

        let tx_size = 4000; // Typical ML-DSA transaction size

        // Estimate fees for different priorities
        let high_fee = mempool.estimate_fee(tx_size, FeePriority::High);
        let medium_fee = mempool.estimate_fee(tx_size, FeePriority::Medium);
        let low_fee = mempool.estimate_fee(tx_size, FeePriority::Low);

        // Higher priority should have higher fee
        assert!(high_fee >= medium_fee);
        assert!(medium_fee >= low_fee);

        // All fees should be positive
        assert!(high_fee > 0);
        assert!(medium_fee > 0);
        assert!(low_fee > 0);
    }

    #[test]
    fn test_empty_mempool_stats() {
        let mempool = Mempool::new();
        let stats = mempool.stats();

        assert_eq!(stats.transaction_count, 0);
        assert_eq!(stats.total_size_bytes, 0);
        assert_eq!(stats.avg_fee_per_byte, 0.0);
        assert_eq!(stats.min_fee_per_byte, 0.0);
        assert_eq!(stats.max_fee_per_byte, 0.0);
        assert_eq!(stats.fee_rate_p25_crd_per_byte, 0.0);
        assert_eq!(stats.fee_rate_p50_crd_per_byte, 0.0);
        assert_eq!(stats.fee_rate_p75_crd_per_byte, 0.0);
        assert!(stats.fee_histogram.is_empty());
    }

    #[test]
    fn test_empty_mempool_fee_priority() {
        let mempool = Mempool::new();

        // Empty mempool should return minimum fee rate
        let high_rate = mempool.get_fee_rate_for_priority(FeePriority::High);
        let medium_rate = mempool.get_fee_rate_for_priority(FeePriority::Medium);
        let low_rate = mempool.get_fee_rate_for_priority(FeePriority::Low);

        let min_rate = mempool.config.min_fee_per_kb as f64;
        assert_eq!(high_rate, min_rate);
        assert_eq!(medium_rate, min_rate);
        assert_eq!(low_rate, min_rate);
    }
}
