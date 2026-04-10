//! Transaction Builder - Feature 007 (T019-T022)
//!
//! Builds valid BTPC transactions with:
//! - T019: TransactionBuilder struct with builder pattern
//! - T020: Dynamic fee calculation based on transaction size
//! - T021: Change output generation
//! - T022: Input/output validation (dust limits, address format)
//!
//! Usage:
//! ```rust,ignore
//! let tx = TransactionBuilder::new()
//!     .add_recipient("btpc1q...", 50_000_000)
//!     .select_utxos(&utxos)
//!     .set_fee_rate_per_kb(10)  // 10 crd/KB default
//!     .set_change_address("btpc1q...")
//!     .build()?;
//! ```

use anyhow::{anyhow, Result};
use btpc_core::crypto::Address;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::error::TransactionError;
use crate::utxo_manager::{Transaction, TxInput, TxOutput, UTXO};

/// Minimum output value to prevent dust (1000 credits / 0.00001 BTPC)
pub const DUST_LIMIT: u64 = 1000;

/// FIX 2026-02-23: Fee rates are now in credits per KILOBYTE (crd/KB).
/// ML-DSA-87 signatures are ~7300 bytes per input, making per-byte rates too coarse.
/// Per-KB granularity allows sub-1-crd/byte fees while keeping u64 integer math.
///
/// Default: 10 crd/KB → a 1-input TX (7,410 bytes) costs ~73 credits (0.00000073 BTPC)
/// During high congestion, mempool can push this to 1000+ crd/KB (≈1 crd/byte)
pub const DEFAULT_FEE_RATE_PER_KB: u64 = 10;

/// Maximum fee rate to prevent fee manipulation (10,000,000 crd/KB ≈ 10,000 crd/byte)
pub const MAX_FEE_RATE_PER_KB: u64 = 10_000_000;

/// Maximum total fee per transaction (1 BTPC = 100,000,000 credits)
pub const MAX_TOTAL_FEE: u64 = 100_000_000;

/// Estimated size per input (bytes) - ML-DSA-87 (Dilithium5) signature + pubkey
/// ML-DSA-87 signature: ~4595 bytes
/// ML-DSA-87 public key: ~2592 bytes
/// Previous output ref: ~36 bytes
/// Script length + sequence: ~5 bytes
/// Total: ~7228 bytes per input (padded to 7300)
pub const ESTIMATED_INPUT_SIZE: usize = 7300;

/// Estimated size per output (bytes)
/// Value: 8 bytes
/// Script pubkey length: 1 byte
/// Script pubkey: ~34 bytes (P2WPKH)
/// Total: ~43 bytes per output
pub const ESTIMATED_OUTPUT_SIZE: usize = 50;

/// Base transaction size (version, locktime, etc.)
pub const BASE_TX_SIZE: usize = 10;

/// T019: Transaction Builder for creating valid BTPC transactions
#[derive(Debug, Clone)]
pub struct TransactionBuilder {
    /// Inputs (UTXOs being spent)
    inputs: Vec<UTXO>,
    /// Outputs (recipients)
    outputs: Vec<(String, u64)>, // (address, amount)
    /// Fee rate in credits per kilobyte (crd/KB)
    fee_rate_per_kb: u64,
    /// Change address (where excess funds go)
    change_address: Option<String>,
    /// Transaction version
    version: u32,
    /// Lock time
    lock_time: u32,
    /// Fork ID for replay protection (0=mainnet, 1=testnet, 2=regtest)
    /// FIX 2025-12-03: Added network-aware fork_id instead of hardcoded value
    fork_id: u8,
}

impl TransactionBuilder {
    /// Create a new transaction builder
    ///
    /// Note: fork_id defaults to 2 (regtest) for backward compatibility.
    /// Call set_fork_id() to set the correct network value:
    /// - 0 = Mainnet
    /// - 1 = Testnet
    /// - 2 = Regtest
    pub fn new() -> Self {
        Self {
            inputs: Vec::new(),
            outputs: Vec::new(),
            fee_rate_per_kb: DEFAULT_FEE_RATE_PER_KB,
            change_address: None,
            version: 1,
            lock_time: 0,
            fork_id: 2, // Default to regtest for backward compatibility
        }
    }

    /// Set fork ID for replay protection
    ///
    /// FIX 2025-12-03: Critical for cross-network transaction validity
    /// - 0 = Mainnet
    /// - 1 = Testnet
    /// - 2 = Regtest
    pub fn set_fork_id(mut self, fork_id: u8) -> Self {
        self.fork_id = fork_id;
        self
    }

