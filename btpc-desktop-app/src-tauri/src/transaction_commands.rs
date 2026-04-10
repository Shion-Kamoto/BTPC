//! Transaction Commands - Feature 007 (T023-T031)
//!
//! Tauri commands for transaction operations:
//! - T026: create_transaction - Build unsigned transaction
//! - T027: sign_transaction - Sign with ML-DSA (using seed)
//! - T028: broadcast_transaction - Send to network
//! - T029: get_transaction_status - Query transaction state
//! - T030: cancel_transaction - Cancel pending transaction
//! - T031: estimate_fee - Calculate transaction fee

use crate::auth_state::SessionState;
use crate::config::NetworkType;
use crate::events::{ReleaseReason, TransactionEvent, UTXOEvent};
use crate::AppState;
use btpc_core::crypto::{Address, Script};
use btpc_desktop_app::transaction_builder::{TransactionBuilder, TransactionSummary};
use btpc_desktop_app::transaction_commands_core; // TD-001: Import core business logic from lib.rs
use btpc_desktop_app::transaction_state::{
    TransactionState, TransactionStatus,
};
use btpc_desktop_app::utxo_manager::Transaction;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::RwLock;
use tauri::{AppHandle, Emitter, State};

// FIX 2026-02-21 (H1): Helper to verify session is authenticated before transaction operations
fn require_authenticated_session(session: &RwLock<SessionState>) -> Result<(), String> {
    let state = session
        .read()
        .expect("SessionState RwLock poisoned");
    if !state.is_authenticated() {
        return Err("SESSION_NOT_AUTHENTICATED: Login required for transaction operations".to_string());
    }
    Ok(())
}

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
    session: State<'_, RwLock<SessionState>>,
    request: CreateTransactionRequest,
    app: AppHandle,
) -> Result<CreateTransactionResponse, String> {
    // FIX 2026-02-21 (H1): Verify session is authenticated before creating transaction
    require_authenticated_session(&session)?;

    println!("🔨 Creating transaction:");
    println!("  Wallet: {}", request.wallet_id);
    println!("  From: {}", request.from_address);
    println!("  To: {}", request.to_address);
    println!("  Amount: {} credits", request.amount);

    // Emit transaction initiated event
    let _ = app.emit(
        "transaction:initiated",
        TransactionEvent::TransactionInitiated {
            wallet_id: request.wallet_id.clone(),
            recipient: request.to_address.clone(),
            amount: request.amount,
            timestamp: Utc::now(),
        },
    );

    // Validate addresses
    if let Err(e) = Address::from_string(&request.from_address) {
        return Err(format!("Invalid from address: {}", e));
    }
    if let Err(e) = Address::from_string(&request.to_address) {
        return Err(format!("Invalid to address: {}", e));
    }

    // Get tx_state from AppState
    let tx_state = &state.tx_state_manager;

    // Get current blockchain height for coinbase maturity check
    let current_height = {
        let node = state.embedded_node.read().await;
        node.get_height()
    };

    // Scope the utxo_manager lock to ensure it's dropped before async operations
    let (utxos, utxo_keys, reservation, temp_tx_id, total_utxo_value, inputs_count) = {
        let utxo_manager = state.utxo_manager.lock().expect("Mutex poisoned");

        // Select UTXOs (with coinbase maturity enforcement)
        let utxos = utxo_manager
            .select_utxos_for_amount(&request.from_address, request.amount + 500_000, current_height) // Add buffer for fee
            .map_err(|e| format!("Failed to select UTXOs: {}", e))?;

        if utxos.is_empty() {
            return Err("No UTXOs available for transaction".to_string());
        }

        // Reserve UTXOs
        let utxo_keys: Vec<String> = utxos
            .iter()
            .map(|utxo| format!("{}:{}", utxo.txid, utxo.vout))
            .collect();

        let temp_tx_id = format!("tx_{}", Utc::now().timestamp_nanos_opt().unwrap_or(0));
        let reservation = utxo_manager
            .reserve_utxos(utxo_keys.clone(), Some(temp_tx_id.clone()))
            .map_err(|e| format!("Failed to reserve UTXOs: {}", e))?;

        let total_utxo_value: u64 = utxos.iter().map(|u| u.value_credits).sum();
        let inputs_count = utxos.len();

        // Return all needed values, utxo_manager drops here
        (
            utxos,
            utxo_keys,
            reservation,
            temp_tx_id,
            total_utxo_value,
            inputs_count,
        )
    };

    // Emit UTXO reserved event (after lock is dropped)
    let _ = app.emit(
        "utxo:reserved",
        UTXOEvent::UTXOReserved {
            reservation_token: reservation.id.clone(),
            transaction_id: Some(temp_tx_id.clone()),
            utxo_count: inputs_count,
            total_amount: total_utxo_value,
            expires_at: Utc::now() + chrono::Duration::minutes(5),
        },
    );

    // T018: Dynamic fee estimation using FeeEstimator service
    let fee_rate = if let Some(custom_rate) = request.fee_rate {
        // User provided custom fee rate - use it directly
        custom_rate
    } else {
        // Use dynamic fee estimation with embedded node (Feature 013: Self-contained app)
        let embedded_node = state.embedded_node.clone();
        let fee_estimator = crate::fee_estimator::FeeEstimator::new(embedded_node);

        // Estimate with expected outputs count (recipient + change = 2)
        let fee_estimate = fee_estimator
            .estimate_fee_for_transaction(inputs_count, 2)
            .await
            .map_err(|e| format!("Fee estimation failed: {}", e))?;

        // Emit fee estimated event
        let _ = app.emit(
            "fee:estimated",
            TransactionEvent::FeeEstimated {
                transaction_id: Some(temp_tx_id.clone()),
                estimated_fee: fee_estimate.estimated_fee,
                fee_rate: fee_estimate.fee_rate,
                estimated_size: fee_estimate.estimated_size,
            },
        );

        println!(
            "💰 Dynamic fee estimation: {} crd/KB (estimated {} credits for {} bytes)",
            fee_estimate.fee_rate, fee_estimate.estimated_fee, fee_estimate.estimated_size
        );

        fee_estimate.fee_rate
    };

    // FIX 2025-12-03: Get fork_id from active network for cross-network transaction validity
    let fork_id = {
        let network = state.active_network.read().await;
        match *network {
            NetworkType::Mainnet => 0,
            NetworkType::Testnet => 1,
            NetworkType::Regtest => 2,
        }
    };
    println!("  Fork ID: {} (network-aware)", fork_id);

    let builder = TransactionBuilder::new()
        .set_fork_id(fork_id) // FIX 2025-12-03: Critical for cross-network validity
        .add_recipient(&request.to_address, request.amount)
        .select_utxos(&utxos)
        .set_fee_rate_per_kb(fee_rate)
        .set_change_address(&request.from_address);

    // Validate transaction
    builder
        .validate()
        .map_err(|e| format!("Transaction validation failed: {}", e))?;

    // Get summary
    let summary = builder
        .summary()
        .map_err(|e| format!("Failed to get transaction summary: {}", e))?;

    // Build transaction
    let transaction = builder
        .build()
        .map_err(|e| format!("Failed to build transaction: {}", e))?;

    let transaction_id = transaction.txid.clone();

    // Store transaction with state and reservation info for later cleanup
    // FIX 2025-12-10: Include sender_address for reliable SENT transaction indexing
    tx_state.set_transaction_with_reservation(
        transaction_id.clone(),
        transaction.clone(),
        TransactionStatus::Validating,
        reservation.id.clone(),
        utxo_keys.clone(),
        request.wallet_id.clone(),
        request.from_address.clone(), // Sender address for broadcast indexing
    );

    // Emit transaction validated event
    let _ = app.emit(
        "transaction:validated",
        TransactionEvent::TransactionValidated {
            transaction_id: transaction_id.clone(),
            inputs_count: transaction.inputs.len(),
            outputs_count: transaction.outputs.len(),
            fee: summary.fee,
            change_amount: summary.change.unwrap_or(0),
            total_input: summary.total_input,
            total_output: summary.total_output,
        },
    );

    println!(
        "✅ Transaction created: {} (reservation: {})",
        transaction_id, reservation.id
    );

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
    session: State<'_, RwLock<SessionState>>,
    request: SignTransactionRequest,
    app: AppHandle,
) -> Result<SignTransactionResponse, String> {
    // FIX 2026-02-21 (H1): Verify session is authenticated before signing
    require_authenticated_session(&session)?;

    println!("✍️ Signing transaction: {}", request.transaction_id);

    let tx_state = &state.tx_state_manager;

    // Update state
    tx_state.set_state(
        request.transaction_id.clone(),
        TransactionStatus::Signing,
        None,
    );

    // Load transaction from state manager
    let mut transaction = tx_state
        .get_transaction(&request.transaction_id)
        .ok_or_else(|| format!("Transaction {} not found", request.transaction_id))?;

    // Emit signing started event
    let inputs_count = transaction.inputs.len();
    let _ = app.emit(
        "transaction:signing_started",
        TransactionEvent::SigningStarted {
            transaction_id: request.transaction_id.clone(),
            inputs_to_sign: inputs_count,
        },
    );

    // Load encrypted wallet file (.dat format with Argon2id encryption)
    // Get wallet info from wallet manager to find the correct file path
    let wallet_manager = state.wallet_manager.lock().map_err(|e| {
        format!("Failed to lock wallet manager: {}", e)
    })?;

    let wallet_info = wallet_manager
        .get_wallet(&request.wallet_id)
        .ok_or_else(|| format!("Wallet {} not found", request.wallet_id))?;

    let wallet_path = &wallet_info.file_path;
    println!("📂 Loading wallet from: {}", wallet_path.display());

    let encrypted_wallet = btpc_core::crypto::EncryptedWallet::load_from_file(wallet_path)
        .map_err(|e| format!("Failed to load encrypted wallet: {}", e))?;

    // Decrypt wallet with Argon2id
    let secure_password = btpc_core::crypto::SecurePassword::new(request.password.clone());
    let wallet_data = encrypted_wallet
        .decrypt(&secure_password)
        .map_err(|e| format!("Failed to decrypt wallet (wrong password?): {}", e))?;

    // T015.1: Validate wallet integrity before signing
    validate_wallet_integrity(&wallet_data, wallet_path).map_err(|e| {
        // Emit wallet corruption failure event
        let _ = app.emit(
            "transaction:failed",
            TransactionEvent::TransactionFailed {
                transaction_id: Some(request.transaction_id.clone()),
                stage: crate::events::TransactionStage::Signing,
                error_type: "WALLET_CORRUPTED".to_string(),
                error_message: e.clone(),
                recoverable: false,
                suggested_action: Some("Restore wallet from backup or seed phrase".to_string()),
            },
        );
        e
    })?;

    // Get the first key from the wallet
    let key_entry = wallet_data
        .keys
        .first()
        .ok_or_else(|| "Wallet has no keys".to_string())?;

    // T024: Use KeyEntry's to_private_key() method (uses seed if available for signing!)
    // This enables transaction signing with seed regeneration
    let private_key = key_entry
        .to_private_key()
        .map_err(|e| format!("Failed to load private key: {}", e))?;

    // Get public key for script creation
    let public_key = private_key.public_key();

    // T025: Sign each input with ML-DSA using proper transaction serialization
    // Serialize transaction WITHOUT signatures (critical for correct signing!)
    let tx_data = serialize_for_signature(&transaction);

    for (i, input) in transaction.inputs.iter_mut().enumerate() {
        // Sign the properly serialized transaction data (matches blockchain validation)
        let signature = private_key
            .sign(&tx_data)
            .map_err(|e| format!("Failed to sign input {}: {}", i, e))?;

        // Create P2PKH unlock script with signature + public key (matches blockchain format)
        let unlock_script = Script::unlock_p2pkh(&signature.to_bytes(), &public_key.to_bytes());

        // Convert Script to raw bytes
        input.signature_script = unlock_script.to_bytes();

        // Emit input_signed event
        let _ = app.emit(
            "transaction:input_signed",
            TransactionEvent::InputSigned {
                transaction_id: request.transaction_id.clone(),
                input_index: i,
                signature_algorithm: "ML-DSA-87".to_string(),
            },
        );

        println!("✅ Signed input {} with ML-DSA signature + P2PKH script", i);
    }

    // Update transaction in state manager
    tx_state.set_transaction(
        request.transaction_id.clone(),
        transaction,
        TransactionStatus::Signed,
    );

    // Emit transaction signed event
    let _ = app.emit(
        "transaction:signed",
        TransactionEvent::TransactionSigned {
            transaction_id: request.transaction_id.clone(),
            signatures_count: inputs_count,
            ready_to_broadcast: true,
        },
    );

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
    session: State<'_, RwLock<SessionState>>,
    request: BroadcastTransactionRequest,
    app: AppHandle,
) -> Result<BroadcastTransactionResponse, String> {
    // FIX 2026-02-21 (H1): Verify session is authenticated before broadcasting
    require_authenticated_session(&session)?;

    println!("📡 Broadcasting transaction: {}", request.transaction_id);

    let tx_state = &state.tx_state_manager;

    // Update state to Broadcasting
    tx_state.set_state(
        request.transaction_id.clone(),
        TransactionStatus::Broadcasting,
        None,
    );

    // Load signed transaction and sender address from state
    let tx_status = tx_state
        .get_state(&request.transaction_id)
        .ok_or_else(|| format!("Transaction {} not found", request.transaction_id))?;

    let transaction = tx_status
        .transaction
        .clone()
        .ok_or_else(|| format!("Transaction {} has no transaction data", request.transaction_id))?;

    // FIX 2025-12-10: Get sender address from state (stored during create_transaction)
    // This is much more reliable than UTXO lookups which often fail
    let sender_address = tx_status.sender_address.clone();

    // Verify transaction is signed
    let all_signed = transaction
        .inputs
        .iter()
        .all(|input| !input.signature_script.is_empty());
    if !all_signed {
        tx_state.set_state(
            request.transaction_id.clone(),
            TransactionStatus::Failed,
            Some("Transaction not fully signed".to_string()),
        );
        return Err("Transaction not fully signed - cannot broadcast".to_string());
    }

    // Convert desktop app Transaction to btpc-core Transaction for embedded node submission
    let core_transaction = convert_to_core_transaction(&transaction)?;

    // Submit transaction to embedded node's mempool (Feature 013: Self-contained app)
    // This replaces the external RPC broadcast with direct mempool access
    let embedded_node = state.embedded_node.read().await;
    match embedded_node.submit_transaction(core_transaction).await {
        Ok(txid) => {
            // FIX 2025-11-27: Migrate transaction state from temp ID to real txid
            // The temp ID (tx_timestamp) is replaced with the real blockchain txid
            tx_state.migrate_transaction_id(&request.transaction_id, txid.clone());

            // Update state to Broadcast (using new txid)
            tx_state.set_state(
                txid.clone(),
                TransactionStatus::Broadcast,
                None,
            );

            println!("✅ Transaction broadcast successful: {}", txid);
            println!("   (migrated from temp ID: {})", request.transaction_id);

            // FIX 2025-12-04: Save transaction to tx_storage for transaction history
            // This ensures sent transactions appear in the transaction list immediately
            {
                // Create a copy with the real txid for storage
                let mut storage_tx = transaction.clone();
                storage_tx.txid = txid.clone();
                storage_tx.block_height = None; // Pending - not yet confirmed
                storage_tx.confirmed_at = None;
                storage_tx.is_coinbase = false;
                // FIX 2025-12-10: Set sender_address for reliable SENT/RECEIVED detection
                // This allows frontend to determine if tx is SENT (sender == wallet) or RECEIVED
                storage_tx.sender_address = sender_address.clone();

                // Get network for address decoding
                let network = state.active_network.read().await.clone();

                // Collect all involved addresses (sender + recipients)
                let mut involved_addresses: Vec<String> = Vec::new();

                // Decode addresses from script_pubkey in outputs (recipients)
                for output in &storage_tx.outputs {
                    // Deserialize script and extract pubkey hash
                    if let Ok(script) = Script::deserialize(&output.script_pubkey) {
                        if let Some(pubkey_hash) = script.extract_pubkey_hash() {
                            let address = Address::from_hash(
                                pubkey_hash,
                                network.clone().into(),
                                btpc_core::crypto::address::AddressType::P2PKH,
                            ).to_string();
                            if !involved_addresses.contains(&address) {
                                involved_addresses.push(address);
                            }
                        }
                    }
                }

                // FIX 2025-12-10: Use sender_address from TransactionState (stored during create_transaction)
                // This replaces the fragile UTXO lookups that often fail
                // The sender address is known at creation time and stored reliably in tx_state
                if let Some(ref sender_addr) = sender_address {
                    if !involved_addresses.contains(sender_addr) {
                        involved_addresses.push(sender_addr.clone());
                        println!("📝 Added sender address {} from tx_state for broadcast tx", &sender_addr[..20.min(sender_addr.len())]);
                    }
                } else {
                    // Fallback: Try UTXO lookups if sender_address wasn't stored (legacy transactions)
                    eprintln!("⚠️ No sender_address in tx_state, trying UTXO fallback");
                    if let Ok(utxo_manager) = state.utxo_manager.lock() {
                        for input in &storage_tx.inputs {
                            if let Some(utxo) = utxo_manager.get_utxo(&input.prev_txid, input.prev_vout) {
                                if !involved_addresses.contains(&utxo.address) {
                                    involved_addresses.push(utxo.address.clone());
                                    println!("📝 Added sender address {} from UTXO fallback", &utxo.address[..20.min(utxo.address.len())]);
                                }
                            }
                        }
                    }
                }

                // Save to tx_storage for each involved address
                // FIX 2025-12-05: Use .read().await for RwLock access
                let tx_storage_guard = state.tx_storage.read().await;
                for address in &involved_addresses {
                    if let Err(e) = tx_storage_guard.add_transaction(&storage_tx, address) {
                        eprintln!("⚠️ Failed to save broadcast tx to storage for {}: {}", address, e);
                    } else {
                        println!("📝 Saved broadcast tx {} for address {}", &txid[..16.min(txid.len())], address);
                    }
                }

                // FIX 2025-12-05: Flush tx_storage to disk for crash safety
                // Ensures broadcast transactions persist even if app crashes
                if let Err(e) = tx_storage_guard.flush() {
                    eprintln!("⚠️ Failed to flush tx_storage after broadcast: {}", e);
                } else {
                    println!("💾 tx_storage flushed after broadcast tx {}", &txid[..16.min(txid.len())]);
                }
            }

            // Emit transaction broadcast event (Tauri-specific)
            let _ = app.emit(
                "transaction:broadcast",
                TransactionEvent::TransactionBroadcast {
                    transaction_id: txid.clone(),
                    broadcast_to_peers: 1, // Direct to external RPC node
                    network_response: "accepted by external node mempool".to_string(),
                },
            );

            // Emit mempool accepted event
            let _ = app.emit(
                "transaction:mempool_accepted",
                TransactionEvent::MempoolAccepted {
                    transaction_id: txid.clone(),
                    mempool_size: 0, // Will be updated by sync service
                    position: 0,
                },
            );

            // FIX 2026-02-21 (C5): Mark input UTXOs as spent IMMEDIATELY on successful broadcast
            // This closes the double-spend race window where another transaction could select
            // the same UTXOs between broadcast and block confirmation.
            // Previously, UTXOs stayed "reserved" but could be selected by other wallets.
            {
                let mut utxo_manager = state.utxo_manager.lock().expect("UTXO mutex poisoned");
                for input in &transaction.inputs {
                    let _ = utxo_manager.mark_utxo_as_spent(&input.prev_txid, input.prev_vout);
                }
                // Also release the reservation since UTXOs are now spent
                let utxo_keys: Vec<String> = transaction
                    .inputs
                    .iter()
                    .map(|i| format!("{}:{}", i.prev_txid, i.prev_vout))
                    .collect();
                utxo_manager.release_utxos(&utxo_keys);
                println!("🔒 Marked {} input UTXOs as spent after broadcast", transaction.inputs.len());
            }

            println!("✅ Transaction broadcast to network");

            Ok(BroadcastTransactionResponse {
                transaction_id: txid,
                broadcast_to_peers: 1,
                mempool_accepted: true,
            })
        }
        Err(e) => {
            let error_msg = e.to_string();
            println!("❌ {}", error_msg);

            tx_state.set_state(
                request.transaction_id.clone(),
                TransactionStatus::Failed,
                Some(error_msg.clone()),
            );

            // Emit failed event (Tauri-specific)
            let _ = app.emit(
                "transaction:failed",
                TransactionEvent::TransactionFailed {
                    transaction_id: Some(request.transaction_id.clone()),
                    stage: crate::events::TransactionStage::Broadcasting,
                    error_type: "MempoolError".to_string(),
                    error_message: error_msg.clone(),
                    recoverable: true,
                    suggested_action: Some("Check transaction validity and try again".to_string()),
                },
            );

            Err(error_msg)
        }
    }
}

