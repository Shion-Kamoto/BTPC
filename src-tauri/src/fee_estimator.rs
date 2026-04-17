//! Dynamic Fee Estimation Service (T017)
//!
//! Provides transaction fee estimation based on:
//! - Transaction size (inputs + outputs)
//! - Current network fee rate (from embedded node mempool)
//! - Fallback conservative estimate
//!
//! FIX 2026-02-23: Fee rates are now in credits per KILOBYTE (crd/KB).
//! ML-DSA-87 signatures are ~7300 bytes per input, making per-byte rates too coarse.

use btpc_desktop_app::embedded_node::EmbeddedNode;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Fee estimation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeEstimate {
    /// Estimated fee in credits
    pub estimated_fee: u64,
    /// Estimated transaction size in bytes
    pub estimated_size: usize,
    /// Fee rate used (credits per kilobyte)
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
    /// Formula (for ML-DSA-87 / Dilithium5):
    /// - Base: 10 bytes (version + locktime + counts)
    /// - Input: ~7300 bytes (sig: ~4595, pubkey: ~2592, txid+vout: 36, overhead: ~77)
    /// - Output: ~40 bytes (value: 8, script_pubkey: ~32)
    pub fn estimate_transaction_size(inputs: usize, outputs: usize) -> usize {
        const BASE_SIZE: usize = 10;
        const INPUT_SIZE: usize = 7300; // ML-DSA-87: ~4595 sig + ~2592 pubkey + overhead
        const OUTPUT_SIZE: usize = 40;

        BASE_SIZE + (inputs * INPUT_SIZE) + (outputs * OUTPUT_SIZE)
    }

    /// Get current network fee rate from embedded node mempool
    ///
    /// Returns fee rate in crd/KB based on mempool statistics.
    /// Falls back to 10 crd/KB if mempool is empty.
    pub async fn get_current_fee_rate(&self) -> Result<u64, String> {
        // Query embedded node mempool stats
        let node = self.embedded_node.read().await;
        match node.get_mempool_stats().await {
            Ok(stats) => {
                if stats.tx_count > 0 && stats.fee_rate_p50_crd_per_byte as u64 > 0 {
                    // Convert mempool's per-byte rate to per-KB
                    let per_byte = stats.fee_rate_p50_crd_per_byte as u64;
                    let per_kb = per_byte.saturating_mul(1024).clamp(1, 10_000_000);

                    println!(
                        "✅ Mempool fee estimate: {} crd/KB ({} txs in mempool)",
                        per_kb, stats.tx_count
                    );
                    Ok(per_kb)
                } else {
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

    /// Calculate fee from size and rate (crd/KB)
    ///
    /// Formula: fee = (size_bytes × rate_per_kb + 1023) / 1024
    /// Uses ceiling division to ensure minimum 1 credit fee.
    pub fn calculate_fee(size: usize, rate_per_kb: u64) -> u64 {
        ((size as u64).saturating_mul(rate_per_kb) + 1023) / 1024
    }

    /// Estimate fee for a complete transaction
    ///
    /// This is the high-level API that combines size estimation and rate queries.
    pub async fn estimate_fee_for_transaction(
        &self,
        inputs_count: usize,
        outputs_count: usize,
    ) -> Result<FeeEstimate, String> {
        let estimated_size = Self::estimate_transaction_size(inputs_count, outputs_count);
        let fee_rate = self.get_current_fee_rate().await?;
        let estimated_fee = Self::calculate_fee(estimated_size, fee_rate);

        println!(
            "💰 Fee estimation: {} inputs, {} outputs = {} bytes × {} crd/KB = {} credits",
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

    /// Get fallback fee rate when mempool is empty (10 crd/KB)
    ///
    /// At 10 crd/KB, a typical 1-input 2-output TX (~7410 bytes) costs ~73 credits (0.00000073 BTPC)
    fn fallback_fee_rate() -> u64 {
        10 // 10 crd/KB — minimal fee
    }

    /// Estimate minimum fee for given transaction parameters
    #[allow(dead_code)]
    pub async fn estimate_minimum_fee(
        &self,
        inputs_count: usize,
        outputs_count: usize,
    ) -> Result<u64, String> {
        let size = Self::estimate_transaction_size(inputs_count, outputs_count);
        // Minimum rate: 1 crd/KB
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
        assert_eq!(size, 10 + 7300 + 40); // 7350 bytes

        // Two inputs, two outputs
        let size = FeeEstimator::estimate_transaction_size(2, 2);
        assert_eq!(size, 10 + 14600 + 80); // 14690 bytes

        // Multiple inputs
        let size = FeeEstimator::estimate_transaction_size(5, 2);
        assert_eq!(size, 10 + 36500 + 80); // 36590 bytes
    }

    #[test]
    fn test_calculate_fee() {
        // 7350 bytes × 10 crd/KB = (7350 * 10 + 1023) / 1024 = 72 credits
        let fee = FeeEstimator::calculate_fee(7350, 10);
        assert_eq!(fee, (7350u64 * 10 + 1023) / 1024);

        // 7410 bytes × 1000 crd/KB ≈ 7236 credits
        let fee = FeeEstimator::calculate_fee(7410, 1000);
        assert_eq!(fee, (7410u64 * 1000 + 1023) / 1024);

        // Edge case: 1 byte × 1 crd/KB = (1 + 1023) / 1024 = 1 credit (ceiling)
        let fee = FeeEstimator::calculate_fee(1, 1);
        assert_eq!(fee, 1);
    }

    #[test]
    fn test_fallback_fee_rate() {
        let rate = FeeEstimator::fallback_fee_rate();
        assert_eq!(rate, 10); // 10 crd/KB
    }

    #[tokio::test]
    async fn test_estimate_fee_for_transaction() {
        use crate::utxo_manager::UTXOManager;
        use std::sync::{Arc, Mutex};
        use tempfile::tempdir;

        let temp_dir = tempdir().expect("Failed to create temp dir");
        let utxo_manager =
            UTXOManager::new(temp_dir.path().to_path_buf()).expect("Failed to create UTXO manager");
        let utxo_manager_arc = Arc::new(Mutex::new(utxo_manager));
        let node = EmbeddedNode::new(
            temp_dir.path().to_path_buf(),
            "regtest",
            utxo_manager_arc.clone(),
        )
        .await
        .expect("Failed to create embedded node");

        let estimator = FeeEstimator::new(node);

        let estimate = estimator.estimate_fee_for_transaction(1, 2).await;
        assert!(estimate.is_ok());

        let estimate = estimate.unwrap();
        assert_eq!(estimate.inputs_count, 1);
        assert_eq!(estimate.outputs_count, 2);
        assert_eq!(estimate.estimated_size, 10 + 7300 + 80); // 7390 bytes
        assert!(estimate.estimated_fee > 0);
        assert!(estimate.fee_rate > 0);
    }

    #[tokio::test]
    async fn test_estimate_minimum_fee() {
        use crate::utxo_manager::UTXOManager;
        use std::sync::{Arc, Mutex};
        use tempfile::tempdir;

        let temp_dir = tempdir().expect("Failed to create temp dir");
        let utxo_manager =
            UTXOManager::new(temp_dir.path().to_path_buf()).expect("Failed to create UTXO manager");
        let utxo_manager_arc = Arc::new(Mutex::new(utxo_manager));
        let node = EmbeddedNode::new(
            temp_dir.path().to_path_buf(),
            "regtest",
            utxo_manager_arc.clone(),
        )
        .await
        .expect("Failed to create embedded node");

        let estimator = FeeEstimator::new(node);

        // Minimum fee for 1 input, 1 output
        let min_fee = estimator.estimate_minimum_fee(1, 1).await.unwrap();

        // 7350 bytes × 1 crd/KB = (7350 + 1023) / 1024 = 8 credits
        assert_eq!(min_fee, (7350u64 + 1023) / 1024);
    }
}
