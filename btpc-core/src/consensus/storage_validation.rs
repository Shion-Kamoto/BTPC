//! Consensus validation integrated with storage layer
//!
//! This module provides validation logic that requires access to persistent storage,
//! including UTXO verification, block chain validation, and stateful consensus rules.

#![allow(unused_imports)]

use std::{collections::HashMap, sync::{Arc, RwLock}};

use anyhow::Result;
use thiserror::Error;

use crate::{
    blockchain::{Block, OutPoint, Transaction},
    consensus::{
        validation::{BlockValidator, TransactionValidator, ValidationError},
        DifficultyTarget,
    },
    crypto::Hash,
    storage::{BlockchainDatabase, StorageError, UTXODatabase},
};

/// Storage-aware block validator that can access blockchain history and UTXO set
pub struct StorageBlockValidator {
    /// Base block validator for stateless validation
    base_validator: BlockValidator,
    /// Blockchain database for accessing block history (with interior mutability)
    blockchain_db: Arc<RwLock<dyn BlockchainDatabase + Send + Sync>>,
    /// UTXO database for transaction input validation (with interior mutability)
    utxo_db: Arc<RwLock<dyn UTXODatabase + Send + Sync>>,
}

impl StorageBlockValidator {
    /// Create a new storage-aware block validator
    pub fn new(
        blockchain_db: Arc<RwLock<dyn BlockchainDatabase + Send + Sync>>,
        utxo_db: Arc<RwLock<dyn UTXODatabase + Send + Sync>>,
    ) -> Self {
        StorageBlockValidator {
            base_validator: BlockValidator::new(),
            blockchain_db,
            utxo_db,
        }
    }

    /// Validate a block with full context including storage lookups
    pub async fn validate_block_with_context(
        &self,
        block: &Block,
    ) -> Result<(), StorageValidationError> {
        // First run stateless validation
        self.base_validator
            .validate_block(block)
            .map_err(StorageValidationError::Validation)?;

        // Validate block context (previous block exists, difficulty is correct)
        self.validate_block_context(block).await?;

        // Validate all transactions in the block and calculate total fees
        let total_fees = self.validate_block_transactions(block).await?;

        // Validate coinbase transaction with fee information
        self.validate_coinbase_transaction(block, total_fees).await?;

        Ok(())
    }

    /// Validate block context (previous block, difficulty, timestamp)
    async fn validate_block_context(&self, block: &Block) -> Result<(), StorageValidationError> {
        // Check if previous block exists (unless genesis)
        if !block.header.prev_hash.is_zero() {
            let blockchain_db = self.blockchain_db.read()
                .map_err(|e| StorageValidationError::LockPoisoned(format!("blockchain_db read lock: {}", e)))?;
            let prev_block = blockchain_db.get_block(&block.header.prev_hash)?;

            let prev_block = match prev_block {
                Some(block) => block,
                None => return Err(StorageValidationError::PreviousBlockNotFound(
                    block.header.prev_hash,
                )),
            };

            drop(blockchain_db); // Release lock before additional storage queries

            // Validate timestamp with median-time-past (BIP 113)
            self.validate_timestamp_with_mtp(block).await?;

            // Validate difficulty adjustment
            self.validate_difficulty_adjustment(block, &prev_block)
                .await?;
        }

        Ok(())
    }

    /// Validate difficulty adjustment
    async fn validate_difficulty_adjustment(
        &self,
        block: &Block,
        prev_block: &Block,
    ) -> Result<(), StorageValidationError> {
        use crate::consensus::{
            constants::DIFFICULTY_ADJUSTMENT_INTERVAL,
            difficulty::DifficultyAdjustment,
        };

        // Get current block height
        let current_height = self.get_block_height(block).await?;

        // Check if this is a difficulty adjustment boundary
        if DifficultyAdjustment::is_adjustment_height(current_height) {
            // This is an adjustment block - validate the new difficulty
            self.validate_adjustment_block(block, current_height).await?;
        } else {
            // Not an adjustment block - difficulty must not change
            if block.header.bits != prev_block.header.bits {
                return Err(StorageValidationError::UnexpectedDifficultyChange {
                    height: current_height,
                    expected: prev_block.header.bits,
                    actual: block.header.bits,
                });
            }
        }

        Ok(())
    }

    /// Validate difficulty adjustment at adjustment boundary
    async fn validate_adjustment_block(
        &self,
        block: &Block,
        height: u32,
    ) -> Result<(), StorageValidationError> {
        use crate::consensus::{
            constants::DIFFICULTY_ADJUSTMENT_INTERVAL,
            difficulty::DifficultyAdjustment,
        };

        // Get the first block of the adjustment period (height - 2016)
        let period_start_height = height - DIFFICULTY_ADJUSTMENT_INTERVAL;

        // Walk backwards to find the first block of this period
        let blockchain_db = self.blockchain_db.read()
            .map_err(|e| StorageValidationError::LockPoisoned(format!("blockchain_db read lock: {}", e)))?;
        let mut current_hash = block.header.prev_hash;
        let mut blocks_back = 1; // Start from prev_block

        // Walk back to the start of the adjustment period
        while blocks_back < DIFFICULTY_ADJUSTMENT_INTERVAL {
            match blockchain_db.get_block(&current_hash)? {
                Some(prev) => {
                    current_hash = prev.header.prev_hash;
                    blocks_back += 1;
                }
                None => {
                    return Err(StorageValidationError::PreviousBlockNotFound(current_hash));
                }
            }
        }

        // Now current_hash points to the first block of the period
        let first_block = blockchain_db.get_block(&current_hash)?
            .ok_or(StorageValidationError::PreviousBlockNotFound(current_hash))?;

        // Get the last block of the previous period (parent of current block)
        let last_block = blockchain_db.get_block(&block.header.prev_hash)?
            .ok_or(StorageValidationError::PreviousBlockNotFound(block.header.prev_hash))?;

        drop(blockchain_db); // Release lock before calculation

        // Calculate actual timespan
        let actual_timespan = last_block.header.timestamp - first_block.header.timestamp;
        let target_timespan = DifficultyAdjustment::get_target_timespan();

        // Calculate expected difficulty
        let prev_target = DifficultyTarget::from_bits(last_block.header.bits);
        let expected_target = DifficultyAdjustment::adjust_difficulty(
            &prev_target,
            actual_timespan,
            target_timespan,
        );

        // Validate the actual difficulty matches expected (Issue #12: Use integer work)
        // Allow small differences due to representation precision
        let actual_target = DifficultyTarget::from_bits(block.header.bits);

        // Check if targets are approximately equal using deterministic integer comparison
        let expected_work = expected_target.work_integer();
        let actual_work = actual_target.work_integer();

        // Calculate difference as percentage (using integer math to avoid f64)
        // diff_pct = abs(expected - actual) * 100 / expected
        let work_diff = expected_work.abs_diff(actual_work);

        // Allow up to 1% difference for representation precision
        // Check if work_diff / expected_work > 0.01
        // Equivalent to: work_diff * 100 > expected_work
        if work_diff * 100 > expected_work {
            return Err(StorageValidationError::IncorrectDifficultyAdjustment {
                height,
                expected: expected_target.bits,
                actual: block.header.bits,
                actual_timespan,
                target_timespan,
            });
        }

        Ok(())
    }

