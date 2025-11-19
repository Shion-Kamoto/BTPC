//! Dynamic Fee Estimation Service (T017)
//!
//! Provides transaction fee estimation based on:
//! - Transaction size (inputs + outputs)
//! - Current network fee rate (from embedded node mempool)
//! - Fallback conservative estimate

use btpc_desktop_app::embedded_node::EmbeddedNode;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Fee estimation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeEstimate {
    /// Estimated fee in satoshis
    pub estimated_fee: u64,
    /// Estimated transaction size in bytes
    pub estimated_size: usize,
    /// Fee rate used (satoshis per byte)
    pub fee_rate: u64,
    /// Number of inputs
    pub inputs_count: usize,
    /// Number of outputs (including change)
    pub outputs_count: usize,
}

/// Fee estimation service
pub struct FeeEstimator {
    /// Embedded node for querying mempool fee rates
    /// Feature 013: Self-contained fee estimation (no external RPC)
    embedded_node: Arc<RwLock<EmbeddedNode>>,
}

impl FeeEstimator {
    /// Create a new fee estimator
    ///
    /// # Feature 013: Self-Contained App
    /// Uses embedded blockchain node instead of external RPC for fee estimation
    pub fn new(embedded_node: Arc<RwLock<EmbeddedNode>>) -> Self {
        Self { embedded_node }
    }

    /// Estimate transaction size in bytes based on inputs and outputs
    ///
    /// Formula (for ML-DSA signatures):
    /// - Base: 10 bytes (version + locktime + counts)
    /// - Input: ~4100 bytes (txid: 64, vout: 4, signature_script: ~4000 for ML-DSA-87, sequence: 4)
    /// - Output: ~40 bytes (value: 8, script_pubkey: ~32)
    ///
    /// This is conservative to account for ML-DSA-87's large signatures
    pub fn estimate_transaction_size(inputs: usize, outputs: usize) -> usize {
        const BASE_SIZE: usize = 10;
        const INPUT_SIZE: usize = 4100; // ML-DSA-87 signature is ~3309 bytes
        const OUTPUT_SIZE: usize = 40;

        BASE_SIZE + (inputs * INPUT_SIZE) + (outputs * OUTPUT_SIZE)
    }

    /// Get current network fee rate from embedded node mempool
    ///
    /// Returns fee rate in satoshis per byte based on mempool statistics.
    /// Falls back to conservative estimate (1000 sat/byte) if mempool is empty.
    ///
    /// # Feature 013: Self-Contained Fee Estimation
    /// Uses embedded blockchain node instead of external RPC
    pub async fn get_current_fee_rate(&self) -> Result<u64, String> {
        // Query embedded node mempool stats
        let node = self.embedded_node.read().await;
        match node.get_mempool_stats().await {
            Ok(stats) => {
                // Use median fee rate (50th percentile) from mempool
                // This represents the typical fee rate for recent transactions
                let median_rate = stats.fee_rate_p50_crd_per_byte as u64;

                // If mempool has transactions, use the median rate
                if stats.tx_count > 0 && median_rate > 0 {
                    // Sanity check: ensure reasonable bounds (10 sat/byte min, 10000 sat/byte max)
                    let bounded_rate = median_rate.clamp(10, 10000);

                    println!(
                        "✅ Mempool fee estimate: {} sat/byte ({} txs in mempool)",
                        bounded_rate, stats.tx_count
                    );
                    Ok(bounded_rate)
                } else {
                    // Mempool is empty - use fallback rate
                    println!("ℹ️  Mempool empty, using fallback fee rate");
                    Ok(Self::fallback_fee_rate())
                }
            }
            Err(e) => {
                println!("⚠️  Mempool query failed ({}), using fallback", e);
                Ok(Self::fallback_fee_rate())
            }
        }
    }

    /// Calculate fee from size and rate
    ///
    /// Formula: fee = size_bytes × fee_rate_per_byte
    pub fn calculate_fee(size: usize, rate: u64) -> u64 {
        (size as u64) * rate
    }

