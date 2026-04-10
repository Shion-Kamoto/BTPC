//! Transaction storage Tauri commands for SQLite-based transaction management
//!
//! These commands provide frontend access to the SQLite transaction history
//! for paginated queries, balance lookups, and mining history.
//!
//! FIX 2025-12-12: Replaced RocksDB tx_storage with SQLite tx_history
//! Constitution Article V: Structured logging for all database operations
//! Constitution Article XI.1: Backend is single source of truth

use tauri::{Emitter, State};

use crate::error::BtpcError;
use btpc_desktop_app::tx_history::{PaginatedTransactions, PaginationParams, TransactionWithOutputs};
use crate::AppState;

/// Get transaction history (DEPRECATED - O(n×m) complexity)
/// Use get_paginated_transaction_history for list queries
/// Use get_transaction_from_storage for single transaction lookups
#[tauri::command]
pub async fn get_transaction_history(
    state: State<'_, AppState>,
) -> Result<Vec<btpc_desktop_app::utxo_manager::Transaction>, String> {
    // ⚠️ DEPRECATED: This command is inefficient (O(n×m) complexity)
    // Use get_paginated_transaction_history for list queries
    // Use get_transaction_from_storage for single transaction lookups
    //
    // PERFORMANCE ISSUE: Scans all transactions × all UTXOs
    // - 10k transactions = 5s load time
    // - 100k transactions = 30s+ app freeze
    //
    // Constitution Article XI.1 Compliance: SQLite is single source of truth
    eprintln!("⚠️  WARNING: get_transaction_history called (DEPRECATED)");
    eprintln!("   This command has O(n×m) complexity and causes severe performance degradation");
    eprintln!("   Use get_paginated_transaction_history or get_transaction_from_storage instead");

    // Get wallet address from WalletManager (not from file)
    // If no default wallet exists, return empty array instead of error
    let address = {
        let wallet_manager = state
            .wallet_manager
            .lock()
            .map_err(|e| format!("Failed to lock wallet manager: {}", e))?;

        match wallet_manager.get_default_wallet() {
            Some(wallet) => wallet.address.clone(),
            None => {
                // No default wallet - return empty transaction history
                return Ok(Vec::new());
            }
        }
    };

    let utxo_manager = state.utxo_manager.lock().map_err(|_| {
        BtpcError::mutex_poison("utxo_manager", "get_transaction_history").to_string()
    })?;
    let transactions: Vec<btpc_desktop_app::utxo_manager::Transaction> = utxo_manager
        .get_transaction_history(&address)
        .into_iter()
        .cloned()
        .collect();

    Ok(transactions)
}

