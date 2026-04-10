//! Mining log retrieval commands for the BTPC desktop application
//!
//! This module provides Tauri commands for retrieving mining log entries.

use tauri::State;

use crate::types::MiningLogEntry;
use crate::AppState;

/// Get mining log entries from the application state
///
/// Returns a vector of mining log entries containing timestamp, level, and message.
///
/// # Arguments
/// * `limit` - Optional maximum number of recent entries to return. Defaults to 100.
///             This prevents transferring all 1000 buffered entries every poll cycle.
///
/// # Performance Note
/// Uses `try_lock()` instead of `lock()` to prevent blocking the async runtime.
/// If the mining thread is actively writing logs, returns an empty vector rather
/// than blocking. This prevents console freeze at high hashrates (100+ MH/s).
/// Based on XMRig architecture: https://xmrig.com/docs/miner
#[tauri::command]
pub async fn get_mining_logs(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<MiningLogEntry>, String> {
    // FIX 2025-12-08: Use try_lock() instead of lock() to prevent async runtime blocking
    // At high hashrates (100 MH/s), mining thread frequently holds this mutex.
    // Blocking here causes the entire Tauri IPC to stall, freezing the WebView.
    // Instead, return empty vector and let frontend use cached state.
    match state.mining_logs.try_lock() {
        Ok(mining_logs) => {
            // Default to 100 entries - enough for display but not memory-heavy
            let limit = limit.unwrap_or(100);
            Ok(mining_logs.get_recent_entries(limit))
        }
        Err(_) => {
            // Mutex is busy (mining thread writing logs) - return empty rather than block
            // Frontend will use cached data from previous successful call
            Ok(Vec::new())
        }
    }
}