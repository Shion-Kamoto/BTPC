//! Transaction Builder - Feature 007 (T019-T022)
//!
//! Builds valid BTPC transactions with:
//! - T019: TransactionBuilder struct with builder pattern
//! - T020: Dynamic fee calculation based on transaction size
//! - T021: Change output generation
//! - T022: Input/output validation (dust limits, address format)
//!
//! Usage:
//! ```rust
//! let tx = TransactionBuilder::new()
//!     .add_recipient("btpc1q...", 50_000_000)
//!     .select_utxos(&utxos)
//!     .set_fee_rate(100)
//!     .set_change_address("btpc1q...")
//!     .build()?;
//! ```

use anyhow::{anyhow, Result};
use btpc_core::crypto::Address;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::error::TransactionError;
use crate::utxo_manager::{UTXO, TxInput, TxOutput, Transaction};

/// Minimum output value to prevent dust (1000 satoshis / 0.00001 BTPC)
pub const DUST_LIMIT: u64 = 1000;

/// Default fee rate (satoshis per byte)
pub const DEFAULT_FEE_RATE: u64 = 100;

/// Estimated size per input (bytes) - ML-DSA signature is large
/// ML-DSA-87 signature: ~4627 bytes
/// Previous output: ~36 bytes
/// Script length: 1 byte
/// Total: ~4664 bytes per input
pub const ESTIMATED_INPUT_SIZE: usize = 4700;

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
    /// Fee rate in satoshis per byte
    fee_rate: u64,
    /// Change address (where excess funds go)
    change_address: Option<String>,
    /// Transaction version
    version: u32,
    /// Lock time
    lock_time: u32,
}

impl TransactionBuilder {
    /// Create a new transaction builder
    pub fn new() -> Self {
        Self {
            inputs: Vec::new(),
            outputs: Vec::new(),
            fee_rate: DEFAULT_FEE_RATE,
            change_address: None,
            version: 1,
            lock_time: 0,
        }
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

    /// Set the fee rate (satoshis per byte)
    pub fn set_fee_rate(mut self, rate: u64) -> Self {
        self.fee_rate = rate;
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
    /// Fee = transaction_size * fee_rate
    /// Transaction size is estimated based on:
    /// - Base transaction overhead
    /// - Number of inputs (with ML-DSA signatures)
    /// - Number of outputs
    pub fn calculate_fee(&self) -> Result<u64> {
        let estimated_size = self.estimate_transaction_size();
        let fee = estimated_size as u64 * self.fee_rate;

        println!("📊 Fee calculation:");
        println!("  Estimated size: {} bytes", estimated_size);
        println!("  Fee rate: {} sat/byte", self.fee_rate);
        println!("  Total fee: {} satoshis", fee);

        Ok(fee)
    }

    /// T020: Estimate transaction size in bytes
    fn estimate_transaction_size(&self) -> usize {
        let num_inputs = self.inputs.len();
        let num_outputs = self.outputs.len() + if self.needs_change() { 1 } else { 0 };

        let size = BASE_TX_SIZE + (num_inputs * ESTIMATED_INPUT_SIZE) + (num_outputs * ESTIMATED_OUTPUT_SIZE);

        size
    }

    /// Check if change output is needed (without recursion)
    fn needs_change(&self) -> bool {
        if let Ok(total_input) = self.calculate_total_input() {
            if let Ok(total_output) = self.calculate_total_output() {
                // Estimate fee without calling calculate_fee() to avoid recursion
                let num_inputs = self.inputs.len();
                let num_outputs = self.outputs.len() + 1; // +1 for potential change
                let estimated_size = BASE_TX_SIZE + (num_inputs * ESTIMATED_INPUT_SIZE) + (num_outputs * ESTIMATED_OUTPUT_SIZE);
                let estimated_fee = estimated_size as u64 * self.fee_rate;

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
            let change_addr = self.change_address.as_ref()
                .ok_or_else(|| anyhow!("Change address required but not set"))?;

            println!("💰 Change output:");
            println!("  Amount: {} satoshis", change);
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
        let total_input = self.calculate_total_input()
            .map_err(|e| TransactionError::ValidationFailed {
                reason: format!("Failed to calculate input: {}", e),
            })?;

        let total_output = self.calculate_total_output()
            .map_err(|e| TransactionError::ValidationFailed {
                reason: format!("Failed to calculate output: {}", e),
            })?;

        let fee = self.calculate_fee()
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
        let inputs: Vec<TxInput> = self.inputs
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
            let addr = Address::from_string(address)
                .map_err(|e| TransactionError::InvalidAddress {
                    address: address.clone(),
                    reason: format!("{}", e),
                })?;

            tx_outputs.push(TxOutput {
                value: *amount,
                script_pubkey: addr.hash160().to_vec(),
            });
        }

        // T021: Add change output if needed
        if let Some((change_addr, change_amount)) = self.generate_change_output()? {
            let addr = Address::from_string(&change_addr)
                .map_err(|e| TransactionError::InvalidAddress {
                    address: change_addr.clone(),
                    reason: format!("{}", e),
                })?;

            tx_outputs.push(TxOutput {
                value: change_amount,
                script_pubkey: addr.hash160().to_vec(),
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
            block_height: None,
            confirmed_at: None,
            is_coinbase: false,
        };

        println!("✅ Transaction built:");
        println!("  TX ID: {}", transaction.txid);
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
    use chrono::Utc;

    fn create_test_utxo(value: u64) -> UTXO {
        UTXO {
            txid: "test_tx".to_string(),
            vout: 0,
            value_credits: value,
            value_btp: value as f64 / 100_000_000.0,
            address: "btpc1qtest000000000000000000000000000000000".to_string(),
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
            .add_recipient("btpc1qrecipient00000000000000000000000000000", 50_000_000)
            .select_utxos(&[utxo])
            .set_change_address("btpc1qchange0000000000000000000000000000000");

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
            .add_recipient("btpc1qrecipient00000000000000000000000000000", 50_000_000)
            .select_utxos(&[utxo])
            .set_fee_rate(100);

        let fee = builder.calculate_fee().unwrap();
        assert!(fee > 0);
        println!("Calculated fee: {} satoshis", fee);
    }

    #[test]
    fn test_dust_output_validation() {
        let utxo = create_test_utxo(100_000_000);

        let builder = TransactionBuilder::new()
            .add_recipient("btpc1qrecipient00000000000000000000000000000", 500) // Below dust limit
            .select_utxos(&[utxo])
            .set_change_address("btpc1qchange0000000000000000000000000000000");

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
            .add_recipient("btpc1qrecipient00000000000000000000000000000", 50_000_000) // More than available
            .select_utxos(&[utxo])
            .set_change_address("btpc1qchange0000000000000000000000000000000");

        let result = builder.validate();
        assert!(result.is_err(), "Validation should fail for insufficient funds");
        match result.unwrap_err() {
            TransactionError::InsufficientFunds { available, required, fee } => {
                assert_eq!(available, 10_000_000);
                assert_eq!(required, 50_000_000);
                assert!(fee > 0);
            }
            other => panic!("Expected InsufficientFunds error, got: {:?}", other),
        }
    }
}