/// Get paginated transaction history from SQLite
///
/// Per-Wallet Transaction Database Feature (Option A):
/// - wallet_id: Optional wallet ID to query. If None, uses default wallet.
/// - This allows viewing transaction history for ANY wallet, not just the default.
///
/// Constitution Article V: SQLite transaction storage with pagination
///
/// Filter parameter:
/// - None/"all" = all transactions
/// - "sent" = non-coinbase where sender_address = wallet address
/// - "received" = non-coinbase where sender_address != wallet address
/// - "mining" = coinbase transactions only
#[tauri::command]
pub async fn get_paginated_transaction_history(
    state: State<'_, AppState>,
    offset: usize,
    limit: usize,
    wallet_id: Option<String>,
    tx_type: Option<String>,
) -> Result<PaginatedTransactions, String> {
    // Resolve wallet address: use specified wallet_id or fall back to default
    let raw_address = {
        let wallet_manager = state
            .wallet_manager
            .lock()
            .map_err(|e| format!("Failed to lock wallet manager: {}", e))?;

        if let Some(ref id) = wallet_id {
            // Query specific wallet by ID
            match wallet_manager.get_wallet(id) {
                Some(wallet) => wallet.address.clone(),
                None => {
                    // Wallet ID not found - return empty result (not error for graceful handling)
                    tracing::warn!("Wallet ID '{}' not found, returning empty history", id);
                    return Ok(PaginatedTransactions {
                        transactions: Vec::new(),
                        total_count: 0,
                        offset,
                        limit,
                        has_more: false,
                    });
                }
            }
        } else {
            // No wallet_id specified - use default wallet (backward compatible)
            match wallet_manager.get_default_wallet() {
                Some(wallet) => wallet.address.clone(),
                None => {
                    // No default wallet - return empty paginated result
                    return Ok(PaginatedTransactions {
                        transactions: Vec::new(),
                        total_count: 0,
                        offset,
                        limit,
                        has_more: false,
                    });
                }
            }
        }
    };

    // FIX 2025-12-09: Clean address by stripping "Address: " prefix if present
    // Transaction storage uses clean addresses (from script_pubkey extraction)
    // but some wallets (especially default wallets created before fixes) may have the prefix
    // Without this cleaning, queries for default wallet transactions return empty results
    let address = if raw_address.starts_with("Address: ") {
        raw_address.strip_prefix("Address: ").unwrap_or(&raw_address).to_string()
    } else {
        raw_address.clone()
    };

    eprintln!(
        "📊 TX History Query: raw='{}' clean='{}' (wallet_id: {:?})",
        &raw_address[..std::cmp::min(30, raw_address.len())],
        &address[..std::cmp::min(30, address.len())],
        wallet_id
    );

    tracing::debug!(
        "Querying transaction history for {} (wallet_id: {:?}, tx_type: {:?})",
        &address[..std::cmp::min(20, address.len())],
        wallet_id,
        tx_type
    );

    // Query SQLite with pagination and optional type filter
    // FIX 2025-12-05: Use .read().await for RwLock access
    // FIX 2025-12-12: Added tx_type filter for backend filtering (sent/received/mining)
    let pagination = PaginationParams { offset, limit };
    let tx_type_ref = tx_type.as_deref();
    state
        .tx_storage
        .read()
        .await
        .get_transactions_for_address_filtered(&address, pagination, tx_type_ref)
        .map_err(|e| format!("Failed to get paginated transactions: {}", e))
}

/// Add a transaction to SQLite storage
/// Emits transaction-added and wallet-balance-updated events (Constitution Article XI.3)
#[tauri::command]
pub async fn add_transaction_to_storage(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    tx: btpc_desktop_app::utxo_manager::Transaction,
    address: String,
) -> Result<String, String> {
    // Add transaction to SQLite
    // FIX 2025-12-05: Use .read().await for RwLock access
    let tx_storage_guard = state.tx_storage.read().await;
    tx_storage_guard
        .add_transaction(&tx, &address)
        .map_err(|e| format!("Failed to add transaction to storage: {}", e))?;

    // Get updated balance and transaction count
    let (balance_credits, balance_btpc) = tx_storage_guard
        .get_balance(&address)
        .map_err(|e| format!("Failed to get updated balance: {}", e))?;
    let tx_count = tx_storage_guard
        .get_transaction_count(&address)
        .map_err(|e| format!("Failed to get transaction count: {}", e))?;

    // Emit transaction-added event (Constitution Article XI.3)
    let event_payload = serde_json::json!({
        "txid": tx.txid,
        "address": address,
        "block_height": tx.block_height,
        "is_coinbase": tx.is_coinbase,
        "output_count": tx.outputs.len(),
        "confirmed_at": tx.confirmed_at,
    });
    if let Err(e) = app.emit("transaction-added", event_payload) {
        eprintln!("⚠️ Failed to emit transaction-added event: {}", e);
    }

    // Emit wallet-balance-updated event
    let balance_payload = serde_json::json!({
        "address": address,
        "balance_credits": balance_credits,
        "balance_btpc": balance_btpc,
        "transaction_count": tx_count,
    });
    if let Err(e) = app.emit("wallet-balance-updated", balance_payload) {
        eprintln!("⚠️ Failed to emit wallet-balance-updated event: {}", e);
    }

    Ok(format!("Transaction {} added to storage", tx.txid))
}

/// Get a single transaction from SQLite by txid with addresses decoded
#[tauri::command]
pub async fn get_transaction_from_storage(
    state: State<'_, AppState>,
    txid: String,
) -> Result<Option<btpc_desktop_app::tx_history::TransactionWithOutputs>, String> {
    // FIX 2025-12-12: Uses SQLite tx_history instead of RocksDB tx_storage
    state
        .tx_storage
        .read()
        .await
        .get_transaction_with_addresses(&txid)
        .map_err(|e| format!("Failed to get transaction from storage: {}", e))
}

