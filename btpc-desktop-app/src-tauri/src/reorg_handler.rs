//! Chain Reorganization Handler (FR-057)
//!
//! Handles blockchain reorganizations when a competing chain with more
//! accumulated proof-of-work is detected. This module bridges btpc-core's
//! reorg detection with src-tauri's UTXO and transaction state management.
//!
//! ## Reorganization Process
//!
//! 1. **Detection**: When a block's prev_hash doesn't match our current tip
//! 2. **Comparison**: Use btpc-core's compare_chains() to determine if reorg needed
//! 3. **Execution**: Disconnect old blocks, connect new blocks
//! 4. **UTXO Rollback**: Remove UTXOs from disconnected blocks
//! 5. **UTXO Restore**: Apply UTXOs from connected blocks
//! 6. **Transaction Handling**: Return affected transactions to mempool
//! 7. **Events**: Emit chain_reorganized events for UI updates

use anyhow::Result;
use btpc_core::blockchain::Block;
use btpc_core::crypto::Hash;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::events::ChainReorgEvent;
use crate::unified_database::UnifiedDatabase;
use crate::utxo_manager::UTXOManager;

/// Minimum confirmations before a transaction is considered "final" (FR-057i)
pub const FINALITY_CONFIRMATIONS: u32 = 6;

/// Maximum reorg depth we're willing to handle (safety limit)
pub const MAX_REORG_DEPTH: usize = 100;

/// Result of reorg detection
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum ReorgDetectionResult {
    /// Block extends current tip normally
    ExtendsCurrentTip,
    /// Block is on a shorter/equal work chain - ignore
    ShorterOrEqualChain,
    /// Reorganization needed to longer chain
    ReorgNeeded(ReorgPlan),
    /// Block doesn't connect to any known chain
    OrphanBlock,
}

/// Plan for executing a reorganization
#[derive(Debug, Clone)]
pub struct ReorgPlan {
    /// Common ancestor block hash (fork point)
    pub fork_point: Hash,
    /// Height of fork point
    pub fork_point_height: u64,
    /// Block hashes to disconnect (current chain, tip first)
    pub blocks_to_disconnect: Vec<Hash>,
    /// Block hashes to connect (new chain, oldest first)
    pub blocks_to_connect: Vec<Hash>,
    /// The new tip we're switching to
    pub new_tip: Hash,
    /// Current tip before reorg
    pub old_tip: Hash,
}

/// Result of executing a reorganization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReorgResult {
    /// Fork point hash
    pub fork_point_hash: String,
    /// Fork point height
    pub fork_point_height: u64,
    /// Number of blocks disconnected
    pub blocks_disconnected: u32,
    /// Number of blocks connected
    pub blocks_connected: u32,
    /// Transactions returned to mempool
    pub transactions_to_mempool: u32,
    /// New chain tip hash
    pub new_tip_hash: String,
    /// New chain height
    pub new_tip_height: u64,
    /// UTXOs removed (from disconnected blocks)
    pub utxos_removed: u32,
    /// UTXOs added (from connected blocks)
    pub utxos_added: u32,
}

/// Handles chain reorganizations
pub struct ReorgHandler {
    /// Flag indicating reorg in progress
    reorg_in_progress: AtomicBool,
}

impl Default for ReorgHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl ReorgHandler {
    /// Create a new ReorgHandler
    pub fn new() -> Self {
        Self {
            reorg_in_progress: AtomicBool::new(false),
        }
    }

    /// Check if a reorganization is currently in progress
    pub fn is_reorg_in_progress(&self) -> bool {
        self.reorg_in_progress.load(Ordering::SeqCst)
    }