    /// Add a recipient to the transaction
    pub fn add_recipient(mut self, address: &str, amount: u64) -> Self {
        self.outputs.push((address.to_string(), amount));
        self
    }

    /// Select UTXOs to spend
    pub fn select_utxos(mut self, utxos: &[UTXO]) -> Self {
        self.inputs = utxos.to_vec();
        self
    }

    /// Set the fee rate (credits per kilobyte), clamped to MAX_FEE_RATE_PER_KB
    pub fn set_fee_rate_per_kb(mut self, rate: u64) -> Self {
        self.fee_rate_per_kb = rate.clamp(1, MAX_FEE_RATE_PER_KB);
        self
    }

    /// Set the change address
    pub fn set_change_address(mut self, address: &str) -> Self {
        self.change_address = Some(address.to_string());
        self
    }

    /// Set transaction version
    pub fn set_version(mut self, version: u32) -> Self {
        self.version = version;
        self
    }

    /// T020: Calculate transaction fee based on size
    ///
    /// Fee = (transaction_size_bytes * fee_rate_per_kb + 1023) / 1024
    /// Ceiling division ensures minimum 1 credit fee for any non-zero rate.
    pub fn calculate_fee(&self) -> Result<u64> {
        let estimated_size = self.estimate_transaction_size();
        // Ceiling division: (size * rate + 1023) / 1024
        let fee = ((estimated_size as u64)
            .saturating_mul(self.fee_rate_per_kb)
            + 1023)
            / 1024;

        let fee = fee.min(MAX_TOTAL_FEE);

        println!("📊 Fee calculation:");
        println!("  Estimated size: {} bytes", estimated_size);
        println!("  Fee rate: {} crd/KB", self.fee_rate_per_kb);
        println!("  Total fee: {} credits", fee);

        Ok(fee)
    }

    /// T020: Estimate transaction size in bytes
    fn estimate_transaction_size(&self) -> usize {
        let num_inputs = self.inputs.len();
        let num_outputs = self.outputs.len() + if self.needs_change() { 1 } else { 0 };

        BASE_TX_SIZE + (num_inputs * ESTIMATED_INPUT_SIZE) + (num_outputs * ESTIMATED_OUTPUT_SIZE)
    }

    /// Check if change output is needed (without recursion)
    fn needs_change(&self) -> bool {
        if let Ok(total_input) = self.calculate_total_input() {
            if let Ok(total_output) = self.calculate_total_output() {
                // Estimate fee without calling calculate_fee() to avoid recursion
                let num_inputs = self.inputs.len();
                let num_outputs = self.outputs.len() + 1; // +1 for potential change
                let estimated_size = BASE_TX_SIZE
                    + (num_inputs * ESTIMATED_INPUT_SIZE)
                    + (num_outputs * ESTIMATED_OUTPUT_SIZE);
                let estimated_fee = (estimated_size as u64 * self.fee_rate_per_kb + 1023) / 1024;

                let change = total_input.saturating_sub(total_output + estimated_fee);
                return change > DUST_LIMIT;
            }
        }
        false
    }

    /// Calculate total input value
    fn calculate_total_input(&self) -> Result<u64> {
        let total: u64 = self.inputs.iter().map(|utxo| utxo.value_credits).sum();
        Ok(total)
    }

    /// Calculate total output value
    fn calculate_total_output(&self) -> Result<u64> {
        let total: u64 = self.outputs.iter().map(|(_, amount)| *amount).sum();
        Ok(total)
    }

    /// T021: Generate change output if needed
    fn generate_change_output(&self) -> Result<Option<(String, u64)>> {
        let total_input = self.calculate_total_input()?;
        let total_output = self.calculate_total_output()?;
        let fee = self.calculate_fee()?;

        let change = total_input.saturating_sub(total_output + fee);

        if change > DUST_LIMIT {
            let change_addr = self
                .change_address
                .as_ref()
                .ok_or_else(|| anyhow!("Change address required but not set"))?;

            println!("💰 Change output:");
            println!("  Amount: {} credits", change);
            println!("  Address: {}", change_addr);

            Ok(Some((change_addr.clone(), change)))
        } else {
            Ok(None)
        }
    }