    /// Validate all transactions in a block and return total fees
    async fn validate_block_transactions(
        &self,
        block: &Block,
    ) -> Result<u64, StorageValidationError> {
        let mut total_fees = 0u64;

        // Check for duplicate transactions across all blocks (Issue #15)
        let blockchain_db = self.blockchain_db.read()
            .map_err(|e| StorageValidationError::LockPoisoned(format!("blockchain_db read lock in validate_block_transactions: {}", e)))?;
        for transaction in &block.transactions {
            let txid = transaction.hash();
            if blockchain_db.has_transaction(&txid)? {
                return Err(StorageValidationError::DuplicateTransaction(txid));
            }
        }
        drop(blockchain_db); // Release lock before validation

        // Skip coinbase transaction (validated separately)
        for (i, transaction) in block.transactions.iter().enumerate().skip(1) {
            let fee = self.validate_transaction_with_utxos(transaction).await?;
            total_fees += fee;
        }

        // Return total fees for coinbase validation
        Ok(total_fees)
    }

    /// Validate a transaction against the UTXO set
    async fn validate_transaction_with_utxos(
        &self,
        transaction: &Transaction,
    ) -> Result<u64, StorageValidationError> {
        use crate::consensus::constants::COINBASE_MATURITY;

        let mut total_input_value = 0u64;
        let mut total_output_value = 0u64;

        // Get transaction data for signature verification
        let tx_data = transaction.serialize_for_signature();

        // Get current blockchain height for coinbase maturity check
        let blockchain_db = self.blockchain_db.read()
            .map_err(|e| StorageValidationError::LockPoisoned(format!("blockchain_db read lock in validate_transaction_with_utxos: {}", e)))?;
        let chain_tip = blockchain_db.get_chain_tip()?;
        let current_height = if let Some(tip) = chain_tip {
            // Calculate height of the tip block
            drop(blockchain_db); // Release lock before calling get_block_height
            self.get_block_height(&tip).await?
        } else {
            // No blocks yet (only genesis), current height is 0
            drop(blockchain_db);
            0
        };

        // Validate all inputs exist in UTXO set
        let utxo_db = self.utxo_db.read()
            .map_err(|e| StorageValidationError::LockPoisoned(format!("utxo_db read lock in validate_transaction_with_utxos: {}", e)))?;
        for (input_index, input) in transaction.inputs.iter().enumerate() {
            let utxo = utxo_db.get_utxo(&input.previous_output)?;

            let utxo = utxo.ok_or({
                StorageValidationError::UTXONotFound(input.previous_output)
            })?;

            // Check coinbase maturity (Bitcoin consensus rule)
            if utxo.is_coinbase {
                let confirmations = current_height - utxo.height;
                if confirmations < COINBASE_MATURITY {
                    return Err(StorageValidationError::ImmatureCoinbase {
                        created_height: utxo.height,
                        current_height,
                        required_confirmations: COINBASE_MATURITY,
                    });
                }
            }

            total_input_value += utxo.output.value;

            // Validate signature by executing combined scripts
            self.validate_input_signature(transaction, input_index, &utxo, &tx_data)?;
        }

        // Calculate total output value
        for output in &transaction.outputs {
            total_output_value += output.value;
        }

        // Validate input value >= output value
        if total_input_value < total_output_value {
            return Err(StorageValidationError::InsufficientInputValue {
                inputs: total_input_value,
                outputs: total_output_value,
            });
        }

        // Return fee (input - output)
        Ok(total_input_value - total_output_value)
    }

    /// Validate a transaction input's signature
    fn validate_input_signature(
        &self,
        transaction: &Transaction,
        input_index: usize,
        utxo: &crate::blockchain::UTXO,
        tx_data: &[u8],
    ) -> Result<(), StorageValidationError> {
        let input = &transaction.inputs[input_index];

        // Combine unlock script (script_sig) with lock script (script_pubkey)
        let mut combined_script = input.script_sig.clone();
        for op in utxo.output.script_pubkey.operations() {
            combined_script.push_op(op.clone());
        }

        // Create script execution context
        let context = crate::crypto::script::ScriptContext {
            transaction_data: tx_data.to_vec(),
            input_index,
        };

        // Execute combined script
        let result = combined_script
            .execute(&context)
            .map_err(|e| StorageValidationError::ScriptExecutionFailed(format!("{}", e)))?;

        if !result {
            return Err(StorageValidationError::SignatureVerificationFailed(
                input_index,
            ));
        }

        Ok(())
    }

    /// Validate coinbase transaction including transaction fees
    async fn validate_coinbase_transaction(
        &self,
        block: &Block,
        total_fees: u64,
    ) -> Result<(), StorageValidationError> {
        if block.transactions.is_empty() {
            return Err(StorageValidationError::NoCoinbaseTransaction);
        }

        let coinbase = &block.transactions[0];

        // Validate coinbase structure
        if coinbase.inputs.len() != 1 {
            return Err(StorageValidationError::InvalidCoinbaseInputs);
        }

        let coinbase_input = &coinbase.inputs[0];
        if coinbase_input.previous_output.txid != Hash::zero()
            || coinbase_input.previous_output.vout != 0xffffffff
        {
            return Err(StorageValidationError::InvalidCoinbaseInput);
        }

        // Validate coinbase reward
        let block_height = self.get_block_height(block).await?;
        let base_reward = crate::consensus::RewardCalculator::calculate_reward(block_height)
            .ok_or(StorageValidationError::InvalidBlockHeight(block_height))?;

        // Calculate total coinbase output value
        let total_coinbase_value: u64 = coinbase.outputs.iter().map(|o| o.value).sum();

        // Validate coinbase doesn't exceed base reward + transaction fees
        let max_allowed = base_reward + total_fees;
        if total_coinbase_value > max_allowed {
            return Err(StorageValidationError::ExcessiveCoinbaseReward {
                coinbase_value: total_coinbase_value,
                max_allowed,
            });
        }

        Ok(())
    }