/// Helper: Convert desktop app Transaction to btpc-core Transaction
/// Needed for submitting to embedded node's mempool
fn convert_to_core_transaction(
    tx: &Transaction,
) -> Result<btpc_core::blockchain::Transaction, String> {
    use btpc_core::blockchain::{
        OutPoint, Transaction as CoreTx, TransactionInput, TransactionOutput,
    };
    use btpc_core::crypto::{Hash, Script};

    // Convert inputs
    let inputs: Result<Vec<TransactionInput>, String> = tx
        .inputs
        .iter()
        .map(|input| {
            // Parse txid as Hash (64-byte SHA-512 hash)
            let prev_hash =
                Hash::from_hex(&input.prev_txid).map_err(|e| format!("Invalid txid: {}", e))?;

            // Deserialize signature_script Vec<u8> to Script type
            let script_sig = Script::deserialize(&input.signature_script)
                .map_err(|e| format!("Invalid signature script: {}", e))?;

            Ok(TransactionInput {
                previous_output: OutPoint {
                    txid: prev_hash,
                    vout: input.prev_vout,
                },
                script_sig,
                sequence: input.sequence,
            })
        })
        .collect();

    // Convert outputs
    let outputs: Result<Vec<TransactionOutput>, String> = tx
        .outputs
        .iter()
        .map(|output| {
            // Deserialize script_pubkey Vec<u8> to Script type
            let script_pubkey = Script::deserialize(&output.script_pubkey)
                .map_err(|e| format!("Invalid script pubkey: {}", e))?;

            Ok(TransactionOutput {
                value: output.value,
                script_pubkey,
            })
        })
        .collect();

    Ok(CoreTx {
        version: tx.version,
        inputs: inputs?,
        outputs: outputs?,
        lock_time: tx.lock_time,
        fork_id: tx.fork_id,
    })
}

