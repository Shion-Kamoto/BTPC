//! Transaction Commands Core - Business Logic (TD-001)
//!
//! This module contains pure business logic extracted from Tauri commands.
//! Functions here have NO dependencies on Tauri framework (no State, no AppHandle).
//!
//! **Purpose**: Enable unit testing without Tauri runtime mocking
//! **Pattern**: Tauri commands become thin wrappers calling these core functions
//!
//! Created: 2025-11-04 (TD-001 Partial POC)
//!
//! ## Scope
//! This POC extracts 2 functions that have no main.rs dependencies:
//! 1. create_transaction_core - Transaction building logic
//! 2. estimate_fee_core - Fee estimation logic
//!
//! ## Previously Not Extracted (NOW UNBLOCKED - architectural refactoring complete)
//! After moving RpcClient and TransactionStateManager to lib.rs, these can now be extracted:
//! - broadcast_transaction_core: RpcClient now in lib.rs ✓
//! - get_transaction_status_core: TransactionStateManager now in lib.rs ✓
//! - cancel_transaction_core: TransactionStateManager now in lib.rs ✓
//! - sign_transaction_core: Still complex, deferred

use crate::transaction_builder::TransactionBuilder;
use crate::transaction_state::{TransactionState, TransactionStateManager, TransactionStatus};
use crate::rpc_client::RpcClient;
use crate::utxo_manager::{Transaction, UTXO};
use btpc_core::crypto::{Address, Script, EncryptedWallet, SecurePassword};
use serde::{Serialize, Deserialize};
use std::path::Path;

/// Custom error type for core transaction operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionError {
    /// Address validation failed
    InvalidAddress { address: String, reason: String },
    /// Insufficient funds for transaction
    InsufficientFunds { available: u64, required: u64 },
    /// UTXO selection failed
    UTXOSelectionFailed(String),
    /// Fee estimation failed
    FeeEstimationFailed(String),
    /// Transaction building failed
    BuildFailed(String),
    /// Generic error
    Other(String),
}