    /// Get block height by traversing the chain
    async fn get_block_height(&self, block: &Block) -> Result<u32, StorageValidationError> {
        // Use an iterative approach to avoid recursion and boxing
        let mut current_hash = block.header.prev_hash;
        let mut height = 0u32;

        // If this is the genesis block, return 0
        if current_hash.is_zero() {
            return Ok(0);
        }

        // Walk backwards through the chain until we hit genesis
        let blockchain_db = self.blockchain_db.read()
            .map_err(|e| StorageValidationError::LockPoisoned(format!("blockchain_db read lock in get_block_height: {}", e)))?;
        loop {
            let prev_block = blockchain_db.get_block(&current_hash)?;

            if let Some(prev) = prev_block {
                height += 1;
                current_hash = prev.header.prev_hash;

                // If we reached genesis block
                if current_hash.is_zero() {
                    break;
                }
            } else {
                return Err(StorageValidationError::PreviousBlockNotFound(current_hash));
            }
        }

        Ok(height)
    }

    /// Get previous N blocks for MTP calculation
    /// Returns blocks in reverse chronological order (most recent first)
    async fn get_previous_blocks(
        &self,
        block: &Block,
        count: usize,
    ) -> Result<Vec<Block>, StorageValidationError> {
        let mut prev_blocks = Vec::new();
        let mut current_hash = block.header.prev_hash;

        // If genesis block, return empty
        if current_hash.is_zero() {
            return Ok(prev_blocks);
        }

        // Retrieve up to `count` previous blocks
        let blockchain_db = self.blockchain_db.read()
            .map_err(|e| StorageValidationError::LockPoisoned(format!("blockchain_db read lock in get_previous_blocks: {}", e)))?;
        for _ in 0..count {
            match blockchain_db.get_block(&current_hash)? {
                Some(prev_block) => {
                    current_hash = prev_block.header.prev_hash;
                    prev_blocks.push(prev_block);

                    // Stop at genesis
                    if current_hash.is_zero() {
                        break;
                    }
                }
                None => {
                    return Err(StorageValidationError::PreviousBlockNotFound(current_hash));
                }
            }
        }

        Ok(prev_blocks)
    }

    /// Calculate median-time-past from previous blocks (Bitcoin BIP 113)
    ///
    /// Returns the median timestamp of the provided blocks.
    /// This prevents time-warp attacks where miners manipulate timestamps
    /// to artificially lower difficulty.
    fn calculate_median_time_past(&self, prev_blocks: &[Block]) -> u64 {
        use crate::consensus::constants::MEDIAN_TIME_PAST_WINDOW;

        if prev_blocks.is_empty() {
            return 0;
        }

        // Take last N blocks (or all if fewer than N)
        let window_size = MEDIAN_TIME_PAST_WINDOW.min(prev_blocks.len());
        let mut timestamps: Vec<u64> = prev_blocks
            .iter()
            .take(window_size)
            .map(|b| b.header.timestamp)
            .collect();

        // Sort timestamps
        timestamps.sort_unstable();

        // Return median
        timestamps[timestamps.len() / 2]
    }

    /// Validate timestamp with median-time-past (Bitcoin BIP 113)
    ///
    /// This prevents time-warp attacks by ensuring block timestamps:
    /// 1. Are greater than the median of the last 11 blocks
    /// 2. Are not too far in the future (max 2 hours)
    /// 3. Meet minimum block time requirements (except Regtest)
    async fn validate_timestamp_with_mtp(&self, block: &Block) -> Result<(), StorageValidationError> {
        use crate::consensus::constants::{
            MAX_FUTURE_BLOCK_TIME, MIN_BLOCK_TIME, MEDIAN_TIME_PAST_WINDOW,
        };

        // Get current system time with fallback for clock issues
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_else(|_| std::time::Duration::from_secs(0))
            .as_secs();

        // Validate timestamp is not too far in the future
        if block.header.timestamp > current_time + MAX_FUTURE_BLOCK_TIME {
            return Err(StorageValidationError::TimestampTooFarInFuture {
                block_time: block.header.timestamp,
                current_time,
                max_future: MAX_FUTURE_BLOCK_TIME,
            });
        }

        // Get previous blocks for MTP calculation
        let prev_blocks = self
            .get_previous_blocks(block, MEDIAN_TIME_PAST_WINDOW)
            .await?;

        if !prev_blocks.is_empty() {
            // Calculate median-time-past
            let mtp = self.calculate_median_time_past(&prev_blocks);

            // Validate timestamp > MTP (BIP 113)
            if block.header.timestamp <= mtp {
                return Err(StorageValidationError::TimestampNotGreaterThanMTP {
                    block_time: block.header.timestamp,
                    mtp,
                });
            }

            // Enforce minimum block time (60 seconds) for Mainnet/Testnet
            // Regtest is exempted for rapid development/testing
            // Note: Network type would need to be passed in for full implementation
            // For now, we enforce this universally except where disabled
            let prev_timestamp = prev_blocks[0].header.timestamp;
            let time_since_prev = block.header.timestamp - prev_timestamp;

            if time_since_prev < MIN_BLOCK_TIME {
                // Note: In production, you'd check network type here
                // For now, we enforce MIN_BLOCK_TIME universally
                return Err(StorageValidationError::BlockMinedTooSoon {
                    time_since_prev,
                    min_time: MIN_BLOCK_TIME,
                });
            }
        }

        Ok(())
    }