/// Helper: Serialize desktop app Transaction to bytes
/// Converts to wire format for RPC submission
#[allow(dead_code)]
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
        return Err(
            "Wallet corruption detected: wallet has no keys (keys array is empty)".to_string(),
        );
    }

    // Check 4: Validate each key entry structure
    for (i, key_entry) in wallet_data.keys.iter().enumerate() {
        // Check private key bytes size (ML-DSA-87 = 4864 bytes)
        const EXPECTED_PRIVATE_KEY_SIZE: usize = 4864;
        if key_entry.private_key_bytes.len() != EXPECTED_PRIVATE_KEY_SIZE {
            return Err(format!(
                "Wallet corruption detected: key {} has invalid private key size (expected {}, got {}). File may be truncated.",
                i,
                EXPECTED_PRIVATE_KEY_SIZE,
                key_entry.private_key_bytes.len()
            ));
        }

        // Check public key bytes size (ML-DSA-87 = 2592 bytes)
        const EXPECTED_PUBLIC_KEY_SIZE: usize = 2592;
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
    if wallet_data.created_at > current_timestamp + 86400
        || wallet_data.modified_at > current_timestamp + 86400
    {
        return Err(
            "Wallet corruption detected: timestamps are in the future (clock skew or corruption)"
                .to_string(),
        );
    }

    println!(
        "✅ Wallet integrity check passed: {} keys validated",
        wallet_data.keys.len()
    );
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
    session: State<'_, RwLock<SessionState>>,
    transaction_id: String,
    app: AppHandle,
) -> Result<(), String> {
    // FIX 2026-02-21 (H1): Verify session is authenticated before cancelling
    require_authenticated_session(&session)?;

    println!("❌ Cancelling transaction: {}", transaction_id);

    // Call core business logic (TD-001: testable without Tauri!)
    // This validates and updates state to Cancelled
    let tx_status = transaction_commands_core::cancel_transaction_core(
        &state.tx_state_manager,
        &transaction_id,
    )
    .map_err(|e| e.to_string())?;

    // Release UTXO reservations using stored reservation token (Tauri-specific side effect)
    if let (Some(reservation_token), Some(utxo_keys)) =
        (tx_status.reservation_token, tx_status.utxo_keys.clone())
    {
        let utxo_manager = state.utxo_manager.lock().expect("Mutex poisoned");
        match utxo_manager.release_reservation(&reservation_token) {
            Ok(_) => {
                println!("✅ Released UTXO reservation: {}", reservation_token);

                // Emit UTXO released event (Tauri-specific)
                let _ = app.emit(
                    "utxo:released",
                    UTXOEvent::UTXOReleased {
                        reservation_token: reservation_token.clone(),
                        reason: ReleaseReason::TransactionCancelled,
                        utxo_count: utxo_keys.len(),
                    },
                );
            }
            Err(e) => {
                println!("⚠️  Failed to release UTXO reservation: {}", e);
            }
        }
    }

    // Emit transaction failed event (cancelled) (Tauri-specific)
    let _ = app.emit(
        "transaction:failed",
        TransactionEvent::TransactionFailed {
            transaction_id: Some(transaction_id.clone()),
            stage: crate::events::TransactionStage::Validation,
            error_type: "Cancelled".to_string(),
            error_message: "Transaction cancelled by user".to_string(),
            recoverable: false,
            suggested_action: None,
        },
    );

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
    println!("  Amount: {} credits", request.amount);

    // FIX 2025-12-03: Get fork_id from active network for consistent fee estimation
    // Get this BEFORE locking utxo_manager to avoid deadlock
    let fork_id = {
        let network = state.active_network.read().await;
        match *network {
            NetworkType::Mainnet => 0,
            NetworkType::Testnet => 1,
            NetworkType::Regtest => 2,
        }
    };

    // Get current blockchain height for coinbase maturity check
    let current_height = {
        let node = state.embedded_node.read().await;
        node.get_height()
    };

    let utxo_manager = state.utxo_manager.lock().expect("Mutex poisoned");

    // Select UTXOs (without reserving, with maturity check)
    let utxos = utxo_manager
        .select_utxos_for_amount(&request.from_address, request.amount + 500_000, current_height)
        .map_err(|e| format!("Failed to select UTXOs: {}", e))?;

    if utxos.is_empty() {
        return Err("No UTXOs available".to_string());
    }

    // Build transaction to get fee estimate (default 10 crd/KB)
    let fee_rate = request.fee_rate.unwrap_or(10);

    let builder = TransactionBuilder::new()
        .set_fork_id(fork_id)
        .add_recipient(&request.to_address, request.amount)
        .select_utxos(&utxos)
        .set_fee_rate_per_kb(fee_rate)
        .set_change_address(&request.from_address);

    let summary = builder
        .summary()
        .map_err(|e| format!("Failed to estimate: {}", e))?;

    println!("✅ Estimated fee: {} credits", summary.fee);

    Ok(EstimateFeeResponse {
        estimated_fee: summary.fee,
        estimated_size: summary.estimated_size,
        fee_rate,
        inputs_count: summary.num_inputs,
        outputs_count: summary.num_outputs,
    })
}

// Tests for TransactionStateManager have been moved to transaction_state.rs module (TD-001)