    /// Estimate fee for a complete transaction
    ///
    /// This is the high-level API that combines size estimation and rate queries.
    ///
    /// # Arguments
    /// * `inputs_count` - Number of inputs in the transaction
    /// * `outputs_count` - Number of outputs (recipient + change if needed)
    pub async fn estimate_fee_for_transaction(
        &self,
        inputs_count: usize,
        outputs_count: usize,
    ) -> Result<FeeEstimate, String> {
        // Step 1: Estimate transaction size
        let estimated_size = Self::estimate_transaction_size(inputs_count, outputs_count);

        // Step 2: Get current fee rate
        let fee_rate = self.get_current_fee_rate().await?;

        // Step 3: Calculate fee
        let estimated_fee = Self::calculate_fee(estimated_size, fee_rate);

        println!(
            "💰 Fee estimation: {} inputs, {} outputs = {} bytes × {} sat/byte = {} satoshis",
            inputs_count, outputs_count, estimated_size, fee_rate, estimated_fee
        );

        Ok(FeeEstimate {
            estimated_fee,
            estimated_size,
            fee_rate,
            inputs_count,
            outputs_count,
        })
    }

    /// Get fallback fee rate when RPC is unavailable
    ///
    /// Returns a conservative 1000 satoshis/byte to ensure transaction inclusion.
    /// This is higher than typical rates but prevents stuck transactions.
    ///
    /// For context:
    /// - Low priority: ~100 sat/byte
    /// - Medium priority: ~500 sat/byte
    /// - High priority: ~1000 sat/byte
    fn fallback_fee_rate() -> u64 {
        1000 // Conservative high-priority rate
    }

    /// Estimate minimum fee for given transaction parameters
    ///
    /// This provides the absolute minimum fee needed, useful for validation.
    #[allow(dead_code)]
    pub async fn estimate_minimum_fee(
        &self,
        inputs_count: usize,
        outputs_count: usize,
    ) -> Result<u64, String> {
        let size = Self::estimate_transaction_size(inputs_count, outputs_count);
        // Minimum rate is 1 sat/byte (dust relay fee)
        const MIN_RATE: u64 = 1;
        Ok(Self::calculate_fee(size, MIN_RATE))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_transaction_size() {
        // Single input, single output
        let size = FeeEstimator::estimate_transaction_size(1, 1);
        assert_eq!(size, 10 + 4100 + 40); // 4150 bytes

        // Two inputs, two outputs (with change)
        let size = FeeEstimator::estimate_transaction_size(2, 2);
        assert_eq!(size, 10 + 8200 + 80); // 8290 bytes

        // Multiple inputs
        let size = FeeEstimator::estimate_transaction_size(5, 2);
        assert_eq!(size, 10 + 20500 + 80); // 20590 bytes
    }

    #[test]
    fn test_calculate_fee() {
        // 4150 bytes × 100 sat/byte = 415000 satoshis
        let fee = FeeEstimator::calculate_fee(4150, 100);
        assert_eq!(fee, 415_000);

        // 8290 bytes × 500 sat/byte = 4,145,000 satoshis
        let fee = FeeEstimator::calculate_fee(8290, 500);
        assert_eq!(fee, 4_145_000);

        // Edge case: 1 byte × 1 sat/byte
        let fee = FeeEstimator::calculate_fee(1, 1);
        assert_eq!(fee, 1);
    }

    #[test]
    fn test_fallback_fee_rate() {
        let rate = FeeEstimator::fallback_fee_rate();
        assert_eq!(rate, 1000); // High priority conservative rate
    }

    #[tokio::test]
    async fn test_estimate_fee_for_transaction() {
        use tempfile::tempdir;

        // Create temporary embedded node for testing
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let node = EmbeddedNode::new(temp_dir.path().to_path_buf(), "regtest")
            .await
            .expect("Failed to create embedded node");

        let estimator = FeeEstimator::new(node);

        // Estimate for 1 input, 2 outputs
        let estimate = estimator.estimate_fee_for_transaction(1, 2).await;

        // Should succeed (uses fallback when mempool empty)
        assert!(estimate.is_ok());

        let estimate = estimate.unwrap();
        assert_eq!(estimate.inputs_count, 1);
        assert_eq!(estimate.outputs_count, 2);
        assert_eq!(estimate.estimated_size, 10 + 4100 + 80); // 4190 bytes
        assert!(estimate.estimated_fee > 0);
        assert!(estimate.fee_rate > 0);
    }

    #[tokio::test]
    async fn test_estimate_minimum_fee() {
        use tempfile::tempdir;

        // Create temporary embedded node for testing
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let node = EmbeddedNode::new(temp_dir.path().to_path_buf(), "regtest")
            .await
            .expect("Failed to create embedded node");

        let estimator = FeeEstimator::new(node);

        // Minimum fee for 1 input, 1 output
        let min_fee = estimator.estimate_minimum_fee(1, 1).await.unwrap();

        // Should be size × 1 sat/byte
        assert_eq!(min_fee, 4150); // 4150 bytes × 1 sat/byte
    }
}