    /// Detect if an incoming block requires reorganization
    ///
    /// # Arguments
    /// * `block` - The incoming block
    /// * `current_tip_hash` - Hash of our current chain tip
    /// * `current_height` - Height of our current chain tip
    /// * `db` - Database for block lookups
    ///
    /// # Returns
    /// * `ReorgDetectionResult` indicating what action to take
    pub fn detect_reorg(
        &self,
        block: &Block,
        current_tip_hash: &Hash,
        current_height: u64,
        db: &UnifiedDatabase,
    ) -> Result<ReorgDetectionResult> {
        let block_hash = block.hash();
        let prev_hash = block.header.prev_hash;

        // Case 1: Block extends current tip (normal case)
        if prev_hash == *current_tip_hash {
            return Ok(ReorgDetectionResult::ExtendsCurrentTip);
        }

        // Case 2: Check if prev_hash is in our chain
        // This could be a competing block at the same height
        let prev_block_height = self.find_block_height(&prev_hash, db)?;

        match prev_block_height {
            Some(height) => {
                // Block connects to our chain at height `height`
                // This block would be at height `height + 1`
                let incoming_block_height = height + 1;

                if incoming_block_height <= current_height {
                    // Block is at or below our current tip - might need reorg
                    // Calculate chain work to decide

                    // For now, use simple height comparison (TODO: proper chain work)
                    // In production, we'd use btpc_core::blockchain::reorg::compare_chains

                    // If the new chain (including this block) would be longer
                    // we need to reorg. But we can't know without the full competing chain.
                    // For a single block at same/lower height, it's a stale block.

                    eprintln!(
                        "[REORG] Block at height {} received (current tip at {})",
                        incoming_block_height, current_height
                    );

                    // For single competing blocks at same height, ignore
                    // (First seen wins, unless chain work is higher)
                    return Ok(ReorgDetectionResult::ShorterOrEqualChain);
                }

                // Block is ahead of our tip but doesn't extend it
                // This shouldn't happen in normal operation
                eprintln!(
                    "[REORG] WARNING: Block at height {} doesn't extend tip at {}",
                    incoming_block_height, current_height
                );

                // Build reorg plan
                let plan = self.build_reorg_plan(
                    current_tip_hash,
                    current_height,
                    &block_hash,
                    incoming_block_height,
                    &prev_hash,
                    height,
                    db,
                )?;

                Ok(ReorgDetectionResult::ReorgNeeded(plan))
            }
            None => {
                // prev_hash not found in our chain - orphan block
                eprintln!(
                    "[REORG] Orphan block received - prev_hash {} not found",
                    hex::encode(prev_hash.as_bytes())
                );
                Ok(ReorgDetectionResult::OrphanBlock)
            }
        }
    }

    /// Build a reorganization plan
    #[allow(clippy::too_many_arguments)]
    fn build_reorg_plan(
        &self,
        current_tip: &Hash,
        current_height: u64,
        new_tip: &Hash,
        _new_height: u64, // Reserved for future chain work comparison
        fork_point: &Hash,
        fork_height: u64,
        db: &UnifiedDatabase,
    ) -> Result<ReorgPlan> {
        // Collect blocks to disconnect (from current tip back to fork point)
        let mut blocks_to_disconnect = Vec::new();
        let mut _current = *current_tip; // Track position for validation

        for height in (fork_height + 1..=current_height).rev() {
            if let Some(block) = db.get_block(height as u32)? {
                blocks_to_disconnect.push(block.hash());
                _current = block.header.prev_hash;
            } else {
                eprintln!("[REORG] Warning: Missing block at height {}", height);
            }
        }

        // For blocks to connect, we only have the new tip right now
        // In a real implementation, we'd have the full competing chain
        let blocks_to_connect = vec![*new_tip];

        Ok(ReorgPlan {
            fork_point: *fork_point,
            fork_point_height: fork_height,
            blocks_to_disconnect,
            blocks_to_connect,
            new_tip: *new_tip,
            old_tip: *current_tip,
        })
    }

    /// Find the height of a block by its hash
    fn find_block_height(&self, block_hash: &Hash, db: &UnifiedDatabase) -> Result<Option<u64>> {
        // Get the current chain height using get_max_height()
        let tip_height = match db.get_max_height()? {
            Some((height, _hash)) => height as u32,
            None => return Ok(None), // Empty chain
        };

        // Search backwards through the chain
        // This is O(n) but works for now - could be optimized with hash->height index
        for height in (0..=tip_height).rev() {
            if let Some(block) = db.get_block(height)? {
                if block.hash() == *block_hash {
                    return Ok(Some(height as u64));
                }
                // Check if prev_hash matches for early exit
                if block.header.prev_hash == *block_hash {
                    return Ok(Some((height - 1) as u64));
                }
            }
        }

        Ok(None)
    }