/// Get wallet balance from SQLite storage
///
/// Per-Wallet Transaction Database Feature (Option A):
/// - wallet_id: Optional wallet ID to query. If None, uses default wallet.
#[tauri::command]
pub async fn get_wallet_balance_from_storage(
    state: State<'_, AppState>,
    wallet_id: Option<String>,
) -> Result<serde_json::Value, String> {
    // Resolve wallet address: use specified wallet_id or fall back to default
    let (raw_address, wallet_nickname) = {
        let wallet_manager = state
            .wallet_manager
            .lock()
            .map_err(|e| format!("Failed to lock wallet manager: {}", e))?;

        if let Some(ref id) = wallet_id {
            match wallet_manager.get_wallet(id) {
                Some(wallet) => (wallet.address.clone(), Some(wallet.nickname.clone())),
                None => {
                    return Ok(serde_json::json!({
                        "credits": 0,
                        "btpc": 0.0,
                        "address": null,
                        "wallet_id": id,
                        "wallet_nickname": null
                    }));
                }
            }
        } else {
            match wallet_manager.get_default_wallet() {
                Some(wallet) => (wallet.address.clone(), Some(wallet.nickname.clone())),
                None => {
                    return Ok(serde_json::json!({
                        "credits": 0,
                        "btpc": 0.0,
                        "address": null,
                        "wallet_id": null,
                        "wallet_nickname": null
                    }));
                }
            }
        }
    };

    // FIX 2025-12-09: Clean address by stripping "Address: " prefix if present
    let address = if raw_address.starts_with("Address: ") {
        raw_address.strip_prefix("Address: ").unwrap_or(&raw_address).to_string()
    } else {
        raw_address
    };

    // FIX 2025-12-05: Use .read().await for RwLock access
    let (credits, btpc) = state
        .tx_storage
        .read()
        .await
        .get_balance(&address)
        .map_err(|e| format!("Failed to get balance from storage: {}", e))?;

    Ok(serde_json::json!({
        "credits": credits,
        "btpc": btpc,
        "address": address,
        "wallet_id": wallet_id,
        "wallet_nickname": wallet_nickname
    }))
}

/// Get transaction count for the current wallet from SQLite storage
#[tauri::command]
pub async fn get_transaction_count_from_storage(state: State<'_, AppState>) -> Result<usize, String> {
    // Get wallet address from WalletManager
    let raw_address = {
        let wallet_manager = state
            .wallet_manager
            .lock()
            .map_err(|e| format!("Failed to lock wallet manager: {}", e))?;

        match wallet_manager.get_default_wallet() {
            Some(wallet) => wallet.address.clone(),
            None => return Ok(0),
        }
    };

    // FIX 2025-12-09: Clean address by stripping "Address: " prefix if present
    let address = if raw_address.starts_with("Address: ") {
        raw_address.strip_prefix("Address: ").unwrap_or(&raw_address).to_string()
    } else {
        raw_address
    };

    // FIX 2025-12-05: Use .read().await for RwLock access
    state
        .tx_storage
        .read()
        .await
        .get_transaction_count(&address)
        .map_err(|e| format!("Failed to get transaction count: {}", e))
}

/// Get mining history (coinbase transactions) from SQLite storage
/// Returns all mined blocks for the current wallet address
#[tauri::command]
pub async fn get_mining_history_from_storage(
    state: State<'_, AppState>,
) -> Result<Vec<TransactionWithOutputs>, String> {
    // Get wallet address from WalletManager
    let raw_address = {
        let wallet_manager = state
            .wallet_manager
            .lock()
            .map_err(|e| format!("Failed to lock wallet manager: {}", e))?;

        match wallet_manager.get_default_wallet() {
            Some(wallet) => wallet.address.clone(),
            None => return Ok(Vec::new()),
        }
    };

    // FIX 2025-12-09: Clean address by stripping "Address: " prefix if present
    let address = if raw_address.starts_with("Address: ") {
        raw_address.strip_prefix("Address: ").unwrap_or(&raw_address).to_string()
    } else {
        raw_address
    };

    // FIX 2025-12-05: Use .read().await for RwLock access
    state
        .tx_storage
        .read()
        .await
        .get_coinbase_transactions(&address)
        .map_err(|e| format!("Failed to get mining history: {}", e))
}

