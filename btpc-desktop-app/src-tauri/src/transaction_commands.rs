//! Transaction Commands - Feature 007 (T023-T031)
//!
//! Tauri commands for transaction operations:
//! - T026: create_transaction - Build unsigned transaction
//! - T027: sign_transaction - Sign with ML-DSA (using seed)
//! - T028: broadcast_transaction - Send to network
//! - T029: get_transaction_status - Query transaction state
//! - T030: cancel_transaction - Cancel pending transaction
//! - T031: estimate_fee - Calculate transaction fee

use crate::AppState;
use crate::events::{TransactionEvent, UTXOEvent, ReleaseReason};
use btpc_desktop_app::transaction_commands_core;  // TD-001: Import core business logic from lib.rs
use btpc_desktop_app::transaction_builder::{TransactionBuilder, TransactionSummary};
use btpc_desktop_app::transaction_state::{TransactionState, TransactionStatus, TransactionStateManager};
use btpc_desktop_app::utxo_manager::Transaction;
use btpc_core::crypto::{Address, Script};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

/// Request to create a transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTransactionRequest {
    pub wallet_id: String,
    pub from_address: String,
    pub to_address: String,
    pub amount: u64,
    pub fee_rate: Option<u64>,
}

/// Response from create_transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTransactionResponse {
    pub transaction_id: String,
    pub summary: TransactionSummary,
    pub requires_signing: bool,
}