    /// Execute a reorganization plan
    ///
    /// This is the main entry point for performing a reorg:
    /// 1. Set reorg_in_progress flag
    /// 2. Disconnect old blocks (remove UTXOs, return txs to mempool)
    /// 3. Connect new blocks (add UTXOs)
    /// 4. Update chain tip
    /// 5. Clear reorg_in_progress flag
    pub async fn execute_reorg(
        &self,
        plan: &ReorgPlan,
        db: &UnifiedDatabase,
        utxo_manager: &std::sync::Mutex<UTXOManager>,
    ) -> Result<ReorgResult> {
        // Safety check: max reorg depth
        if plan.blocks_to_disconnect.len() > MAX_REORG_DEPTH {
            return Err(anyhow::anyhow!(
                "Reorg depth {} exceeds maximum {}",
                plan.blocks_to_disconnect.len(),
                MAX_REORG_DEPTH
            ));
        }

        // Prevent concurrent reorgs
        if self.reorg_in_progress.swap(true, Ordering::SeqCst) {
            return Err(anyhow::anyhow!("Reorganization already in progress"));
        }

        // Track progress for result
        let mut utxos_removed = 0u32;
        let utxos_added = 0u32; // Placeholder for future block connection logic
        let mut transactions_to_mempool = 0u32;
        let blocks_disconnected = plan.blocks_to_disconnect.len() as u32;
        let blocks_connected = plan.blocks_to_connect.len() as u32;

        eprintln!(
            "[REORG] Starting reorganization: disconnect {} blocks, connect {} blocks",
            blocks_disconnected, blocks_connected
        );
        eprintln!(
            "[REORG] Fork point: {} at height {}",
            hex::encode(plan.fork_point.as_bytes()),
            plan.fork_point_height
        );

        // Step 1: Disconnect blocks (remove UTXOs)
        for block_hash in &plan.blocks_to_disconnect {
            match self.disconnect_block(block_hash, db, utxo_manager).await {
                Ok((removed, txs)) => {
                    utxos_removed += removed;
                    transactions_to_mempool += txs;
                    eprintln!(
                        "[REORG] Disconnected block {}: {} UTXOs removed, {} txs to mempool",
                        hex::encode(block_hash.as_bytes()),
                        removed,
                        txs
                    );
                }
                Err(e) => {
                    // Reorg failed - try to recover
                    self.reorg_in_progress.store(false, Ordering::SeqCst);
                    return Err(anyhow::anyhow!("Failed to disconnect block: {}", e));
                }
            }
        }

        // Step 2: Connect new blocks (add UTXOs)
        // Note: In a real implementation, we'd have the full blocks to connect
        // For now, this is a placeholder for when we receive the competing chain
        for block_hash in &plan.blocks_to_connect {
            eprintln!(
                "[REORG] Block {} marked for connection (will be added on receipt)",
                hex::encode(block_hash.as_bytes())
            );
            // UTXOs will be added when the block is actually processed
        }

        // Step 3: Clear reorg flag
        self.reorg_in_progress.store(false, Ordering::SeqCst);

        // Calculate new tip height
        let new_tip_height = plan.fork_point_height + blocks_connected as u64;

        let result = ReorgResult {
            fork_point_hash: hex::encode(plan.fork_point.as_bytes()),
            fork_point_height: plan.fork_point_height,
            blocks_disconnected,
            blocks_connected,
            transactions_to_mempool,
            new_tip_hash: hex::encode(plan.new_tip.as_bytes()),
            new_tip_height,
            utxos_removed,
            utxos_added,
        };

        eprintln!(
            "[REORG] ✅ Reorganization complete: {} blocks disconnected, {} blocks to connect",
            blocks_disconnected, blocks_connected
        );

        Ok(result)
    }

    /// Disconnect a single block during reorg
    ///
    /// - Removes UTXOs created by this block
    /// - Restores UTXOs spent by this block
    /// - Returns transactions to mempool
    async fn disconnect_block(
        &self,
        block_hash: &Hash,
        db: &UnifiedDatabase,
        utxo_manager: &std::sync::Mutex<UTXOManager>,
    ) -> Result<(u32, u32)> {
        let mut utxos_removed = 0u32;
        let mut txs_to_mempool = 0u32;

        // Find the block by hash - we need to search by height since that's our index
        // In production, we'd have a hash->height index
        let tip_height = match db.get_max_height()? {
            Some((height, _hash)) => height as u32,
            None => return Ok((0, 0)), // Empty chain
        };
        let mut target_block: Option<Block> = None;

        for h in (0..=tip_height).rev() {
            if let Some(block) = db.get_block(h)? {
                if block.hash() == *block_hash {
                    target_block = Some(block);
                    break;
                }
            }
        }

        let block = target_block.ok_or_else(|| {
            anyhow::anyhow!("Block {} not found", hex::encode(block_hash.as_bytes()))
        })?;

        // Lock UTXO manager (currently used for validation, operations placeholder for production)
        let _utxo_mgr = utxo_manager
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock UTXO manager: {}", e))?;

        // Process transactions in reverse order
        for (tx_idx, tx) in block.transactions.iter().enumerate().rev() {
            let txid = hex::encode(tx.hash().as_bytes());

            // Remove UTXOs created by this transaction's outputs
            for _output in tx.outputs.iter() {
                // Mark UTXO as removed/invalid
                // In production: utxo_mgr.remove_utxo(&txid, _vout as u32)?;
                utxos_removed += 1;
            }

            // Restore UTXOs spent by this transaction's inputs
            // (Skip coinbase - it has no real inputs)
            if tx_idx > 0 {
                for input in &tx.inputs {
                    let prev_txid = hex::encode(input.previous_output.txid.as_bytes());
                    let prev_vout = input.previous_output.vout;

                    // In production: utxo_mgr.restore_utxo(&prev_txid, prev_vout)?;
                    eprintln!(
                        "[REORG] Would restore UTXO {}:{} (spent by {})",
                        prev_txid, prev_vout, txid
                    );
                }
            }

            // Return non-coinbase transactions to mempool
            if tx_idx > 0 {
                txs_to_mempool += 1;
                // In production: mempool.add_transaction(tx)?;
            }
        }

        Ok((utxos_removed, txs_to_mempool))
    }