/// Migrate transactions from JSON wallet file to SQLite
#[tauri::command]
pub async fn migrate_json_transactions_to_rocksdb(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    use chrono::DateTime;
    use std::fs;

    // Get wallet address
    let address = {
        let wallet_manager = state
            .wallet_manager
            .lock()
            .map_err(|e| format!("Failed to lock wallet manager: {}", e))?;

        match wallet_manager.get_default_wallet() {
            Some(wallet) => wallet.address.clone(),
            None => return Err("No wallet available for migration".to_string()),
        }
    };

    // Read JSON transactions file
    let json_path = state
        .config
        .data_dir
        .join("wallet")
        .join("wallet_transactions.json");
    if !json_path.exists() {
        return Ok(serde_json::json!({
            "success": true,
            "message": "No JSON transactions file found - nothing to migrate",
            "migrated": 0
        }));
    }

    let json_content =
        fs::read_to_string(&json_path).map_err(|e| format!("Failed to read JSON file: {}", e))?;

    #[derive(serde::Deserialize)]
    struct JsonTransaction {
        txid: String,
        version: u32,
        inputs: Vec<btpc_desktop_app::utxo_manager::TxInput>,
        outputs: Vec<JsonOutput>,
        lock_time: u32,
        fork_id: u8,
        block_height: Option<u64>,
        confirmed_at: Option<String>,
        is_coinbase: bool,
    }

    #[derive(serde::Deserialize)]
    struct JsonOutput {
        value: u64,
        script_pubkey: Vec<u8>,
    }

    let json_txs: Vec<JsonTransaction> =
        serde_json::from_str(&json_content).map_err(|e| format!("Failed to parse JSON: {}", e))?;

    let mut migrated_count = 0;
    let mut skipped_count = 0;

    // Migrate each transaction
    for json_tx in json_txs {
        // Convert to Transaction struct
        let confirmed_at = json_tx.confirmed_at.as_ref().and_then(|s| {
            DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|dt| dt.with_timezone(&chrono::Utc))
        });

        let tx = btpc_desktop_app::utxo_manager::Transaction {
            txid: json_tx.txid,
            version: json_tx.version,
            inputs: json_tx.inputs,
            outputs: json_tx
                .outputs
                .into_iter()
                .map(|o| btpc_desktop_app::utxo_manager::TxOutput {
                    value: o.value,
                    script_pubkey: o.script_pubkey,
                })
                .collect(),
            lock_time: json_tx.lock_time,
            fork_id: json_tx.fork_id,
            block_height: json_tx.block_height,
            confirmed_at,
            is_coinbase: json_tx.is_coinbase,
            // Migrated transactions don't have sender_address - fallback logic handles them
            sender_address: None,
        };

        // Try to add to SQLite
        // FIX 2025-12-05: Use .read().await for RwLock access
        match state.tx_storage.read().await.add_transaction(&tx, &address) {
            Ok(_) => {
                migrated_count += 1;
            }
            Err(e) => {
                // Skip if already exists
                if e.to_string().contains("already exists") {
                    skipped_count += 1;
                } else {
                    eprintln!("Failed to migrate transaction {}: {}", tx.txid, e);
                }
            }
        }
    }

    Ok(serde_json::json!({
        "success": true,
        "message": format!("Migration complete: {} transactions migrated, {} skipped", migrated_count, skipped_count),
        "migrated": migrated_count,
        "skipped": skipped_count
    }))
}

