//! Mining log retrieval commands for the BTPC desktop application
//!
//! This module provides Tauri commands for retrieving mining log entries.

use tauri::State;

use crate::error::BtpcError;
use crate::types::MiningLogEntry;
use crate::AppState;

/// Get mining log entries from the application state
///
/// Returns a vector of mining log entries containing timestamp, level, and message.
#[tauri::command]
pub async fn get_mining_logs(state: State<'_, AppState>) -> Result<Vec<MiningLogEntry>, String> {
    let mining_logs = state
        .mining_logs
        .lock()
        .map_err(|_| BtpcError::mutex_poison("mining_logs", "get_mining_logs").to_string())?;
    Ok(mining_logs.get_entries())
}