    /// Apply block to storage (update UTXO set) with atomic double-spend prevention
    pub async fn apply_block(&self, block: &Block) -> Result<(), StorageValidationError> {
        // First validate the block (optimistic validation with read locks)
        self.validate_block_with_context(block).await?;

        // Get block height for UTXO tracking
        let block_height = self.get_block_height(block).await?;

        // CRITICAL SECTION: Acquire UTXO write lock for atomic application
        // This prevents race conditions where two threads try to spend the same UTXO
        {
            let mut utxo_db = self.utxo_db.write()
                .map_err(|e| StorageValidationError::LockPoisoned(format!("utxo_db write lock in apply_block: {}", e)))?;

            // Re-validate that all UTXOs still exist under write lock (double-spend prevention)
            // This is the "check-lock-check" pattern to prevent TOCTOU (Time-Of-Check-Time-Of-Use) attacks
            for transaction in &block.transactions {
                // Skip coinbase transaction
                if transaction.is_coinbase() {
                    continue;
                }

                // Verify all inputs still exist atomically
                for input in &transaction.inputs {
                    if input.previous_output.txid != Hash::zero() {
                        let utxo = utxo_db.get_utxo(&input.previous_output)?;
                        if utxo.is_none() {
                            // UTXO was spent by another block between validation and application
                            return Err(StorageValidationError::UTXONotFound(
                                input.previous_output,
                            ));
                        }
                    }
                }
            }

            // All UTXOs verified - now apply in single atomic batch (Issue #5: Race Conditions)
            let mut utxos_to_remove: Vec<OutPoint> = Vec::new();
            let mut utxos_to_add: Vec<crate::blockchain::UTXO> = Vec::new();

            for transaction in &block.transactions {
                let txid = transaction.hash();

                // Collect spent UTXOs for removal (skip coinbase inputs)
                for input in &transaction.inputs {
                    if input.previous_output.txid != Hash::zero() {
                        utxos_to_remove.push(input.previous_output);
                    }
                }

                // Collect new UTXOs for addition
                for (vout, output) in transaction.outputs.iter().enumerate() {
                    let outpoint = OutPoint {
                        txid,
                        vout: vout as u32,
                    };

                    let utxo = crate::blockchain::UTXO {
                        outpoint,
                        output: output.clone(),
                        height: block_height,
                        is_coinbase: transaction.is_coinbase(),
                    };

                    utxos_to_add.push(utxo);
                }
            }

            // Apply all UTXO changes atomically via RocksDB WriteBatch
            let remove_refs: Vec<&OutPoint> = utxos_to_remove.iter().collect();
            let add_refs: Vec<&crate::blockchain::UTXO> = utxos_to_add.iter().collect();
            utxo_db.apply_utxo_batch(&remove_refs, &add_refs)?;
            // UTXO write lock released here
        }

        // Store the block (requires write lock)
        let block_hash = block.hash();
        let mut blockchain_db = self.blockchain_db.write()
            .map_err(|e| StorageValidationError::LockPoisoned(format!("blockchain_db write lock in apply_block: {}", e)))?;
        blockchain_db.store_block(block)?;

        // Store transaction IDs for duplicate detection (Issue #15)
        for transaction in &block.transactions {
            let txid = transaction.hash();
            blockchain_db.store_transaction(&txid, &block_hash)?;
        }

        Ok(())
    }

    /// Apply a transaction to the UTXO set
    async fn apply_transaction(
        &self,
        transaction: &Transaction,
        block_height: u32,
    ) -> Result<(), StorageValidationError> {
        let txid = transaction.hash();

        // Acquire write lock for UTXO database
        let mut utxo_db = self.utxo_db.write()
            .map_err(|e| StorageValidationError::LockPoisoned(format!("utxo_db write lock in apply_transaction: {}", e)))?;

        // Remove spent UTXOs (skip coinbase inputs)
        for input in &transaction.inputs {
            if input.previous_output.txid != Hash::zero() {
                utxo_db.remove_utxo(&input.previous_output)?;
            }
        }

        // Add new UTXOs
        for (vout, output) in transaction.outputs.iter().enumerate() {
            let outpoint = OutPoint {
                txid,
                vout: vout as u32,
            };

            let utxo = crate::blockchain::UTXO {
                outpoint,
                output: output.clone(),
                height: block_height, // Fixed: Now using actual block height (Priority 5)
                is_coinbase: transaction.is_coinbase(),
            };

            utxo_db.store_utxo(&utxo)?;
        }

        Ok(())
    }

    /// Get current blockchain tip
    pub async fn get_chain_tip(&self) -> Result<Option<Block>, StorageValidationError> {
        let blockchain_db = self.blockchain_db.read()
            .map_err(|e| StorageValidationError::LockPoisoned(format!("blockchain_db read lock in get_chain_tip: {}", e)))?;
        Ok(blockchain_db.get_chain_tip()?)
    }

    /// Check if a block is already in the blockchain
    pub async fn has_block(&self, block_hash: &Hash) -> Result<bool, StorageValidationError> {
        let blockchain_db = self.blockchain_db.read()
            .map_err(|e| StorageValidationError::LockPoisoned(format!("blockchain_db read lock in has_block: {}", e)))?;
        let block = blockchain_db.get_block(block_hash)?;
        Ok(block.is_some())
    }
}

/// Storage-aware transaction validator
pub struct StorageTransactionValidator {
    /// Base transaction validator
    base_validator: TransactionValidator,
    /// UTXO database for input validation (with interior mutability)
    utxo_db: Arc<RwLock<dyn UTXODatabase + Send + Sync>>,
}

impl StorageTransactionValidator {
    /// Create a new storage-aware transaction validator
    pub fn new(utxo_db: Arc<RwLock<dyn UTXODatabase + Send + Sync>>) -> Self {
        StorageTransactionValidator {
            base_validator: TransactionValidator::new(),
            utxo_db,
        }
    }

    /// Validate a transaction with UTXO context
    pub async fn validate_transaction_with_context(
        &self,
        transaction: &Transaction,
    ) -> Result<(), StorageValidationError> {
        // First run stateless validation
        self.base_validator
            .validate_transaction(transaction)
            .map_err(StorageValidationError::Validation)?;

        // Validate against UTXO set
        self.validate_transaction_inputs(transaction).await?;

        Ok(())
    }

    /// Validate transaction inputs against UTXO set
    async fn validate_transaction_inputs(
        &self,
        transaction: &Transaction,
    ) -> Result<(), StorageValidationError> {
        let utxo_db = self.utxo_db.read()
            .map_err(|e| StorageValidationError::LockPoisoned(format!("utxo_db read lock in validate_transaction_inputs: {}", e)))?;

        // Get transaction data for signature verification
        let tx_data = transaction.serialize_for_signature();

        for (input_index, input) in transaction.inputs.iter().enumerate() {
            // Skip coinbase inputs
            if input.previous_output.txid == Hash::zero() {
                continue;
            }

            // Check if UTXO exists
            let utxo = utxo_db.get_utxo(&input.previous_output)?;

            if utxo.is_none() {
                return Err(StorageValidationError::UTXONotFound(
                    input.previous_output,
                ));
            }

            // Validate signature and script execution
            let utxo = utxo.unwrap();
            self.validate_input_signature(transaction, input_index, &utxo, &tx_data)?;
        }

        Ok(())
    }