/// T026: Create unsigned transaction
///
/// Steps:
/// 1. Validate addresses and amount
/// 2. Select and reserve UTXOs
/// 3. Build transaction with TransactionBuilder
/// 4. Return unsigned transaction for signing
#[tauri::command]
pub async fn create_transaction(
    state: State<'_, AppState>,
    wallet_id: String,
    from_address: String,
    to_address: String,
    amount: u64,
    fee_rate: Option<u64>,
    app: AppHandle,
) -> Result<CreateTransactionResponse, String> {
    // Reconstruct request from flattened parameters
    let request = CreateTransactionRequest {
        wallet_id: wallet_id.clone(),
        from_address: from_address.clone(),
        to_address: to_address.clone(),
        amount,
        fee_rate,
    };

    println!("🔨 Creating transaction:");
    println!("  Wallet: {}", wallet_id);
    println!("  From: {}", from_address);
    println!("  To: {}", to_address);
    println!("  Amount: {} satoshis", amount);

    // Emit transaction initiated event
    let _ = app.emit("transaction:initiated", TransactionEvent::TransactionInitiated {
        wallet_id: request.wallet_id.clone(),
        recipient: request.to_address.clone(),
        amount: request.amount,
        timestamp: Utc::now(),
    });

    // Validate addresses
    if let Err(e) = Address::from_string(&request.from_address) {
        return Err(format!("Invalid from address: {}", e));
    }
    if let Err(e) = Address::from_string(&request.to_address) {
        return Err(format!("Invalid to address: {}", e));
    }

    // Get tx_state from AppState
    let tx_state = &state.tx_state_manager;

    // Scope the utxo_manager lock to ensure it's dropped before async operations
    let (utxos, utxo_keys, reservation, temp_tx_id, total_utxo_value, inputs_count) = {
        let utxo_manager = state.utxo_manager.lock().expect("Mutex poisoned");

        // Select UTXOs
        let utxos = utxo_manager
            .select_utxos_for_amount(&request.from_address, request.amount + 500_000) // Add buffer for fee
            .map_err(|e| format!("Failed to select UTXOs: {}", e))?;

        if utxos.is_empty() {
            return Err("No UTXOs available for transaction".to_string());
        }

        // Reserve UTXOs
        let utxo_keys: Vec<String> = utxos.iter()
            .map(|utxo| format!("{}:{}", utxo.txid, utxo.vout))
            .collect();

        let temp_tx_id = format!("tx_{}", Utc::now().timestamp_nanos_opt().unwrap_or(0));
        let reservation = utxo_manager
            .reserve_utxos(utxo_keys.clone(), Some(temp_tx_id.clone()))
            .map_err(|e| format!("Failed to reserve UTXOs: {}", e))?;

        let total_utxo_value: u64 = utxos.iter().map(|u| u.value_credits).sum();
        let inputs_count = utxos.len();

        // Return all needed values, utxo_manager drops here
        (utxos, utxo_keys, reservation, temp_tx_id, total_utxo_value, inputs_count)
    };

    // Emit UTXO reserved event (after lock is dropped)
    let _ = app.emit("utxo:reserved", UTXOEvent::UTXOReserved {
        reservation_token: reservation.id.clone(),
        transaction_id: Some(temp_tx_id.clone()),
        utxo_count: inputs_count,
        total_amount: total_utxo_value,
        expires_at: Utc::now() + chrono::Duration::minutes(5),
    });

    // T018: Dynamic fee estimation using FeeEstimator service
    let fee_rate = if let Some(custom_rate) = request.fee_rate {
        // User provided custom fee rate - use it directly
        custom_rate
    } else {
        // Use dynamic fee estimation
        let rpc_port = *state.active_rpc_port.read().await;
        let fee_estimator = crate::fee_estimator::FeeEstimator::new(rpc_port);

        // Estimate with expected outputs count (recipient + change = 2)
        let fee_estimate = fee_estimator.estimate_fee_for_transaction(inputs_count, 2).await
            .map_err(|e| format!("Fee estimation failed: {}", e))?;

        // Emit fee estimated event
        let _ = app.emit("fee:estimated", TransactionEvent::FeeEstimated {
            transaction_id: Some(temp_tx_id.clone()),
            estimated_fee: fee_estimate.estimated_fee,
            fee_rate: fee_estimate.fee_rate,
            estimated_size: fee_estimate.estimated_size,
        });

        println!("💰 Dynamic fee estimation: {} sat/byte (estimated {} satoshis for {} bytes)",
            fee_estimate.fee_rate, fee_estimate.estimated_fee, fee_estimate.estimated_size);

        fee_estimate.fee_rate
    };

    let builder = TransactionBuilder::new()
        .add_recipient(&request.to_address, request.amount)
        .select_utxos(&utxos)
        .set_fee_rate(fee_rate)
        .set_change_address(&request.from_address);

    // Validate transaction
    builder.validate()
        .map_err(|e| format!("Transaction validation failed: {}", e))?;

    // Get summary
    let summary = builder.summary()
        .map_err(|e| format!("Failed to get transaction summary: {}", e))?;

    // Build transaction
    let transaction = builder.build()
        .map_err(|e| format!("Failed to build transaction: {}", e))?;

    let transaction_id = transaction.txid.clone();

    // Store transaction with state and reservation info for later cleanup
    tx_state.set_transaction_with_reservation(
        transaction_id.clone(),
        transaction.clone(),
        TransactionStatus::Validating,
        reservation.id.clone(),
        utxo_keys.clone(),
        request.wallet_id.clone(),
    );

    // Emit transaction validated event
    let _ = app.emit("transaction:validated", TransactionEvent::TransactionValidated {
        transaction_id: transaction_id.clone(),
        inputs_count: transaction.inputs.len(),
        outputs_count: transaction.outputs.len(),
        fee: summary.fee,
        change_amount: summary.change.unwrap_or(0),
        total_input: summary.total_input,
        total_output: summary.total_output,
    });

    println!("✅ Transaction created: {} (reservation: {})", transaction_id, reservation.id);

    Ok(CreateTransactionResponse {
        transaction_id,
        summary,
        requires_signing: true,
    })
}

/// Request to sign a transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignTransactionRequest {
    pub transaction_id: String,
    pub wallet_id: String,
    pub password: String,
}

/// Response from sign_transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignTransactionResponse {
    pub transaction_id: String,
    pub signatures_count: usize,
    pub ready_to_broadcast: bool,
}