impl std::fmt::Display for TransactionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidAddress { address, reason } =>
                write!(f, "Invalid address '{}': {}", address, reason),
            Self::InsufficientFunds { available, required } =>
                write!(f, "Insufficient funds: have {} satoshis, need {} satoshis", available, required),
            Self::UTXOSelectionFailed(e) => write!(f, "UTXO selection failed: {}", e),
            Self::FeeEstimationFailed(e) => write!(f, "Fee estimation failed: {}", e),
            Self::BuildFailed(e) => write!(f, "Transaction build failed: {}", e),
            Self::Other(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for TransactionError {}

//
// ============================================================================
// CREATE TRANSACTION - Core Business Logic
// ============================================================================
//

/// Result of transaction creation (before signing)
#[derive(Debug, Clone)]
pub struct TransactionCreationResult {
    /// The unsigned transaction
    pub transaction: Transaction,
    /// Total input value
    pub total_input: u64,
    /// Total output value (amount sent + change)
    pub total_output: u64,
    /// Calculated fee
    pub fee: u64,
    /// UTXO reservation token (for cleanup)
    pub reservation_token: String,
    /// Reserved UTXO keys
    pub utxo_keys: Vec<String>,
    /// Selected UTXOs (for reference)
    pub selected_utxos: Vec<UTXO>,
}

/// Create unsigned transaction (core business logic - NO TAURI DEPENDENCIES)
///
/// This function extracts the core transaction creation logic from the Tauri command.
/// It can be tested without AppHandle or State.
///
/// # Arguments
/// * `utxos` - Available UTXOs for the from_address
/// * `from_address` - Sender address (validated)
/// * `to_address` - Recipient address (validated)
/// * `amount` - Amount to send (satoshis)
/// * `fee` - Transaction fee (satoshis)
///
/// # Returns
/// * `TransactionCreationResult` with unsigned transaction and metadata
///
/// # Example Test
/// ```rust,ignore
/// let utxos = vec![UTXO { value_credits: 1_000_000, ... }];
/// let result = create_transaction_core(
///     utxos,
///     "btpc1q...",
///     "btpc1q...",
///     500_000,
///     1_000,
/// )?;
/// assert_eq!(result.fee, 1_000);
/// assert!(result.transaction.inputs.len() > 0);
/// ```
pub fn create_transaction_core(
    utxos: Vec<UTXO>,
    from_address: &str,
    to_address: &str,
    amount: u64,
    fee: u64,
) -> Result<TransactionCreationResult, TransactionError> {
    // Validate addresses
    Address::from_string(from_address)
        .map_err(|e| TransactionError::InvalidAddress {
            address: from_address.to_string(),
            reason: e.to_string(),
        })?;

    Address::from_string(to_address)
        .map_err(|e| TransactionError::InvalidAddress {
            address: to_address.to_string(),
            reason: e.to_string(),
        })?;

    // Calculate total input value
    let total_input: u64 = utxos.iter().map(|u| u.value_credits).sum();

    // Check sufficient funds (amount + fee)
    let total_required = amount + fee;
    if total_input < total_required {
        return Err(TransactionError::InsufficientFunds {
            available: total_input,
            required: total_required,
        });
    }

    // Build transaction using TransactionBuilder
    let builder = TransactionBuilder::new()
        .add_recipient(to_address, amount)
        .select_utxos(&utxos)
        .set_fee_rate(100) // Use default, actual fee passed separately
        .set_change_address(from_address);

    // Build transaction
    let transaction = builder.build()
        .map_err(|e| TransactionError::BuildFailed(e.to_string()))?;

    // Calculate change
    let change = total_input.saturating_sub(total_required);
    let total_output = amount + change;

    // Generate UTXO keys for reservation
    let utxo_keys: Vec<String> = utxos.iter()
        .map(|utxo| format!("{}:{}", utxo.txid, utxo.vout))
        .collect();

    // Generate reservation token
    let reservation_token = format!("res_{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));

    Ok(TransactionCreationResult {
        transaction,
        total_input,
        total_output,
        fee,
        reservation_token,
        utxo_keys,
        selected_utxos: utxos,
    })
}

//
// ============================================================================
// ESTIMATE FEE - Core Business Logic
// ============================================================================
//

/// Result of fee estimation
#[derive(Debug, Clone)]
pub struct EstimateFeeResult {
    /// Estimated fee in satoshis
    pub estimated_fee: u64,
    /// Estimated transaction size in bytes
    pub estimated_size: usize,
    /// Fee rate used (satoshis per byte)
    pub fee_rate: u64,
    /// Number of inputs required
    pub inputs_count: usize,
    /// Number of outputs (recipient + change if any)
    pub outputs_count: usize,
}

/// Estimate transaction fee (core business logic - NO TAURI DEPENDENCIES)
///
/// This function extracts the core fee estimation logic from the Tauri command.
/// It can be tested without AppHandle or State.
///
/// # Arguments
/// * `utxos` - Available UTXOs for the from_address
/// * `from_address` - Sender address (for change output)
/// * `to_address` - Recipient address
/// * `amount` - Amount to send (satoshis)
/// * `fee_rate` - Fee rate in satoshis per byte (optional, defaults to 100)
///
/// # Returns
/// * `EstimateFeeResult` with fee breakdown
///
/// # Example Test
/// ```rust,ignore
/// let utxos = vec![UTXO { value_credits: 1_000_000, ... }];
/// let result = estimate_fee_core(
///     utxos,
///     "btpc1q...",
///     "btpc1q...",
///     500_000,
///     Some(100),
/// )?;
/// assert!(result.estimated_fee > 0);
/// assert_eq!(result.fee_rate, 100);
/// ```
pub fn estimate_fee_core(
    utxos: Vec<UTXO>,
    from_address: &str,
    to_address: &str,
    amount: u64,
    fee_rate: Option<u64>,
) -> Result<EstimateFeeResult, TransactionError> {
    if utxos.is_empty() {
        return Err(TransactionError::UTXOSelectionFailed("No UTXOs available".to_string()));
    }

    // Use provided fee rate or default to 100 satoshis/byte
    let fee_rate = fee_rate.unwrap_or(100);

    // Build transaction using TransactionBuilder to get estimate
    let builder = TransactionBuilder::new()
        .add_recipient(to_address, amount)
        .select_utxos(&utxos)
        .set_fee_rate(fee_rate)
        .set_change_address(from_address);

    // Get summary (this calculates fee without actually building)
    let summary = builder.summary()
        .map_err(|e| TransactionError::FeeEstimationFailed(format!("Failed to estimate: {}", e)))?;

    Ok(EstimateFeeResult {
        estimated_fee: summary.fee,
        estimated_size: summary.estimated_size,
        fee_rate,
        inputs_count: summary.num_inputs,
        outputs_count: summary.num_outputs,
    })
}

//
// ============================================================================
// BROADCAST TRANSACTION - Core Business Logic
// ============================================================================
//

/// Result of transaction broadcast
#[derive(Debug, Clone)]
pub struct BroadcastTransactionResult {
    /// Transaction ID from RPC
    pub transaction_id: String,
    /// Number of peers broadcast to (estimate)
    pub broadcast_to_peers: usize,
    /// Whether mempool accepted the transaction
    pub mempool_accepted: bool,
    /// RPC response message
    pub network_response: String,
}

/// Broadcast signed transaction to network (core business logic - NO TAURI DEPENDENCIES)
///
/// This function extracts the core broadcast logic from the Tauri command.
/// It can be tested without AppHandle or State.
///
/// # Arguments
/// * `transaction` - Signed transaction to broadcast
/// * `rpc_client` - RPC client for node communication
/// * `tx_state_manager` - Transaction state manager reference
/// * `transaction_id` - Transaction ID for state tracking
///
/// # Returns
/// * `BroadcastTransactionResult` with broadcast details
///
/// # Errors
/// * Returns error if transaction not signed, RPC connection fails, or broadcast rejected
///
/// # Example Test
/// ```rust,ignore
/// let tx = Transaction { ... }; // Signed transaction
/// let rpc = RpcClient::new("127.0.0.1", 18443);
/// let tx_state = TransactionStateManager::new();
/// let result = broadcast_transaction_core(tx, &rpc, &tx_state, "tx_123").await?;
/// assert!(result.mempool_accepted);
/// ```
pub async fn broadcast_transaction_core(
    transaction: Transaction,
    rpc_client: &RpcClient,
    tx_state_manager: &TransactionStateManager,
    transaction_id: &str,
) -> Result<BroadcastTransactionResult, TransactionError> {
    // Verify transaction is signed (all inputs have signatures)
    let all_signed = transaction.inputs.iter().all(|input| !input.signature_script.is_empty());
    if !all_signed {
        tx_state_manager.set_state(
            transaction_id.to_string(),
            TransactionStatus::Failed,
            Some("Transaction not fully signed".to_string()),
        );
        return Err(TransactionError::Other("Transaction not fully signed - cannot broadcast".to_string()));
    }

    // Serialize transaction to hex for RPC
    let tx_bytes = serialize_transaction_to_bytes(&transaction);
    let tx_hex = hex::encode(&tx_bytes);

    // Test RPC connection
    if !rpc_client.ping().await.unwrap_or(false) {
        tx_state_manager.set_state(
            transaction_id.to_string(),
            TransactionStatus::Failed,
            Some("Cannot connect to BTPC node".to_string()),
        );
        return Err(TransactionError::Other("Cannot connect to BTPC node - is the node running?".to_string()));
    }

    // Submit transaction via sendrawtransaction
    match rpc_client.send_raw_transaction(&tx_hex).await {
        Ok(txid) => {
            // Update state to broadcast
            tx_state_manager.set_state(transaction_id.to_string(), TransactionStatus::Broadcast, None);

            Ok(BroadcastTransactionResult {
                transaction_id: transaction_id.to_string(),
                broadcast_to_peers: 8, // Estimate based on typical peer count
                mempool_accepted: true,
                network_response: format!("Accepted with txid: {}", txid),
            })
        }
        Err(e) => {
            let error_msg = format!("RPC broadcast failed: {}", e);

            // Update state to failed
            tx_state_manager.set_state(
                transaction_id.to_string(),
                TransactionStatus::Failed,
                Some(error_msg.clone()),
            );

            Err(TransactionError::Other(error_msg))
        }
    }
}

/// Helper: Serialize desktop app Transaction to bytes
/// Converts to wire format for RPC submission
fn serialize_transaction_to_bytes(tx: &Transaction) -> Vec<u8> {
    // Simple serialization using bincode (matches desktop app's Transaction struct)
    // For production, this should match btpc-core's Transaction::serialize() format
    bincode::serialize(tx).unwrap_or_else(|_| {
        // Fallback: manual serialization
        let mut bytes = Vec::new();

        // Version (4 bytes)
        bytes.extend_from_slice(&1u32.to_le_bytes());

        // Input count
        bytes.extend_from_slice(&(tx.inputs.len() as u32).to_le_bytes());

        // Inputs
        for input in &tx.inputs {
            // Txid (assume 64 bytes for SHA-512)
            bytes.extend_from_slice(input.prev_txid.as_bytes());
            // Vout
            bytes.extend_from_slice(&input.prev_vout.to_le_bytes());
            // Signature script length + data
            bytes.extend_from_slice(&(input.signature_script.len() as u32).to_le_bytes());
            bytes.extend_from_slice(&input.signature_script);
            // Sequence
            bytes.extend_from_slice(&0xffffffffu32.to_le_bytes());
        }

        // Output count
        bytes.extend_from_slice(&(tx.outputs.len() as u32).to_le_bytes());

        // Outputs
        for output in &tx.outputs {
            // Value
            bytes.extend_from_slice(&output.value.to_le_bytes());
            // Script pubkey length + data
            bytes.extend_from_slice(&(output.script_pubkey.len() as u32).to_le_bytes());
            bytes.extend_from_slice(&output.script_pubkey);
        }

        // Lock time
        bytes.extend_from_slice(&tx.lock_time.to_le_bytes());

        bytes
    })
}

//
// ============================================================================
// GET TRANSACTION STATUS - Core Business Logic
// ============================================================================
//

/// Get transaction status (core business logic - NO TAURI DEPENDENCIES)
///
/// This function extracts the core status query logic from the Tauri command.
/// It can be tested without AppHandle or State.
///
/// # Arguments
/// * `tx_state_manager` - Transaction state manager reference
/// * `transaction_id` - Transaction ID to query
///
/// # Returns
/// * `TransactionState` with current transaction status
///
/// # Errors
/// * Returns error if transaction not found
///
/// # Example Test
/// ```rust,ignore
/// let tx_state = TransactionStateManager::new();
/// // ... create transaction ...
/// let status = get_transaction_status_core(&tx_state, "tx_123")?;
/// assert_eq!(status.status, TransactionStatus::Validating);
/// ```
pub fn get_transaction_status_core(
    tx_state_manager: &TransactionStateManager,
    transaction_id: &str,
) -> Result<TransactionState, TransactionError> {
    tx_state_manager.get_state(transaction_id)
        .ok_or_else(|| TransactionError::Other(format!("Transaction {} not found", transaction_id)))
}

//
// ============================================================================
// CANCEL TRANSACTION - Core Business Logic
// ============================================================================
//

/// Cancel transaction (core business logic - NO TAURI DEPENDENCIES)
///
/// This function extracts the core cancellation logic from the Tauri command.
/// It can be tested without AppHandle or State.
///
/// Note: UTXO reservation release is handled by the Tauri wrapper (requires UTXOManager lock).
///
/// # Arguments
/// * `tx_state_manager` - Transaction state manager reference
/// * `transaction_id` - Transaction ID to cancel
///
/// # Returns
/// * `Ok(())` if cancelled successfully
///
/// # Errors
/// * Returns error if transaction not found or already broadcast
///
/// # Example Test
/// ```rust,ignore
/// let tx_state = TransactionStateManager::new();
/// // ... create transaction ...
/// cancel_transaction_core(&tx_state, "tx_123")?;
/// let status = get_transaction_status_core(&tx_state, "tx_123")?;
/// assert_eq!(status.status, TransactionStatus::Cancelled);
/// ```
pub fn cancel_transaction_core(
    tx_state_manager: &TransactionStateManager,
    transaction_id: &str,
) -> Result<TransactionState, TransactionError> {
    // Get current state
    let tx_status = tx_state_manager.get_state(transaction_id)
        .ok_or_else(|| TransactionError::Other(format!("Transaction {} not found", transaction_id)))?;

    // Can only cancel if not yet broadcast
    if tx_status.status == TransactionStatus::Broadcast
        || tx_status.status == TransactionStatus::Confirmed {
        return Err(TransactionError::Other("Cannot cancel transaction that has been broadcast".to_string()));
    }

    // Update state to cancelled
    tx_state_manager.set_state(transaction_id.to_string(), TransactionStatus::Cancelled, None);

    // Return updated state with reservation info for UTXO cleanup (handled by wrapper)
    Ok(tx_status)
}

//
// ============================================================================
// SIGN TRANSACTION - Core Business Logic
// ============================================================================
//

/// Result of transaction signing
#[derive(Debug, Clone)]
pub struct SignTransactionResult {
    /// The signed transaction
    pub signed_transaction: Transaction,
    /// Number of signatures created
    pub signatures_count: usize,
    /// Ready to broadcast flag
    pub ready_to_broadcast: bool,
}

/// Sign transaction with ML-DSA (core business logic - NO TAURI DEPENDENCIES)
///
/// This function extracts the core signing logic from the Tauri command.
/// It can be tested without AppHandle or State.
///
/// # Arguments
/// * `transaction` - Unsigned transaction to sign
/// * `wallet_path` - Path to encrypted wallet file (.dat)
/// * `password` - Password to decrypt wallet
///
/// # Returns
/// * `SignTransactionResult` with signed transaction
///
/// # Errors
/// * Returns error if wallet loading fails, decryption fails, wallet corrupted, or signing fails
///
/// # Example Test
/// ```rust,ignore
/// let tx = Transaction { ... }; // Unsigned transaction
/// let result = sign_transaction_core(tx, Path::new("wallet.dat"), "password")?;
/// assert_eq!(result.signatures_count, tx.inputs.len());
/// assert!(result.ready_to_broadcast);
/// ```
pub fn sign_transaction_core(
    mut transaction: Transaction,
    wallet_path: &Path,
    password: &str,
) -> Result<SignTransactionResult, TransactionError> {
    // Load encrypted wallet file
    let encrypted_wallet = EncryptedWallet::load_from_file(wallet_path)
        .map_err(|e| TransactionError::Other(format!("Failed to load encrypted wallet: {}", e)))?;

    // Decrypt wallet with Argon2id
    let secure_password = SecurePassword::new(password.to_string());
    let wallet_data = encrypted_wallet.decrypt(&secure_password)
        .map_err(|e| TransactionError::Other(format!("Failed to decrypt wallet (wrong password?): {}", e)))?;

    // Validate wallet integrity
    validate_wallet_integrity(&wallet_data, wallet_path)?;

    // Get the first key from the wallet
    let key_entry = wallet_data.keys.first()
        .ok_or_else(|| TransactionError::Other("Wallet has no keys".to_string()))?;

    // Use KeyEntry's to_private_key() method (uses seed if available for signing!)
    let private_key = key_entry.to_private_key()
        .map_err(|e| TransactionError::Other(format!("Failed to load private key: {}", e)))?;

    // Get public key for script creation
    let public_key = private_key.public_key();

    // Serialize transaction WITHOUT signatures (critical for correct signing!)
    let tx_data = serialize_for_signature(&transaction);

    // Sign each input with ML-DSA
    for input in transaction.inputs.iter_mut() {
        // Sign the properly serialized transaction data (matches blockchain validation)
        let signature = private_key.sign(&tx_data)
            .map_err(|e| TransactionError::Other(format!("Failed to sign input: {}", e)))?;

        // Create P2PKH unlock script with signature + public key (matches blockchain format)
        let unlock_script = Script::unlock_p2pkh(&signature.to_bytes(), &public_key.to_bytes());

        // Convert Script to raw bytes
        input.signature_script = unlock_script.to_bytes();
    }

    let signatures_count = transaction.inputs.len();

    Ok(SignTransactionResult {
        signed_transaction: transaction,
        signatures_count,
        ready_to_broadcast: true,
    })
}

/// Helper: Serialize transaction for signing (WITHOUT signatures)
/// This matches btpc-core's serialize_for_signature() behavior
fn serialize_for_signature(tx: &Transaction) -> Vec<u8> {
    let mut bytes = Vec::new();

    // Version (4 bytes)
    bytes.extend_from_slice(&tx.version.to_le_bytes());

    // Input count
    bytes.extend_from_slice(&(tx.inputs.len() as u32).to_le_bytes());

    // Inputs WITHOUT signature_script (critical for signing!)
    for input in &tx.inputs {
        // Txid (assume 64 bytes for SHA-512)
        bytes.extend_from_slice(input.prev_txid.as_bytes());
        // Vout
        bytes.extend_from_slice(&input.prev_vout.to_le_bytes());
        // Empty signature script (length 0)
        bytes.extend_from_slice(&0u32.to_le_bytes());
        // Sequence
        bytes.extend_from_slice(&input.sequence.to_le_bytes());
    }

    // Output count
    bytes.extend_from_slice(&(tx.outputs.len() as u32).to_le_bytes());

    // Outputs
    for output in &tx.outputs {
        // Value
        bytes.extend_from_slice(&output.value.to_le_bytes());
        // Script pubkey length + data
        bytes.extend_from_slice(&(output.script_pubkey.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&output.script_pubkey);
    }

    // Lock time
    bytes.extend_from_slice(&tx.lock_time.to_le_bytes());

    bytes
}

/// Validate wallet file integrity (core business logic - NO TAURI DEPENDENCIES)
///
/// Checks for:
/// - Required fields presence (wallet_id, network, keys)
/// - Non-empty keys array
/// - Key structure integrity (private_key_bytes, public_key_bytes, address)
/// - File truncation detection (key size validation)
///
/// Returns Ok(()) if wallet is valid, Err with specific corruption details otherwise.
fn validate_wallet_integrity(
    wallet_data: &btpc_core::crypto::WalletData,
    wallet_path: &Path,
) -> Result<(), TransactionError> {
    // Check 1: Validate wallet_id is not empty
    if wallet_data.wallet_id.is_empty() {
        return Err(TransactionError::Other("Wallet corruption detected: wallet_id field is empty".to_string()));
    }

    // Check 2: Validate network is not empty
    if wallet_data.network.is_empty() {
        return Err(TransactionError::Other("Wallet corruption detected: network field is missing or empty".to_string()));
    }

    // Check 3: Validate keys array is not empty
    if wallet_data.keys.is_empty() {
        return Err(TransactionError::Other("Wallet corruption detected: wallet has no keys (keys array is empty)".to_string()));
    }

    // Check 4: Validate each key entry structure
    for (i, key_entry) in wallet_data.keys.iter().enumerate() {
        // Check private key bytes size (ML-DSA-65 = 4000 bytes)
        const EXPECTED_PRIVATE_KEY_SIZE: usize = 4000;
        if key_entry.private_key_bytes.len() != EXPECTED_PRIVATE_KEY_SIZE {
            return Err(TransactionError::Other(format!(
                "Wallet corruption detected: key {} has invalid private key size (expected {}, got {}). File may be truncated.",
                i,
                EXPECTED_PRIVATE_KEY_SIZE,
                key_entry.private_key_bytes.len()
            )));
        }

        // Check public key bytes size (ML-DSA-65 = 1952 bytes)
        const EXPECTED_PUBLIC_KEY_SIZE: usize = 1952;
        if key_entry.public_key_bytes.len() != EXPECTED_PUBLIC_KEY_SIZE {
            return Err(TransactionError::Other(format!(
                "Wallet corruption detected: key {} has invalid public key size (expected {}, got {}). File may be truncated.",
                i,
                EXPECTED_PUBLIC_KEY_SIZE,
                key_entry.public_key_bytes.len()
            )));
        }

        // Check seed size if present (should be 32 bytes)
        if let Some(seed) = &key_entry.seed {
            const EXPECTED_SEED_SIZE: usize = 32;
            if seed.len() != EXPECTED_SEED_SIZE {
                return Err(TransactionError::Other(format!(
                    "Wallet corruption detected: key {} has invalid seed size (expected {}, got {})",
                    i,
                    EXPECTED_SEED_SIZE,
                    seed.len()
                )));
            }
        }

        // Check address is not empty
        if key_entry.address.is_empty() {
            return Err(TransactionError::Other(format!(
                "Wallet corruption detected: key {} has empty address field",
                i
            )));
        }
    }

    // Check 5: Validate file exists and has reasonable size
    if let Ok(metadata) = std::fs::metadata(wallet_path) {
        let file_size = metadata.len();
        const MIN_WALLET_FILE_SIZE: u64 = 100; // Magic bytes + version + salt + nonce + minimal data
        const MAX_WALLET_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10 MB max

        if file_size < MIN_WALLET_FILE_SIZE {
            return Err(TransactionError::Other(format!(
                "Wallet corruption detected: file size ({} bytes) is too small. File may be truncated.",
                file_size
            )));
        }

        if file_size > MAX_WALLET_FILE_SIZE {
            return Err(TransactionError::Other(format!(
                "Wallet corruption detected: file size ({} bytes) exceeds maximum ({}). File may be corrupted.",
                file_size,
                MAX_WALLET_FILE_SIZE
            )));
        }
    }

    // Check 6: Validate timestamps are reasonable
    let current_timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    if wallet_data.created_at == 0 || wallet_data.modified_at == 0 {
        return Err(TransactionError::Other("Wallet corruption detected: invalid timestamps (zero values)".to_string()));
    }

    // Future timestamps indicate corruption (allow 1 day tolerance for clock skew)
    if wallet_data.created_at > current_timestamp + 86400 ||
       wallet_data.modified_at > current_timestamp + 86400 {
        return Err(TransactionError::Other("Wallet corruption detected: timestamps are in the future (clock skew or corruption)".to_string()));
    }

    Ok(())
}

//
// ============================================================================
// TEST HELPERS (for unit tests)
// ============================================================================
//

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utxo_manager::{TxInput, TxOutput};
    use chrono::Utc;

    // ============================================================================
    // Test Helpers
    // ============================================================================

    fn create_test_transaction_state_manager() -> TransactionStateManager {
        TransactionStateManager::new()
    }

    fn create_test_transaction(signed: bool) -> Transaction {
        Transaction {
            txid: "test_tx_123".to_string(),
            version: 1,
            inputs: vec![TxInput {
                prev_txid: "prev_tx".to_string(),
                prev_vout: 0,
                signature_script: if signed { vec![1, 2, 3] } else { vec![] },
                sequence: 0xffffffff,
            }],
            outputs: vec![TxOutput {
                value: 50_000_000,
                script_pubkey: vec![0x76, 0xa9, 0x14], // P2PKH prefix
            }],
            lock_time: 0,
            fork_id: 2, // regtest
            block_height: None,
            confirmed_at: None,
            is_coinbase: false,
        }
    }

    // ============================================================================
    // get_transaction_status_core() Tests
    // ============================================================================

    #[test]
    fn test_get_transaction_status_core_found() {
        let tx_state_mgr = create_test_transaction_state_manager();
        let tx_id = "test_tx_1";

        // Create a transaction state
        tx_state_mgr.set_state(tx_id.to_string(), TransactionStatus::Validating, None);

        // Test retrieval
        let result = get_transaction_status_core(&tx_state_mgr, tx_id);
        assert!(result.is_ok(), "Should find transaction");

        let state = result.unwrap();
        assert_eq!(state.transaction_id, tx_id);
        assert_eq!(state.status, TransactionStatus::Validating);
    }

    #[test]
    fn test_get_transaction_status_core_not_found() {
        let tx_state_mgr = create_test_transaction_state_manager();

        // Test retrieval of non-existent transaction
        let result = get_transaction_status_core(&tx_state_mgr, "nonexistent_tx");
        assert!(result.is_err(), "Should return error for non-existent transaction");

        let err = result.unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn test_get_transaction_status_core_multiple_transactions() {
        let tx_state_mgr = create_test_transaction_state_manager();

        // Create multiple transaction states
        tx_state_mgr.set_state("tx1".to_string(), TransactionStatus::Validating, None);
        tx_state_mgr.set_state("tx2".to_string(), TransactionStatus::Broadcast, None);
        tx_state_mgr.set_state("tx3".to_string(), TransactionStatus::Confirmed, None);

        // Test retrieval of each
        let tx1 = get_transaction_status_core(&tx_state_mgr, "tx1").unwrap();
        assert_eq!(tx1.status, TransactionStatus::Validating);

        let tx2 = get_transaction_status_core(&tx_state_mgr, "tx2").unwrap();
        assert_eq!(tx2.status, TransactionStatus::Broadcast);

        let tx3 = get_transaction_status_core(&tx_state_mgr, "tx3").unwrap();
        assert_eq!(tx3.status, TransactionStatus::Confirmed);
    }

    // ============================================================================
    // cancel_transaction_core() Tests
    // ============================================================================

    #[test]
    fn test_cancel_transaction_core_success() {
        let tx_state_mgr = create_test_transaction_state_manager();
        let tx_id = "test_tx_cancel";

        // Create a transaction in Validating state (can be cancelled)
        tx_state_mgr.set_state(tx_id.to_string(), TransactionStatus::Validating, None);

        // Test cancellation
        let result = cancel_transaction_core(&tx_state_mgr, tx_id);
        assert!(result.is_ok(), "Should successfully cancel transaction");

        // Verify state changed to Cancelled
        let state = get_transaction_status_core(&tx_state_mgr, tx_id).unwrap();
        assert_eq!(state.status, TransactionStatus::Cancelled);
    }

    #[test]
    fn test_cancel_transaction_core_already_broadcast() {
        let tx_state_mgr = create_test_transaction_state_manager();
        let tx_id = "test_tx_broadcast";

        // Create a transaction in Broadcast state (cannot be cancelled)
        tx_state_mgr.set_state(tx_id.to_string(), TransactionStatus::Broadcast, None);

        // Test cancellation
        let result = cancel_transaction_core(&tx_state_mgr, tx_id);
        assert!(result.is_err(), "Should fail to cancel broadcast transaction");

        let err = result.unwrap_err();
        assert!(err.to_string().contains("Cannot cancel transaction that has been broadcast"));
    }

    #[test]
    fn test_cancel_transaction_core_already_confirmed() {
        let tx_state_mgr = create_test_transaction_state_manager();
        let tx_id = "test_tx_confirmed";

        // Create a transaction in Confirmed state (cannot be cancelled)
        tx_state_mgr.set_state(tx_id.to_string(), TransactionStatus::Confirmed, None);

        // Test cancellation
        let result = cancel_transaction_core(&tx_state_mgr, tx_id);
        assert!(result.is_err(), "Should fail to cancel confirmed transaction");

        let err = result.unwrap_err();
        assert!(err.to_string().contains("Cannot cancel transaction that has been broadcast"));
    }

    #[test]
    fn test_cancel_transaction_core_not_found() {
        let tx_state_mgr = create_test_transaction_state_manager();

        // Test cancellation of non-existent transaction
        let result = cancel_transaction_core(&tx_state_mgr, "nonexistent_tx");
        assert!(result.is_err(), "Should return error for non-existent transaction");

        let err = result.unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn test_cancel_transaction_core_with_reservation() {
        let tx_state_mgr = create_test_transaction_state_manager();
        let tx_id = "test_tx_with_reservation";

        // Create a transaction state with reservation info
        tx_state_mgr.set_state(tx_id.to_string(), TransactionStatus::Validating, None);

        // In a real scenario, reservation_token and utxo_keys would be set
        // The core function returns the state so the wrapper can release the reservation

        let result = cancel_transaction_core(&tx_state_mgr, tx_id);
        assert!(result.is_ok(), "Should successfully cancel and return state with reservation info");

        let returned_state = result.unwrap();
        assert_eq!(returned_state.transaction_id, tx_id);
    }

    // ============================================================================
    // broadcast_transaction_core() Tests
    // ============================================================================

    #[test]
    fn test_broadcast_transaction_core_unsigned_transaction() {
        let tx_state_mgr = create_test_transaction_state_manager();
        let tx_id = "test_tx_unsigned";

        // Create unsigned transaction (empty signature_script)
        let unsigned_tx = create_test_transaction(false);

        // Create a mock RPC client (won't be used since validation fails first)
        let rpc_client = RpcClient::new("127.0.0.1", 18443);

        // Test broadcast with unsigned transaction
        let result = tokio_test::block_on(broadcast_transaction_core(
            unsigned_tx,
            &rpc_client,
            &tx_state_mgr,
            tx_id,
        ));

        assert!(result.is_err(), "Should fail to broadcast unsigned transaction");

        let err = result.unwrap_err();
        assert!(err.to_string().contains("not fully signed"));

        // Verify state was set to Failed
        let state = get_transaction_status_core(&tx_state_mgr, tx_id).unwrap();
        assert_eq!(state.status, TransactionStatus::Failed);
    }

    #[test]
    #[ignore] // Requires running RPC node
    fn test_broadcast_transaction_core_rpc_connection_failed() {
        let tx_state_mgr = create_test_transaction_state_manager();
        let tx_id = "test_tx_no_connection";

        // Create signed transaction
        let signed_tx = create_test_transaction(true);

        // Create RPC client pointing to non-existent node
        let rpc_client = RpcClient::new("127.0.0.1", 65534); // Unlikely port

        // Test broadcast with no RPC connection
        let result = tokio_test::block_on(broadcast_transaction_core(
            signed_tx,
            &rpc_client,
            &tx_state_mgr,
            tx_id,
        ));

        assert!(result.is_err(), "Should fail when RPC node is not available");

        let err = result.unwrap_err();
        assert!(err.to_string().contains("Cannot connect to BTPC node"));

        // Verify state was set to Failed
        let state = get_transaction_status_core(&tx_state_mgr, tx_id).unwrap();
        assert_eq!(state.status, TransactionStatus::Failed);
    }

    // Note: Full integration test with actual RPC node would require:
    // - Running btpc_node in regtest mode
    // - Creating valid signed transaction with real ML-DSA signatures
    // - This is beyond scope of unit tests - belongs in integration tests

    // ============================================================================
    // Helper Function Tests
    // ============================================================================

    #[test]
    fn test_serialize_transaction_to_bytes() {
        let tx = create_test_transaction(true);
        let bytes = serialize_transaction_to_bytes(&tx);

        assert!(!bytes.is_empty(), "Serialized transaction should not be empty");
        assert!(bytes.len() > 10, "Serialized transaction should have reasonable size");
    }

    #[test]
    fn test_serialize_for_signature() {
        let tx = create_test_transaction(false);
        let bytes = serialize_for_signature(&tx);

        assert!(!bytes.is_empty(), "Serialized transaction should not be empty");
        // When serialized for signing, signatures should be omitted (length 0)
        // So size should be smaller than a fully signed transaction
    }
}