    /// Validate a transaction input's signature
    fn validate_input_signature(
        &self,
        transaction: &Transaction,
        input_index: usize,
        utxo: &crate::blockchain::UTXO,
        tx_data: &[u8],
    ) -> Result<(), StorageValidationError> {
        let input = &transaction.inputs[input_index];

        // Combine unlock script (script_sig) with lock script (script_pubkey)
        let mut combined_script = input.script_sig.clone();
        for op in utxo.output.script_pubkey.operations() {
            combined_script.push_op(op.clone());
        }

        // Create script execution context
        let context = crate::crypto::script::ScriptContext {
            transaction_data: tx_data.to_vec(),
            input_index,
        };

        // Execute combined script
        let result = combined_script
            .execute(&context)
            .map_err(|e| StorageValidationError::ScriptExecutionFailed(format!("{}", e)))?;

        if !result {
            return Err(StorageValidationError::SignatureVerificationFailed(
                input_index,
            ));
        }

        Ok(())
    }
}

/// Error types for storage-aware validation
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum StorageValidationError {
    #[error("Base validation error: {0}")]
    Validation(ValidationError),
    #[error("Storage error: {0}")]
    Storage(StorageError),
    #[error("Previous block not found: {0}")]
    PreviousBlockNotFound(Hash),
    #[error("UTXO not found: {0:?}")]
    UTXONotFound(OutPoint),
    #[error("Duplicate transaction: {0}")]
    DuplicateTransaction(Hash),
    #[error("Invalid timestamp: block={block_time}, previous={prev_time}")]
    InvalidTimestamp { block_time: u32, prev_time: u32 },
    #[error("Block timestamp too far in future: {block_time} > {current_time} (max {max_future} seconds ahead)")]
    TimestampTooFarInFuture {
        block_time: u64,
        current_time: u64,
        max_future: u64,
    },
    #[error("Block timestamp {block_time} must be greater than median-time-past {mtp}")]
    TimestampNotGreaterThanMTP { block_time: u64, mtp: u64 },
    #[error("Block mined too soon: {time_since_prev} seconds < {min_time} second minimum")]
    BlockMinedTooSoon {
        time_since_prev: u64,
        min_time: u64,
    },
    #[error("Coinbase UTXO not mature: created at height {created_height}, current height {current_height}, requires {required_confirmations} confirmations")]
    ImmatureCoinbase {
        created_height: u32,
        current_height: u32,
        required_confirmations: u32,
    },
    #[error("Invalid difficulty adjustment: current=0x{current:08x}, previous=0x{previous:08x}")]
    InvalidDifficultyAdjustment { current: u32, previous: u32 },
    #[error("Unexpected difficulty change at height {height}: expected=0x{expected:08x}, actual=0x{actual:08x}")]
    UnexpectedDifficultyChange {
        height: u32,
        expected: u32,
        actual: u32,
    },
    #[error("Incorrect difficulty adjustment at height {height}: expected=0x{expected:08x}, actual=0x{actual:08x} (timespan: {actual_timespan}s vs target: {target_timespan}s)")]
    IncorrectDifficultyAdjustment {
        height: u32,
        expected: u32,
        actual: u32,
        actual_timespan: u64,
        target_timespan: u64,
    },
    #[error("Insufficient input value: inputs={inputs}, outputs={outputs}")]
    InsufficientInputValue { inputs: u64, outputs: u64 },
    #[error("No coinbase transaction")]
    NoCoinbaseTransaction,
    #[error("Invalid coinbase inputs")]
    InvalidCoinbaseInputs,
    #[error("Invalid coinbase input")]
    InvalidCoinbaseInput,
    #[error("Excessive coinbase reward: {coinbase_value} > {max_allowed}")]
    ExcessiveCoinbaseReward {
        coinbase_value: u64,
        max_allowed: u64,
    },
    #[error("Invalid block height: {0}")]
    InvalidBlockHeight(u32),
    #[error("Script execution failed: {0}")]
    ScriptExecutionFailed(String),
    #[error("Signature verification failed for input {0}")]
    SignatureVerificationFailed(usize),
    #[error("Lock poisoned: {0}")]
    LockPoisoned(String),
}

impl From<ValidationError> for StorageValidationError {
    fn from(err: ValidationError) -> Self {
        StorageValidationError::Validation(err)
    }
}

impl From<StorageError> for StorageValidationError {
    fn from(err: StorageError) -> Self {
        StorageValidationError::Storage(err)
    }
}

impl From<crate::storage::BlockchainDbError> for StorageValidationError {
    fn from(err: crate::storage::BlockchainDbError) -> Self {
        StorageValidationError::Storage(StorageError::BlockchainDb(err))
    }
}