/// T027: Sign transaction with ML-DSA
///
/// Steps:
/// 1. Load wallet and decrypt with password
/// 2. For each input, get corresponding private key
/// 3. Sign input with ML-DSA using seed regeneration
/// 4. Attach signature to transaction
/// 5. Verify all signatures
#[tauri::command]
pub async fn sign_transaction(
    state: State<'_, AppState>,
    request: SignTransactionRequest,
    app: AppHandle,
) -> Result<SignTransactionResponse, String> {
    println!("✍️ Signing transaction: {}", request.transaction_id);

    let tx_state = &state.tx_state_manager;

    // Update state
    tx_state.set_state(request.transaction_id.clone(), TransactionStatus::Signing, None);

    // Load transaction from state manager
    let mut transaction = tx_state.get_transaction(&request.transaction_id)
        .ok_or_else(|| format!("Transaction {} not found", request.transaction_id))?;

    // Emit signing started event
    let inputs_count = transaction.inputs.len();
    let _ = app.emit("transaction:signing_started", TransactionEvent::SigningStarted {
        transaction_id: request.transaction_id.clone(),
        inputs_to_sign: inputs_count,
    });

    // Load encrypted wallet file (.dat format with Argon2id encryption)
    let wallet_path = std::path::PathBuf::from(&request.wallet_id)
        .with_extension("dat");
    let encrypted_wallet = btpc_core::crypto::EncryptedWallet::load_from_file(&wallet_path)
        .map_err(|e| format!("Failed to load encrypted wallet: {}", e))?;

    // Decrypt wallet with Argon2id
    let secure_password = btpc_core::crypto::SecurePassword::new(request.password.clone());
    let wallet_data = encrypted_wallet.decrypt(&secure_password)
        .map_err(|e| format!("Failed to decrypt wallet (wrong password?): {}", e))?;

    // T015.1: Validate wallet integrity before signing
    validate_wallet_integrity(&wallet_data, &wallet_path)
        .map_err(|e| {
            // Emit wallet corruption failure event
            let _ = app.emit("transaction:failed", TransactionEvent::TransactionFailed {
                transaction_id: Some(request.transaction_id.clone()),
                stage: crate::events::TransactionStage::Signing,
                error_type: "WALLET_CORRUPTED".to_string(),
                error_message: e.clone(),
                recoverable: false,
                suggested_action: Some("Restore wallet from backup or seed phrase".to_string()),
            });
            e
        })?;

    // Get the first key from the wallet
    let key_entry = wallet_data.keys.first()
        .ok_or_else(|| "Wallet has no keys".to_string())?;

    // T024: Use KeyEntry's to_private_key() method (uses seed if available for signing!)
    // This enables transaction signing with seed regeneration
    let private_key = key_entry.to_private_key()
        .map_err(|e| format!("Failed to load private key: {}", e))?;

    // Get public key for script creation
    let public_key = private_key.public_key();

    // T025: Sign each input with ML-DSA using proper transaction serialization
    // Serialize transaction WITHOUT signatures (critical for correct signing!)
    let tx_data = serialize_for_signature(&transaction);

    for (i, input) in transaction.inputs.iter_mut().enumerate() {
        // Sign the properly serialized transaction data (matches blockchain validation)
        let signature = private_key.sign(&tx_data)
            .map_err(|e| format!("Failed to sign input {}: {}", i, e))?;

        // Create P2PKH unlock script with signature + public key (matches blockchain format)
        let unlock_script = Script::unlock_p2pkh(&signature.to_bytes(), &public_key.to_bytes());

        // Convert Script to raw bytes
        input.signature_script = unlock_script.to_bytes();

        // Emit input_signed event
        let _ = app.emit("transaction:input_signed", TransactionEvent::InputSigned {
            transaction_id: request.transaction_id.clone(),
            input_index: i,
            signature_algorithm: "ML-DSA-87".to_string(),
        });

        println!("✅ Signed input {} with ML-DSA signature + P2PKH script", i);
    }

    // Update transaction in state manager
    tx_state.set_transaction(request.transaction_id.clone(), transaction, TransactionStatus::Signed);

    // Emit transaction signed event
    let _ = app.emit("transaction:signed", TransactionEvent::TransactionSigned {
        transaction_id: request.transaction_id.clone(),
        signatures_count: inputs_count,
        ready_to_broadcast: true,
    });

    println!("✅ Transaction signed with {} signatures", inputs_count);

    Ok(SignTransactionResponse {
        transaction_id: request.transaction_id,
        signatures_count: inputs_count,
        ready_to_broadcast: true,
    })
}

/// Request to broadcast a transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroadcastTransactionRequest {
    pub transaction_id: String,
}

/// Response from broadcast_transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroadcastTransactionResponse {
    pub transaction_id: String,
    pub broadcast_to_peers: usize,
    pub mempool_accepted: bool,
}