    /// T022: Validate transaction before building
    pub fn validate(&self) -> Result<(), TransactionError> {
        // Validate inputs
        if self.inputs.is_empty() {
            return Err(TransactionError::ValidationFailed {
                reason: "No inputs provided".to_string(),
            });
        }

        // Validate outputs
        if self.outputs.is_empty() {
            return Err(TransactionError::ValidationFailed {
                reason: "No outputs provided".to_string(),
            });
        }

        // T022: Validate each recipient address
        for (address, amount) in &self.outputs {
            // Validate address format
            if let Err(e) = Address::from_string(address) {
                return Err(TransactionError::InvalidAddress {
                    address: address.clone(),
                    reason: format!("Invalid address format: {}", e),
                });
            }

            // T022: Check for dust outputs
            if *amount < DUST_LIMIT {
                return Err(TransactionError::DustOutput {
                    amount: *amount,
                    dust_limit: DUST_LIMIT,
                });
            }
        }

        // Validate sufficient funds
        let total_input =
            self.calculate_total_input()
                .map_err(|e| TransactionError::ValidationFailed {
                    reason: format!("Failed to calculate input: {}", e),
                })?;

        let total_output =
            self.calculate_total_output()
                .map_err(|e| TransactionError::ValidationFailed {
                    reason: format!("Failed to calculate output: {}", e),
                })?;

        let fee = self
            .calculate_fee()
            .map_err(|e| TransactionError::ValidationFailed {
                reason: format!("Failed to calculate fee: {}", e),
            })?;

        if total_input < total_output + fee {
            return Err(TransactionError::InsufficientFunds {
                available: total_input,
                required: total_output,
                fee,
            });
        }

        // Validate change address if change is needed
        if self.needs_change() && self.change_address.is_none() {
            return Err(TransactionError::ValidationFailed {
                reason: "Change address required but not set".to_string(),
            });
        }

        Ok(())
    }

    /// Build the transaction
    ///
    /// Returns an unsigned transaction ready for signing.
    pub fn build(self) -> Result<Transaction, TransactionError> {
        // T022: Validate before building
        self.validate()?;

        // Create inputs
        let inputs: Vec<TxInput> = self
            .inputs
            .iter()
            .map(|utxo| TxInput {
                prev_txid: utxo.txid.clone(),
                prev_vout: utxo.vout,
                signature_script: Vec::new(), // Empty until signed
                sequence: 0xffffffff,
            })
            .collect();

        // Create outputs
        let mut tx_outputs: Vec<TxOutput> = Vec::new();

        for (address, amount) in &self.outputs {
            let addr =
                Address::from_string(address).map_err(|e| TransactionError::InvalidAddress {
                    address: address.clone(),
                    reason: format!("{}", e),
                })?;

            // Create proper P2PKH script (not just the hash)
            let script = btpc_core::crypto::Script::new_p2pkh(addr.hash160());
            tx_outputs.push(TxOutput {
                value: *amount,
                script_pubkey: script.serialize(),
            });
        }

        // T021: Add change output if needed
        if let Some((change_addr, change_amount)) = self.generate_change_output()? {
            let addr = Address::from_string(&change_addr).map_err(|e| {
                TransactionError::InvalidAddress {
                    address: change_addr.clone(),
                    reason: format!("{}", e),
                }
            })?;

            // Create proper P2PKH script for change output (not just the hash)
            let script = btpc_core::crypto::Script::new_p2pkh(addr.hash160());
            tx_outputs.push(TxOutput {
                value: change_amount,
                script_pubkey: script.serialize(),
            });
        }

        // Generate transaction ID
        let txid = format!("tx_{}", Utc::now().timestamp_nanos_opt().unwrap_or(0));

        let transaction = Transaction {
            txid,
            version: self.version,
            inputs,
            outputs: tx_outputs,
            lock_time: self.lock_time,
            fork_id: self.fork_id, // FIX 2025-12-03: Use network-aware fork_id
            block_height: None,
            confirmed_at: None,
            is_coinbase: false,
            // FIX 2025-12-10: sender_address set during create_transaction via TransactionState
            // TransactionBuilder doesn't know the sender address - it's set by the caller
            sender_address: None,
        };

        println!("✅ Transaction built:");
        println!("  TX ID: {}", transaction.txid);
        println!("  Fork ID: {} ({})", self.fork_id, match self.fork_id { 0 => "mainnet", 1 => "testnet", _ => "regtest" });
        println!("  Inputs: {}", transaction.inputs.len());
        println!("  Outputs: {}", transaction.outputs.len());

        Ok(transaction)
    }