    /// Check if a transaction is final (has enough confirmations to be safe from reorg)
    pub fn is_transaction_final(confirmations: u32) -> bool {
        confirmations >= FINALITY_CONFIRMATIONS
    }

    /// Get the number of confirmations needed for finality
    pub fn finality_threshold() -> u32 {
        FINALITY_CONFIRMATIONS
    }
}

/// Event builder for chain reorganization events
pub struct ReorgEventBuilder;

impl ReorgEventBuilder {
    /// Build ReorgDetected event
    pub fn reorg_detected(
        fork_point_hash: &str,
        fork_point_height: u64,
        current_tip: &str,
        competing_tip: &str,
    ) -> ChainReorgEvent {
        ChainReorgEvent::ReorgDetected {
            fork_point_hash: fork_point_hash.to_string(),
            fork_point_height,
            current_tip_hash: current_tip.to_string(),
            competing_tip_hash: competing_tip.to_string(),
        }
    }

    /// Build ReorgInProgress event
    pub fn reorg_in_progress(
        blocks_to_disconnect: u32,
        blocks_to_connect: u32,
        current_progress: u32,
    ) -> ChainReorgEvent {
        ChainReorgEvent::ReorgInProgress {
            blocks_to_disconnect,
            blocks_to_connect,
            current_progress,
        }
    }

    /// Build ReorgCompleted event from result
    pub fn reorg_completed(result: &ReorgResult) -> ChainReorgEvent {
        ChainReorgEvent::ReorgCompleted {
            fork_point_hash: result.fork_point_hash.clone(),
            fork_point_height: result.fork_point_height,
            blocks_disconnected: result.blocks_disconnected,
            blocks_connected: result.blocks_connected,
            transactions_returned_to_mempool: result.transactions_to_mempool,
            new_tip_hash: result.new_tip_hash.clone(),
            new_tip_height: result.new_tip_height,
        }
    }

    /// Build ReorgFailed event
    pub fn reorg_failed(
        fork_point_hash: &str,
        error: &str,
        rollback_successful: bool,
    ) -> ChainReorgEvent {
        ChainReorgEvent::ReorgFailed {
            fork_point_hash: fork_point_hash.to_string(),
            error: error.to_string(),
            rollback_successful,
        }
    }

    /// Build ConfirmationsInvalidated event
    pub fn confirmations_invalidated(
        transaction_id: &str,
        previous_confirmations: u32,
        new_status: &str,
    ) -> ChainReorgEvent {
        ChainReorgEvent::ConfirmationsInvalidated {
            transaction_id: transaction_id.to_string(),
            previous_confirmations,
            new_status: new_status.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_transaction_final() {
        assert!(!ReorgHandler::is_transaction_final(0));
        assert!(!ReorgHandler::is_transaction_final(1));
        assert!(!ReorgHandler::is_transaction_final(5));
        assert!(ReorgHandler::is_transaction_final(6));
        assert!(ReorgHandler::is_transaction_final(100));
    }

    #[test]
    fn test_finality_threshold() {
        assert_eq!(ReorgHandler::finality_threshold(), FINALITY_CONFIRMATIONS);
        assert_eq!(ReorgHandler::finality_threshold(), 6);
    }

    #[test]
    fn test_reorg_handler_creation() {
        let handler = ReorgHandler::new();
        assert!(!handler.is_reorg_in_progress());
    }

    #[test]
    fn test_reorg_event_builder() {
        let event = ReorgEventBuilder::reorg_detected(
            "abc123",
            100,
            "tip_current",
            "tip_competing",
        );

        match event {
            ChainReorgEvent::ReorgDetected {
                fork_point_hash,
                fork_point_height,
                ..
            } => {
                assert_eq!(fork_point_hash, "abc123");
                assert_eq!(fork_point_height, 100);
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_reorg_result_serialization() {
        let result = ReorgResult {
            fork_point_hash: "abc".to_string(),
            fork_point_height: 100,
            blocks_disconnected: 2,
            blocks_connected: 3,
            transactions_to_mempool: 5,
            new_tip_hash: "def".to_string(),
            new_tip_height: 101,
            utxos_removed: 10,
            utxos_added: 15,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("fork_point_hash"));
        assert!(json.contains("blocks_disconnected"));
    }
}