/// Create a transaction preview for the UI
/// Shows UTXO selection, fees, and change calculation before signing
#[tauri::command]
pub async fn create_transaction_preview(
    state: State<'_, AppState>,
    to_address: String,
    amount: f64,
) -> Result<serde_json::Value, String> {
    if amount <= 0.0 {
        return Err("Amount must be greater than zero".to_string());
    }

    let wallet_file = state
        .config
        .data_dir
        .join("wallet")
        .join(&state.config.wallet.default_wallet_file);
    let from_address = match state.btpc.get_wallet_address(&wallet_file) {
        Ok(addr) => addr,
        Err(e) => return Err(format!("Failed to get wallet address: {}", e)),
    };

    let amount_credits = (amount * 100_000_000.0) as u64;
    let fee_credits = 10000u64; // Standard fee
    let total_needed = amount_credits + fee_credits;

    // Get blockchain height for maturity check
    let current_height = state.embedded_node.read().await.get_height();

    let utxo_manager = state.utxo_manager.lock().map_err(|_| {
        BtpcError::mutex_poison("utxo_manager", "create_transaction_preview").to_string()
    })?;

    // Get spendable balance (excludes immature coinbase)
    let (available_credits, available_btp) = utxo_manager.get_spendable_balance(&from_address, current_height);

    // Try to select UTXOs (with maturity check)
    let selected_utxos = match utxo_manager.select_utxos_for_spending(&from_address, total_needed, current_height) {
        Ok(utxos) => utxos,
        Err(e) => return Err(format!("Cannot create transaction: {}", e)),
    };

    let total_input: u64 = selected_utxos.iter().map(|u| u.value_credits).sum();
    let change_amount = total_input - amount_credits - fee_credits;

    let preview = serde_json::json!({
        "from_address": from_address,
        "to_address": to_address,
        "amount_btp": amount,
        "amount_credits": amount_credits,
        "fee_btp": fee_credits as f64 / 100_000_000.0,
        "fee_credits": fee_credits,
        "change_btp": change_amount as f64 / 100_000_000.0,
        "change_credits": change_amount,
        "total_input_credits": total_input,
        "available_balance_btp": available_btp,
        "available_balance_credits": available_credits,
        "inputs_count": selected_utxos.len(),
        "outputs_count": if change_amount > 0 { 2 } else { 1 },
        "selected_utxos": selected_utxos.iter().map(|u| serde_json::json!({
            "txid": u.txid,
            "vout": u.vout,
            "value_credits": u.value_credits,
            "value_btp": u.value_btp,
            "is_coinbase": u.is_coinbase
        })).collect::<Vec<_>>()
    });

    Ok(preview)
}

/// FIX 2025-12-11: Debug command to get detailed transaction diagnostic info
/// Helps track down disappearing transactions by showing all index entries
#[tauri::command]
pub async fn debug_tx_diagnostic(
    state: State<'_, AppState>,
    wallet_id: Option<String>,
) -> Result<String, String> {
    // Resolve wallet address
    let raw_address = {
        let wallet_manager = state
            .wallet_manager
            .lock()
            .map_err(|e| format!("Failed to lock wallet manager: {}", e))?;

        if let Some(ref id) = wallet_id {
            match wallet_manager.get_wallet(id) {
                Some(wallet) => wallet.address.clone(),
                None => return Err(format!("Wallet ID '{}' not found", id)),
            }
        } else {
            match wallet_manager.get_default_wallet() {
                Some(wallet) => wallet.address.clone(),
                None => return Err("No default wallet found".to_string()),
            }
        }
    };

    // Clean address
    let address = if raw_address.starts_with("Address: ") {
        raw_address.strip_prefix("Address: ").unwrap_or(&raw_address).to_string()
    } else {
        raw_address
    };

    // Get diagnostic info
    state
        .tx_storage
        .read()
        .await
        .get_diagnostic_info(&address)
        .map_err(|e| format!("Failed to get diagnostic info: {}", e))
}

/// FIX 2025-12-11: Debug command to run orphaned pending TX cleanup
/// This scans for pending transactions and logs what would be cleaned up
#[tauri::command]
pub async fn debug_cleanup_pending_txs(
    state: State<'_, AppState>,
    max_age_hours: Option<u32>,
) -> Result<serde_json::Value, String> {
    let max_age_seconds = max_age_hours.unwrap_or(1) as i64 * 3600;

    let cleaned = state
        .tx_storage
        .read()
        .await
        .cleanup_orphaned_pending_transactions(max_age_seconds)
        .map_err(|e| format!("Cleanup failed: {}", e))?;

    Ok(serde_json::json!({
        "success": true,
        "message": format!("Cleanup complete: {} orphaned transactions removed", cleaned),
        "cleaned": cleaned
    }))
}