/// T028: Broadcast transaction to network (THIN WRAPPER - TD-001)
///
/// This is now a thin wrapper that:
/// 1. Extracts Tauri-specific concerns (State, AppHandle, events)
/// 2. Calls broadcast_transaction_core() with pure business logic
/// 3. Emits events based on results
#[tauri::command]
pub async fn broadcast_transaction(
    state: State<'_, AppState>,
    request: BroadcastTransactionRequest,
    app: AppHandle,
) -> Result<BroadcastTransactionResponse, String> {
    println!("📡 Broadcasting transaction: {}", request.transaction_id);

    let tx_state = &state.tx_state_manager;

    // Update state to Broadcasting
    tx_state.set_state(request.transaction_id.clone(), TransactionStatus::Broadcasting, None);

    // Load signed transaction
    let transaction = tx_state.get_transaction(&request.transaction_id)
        .ok_or_else(|| format!("Transaction {} not found", request.transaction_id))?;

    // Get RPC client
    let rpc_port = *state.active_rpc_port.read().await;
    let rpc_client = btpc_desktop_app::rpc_client::RpcClient::new("127.0.0.1", rpc_port);

    // Call core business logic (TD-001: testable without Tauri!)
    match transaction_commands_core::broadcast_transaction_core(
        transaction,
        &rpc_client,
        tx_state,
        &request.transaction_id,
    ).await {
        Ok(result) => {
            println!("✅ Transaction broadcast successful: {}", result.transaction_id);

            // Emit transaction broadcast event (Tauri-specific)
            let _ = app.emit("transaction:broadcast", TransactionEvent::TransactionBroadcast {
                transaction_id: result.transaction_id.clone(),
                broadcast_to_peers: result.broadcast_to_peers,
                network_response: result.network_response.clone(),
            });

            // Emit mempool accepted event
            let _ = app.emit("transaction:mempool_accepted", TransactionEvent::MempoolAccepted {
                transaction_id: result.transaction_id.clone(),
                mempool_size: 0, // Will be updated by sync service
                position: 0,
            });

            // Note: UTXO reservations are automatically released by the transaction monitor
            // (transaction_monitor.rs) when the transaction is confirmed.
            // See transaction_monitor.rs:163 for implementation.

            println!("✅ Transaction broadcast to network");

            Ok(BroadcastTransactionResponse {
                transaction_id: result.transaction_id,
                broadcast_to_peers: result.broadcast_to_peers,
                mempool_accepted: result.mempool_accepted,
            })
        }
        Err(e) => {
            let error_msg = e.to_string();
            println!("❌ {}", error_msg);

            // Emit failed event (Tauri-specific)
            let _ = app.emit("transaction:failed", TransactionEvent::TransactionFailed {
                transaction_id: Some(request.transaction_id.clone()),
                stage: crate::events::TransactionStage::Broadcasting,
                error_type: "NetworkError".to_string(),
                error_message: error_msg.clone(),
                recoverable: true,
                suggested_action: Some("Check node connection and try again".to_string()),
            });

            Err(error_msg)
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

    // CRITICAL FIX: Fork ID for replay protection (must match btpc-core validation!)
    // This byte MUST be included for signatures to be valid
    bytes.push(tx.fork_id);

    bytes
}

/// T015.1: Validate wallet file integrity before signing
///
/// Checks for:
/// - Required fields presence (wallet_id, network, keys)
/// - Non-empty keys array
/// - Key structure integrity (private_key_bytes, public_key_bytes, address)
/// - File truncation detection (key size validation)
///
/// Returns Ok(()) if wallet is valid, Err(String) with specific corruption details otherwise.
fn validate_wallet_integrity(
    wallet_data: &btpc_core::crypto::WalletData,
    wallet_path: &std::path::Path,
) -> Result<(), String> {
    // Check 1: Validate wallet_id is not empty
    if wallet_data.wallet_id.is_empty() {
        return Err("Wallet corruption detected: wallet_id field is empty".to_string());
    }

    // Check 2: Validate network is not empty
    if wallet_data.network.is_empty() {
        return Err("Wallet corruption detected: network field is missing or empty".to_string());
    }

    // Check 3: Validate keys array is not empty
    if wallet_data.keys.is_empty() {
        return Err("Wallet corruption detected: wallet has no keys (keys array is empty)".to_string());
    }

    // Check 4: Validate each key entry structure
    for (i, key_entry) in wallet_data.keys.iter().enumerate() {
        // Check private key bytes size (ML-DSA-65 = 4000 bytes)
        const EXPECTED_PRIVATE_KEY_SIZE: usize = 4000;
        if key_entry.private_key_bytes.len() != EXPECTED_PRIVATE_KEY_SIZE {
            return Err(format!(
                "Wallet corruption detected: key {} has invalid private key size (expected {}, got {}). File may be truncated.",
                i,
                EXPECTED_PRIVATE_KEY_SIZE,
                key_entry.private_key_bytes.len()
            ));
        }

        // Check public key bytes size (ML-DSA-65 = 1952 bytes)
        const EXPECTED_PUBLIC_KEY_SIZE: usize = 1952;
        if key_entry.public_key_bytes.len() != EXPECTED_PUBLIC_KEY_SIZE {
            return Err(format!(
                "Wallet corruption detected: key {} has invalid public key size (expected {}, got {}). File may be truncated.",
                i,
                EXPECTED_PUBLIC_KEY_SIZE,
                key_entry.public_key_bytes.len()
            ));
        }

        // Check seed size if present (should be 32 bytes)
        if let Some(seed) = &key_entry.seed {
            const EXPECTED_SEED_SIZE: usize = 32;
            if seed.len() != EXPECTED_SEED_SIZE {
                return Err(format!(
                    "Wallet corruption detected: key {} has invalid seed size (expected {}, got {})",
                    i,
                    EXPECTED_SEED_SIZE,
                    seed.len()
                ));
            }
        }

        // Check address is not empty
        if key_entry.address.is_empty() {
            return Err(format!(
                "Wallet corruption detected: key {} has empty address field",
                i
            ));
        }

        // Check label is not empty (should at least have a default value)
        if key_entry.label.is_empty() {
            // Warning, not fatal
            println!("⚠️  Warning: key {} has empty label (non-critical)", i);
        }
    }

    // Check 5: Validate file exists and has reasonable size
    // (This helps detect partial file writes or filesystem corruption)
    if let Ok(metadata) = std::fs::metadata(wallet_path) {
        let file_size = metadata.len();
        const MIN_WALLET_FILE_SIZE: u64 = 100; // Magic bytes + version + salt + nonce + minimal data
        const MAX_WALLET_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10 MB max (very generous)

        if file_size < MIN_WALLET_FILE_SIZE {
            return Err(format!(
                "Wallet corruption detected: file size ({} bytes) is too small. File may be truncated.",
                file_size
            ));
        }

        if file_size > MAX_WALLET_FILE_SIZE {
            return Err(format!(
                "Wallet corruption detected: file size ({} bytes) exceeds maximum ({}). File may be corrupted.",
                file_size,
                MAX_WALLET_FILE_SIZE
            ));
        }
    }

    // Check 6: Validate timestamps are reasonable
    // (Helps detect deserialization issues)
    let current_timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    if wallet_data.created_at == 0 || wallet_data.modified_at == 0 {
        return Err("Wallet corruption detected: invalid timestamps (zero values)".to_string());
    }

    // Future timestamps indicate corruption
    if wallet_data.created_at > current_timestamp + 86400 ||
       wallet_data.modified_at > current_timestamp + 86400 {
        return Err("Wallet corruption detected: timestamps are in the future (clock skew or corruption)".to_string());
    }

    println!("✅ Wallet integrity check passed: {} keys validated", wallet_data.keys.len());
    Ok(())
}

/// T029: Get transaction status (THIN WRAPPER - TD-001)
///
/// This is now a thin wrapper that:
/// 1. Extracts Tauri State
/// 2. Calls get_transaction_status_core() with pure business logic
/// 3. Maps error to String for Tauri
#[tauri::command]
pub async fn get_transaction_status(
    state: State<'_, AppState>,
    transaction_id: String,
) -> Result<TransactionState, String> {
    println!("🔍 Getting status for transaction: {}", transaction_id);

    // Call core business logic (TD-001: testable without Tauri!)
    transaction_commands_core::get_transaction_status_core(&state.tx_state_manager, &transaction_id)
        .map_err(|e| e.to_string())
}

/// T030: Cancel transaction (THIN WRAPPER - TD-001)
///
/// This is now a thin wrapper that:
/// 1. Calls cancel_transaction_core() for validation and state update
/// 2. Handles UTXO reservation release (side effect)
/// 3. Emits events (Tauri-specific)
#[tauri::command]
pub async fn cancel_transaction(
    state: State<'_, AppState>,
    transaction_id: String,
    app: AppHandle,
) -> Result<(), String> {
    println!("❌ Cancelling transaction: {}", transaction_id);

    // Call core business logic (TD-001: testable without Tauri!)
    // This validates and updates state to Cancelled
    let tx_status = transaction_commands_core::cancel_transaction_core(&state.tx_state_manager, &transaction_id)
        .map_err(|e| e.to_string())?;

    // Release UTXO reservations using stored reservation token (Tauri-specific side effect)
    if let (Some(reservation_token), Some(utxo_keys)) = (tx_status.reservation_token, tx_status.utxo_keys.clone()) {
        let utxo_manager = state.utxo_manager.lock().expect("Mutex poisoned");
        match utxo_manager.release_reservation(&reservation_token) {
            Ok(_) => {
                println!("✅ Released UTXO reservation: {}", reservation_token);

                // Emit UTXO released event (Tauri-specific)
                let _ = app.emit("utxo:released", UTXOEvent::UTXOReleased {
                    reservation_token: reservation_token.clone(),
                    reason: ReleaseReason::TransactionCancelled,
                    utxo_count: utxo_keys.len(),
                });
            }
            Err(e) => {
                println!("⚠️  Failed to release UTXO reservation: {}", e);
            }
        }
    }

    // Emit transaction failed event (cancelled) (Tauri-specific)
    let _ = app.emit("transaction:failed", TransactionEvent::TransactionFailed {
        transaction_id: Some(transaction_id.clone()),
        stage: crate::events::TransactionStage::Validation,
        error_type: "Cancelled".to_string(),
        error_message: "Transaction cancelled by user".to_string(),
        recoverable: false,
        suggested_action: None,
    });

    println!("✅ Transaction cancelled: {}", transaction_id);

    Ok(())
}

/// Request to estimate transaction fee
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstimateFeeRequest {
    pub from_address: String,
    pub to_address: String,
    pub amount: u64,
    pub fee_rate: Option<u64>,
}

/// Response from estimate_fee
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstimateFeeResponse {
    pub estimated_fee: u64,
    pub estimated_size: usize,
    pub fee_rate: u64,
    pub inputs_count: usize,
    pub outputs_count: usize,
}

/// T031: Estimate transaction fee
#[tauri::command]
pub async fn estimate_fee(
    state: State<'_, AppState>,
    request: EstimateFeeRequest,
) -> Result<EstimateFeeResponse, String> {
    println!("💰 Estimating fee:");
    println!("  Amount: {} satoshis", request.amount);

    let utxo_manager = state.utxo_manager.lock().expect("Mutex poisoned");

    // Select UTXOs (without reserving)
    let utxos = utxo_manager
        .select_utxos_for_amount(&request.from_address, request.amount + 500_000)
        .map_err(|e| format!("Failed to select UTXOs: {}", e))?;

    if utxos.is_empty() {
        return Err("No UTXOs available".to_string());
    }

    // Build transaction to get fee estimate
    let fee_rate = request.fee_rate.unwrap_or(100);
    let builder = TransactionBuilder::new()
        .add_recipient(&request.to_address, request.amount)
        .select_utxos(&utxos)
        .set_fee_rate(fee_rate)
        .set_change_address(&request.from_address);

    let summary = builder.summary()
        .map_err(|e| format!("Failed to estimate: {}", e))?;

    println!("✅ Estimated fee: {} satoshis", summary.fee);

    Ok(EstimateFeeResponse {
        estimated_fee: summary.fee,
        estimated_size: summary.estimated_size,
        fee_rate,
        inputs_count: summary.num_inputs,
        outputs_count: summary.num_outputs,
    })
}

// Tests for TransactionStateManager have been moved to transaction_state.rs module (TD-001)