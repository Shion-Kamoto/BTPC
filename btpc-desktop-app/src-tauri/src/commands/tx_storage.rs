//! Transaction storage Tauri commands for RocksDB-based transaction management
//!
//! These commands provide frontend access to the RocksDB transaction storage
//! for paginated queries, balance lookups, and mining history.
//!
//! Constitution Article V: Structured logging for all database operations
//! Constitution Article XI.1: Backend is single source of truth

use tauri::{Emitter, State};

use crate::error::BtpcError;
use btpc_desktop_app::tx_storage::{PaginatedTransactions, PaginationParams, TransactionWithOutputs};
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
    // Constitution Article XI.1 Compliance: RocksDB is single source of truth
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

/// Get paginated transaction history from RocksDB
/// Constitution Article V: RocksDB transaction storage with pagination
#[tauri::command]
pub async fn get_paginated_transaction_history(
    state: State<'_, AppState>,
    offset: usize,
    limit: usize,
) -> Result<PaginatedTransactions, String> {
    // Get wallet address from WalletManager
    let address = {
        let wallet_manager = state
            .wallet_manager
            .lock()
            .map_err(|e| format!("Failed to lock wallet manager: {}", e))?;

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
    };

    // Query RocksDB with pagination
    let pagination = PaginationParams { offset, limit };
    state
        .tx_storage
        .get_transactions_for_address(&address, pagination)
        .map_err(|e| format!("Failed to get paginated transactions: {}", e))
}

/// Add a transaction to RocksDB storage
/// Emits transaction-added and wallet-balance-updated events (Constitution Article XI.3)
#[tauri::command]
pub async fn add_transaction_to_storage(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    tx: btpc_desktop_app::utxo_manager::Transaction,
    address: String,
) -> Result<String, String> {
    // Add transaction to RocksDB
    state
        .tx_storage
        .add_transaction(&tx, &address)
        .map_err(|e| format!("Failed to add transaction to storage: {}", e))?;

    // Get updated balance and transaction count
    let (balance_credits, balance_btpc) = state
        .tx_storage
        .get_balance(&address)
        .map_err(|e| format!("Failed to get updated balance: {}", e))?;
    let tx_count = state
        .tx_storage
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

/// Get a single transaction from RocksDB by txid
#[tauri::command]
pub async fn get_transaction_from_storage(
    state: State<'_, AppState>,
    txid: String,
) -> Result<Option<btpc_desktop_app::utxo_manager::Transaction>, String> {
    state
        .tx_storage
        .get_transaction(&txid)
        .map_err(|e| format!("Failed to get transaction from storage: {}", e))
}

/// Get wallet balance from RocksDB storage
#[tauri::command]
pub async fn get_wallet_balance_from_storage(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    // Get wallet address from WalletManager
    let address = {
        let wallet_manager = state
            .wallet_manager
            .lock()
            .map_err(|e| format!("Failed to lock wallet manager: {}", e))?;

        match wallet_manager.get_default_wallet() {
            Some(wallet) => wallet.address.clone(),
            None => {
                return Ok(serde_json::json!({
                    "credits": 0,
                    "btpc": 0.0,
                    "address": null
                }));
            }
        }
    };

    let (credits, btpc) = state
        .tx_storage
        .get_balance(&address)
        .map_err(|e| format!("Failed to get balance from storage: {}", e))?;

    Ok(serde_json::json!({
        "credits": credits,
        "btpc": btpc,
        "address": address
    }))
}

/// Get transaction count for the current wallet from RocksDB storage
#[tauri::command]
pub async fn get_transaction_count_from_storage(state: State<'_, AppState>) -> Result<usize, String> {
    // Get wallet address from WalletManager
    let address = {
        let wallet_manager = state
            .wallet_manager
            .lock()
            .map_err(|e| format!("Failed to lock wallet manager: {}", e))?;

        match wallet_manager.get_default_wallet() {
            Some(wallet) => wallet.address.clone(),
            None => return Ok(0),
        }
    };

    state
        .tx_storage
        .get_transaction_count(&address)
        .map_err(|e| format!("Failed to get transaction count: {}", e))
}

/// Get mining history (coinbase transactions) from RocksDB storage
/// Returns all mined blocks for the current wallet address
#[tauri::command]
pub async fn get_mining_history_from_storage(
    state: State<'_, AppState>,
) -> Result<Vec<TransactionWithOutputs>, String> {
    // Get wallet address from WalletManager
    let address = {
        let wallet_manager = state
            .wallet_manager
            .lock()
            .map_err(|e| format!("Failed to lock wallet manager: {}", e))?;

        match wallet_manager.get_default_wallet() {
            Some(wallet) => wallet.address.clone(),
            None => return Ok(Vec::new()),
        }
    };

    state
        .tx_storage
        .get_coinbase_transactions(&address)
        .map_err(|e| format!("Failed to get mining history: {}", e))
}

/// Migrate transactions from JSON wallet file to RocksDB
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
        };

        // Try to add to RocksDB
        match state.tx_storage.add_transaction(&tx, &address) {
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

    let utxo_manager = state.utxo_manager.lock().map_err(|_| {
        BtpcError::mutex_poison("utxo_manager", "create_transaction_preview").to_string()
    })?;

    // Get current balance
    let (available_credits, available_btp) = utxo_manager.get_balance(&from_address);

    // Try to select UTXOs
    let selected_utxos = match utxo_manager.select_utxos_for_spending(&from_address, total_needed) {
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