    /// Get transaction summary before building
    pub fn summary(&self) -> Result<TransactionSummary> {
        let total_input = self.calculate_total_input()?;
        let total_output = self.calculate_total_output()?;
        let fee = self.calculate_fee()?;
        let change = if self.needs_change() {
            Some(total_input.saturating_sub(total_output + fee))
        } else {
            None
        };

        Ok(TransactionSummary {
            total_input,
            total_output,
            fee,
            change,
            num_inputs: self.inputs.len(),
            num_outputs: self.outputs.len() + if change.is_some() { 1 } else { 0 },
            estimated_size: self.estimate_transaction_size(),
        })
    }
}

impl Default for TransactionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary of transaction before building
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionSummary {
    pub total_input: u64,
    pub total_output: u64,
    pub fee: u64,
    pub change: Option<u64>,
    pub num_inputs: usize,
    pub num_outputs: usize,
    pub estimated_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use btpc_core::crypto::PrivateKey;
    use btpc_core::Network;
    use chrono::Utc;

    /// Generate a valid test BTPC address from a deterministic seed
    fn generate_test_address(seed_byte: u8) -> String {
        let seed = [seed_byte; 32];
        let private_key = PrivateKey::from_seed(&seed).expect("Failed to generate test key");
        let public_key = private_key.public_key();
        let address = Address::from_public_key(&public_key, Network::Regtest);
        address.to_string()
    }

    fn create_test_utxo(value: u64) -> UTXO {
        UTXO {
            txid: "test_tx".to_string(),
            vout: 0,
            value_credits: value,
            value_btp: value as f64 / 100_000_000.0,
            address: generate_test_address(1), // Use deterministic seed 1
            block_height: 1,
            is_coinbase: false,
            created_at: Utc::now(),
            spent: false,
            spent_in_tx: None,
            spent_at_height: None,
            script_pubkey: Vec::new(),
        }
    }

    #[test]
    fn test_transaction_builder_basic() {
        let utxo = create_test_utxo(100_000_000); // 1 BTPC

        let builder = TransactionBuilder::new()
            .add_recipient(&generate_test_address(2), 50_000_000) // Use deterministic seed 2
            .select_utxos(&[utxo])
            .set_change_address(&generate_test_address(3)); // Use deterministic seed 3

        let summary = builder.summary().unwrap();
        assert_eq!(summary.total_input, 100_000_000);
        assert_eq!(summary.total_output, 50_000_000);
        assert!(summary.fee > 0);
        assert!(summary.change.is_some());
    }

    #[test]
    fn test_fee_calculation() {
        let utxo = create_test_utxo(100_000_000);

        let builder = TransactionBuilder::new()
            .add_recipient(&generate_test_address(2), 50_000_000)
            .select_utxos(&[utxo])
            .set_fee_rate_per_kb(10); // 10 crd/KB

        let fee = builder.calculate_fee().unwrap();
        assert!(fee > 0);
        // 1 input + 2 outputs (recipient + change) = 10 + 7300 + 100 = 7410 bytes
        // Fee = (7410 * 10 + 1023) / 1024 = 73 credits
        println!("Calculated fee: {} credits", fee);
    }

    #[test]
    fn test_dust_output_validation() {
        let utxo = create_test_utxo(100_000_000);

        let builder = TransactionBuilder::new()
            .add_recipient(&generate_test_address(2), 500) // Below dust limit, use deterministic seed 2
            .select_utxos(&[utxo])
            .set_change_address(&generate_test_address(3)); // Use deterministic seed 3

        let result = builder.validate();
        assert!(result.is_err(), "Validation should fail for dust output");
        match result.unwrap_err() {
            TransactionError::DustOutput { amount, dust_limit } => {
                assert_eq!(amount, 500);
                assert_eq!(dust_limit, DUST_LIMIT);
            }
            other => panic!("Expected DustOutput error, got: {:?}", other),
        }
    }

    #[test]
    fn test_insufficient_funds() {
        let utxo = create_test_utxo(10_000_000); // 0.1 BTPC

        let builder = TransactionBuilder::new()
            .add_recipient(&generate_test_address(2), 50_000_000) // More than available, use deterministic seed 2
            .select_utxos(&[utxo])
            .set_change_address(&generate_test_address(3)); // Use deterministic seed 3

        let result = builder.validate();
        assert!(
            result.is_err(),
            "Validation should fail for insufficient funds"
        );
        match result.unwrap_err() {
            TransactionError::InsufficientFunds {
                available,
                required,
                fee,
            } => {
                assert_eq!(available, 10_000_000);
                assert_eq!(required, 50_000_000);
                assert!(fee > 0);
            }
            other => panic!("Expected InsufficientFunds error, got: {:?}", other),
        }
    }
}
