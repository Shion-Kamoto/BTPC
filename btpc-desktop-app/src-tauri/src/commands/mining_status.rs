//! Mining status commands for the BTPC desktop application
//!
//! This module provides Tauri commands for retrieving mining status information.

use tauri::State;

use crate::AppState;

/// Get current mining status including hashrate, blocks found, and blockchain height
///
/// Returns a JSON object with mining status information:
/// - `is_mining`: Whether mining is currently active
/// - `hashrate`: Current total hashrate
/// - `blocks_found`: Number of blocks found
/// - `current_height`: Current blockchain height (for linear decay calculation)
#[tauri::command]
pub async fn get_mining_status(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    // Feature 012: Use new MiningThreadPool instead of mining_processes
    let mining_pool_guard = state.mining_pool.read().await;

    if mining_pool_guard.is_none() {
        // Mining pool not initialized, return stopped state
        return Ok(serde_json::json!({
            "is_mining": false,
            "hashrate": 0,
            "blocks_found": 0
        }));
    }

    if let Some(ref pool) = *mining_pool_guard {
        let stats = pool.get_stats();

        // Get current blockchain height for linear decay calculation (Issue #3)
        let current_height = {
            let node = state.embedded_node.read().await;
            node.get_blockchain_state().await
                .map(|state| state.current_height)
                .unwrap_or(0)
        };

        Ok(serde_json::json!({
            "is_mining": stats.is_mining,
            "hashrate": stats.total_hashrate,
            "blocks_found": stats.blocks_found,
            "current_height": current_height
        }))
    } else {
        Ok(serde_json::json!({
            "is_mining": false,
            "hashrate": 0,
            "blocks_found": 0,
            "current_height": 0
        }))
    }
}