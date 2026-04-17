//! UTXO-related Tauri commands for the BTPC desktop application
//!
//! This module provides commands for managing the UTXO (Unspent Transaction Output) set,
//! including querying balances, stats, and syncing with the blockchain.

use std::sync::{Arc, Mutex};
use tauri::State;

use crate::error::BtpcError;
use crate::AppState;
use btpc_desktop_app::utxo_manager::{UTXOManager, UTXOStats, UTXO};

// Helper function to add mining rewards to UTXO set
pub fn add_mining_reward_utxo(
    utxo_manager: &Arc<Mutex<UTXOManager>>,
    address: &str,
    amount_credits: u64,
    block_height: u64,
    fork_id: u8,
) -> Result<(), String> {
    let txid = format!(
        "coinbase_{}_{}",
        block_height,
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
    );

    let mut manager = utxo_manager.lock().map_err(|_| {
        BtpcError::mutex_poison("utxo_manager", "add_mining_reward_utxo").to_string()
    })?;
    manager
        .add_coinbase_utxo(
            txid,
            0, // vout = 0 for coinbase
            amount_credits,
            address.to_string(),
            block_height,
            fork_id,
        )
        .map_err(|e| format!("Failed to add mining UTXO: {}", e))?;

    println!(
        "[UTXO] Added mining UTXO: {} credits to address {} (block {})",
        amount_credits, address, block_height
    );
    Ok(())
}

#[tauri::command]
pub async fn reload_utxos(state: State<'_, AppState>) -> Result<String, String> {
    let mut utxo_manager = state
        .utxo_manager
        .lock()
        .map_err(|e| format!("Failed to lock UTXO manager: {}", e))?;
    match utxo_manager.reload_utxos() {
        Ok(_) => Ok("UTXO data reloaded successfully".to_string()),
        Err(e) => Err(format!("Failed to reload UTXOs: {}", e)),
    }
}

#[tauri::command]
pub async fn get_utxo_stats(state: State<'_, AppState>) -> Result<UTXOStats, String> {
    let utxo_manager = state
        .utxo_manager
        .lock()
        .map_err(|_| BtpcError::mutex_poison("utxo_manager", "get_utxo_stats").to_string())?;
    Ok(utxo_manager.get_stats())
}

#[tauri::command]
pub async fn get_wallet_utxos(state: State<'_, AppState>) -> Result<Vec<UTXO>, String> {
    let wallet_file = state
        .config
        .data_dir
        .join("wallet")
        .join(&state.config.wallet.default_wallet_file);
    let address = match state.btpc.get_wallet_address(&wallet_file) {
        Ok(addr) => addr,
        Err(e) => return Err(format!("Failed to get wallet address: {}", e)),
    };

    let utxo_manager = state
        .utxo_manager
        .lock()
        .map_err(|_| BtpcError::mutex_poison("utxo_manager", "get_wallet_utxos").to_string())?;
    let utxos: Vec<UTXO> = utxo_manager
        .get_all_utxos(&address)
        .into_iter()
        .cloned()
        .collect();

    Ok(utxos)
}

#[tauri::command]
pub async fn get_spendable_utxos(state: State<'_, AppState>) -> Result<Vec<UTXO>, String> {
    let wallet_file = state
        .config
        .data_dir
        .join("wallet")
        .join(&state.config.wallet.default_wallet_file);
    let address = match state.btpc.get_wallet_address(&wallet_file) {
        Ok(addr) => addr,
        Err(e) => return Err(format!("Failed to get wallet address: {}", e)),
    };

    // Get actual blockchain height for coinbase maturity check
    let current_height = {
        let node = state.embedded_node.read().await;
        node.get_height()
    };

    let utxo_manager = state
        .utxo_manager
        .lock()
        .map_err(|_| BtpcError::mutex_poison("utxo_manager", "get_spendable_utxos").to_string())?;

    let spendable_utxos: Vec<UTXO> = utxo_manager
        .get_unspent_utxos(&address)
        .into_iter()
        .filter(|utxo| utxo.is_spendable(current_height))
        .cloned()
        .collect();

    Ok(spendable_utxos)
}

#[tauri::command]
pub async fn add_mining_utxo(
    state: State<'_, AppState>,
    address: String,
    amount_credits: u64,
    block_height: u64,
) -> Result<String, String> {
    let fork_id = state.active_network.read().await.fork_id();
    add_mining_reward_utxo(
        &state.utxo_manager,
        &address,
        amount_credits,
        block_height,
        fork_id,
    )
    .map(|_| {
        format!(
            "Added mining UTXO: {} credits at block {}",
            amount_credits, block_height
        )
    })
}

#[tauri::command]
pub async fn sync_wallet_utxos(state: State<'_, AppState>) -> Result<String, String> {
    let wallet_file = state
        .config
        .data_dir
        .join("wallet")
        .join(&state.config.wallet.default_wallet_file);
    let address = match state.btpc.get_wallet_address(&wallet_file) {
        Ok(addr) => addr,
        Err(e) => return Err(format!("Failed to get wallet address: {}", e)),
    };

    // In a real implementation, this would query the blockchain node
    // For now, we'll export UTXO data for Python integration compatibility
    let export_result = {
        let utxo_manager = state.utxo_manager.lock().map_err(|_| {
            BtpcError::mutex_poison("utxo_manager", "sync_wallet_utxos").to_string()
        })?;
        utxo_manager.export_utxos_for_integration(&address)
    };

    match export_result {
        Ok(()) => Ok(format!("UTXO sync completed for address: {}", address)),
        Err(e) => Err(format!("UTXO sync failed: {}", e)),
    }
}