impl From<crate::storage::UTXODbError> for StorageValidationError {
    fn from(err: crate::storage::UTXODbError) -> Self {
        StorageValidationError::Storage(StorageError::UTXODb(err))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tempfile::TempDir;

    use super::*;
    use crate::{
        blockchain::Block,
        storage::{
            database::{Database, DatabaseConfig},
            BlockchainDb, UtxoDb,
        },
    };

    async fn create_test_validator() -> (StorageBlockValidator, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_config = DatabaseConfig::test();
        let database = Arc::new(Database::open(temp_dir.path(), db_config).unwrap());

        let blockchain_db = Arc::new(RwLock::new(BlockchainDb::new(database.clone())))
            as Arc<RwLock<dyn BlockchainDatabase + Send + Sync>>;
        let utxo_db = Arc::new(RwLock::new(UtxoDb::new(database)))
            as Arc<RwLock<dyn UTXODatabase + Send + Sync>>;

        let validator = StorageBlockValidator::new(blockchain_db, utxo_db);
        (validator, temp_dir)
    }

    #[tokio::test]
    async fn test_storage_validator_creation() {
        let (validator, _temp_dir) = create_test_validator().await;

        // Test basic validation works
        let mut test_block = Block::create_test_block();

        // Mine the block to get valid PoW
        use crate::consensus::pow::{ProofOfWork, MiningTarget};
        let target = MiningTarget::easy_target();
        if let Ok(proof) = ProofOfWork::mine(&test_block.header, &target) {
            test_block.header.nonce = proof.nonce() as u32;
        }

        // Should pass stateless validation with valid PoW
        let result = validator.base_validator.validate_block(&test_block);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_genesis_block_validation() {
        let (validator, _temp_dir) = create_test_validator().await;

        // Create a genesis block
        let mut genesis = Block::create_test_block();
        genesis.header.prev_hash = Hash::zero();

        // Mine the block to get valid PoW
        use crate::consensus::pow::{ProofOfWork, MiningTarget};
        let target = MiningTarget::easy_target();
        if let Ok(proof) = ProofOfWork::mine(&genesis.header, &target) {
            genesis.header.nonce = proof.nonce() as u32;
        }

        // Genesis block should validate successfully
        let result = validator.validate_block_with_context(&genesis).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_transaction_validator_creation() {
        let temp_dir = TempDir::new().unwrap();
        let db_config = DatabaseConfig::test();
        let database = Arc::new(Database::open(temp_dir.path(), db_config).unwrap());
        let utxo_db = Arc::new(RwLock::new(UtxoDb::new(database)))
            as Arc<RwLock<dyn UTXODatabase + Send + Sync>>;

        let validator = StorageTransactionValidator::new(utxo_db);

        let test_tx = crate::blockchain::Transaction::create_test_transfer(1000, Hash::random());
        let result = validator.base_validator.validate_transaction(&test_tx);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_utxo_not_found_error() {
        let (validator, _temp_dir) = create_test_validator().await;

        // Create a transaction that references non-existent UTXO
        let mut test_tx =
            crate::blockchain::Transaction::create_test_transfer(1000, Hash::random());

        // Make it a non-coinbase transaction
        test_tx.inputs[0].previous_output.txid = Hash::random();
        test_tx.inputs[0].previous_output.vout = 0;

        let result = validator.validate_transaction_with_utxos(&test_tx).await;
        assert!(matches!(
            result,
            Err(StorageValidationError::UTXONotFound(_))
        ));
    }

    /// Test concurrent reads from blockchain database
    /// This verifies that multiple threads can read simultaneously with RwLock
    #[tokio::test]
    async fn test_concurrent_blockchain_reads() {
        let (validator, _temp_dir) = create_test_validator().await;

        // Store a genesis block first
        let mut genesis = Block::create_test_block();
        genesis.header.prev_hash = Hash::zero();

        // Mine the block
        use crate::consensus::pow::{ProofOfWork, MiningTarget};
        let target = MiningTarget::easy_target();
        if let Ok(proof) = ProofOfWork::mine(&genesis.header, &target) {
            genesis.header.nonce = proof.nonce() as u32;
        }

        // Store the genesis block
        let block_hash = genesis.hash();
        {
            let mut blockchain_db = validator.blockchain_db.write().unwrap();
            blockchain_db.store_block(&genesis).unwrap();
        }

        // Spawn 10 concurrent read tasks
        let mut handles = vec![];
        for _ in 0..10 {
            let blockchain_db = validator.blockchain_db.clone();
            let block_hash_clone = block_hash.clone();

            let handle = tokio::spawn(async move {
                // Each task reads the same block multiple times
                for _ in 0..5 {
                    let db = blockchain_db.read().unwrap();
                    let result = db.get_block(&block_hash_clone);
                    assert!(result.is_ok());
                    assert!(result.unwrap().is_some());
                }
            });

            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }
    }

    /// Test concurrent UTXO reads
    /// This verifies that multiple threads can read the UTXO set simultaneously
    #[tokio::test]
    async fn test_concurrent_utxo_reads() {
        let (validator, _temp_dir) = create_test_validator().await;

        // Create and store a test UTXO
        let test_outpoint = OutPoint {
            txid: Hash::random(),
            vout: 0,
        };

        let test_utxo = crate::blockchain::UTXO {
            outpoint: test_outpoint.clone(),
            output: crate::blockchain::TransactionOutput {
                value: 1000,
                script_pubkey: crate::crypto::Script::from_bytes(vec![1, 2, 3]),
            },
            height: 1,
            is_coinbase: false,
        };

        // Store the UTXO
        {
            let mut utxo_db = validator.utxo_db.write().unwrap();
            utxo_db.store_utxo(&test_utxo).unwrap();
        }

        // Spawn 10 concurrent read tasks
        let mut handles = vec![];
        for _ in 0..10 {
            let utxo_db = validator.utxo_db.clone();
            let outpoint_clone = test_outpoint.clone();

            let handle = tokio::spawn(async move {
                // Each task reads the same UTXO multiple times
                for _ in 0..5 {
                    let db = utxo_db.read().unwrap();
                    let result = db.get_utxo(&outpoint_clone);
                    assert!(result.is_ok());
                    assert!(result.unwrap().is_some());
                }
            });

            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }
    }

    /// Test read/write contention
    /// This verifies that writes block readers and vice versa
    #[tokio::test]
    async fn test_read_write_contention() {
        let (validator, _temp_dir) = create_test_validator().await;

        // Create multiple test UTXOs
        let mut utxos = vec![];
        for i in 0..5 {
            let outpoint = OutPoint {
                txid: Hash::random(),
                vout: i,
            };

            let utxo = crate::blockchain::UTXO {
                outpoint: outpoint.clone(),
                output: crate::blockchain::TransactionOutput {
                    value: 1000 + i as u64,
                    script_pubkey: crate::crypto::Script::from_bytes(vec![1, 2, 3]),
                },
                height: 1,
                is_coinbase: false,
            };

            utxos.push((outpoint, utxo));
        }

        // Spawn mixed read and write tasks
        let mut handles = vec![];

        // Writer tasks
        for (outpoint, utxo) in utxos.iter() {
            let utxo_db = validator.utxo_db.clone();
            let utxo_clone = utxo.clone();

            let handle = tokio::spawn(async move {
                let mut db = utxo_db.write().unwrap();
                db.store_utxo(&utxo_clone).unwrap();
            });

            handles.push(handle);
        }

        // Reader tasks (reading the first UTXO)
        let first_outpoint = utxos[0].0.clone();
        for _ in 0..5 {
            let utxo_db = validator.utxo_db.clone();
            let outpoint_clone = first_outpoint.clone();

            let handle = tokio::spawn(async move {
                let db = utxo_db.read().unwrap();
                let _ = db.get_utxo(&outpoint_clone);
                // May or may not find it depending on timing, just checking no deadlock
            });

            handles.push(handle);
        }

        // Wait for all tasks to complete (should not deadlock)
        for handle in handles {
            handle.await.unwrap();
        }
    }

    /// Test concurrent block applications
    /// This verifies that sequential block writes work correctly
    #[tokio::test]
    async fn test_concurrent_block_applications() {
        let (validator, _temp_dir) = create_test_validator().await;

        // Create and store genesis block
        let mut genesis = Block::create_test_block();
        genesis.header.prev_hash = Hash::zero();
        genesis.header.timestamp = 1000;

        use crate::consensus::pow::{ProofOfWork, MiningTarget};
        let target = MiningTarget::easy_target();
        if let Ok(proof) = ProofOfWork::mine(&genesis.header, &target) {
            genesis.header.nonce = proof.nonce() as u32;
        }

        // Store genesis
        {
            let mut blockchain_db = validator.blockchain_db.write().unwrap();
            blockchain_db.store_block(&genesis).unwrap();
        }

        // Create 3 blocks to apply sequentially
        let mut blocks = vec![];
        let mut prev_hash = genesis.hash();

        for i in 1..=3 {
            let mut block = Block::create_test_block();
            block.header.prev_hash = prev_hash.clone();
            block.header.timestamp = 1000 + (i * 100);

            if let Ok(proof) = ProofOfWork::mine(&block.header, &target) {
                block.header.nonce = proof.nonce() as u32;
            }

            prev_hash = block.hash();
            blocks.push(block);
        }

        // Apply blocks concurrently (they should serialize internally)
        let mut handles = vec![];
        for block in blocks {
            let blockchain_db = validator.blockchain_db.clone();

            let handle = tokio::spawn(async move {
                let mut db = blockchain_db.write().unwrap();
                let _ = db.store_block(&block);
            });

            handles.push(handle);
        }

        // Wait for all applications to complete
        for handle in handles {
            handle.await.unwrap();
        }

        // Verify we can read the chain tip
        let tip_result = validator.get_chain_tip().await;
        assert!(tip_result.is_ok());
    }

    /// Test that RwLock doesn't deadlock with recursive reads
    #[tokio::test]
    async fn test_no_recursive_deadlock() {
        let (validator, _temp_dir) = create_test_validator().await;

        // Create a genesis block
        let mut genesis = Block::create_test_block();
        genesis.header.prev_hash = Hash::zero();

        use crate::consensus::pow::{ProofOfWork, MiningTarget};
        let target = MiningTarget::easy_target();
        if let Ok(proof) = ProofOfWork::mine(&genesis.header, &target) {
            genesis.header.nonce = proof.nonce() as u32;
        }

        // Store the genesis
        {
            let mut blockchain_db = validator.blockchain_db.write().unwrap();
            blockchain_db.store_block(&genesis).unwrap();
        }

        // Multiple readers can coexist (RwLock allows this)
        let db1 = validator.blockchain_db.read().unwrap();
        let db2 = validator.blockchain_db.read().unwrap();

        let result1 = db1.get_chain_tip();
        let result2 = db2.get_chain_tip();

        assert!(result1.is_ok());
        assert!(result2.is_ok());

        // Both locks released here
    }

    /// Test valid signature acceptance
    /// Verifies that a properly signed transaction passes signature verification
    #[tokio::test]
    async fn test_valid_signature_acceptance() {
        use crate::crypto::{PrivateKey, Script};

        let (validator, _temp_dir) = create_test_validator().await;

        // Generate a keypair for the recipient
        let private_key = PrivateKey::generate_ml_dsa().unwrap();
        let public_key = private_key.public_key();
        let pubkey_hash = public_key.hash();

        // Create a UTXO locked to this public key
        let utxo_outpoint = OutPoint {
            txid: Hash::random(),
            vout: 0,
        };

        let utxo = crate::blockchain::UTXO {
            outpoint: utxo_outpoint.clone(),
            output: crate::blockchain::TransactionOutput {
                value: 5000,
                script_pubkey: Script::pay_to_pubkey_hash(pubkey_hash),
            },
            height: 1,
            is_coinbase: false,
        };

        // Store the UTXO
        {
            let mut utxo_db = validator.utxo_db.write().unwrap();
            utxo_db.store_utxo(&utxo).unwrap();
        }

        // Create a transaction spending this UTXO
        let mut transaction = Transaction {
            version: 1,
            inputs: vec![crate::blockchain::TransactionInput {
                previous_output: utxo_outpoint.clone(),
                script_sig: Script::new(), // Will be replaced with proper signature
                sequence: 0xffffffff,
            }],
            outputs: vec![crate::blockchain::TransactionOutput {
                value: 4500, // 500 sat fee
                script_pubkey: Script::pay_to_pubkey_hash(Hash::random()),
            }],
            lock_time: 0,
            fork_id: 0, // Test transaction (Issue #6)
        };

        // Sign the transaction
        let tx_data = transaction.serialize_for_signature();
        let signature = private_key.sign(&tx_data).unwrap();

        // Create unlock script with signature and public key
        transaction.inputs[0].script_sig =
            Script::unlock_p2pkh(&signature.to_bytes(), &public_key.to_bytes());

        // Validate the transaction - should succeed
        let result = validator.validate_transaction_with_utxos(&transaction).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 500); // Fee should be 500 sats
    }

    /// Test invalid signature rejection
    /// Verifies that a transaction with an invalid signature is rejected
    #[tokio::test]
    async fn test_invalid_signature_rejection() {
        use crate::crypto::{PrivateKey, Script, Signature};

        let (validator, _temp_dir) = create_test_validator().await;

        // Generate a keypair for the recipient
        let private_key = PrivateKey::generate_ml_dsa().unwrap();
        let public_key = private_key.public_key();
        let pubkey_hash = public_key.hash();

        // Create a UTXO
        let utxo_outpoint = OutPoint {
            txid: Hash::random(),
            vout: 0,
        };

        let utxo = crate::blockchain::UTXO {
            outpoint: utxo_outpoint.clone(),
            output: crate::blockchain::TransactionOutput {
                value: 5000,
                script_pubkey: Script::pay_to_pubkey_hash(pubkey_hash),
            },
            height: 1,
            is_coinbase: false,
        };

        // Store the UTXO
        {
            let mut utxo_db = validator.utxo_db.write().unwrap();
            utxo_db.store_utxo(&utxo).unwrap();
        }

        // Create a transaction
        let mut transaction = Transaction {
            version: 1,
            inputs: vec![crate::blockchain::TransactionInput {
                previous_output: utxo_outpoint.clone(),
                script_sig: Script::new(),
                sequence: 0xffffffff,
            }],
            outputs: vec![crate::blockchain::TransactionOutput {
                value: 4500,
                script_pubkey: Script::pay_to_pubkey_hash(Hash::random()),
            }],
            lock_time: 0,
            fork_id: 0, // Test transaction (Issue #6)
        };

        // Create a corrupted signature (all zeros)
        let invalid_signature = vec![0u8; 4627]; // ML-DSA signature size

        // Create unlock script with invalid signature
        transaction.inputs[0].script_sig =
            Script::unlock_p2pkh(&invalid_signature, &public_key.to_bytes());

        // Validate the transaction - should fail
        let result = validator.validate_transaction_with_utxos(&transaction).await;
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(StorageValidationError::ScriptExecutionFailed(_))
                | Err(StorageValidationError::SignatureVerificationFailed(_))
        ));
    }

    /// Test malformed signature handling
    /// Verifies that signatures with incorrect length are rejected
    #[tokio::test]
    async fn test_malformed_signature_handling() {
        use crate::crypto::{PrivateKey, Script};

        let (validator, _temp_dir) = create_test_validator().await;

        // Generate a keypair
        let private_key = PrivateKey::generate_ml_dsa().unwrap();
        let public_key = private_key.public_key();
        let pubkey_hash = public_key.hash();

        // Create a UTXO
        let utxo_outpoint = OutPoint {
            txid: Hash::random(),
            vout: 0,
        };

        let utxo = crate::blockchain::UTXO {
            outpoint: utxo_outpoint.clone(),
            output: crate::blockchain::TransactionOutput {
                value: 5000,
                script_pubkey: Script::pay_to_pubkey_hash(pubkey_hash),
            },
            height: 1,
            is_coinbase: false,
        };

        // Store the UTXO
        {
            let mut utxo_db = validator.utxo_db.write().unwrap();
            utxo_db.store_utxo(&utxo).unwrap();
        }

        // Create a transaction
        let mut transaction = Transaction {
            version: 1,
            inputs: vec![crate::blockchain::TransactionInput {
                previous_output: utxo_outpoint.clone(),
                script_sig: Script::new(),
                sequence: 0xffffffff,
            }],
            outputs: vec![crate::blockchain::TransactionOutput {
                value: 4500,
                script_pubkey: Script::pay_to_pubkey_hash(Hash::random()),
            }],
            lock_time: 0,
            fork_id: 0, // Test transaction (Issue #6)
        };

        // Create malformed signature (wrong length)
        let malformed_signature = vec![1u8; 100]; // Too short

        // Create unlock script with malformed signature
        transaction.inputs[0].script_sig =
            Script::unlock_p2pkh(&malformed_signature, &public_key.to_bytes());

        // Validate the transaction - should fail
        let result = validator.validate_transaction_with_utxos(&transaction).await;
        assert!(result.is_err());
    }

    /// Test signature for wrong transaction
    /// Verifies that a valid signature for a different transaction is rejected
    #[tokio::test]
    async fn test_signature_for_wrong_transaction() {
        use crate::crypto::{PrivateKey, Script};

        let (validator, _temp_dir) = create_test_validator().await;

        // Generate a keypair
        let private_key = PrivateKey::generate_ml_dsa().unwrap();
        let public_key = private_key.public_key();
        let pubkey_hash = public_key.hash();

        // Create a UTXO
        let utxo_outpoint = OutPoint {
            txid: Hash::random(),
            vout: 0,
        };

        let utxo = crate::blockchain::UTXO {
            outpoint: utxo_outpoint.clone(),
            output: crate::blockchain::TransactionOutput {
                value: 5000,
                script_pubkey: Script::pay_to_pubkey_hash(pubkey_hash),
            },
            height: 1,
            is_coinbase: false,
        };

        // Store the UTXO
        {
            let mut utxo_db = validator.utxo_db.write().unwrap();
            utxo_db.store_utxo(&utxo).unwrap();
        }

        // Create a transaction
        let mut transaction = Transaction {
            version: 1,
            inputs: vec![crate::blockchain::TransactionInput {
                previous_output: utxo_outpoint.clone(),
                script_sig: Script::new(),
                sequence: 0xffffffff,
            }],
            outputs: vec![crate::blockchain::TransactionOutput {
                value: 4500,
                script_pubkey: Script::pay_to_pubkey_hash(Hash::random()),
            }],
            lock_time: 0,
            fork_id: 0, // Test transaction (Issue #6)
        };

        // Sign DIFFERENT transaction data (wrong message)
        let wrong_data = b"This is the wrong transaction data";
        let signature = private_key.sign(wrong_data).unwrap();

        // Create unlock script with signature for wrong transaction
        transaction.inputs[0].script_sig =
            Script::unlock_p2pkh(&signature.to_bytes(), &public_key.to_bytes());

        // Validate the transaction - should fail
        let result = validator.validate_transaction_with_utxos(&transaction).await;
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(StorageValidationError::SignatureVerificationFailed(_))
        ));
    }

    /// Test missing signature detection
    /// Verifies that transactions with no signature are rejected
    #[tokio::test]
    async fn test_missing_signature_detection() {
        use crate::crypto::{PrivateKey, Script};

        let (validator, _temp_dir) = create_test_validator().await;

        // Generate a keypair
        let private_key = PrivateKey::generate_ml_dsa().unwrap();
        let public_key = private_key.public_key();
        let pubkey_hash = public_key.hash();

        // Create a UTXO
        let utxo_outpoint = OutPoint {
            txid: Hash::random(),
            vout: 0,
        };

        let utxo = crate::blockchain::UTXO {
            outpoint: utxo_outpoint.clone(),
            output: crate::blockchain::TransactionOutput {
                value: 5000,
                script_pubkey: Script::pay_to_pubkey_hash(pubkey_hash),
            },
            height: 1,
            is_coinbase: false,
        };

        // Store the UTXO
        {
            let mut utxo_db = validator.utxo_db.write().unwrap();
            utxo_db.store_utxo(&utxo).unwrap();
        }

        // Create a transaction with empty script_sig (no signature)
        let transaction = Transaction {
            version: 1,
            inputs: vec![crate::blockchain::TransactionInput {
                previous_output: utxo_outpoint.clone(),
                script_sig: Script::new(), // Empty script - no signature
                sequence: 0xffffffff,
            }],
            outputs: vec![crate::blockchain::TransactionOutput {
                value: 4500,
                script_pubkey: Script::pay_to_pubkey_hash(Hash::random()),
            }],
            lock_time: 0,
            fork_id: 0, // Test transaction (Issue #6)
        };

        // Validate the transaction - should fail
        let result = validator.validate_transaction_with_utxos(&transaction).await;
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(StorageValidationError::ScriptExecutionFailed(_))
        ));
    }
}
