//! Blockchain Explorer Tauri Commands
//!
//! This module contains all Tauri commands related to blockchain exploration,
//! including block and transaction queries, search functionality, and blockchain info.

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::AppState;

// ============================================================================
// Data Structures
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockInfo {
    pub height: u64,
    pub hash: String,
    pub prev_hash: String,
    pub merkle_root: Option<String>,
    pub timestamp: u64,
    pub bits: u32,
    pub nonce: u64,
    pub version: u32,
    pub tx_count: usize,
    pub size: usize,
    pub transactions: Vec<TransactionInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionInfo {
    pub hash: String,
    pub block_height: Option<u64>,
    pub inputs: usize,
    pub outputs: usize,
    pub total_value: u64,
    pub timestamp: u64,
}

#[allow(dead_code)] // Reserved for explorer API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainInfo {
    pub height: u64,
    pub total_transactions: u64,
    pub difficulty: f64,
    pub hash_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum SearchResult {
    Block(BlockInfo),
    Transaction(TransactionInfo),
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert compact difficulty bits to decimal representation (simple version)
/// DEPRECATED: Use bits_to_difficulty() instead which calculates proper difficulty value
#[allow(dead_code)]
fn bits_to_decimal(bits: u32) -> f64 {
    // Simply return the bits value as a decimal number
    bits as f64
}

/// Calculate effective difficulty based on actual block mining rate
///
/// This calculates what the difficulty SHOULD be if the network had proper
/// difficulty adjustment, based on how fast blocks are actually being mined.
///
/// Formula: effective_difficulty = (target_time / actual_time) * base_difficulty
///
/// If blocks are being mined faster than the 10-minute target, difficulty is higher.
///
/// NOTE: Currently unused - we display network consensus difficulty instead.
/// Kept for future use (e.g., difficulty estimation before adjustment blocks).
#[allow(dead_code)]
async fn calculate_effective_difficulty(
    node: &btpc_desktop_app::embedded_node::EmbeddedNode,
    current_height: u64,
) -> Result<f64, String> {
    // Need at least 10 blocks to calculate meaningful difficulty
    if current_height < 10 {
        eprintln!(
            "[BTPC::Chain] Height {} < 10, returning 1.0",
            current_height
        );
        return Ok(1.0);
    }

    // Try different sample sizes, starting large and working down
    let sample_sizes: [u64; 4] = [100, 50, 20, 10];

    let db = node.get_database();

    // Find the end block by searching backwards from current_height
    // IMPORTANT: The current_height might be the network consensus height (e.g., 21320),
    // but we only have locally stored blocks (e.g., 0-100). We need to search
    // aggressively to find the highest block that actually exists in our database.
    let mut end_block = None;
    let mut end_height = 0u64;

    // Strategy 1: Try recent blocks first (maybe we're synced)
    for h in (0..current_height).rev().take(100) {
        if let Ok(Some(block)) = db.get_block(h as u32) {
            end_block = Some(block);
            end_height = h;
            tracing::debug!(
                "Found end block at height {} (searched from {})",
                end_height,
                current_height
            );
            break;
        }
    }

    // Strategy 2: If no recent blocks found, search from 0 upwards to find highest stored block
    if end_block.is_none() {
        tracing::debug!("No recent blocks found, searching from genesis...");
        for h in 0..1000 {
            if let Ok(Some(block)) = db.get_block(h as u32) {
                end_block = Some(block);
                end_height = h;
                // Don't break - keep searching to find the highest one
            }
        }
        if end_block.is_some() {
            tracing::debug!("Found highest stored block at height {}", end_height);
        }
    }

    let end_block = end_block.ok_or_else(|| {
        tracing::debug!("Failed to find ANY blocks in database (searched 0-999)");
        "End block not found - no blocks in local database".to_string()
    })?;

    // Try to find a start block with progressively smaller sample sizes
    let mut start_block = None;
    let mut actual_sample_size = 0u64;

    for &sample_size in &sample_sizes {
        if end_height <= sample_size {
            continue;
        }

        // Search backwards from the target height for an available block
        let target_start = end_height.saturating_sub(sample_size);
        for h in (0..=target_start).rev().take(20) {
            if let Ok(Some(block)) = db.get_block(h as u32) {
                start_block = Some(block);
                actual_sample_size = end_height - h;
                break;
            }
        }
        if start_block.is_some() {
            break;
        }
    }

    let start_block =
        start_block.ok_or_else(|| "No start block found for difficulty calculation".to_string())?;

    tracing::debug!(
        "Calculating difficulty for height {}, using {} block sample",
        current_height,
        actual_sample_size
    );

    // Calculate actual time for these blocks
    let start_ts = start_block.header.timestamp;
    let end_ts = end_block.header.timestamp;
    let actual_time_seconds = end_ts.saturating_sub(start_ts);

    tracing::debug!(
        "Start timestamp: {}, End timestamp: {}, Time diff: {} seconds",
        start_ts,
        end_ts,
        actual_time_seconds
    );

    // Target time: 10 minutes per block = 600 seconds
    const TARGET_BLOCK_TIME_SECONDS: u64 = 600;
    let expected_time_seconds = actual_sample_size * TARGET_BLOCK_TIME_SECONDS;

    // If actual time is very small (less than 1 second per block), use minimum
    if actual_time_seconds < actual_sample_size {
        // Blocks mined extremely fast - very high difficulty
        let ratio = expected_time_seconds as f64 / actual_sample_size as f64;
        tracing::debug!("Blocks mined very fast, effective difficulty: {}", ratio);
        return Ok(ratio.max(1.0));
    }

    // Calculate effective difficulty: how much harder mining should be
    // If blocks take 1 second instead of 600, difficulty should be 600x higher
    let effective_difficulty = expected_time_seconds as f64 / actual_time_seconds as f64;

    tracing::debug!(
        "Expected time: {}s, Actual time: {}s, Effective difficulty: {:.2}",
        expected_time_seconds,
        actual_time_seconds,
        effective_difficulty
    );

    // Ensure minimum of 1.0
    Ok(effective_difficulty.max(1.0))
}

// ============================================================================
// Tauri Commands
// ============================================================================

#[tauri::command]
pub async fn get_blockchain_info(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    tracing::debug!("get_blockchain_info called");

    // FIX 2025-11-26: Use EmbeddedNode instead of external RPC client
    // This ensures blockchain info comes from the same database where mining stores blocks
    // Previously used RpcClient which connected to external btpc_node with separate database
    let embedded_node = state.embedded_node.read().await;

    // Get blockchain state from embedded node (same database as mining)
    let blockchain_state = embedded_node
        .get_blockchain_state()
        .await
        .map_err(|e| format!("Failed to get blockchain state: {}", e))?;

    let current_height = blockchain_state.current_height;
    let best_hash = blockchain_state.best_block_hash.clone();
    let difficulty_bits = blockchain_state.difficulty_bits;

    tracing::debug!("blockchain height from EmbeddedNode = {}", current_height);

    // Convert difficulty bits to human-readable difficulty value
    // Using the same formula as integrated_handlers.rs
    let difficulty = bits_to_difficulty(difficulty_bits);

    tracing::debug!(
        "Network difficulty at height {}: {:.2} (bits: 0x{:08x})",
        current_height,
        difficulty,
        difficulty_bits
    );

    // Get network type from embedded node
    let network = embedded_node.get_network();

    // FIX 2026-04-12: Use embedded node's sync progress instead of the
    // (now-disabled) RPC sync service. The P2P event loop handles block sync.
    // Sync progress is based on comparing our height to the best peer height.
    // FIX 2026-04-15: Also extract network_height (max peer tip) so the UI can
    // show "network is at N" independent of local catch-up progress.
    let (sync_progress, is_synced, network_height) = {
        let sync_result = embedded_node.get_sync_progress();
        match sync_result {
            Ok(progress) if progress.is_syncing => {
                (progress.sync_percentage, false, progress.target_height)
            }
            Ok(progress) => {
                // Not syncing — check if we're behind any peer
                let peer_count = embedded_node.get_peer_count();
                if peer_count == 0 && current_height == 0 {
                    // No peers and empty chain — show 0%
                    (0.0, false, progress.target_height)
                } else {
                    (
                        progress.sync_percentage,
                        progress.sync_percentage >= 99.9,
                        progress.target_height,
                    )
                }
            }
            Err(_) => (100.0, true, current_height),
        }
    };

    eprintln!(
        "DEBUG: Sync progress: {:.1}% (height: {}, peers: {})",
        sync_progress,
        current_height,
        embedded_node.get_peer_count()
    );

    // FIX 2025-12-27: Get actual peer count from embedded node instead of hardcoded 1
    // Also check if node is running - show 0 connections when stopped
    // FIX 2026-04-12: Removed phantom +1 that inflated peer count. The local node
    // is not a "connection" — only actual remote P2P peers should be counted.
    let node_running = state.node_status.read().map(|s| s.running).unwrap_or(false);
    let connections = if node_running {
        embedded_node.get_peer_count() // Only actual remote peers
    } else {
        0
    };

    // Return blockchain info in the format the frontend expects
    // FIX 2026-04-15: Added `network_height` = max(peer.height, local_tip).
    // The Dashboard and Node > Blockchain Info tiles render this as the primary
    // "block height" so a freshly-booted node shows the network tip (e.g. 6309)
    // instead of its own local 0 while sync catches up. `height` / `blocks`
    // still return the local tip for sync_progress math and per-machine state.
    Ok(serde_json::json!({
        "blocks": current_height,
        "height": current_height,  // Alias for compatibility (LOCAL tip)
        "network_height": network_height,  // Max peer tip — what the UI tiles show
        "local_height": current_height,  // Explicit alias for the local tip
        "headers": network_height,
        "chain": network,
        "difficulty": difficulty,  // Difficulty calculated from bits
        "difficulty_bits": difficulty_bits,  // Raw bits value
        "best_block_hash": best_hash,
        "bestblockhash": best_hash.clone(),  // Alias for compatibility
        "connections": connections,  // Actual peer count when running, 0 when stopped
        "node_offline": !node_running,
        "sync_progress": sync_progress,  // Accurate sync progress from sync service
        "is_synced": is_synced,
    }))
}

/// Get network health information (bootstrap status, hashrate, EDA triggers)
///
/// Returns comprehensive health metrics for monitoring network stability
/// during the bootstrap phase (first 20,160 blocks) and beyond.
#[tauri::command]
pub async fn get_network_health_info(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let node = state.embedded_node.read().await;
    Ok(node.get_network_health_info())
}

/// Convert compact bits format to difficulty value for display
///
/// For BTPC, we display the raw bits value in decimal format.
/// This matches the frontend expectation (e.g., 545,259,519 for 0x207fffff regtest)
///
/// Note: Bitcoin-style difficulty calculation would produce tiny values for regtest
/// (0x207fffff → ~4.66e-10), which is not useful for display.
pub fn bits_to_difficulty(bits: u32) -> f64 {
    // Return raw bits value as decimal for display
    // This is what the frontend expects and displays as "Bits (decimal)"
    bits as f64
}

#[tauri::command]
pub async fn get_recent_blocks(
    state: State<'_, AppState>,
    limit: usize,
    offset: usize,
) -> Result<Vec<BlockInfo>, String> {
    use btpc_desktop_app::rpc_client::RpcClient;
    let rpc_client = RpcClient::new(&state.config.rpc.host, state.config.rpc.port);

    // Get current blockchain height
    let info = rpc_client
        .get_blockchain_info()
        .await
        .map_err(|e| format!("Failed to get blockchain info: {}", e))?;

    let current_height = info.blocks;

    if current_height == 0 {
        return Ok(Vec::new());
    }

    let start_height = current_height.saturating_sub(offset as u64);

    let end_height = start_height.saturating_sub(limit as u64);

    let mut blocks = Vec::new();

    for height in (end_height..=start_height).rev() {
        match rpc_client.get_block_by_height(height).await {
            Ok(block_data) => {
                blocks.push(BlockInfo {
                    height: block_data.height,
                    hash: block_data.hash,
                    prev_hash: block_data.previous_block_hash.unwrap_or_default(),
                    merkle_root: Some(block_data.merkle_root),
                    timestamp: block_data.time,
                    bits: block_data.bits,
                    nonce: block_data.nonce,
                    version: block_data.version,
                    tx_count: block_data.tx.len(),
                    size: 0, // Not provided by RPC BlockInfo, would need separate query
                    transactions: Vec::new(), // Populated separately if needed
                });
            }
            Err(_) => continue,
        }
    }

    Ok(blocks)
}

#[tauri::command]
pub async fn get_recent_transactions(
    state: State<'_, AppState>,
    limit: usize,
    offset: usize,
) -> Result<Vec<TransactionInfo>, String> {
    use btpc_desktop_app::rpc_client::RpcClient;
    let rpc_client = RpcClient::new(&state.config.rpc.host, state.config.rpc.port);

    // Get recent transactions from node
    match rpc_client.get_recent_transactions(limit, offset).await {
        Ok(txs) => {
            let transactions: Vec<TransactionInfo> = txs
                .iter()
                .map(|tx| TransactionInfo {
                    hash: tx.hash.clone(),
                    block_height: tx.block_height,
                    inputs: tx.inputs,
                    outputs: tx.outputs,
                    total_value: tx.total_value,
                    timestamp: tx.timestamp,
                })
                .collect();

            Ok(transactions)
        }
        Err(e) => Err(format!("Failed to get recent transactions: {}", e)),
    }
}

#[tauri::command]
pub async fn search_blockchain(
    state: State<'_, AppState>,
    query: String,
) -> Result<SearchResult, String> {
    use btpc_desktop_app::rpc_client::RpcClient;
    let rpc_client = RpcClient::new(&state.config.rpc.host, state.config.rpc.port);

    // Try to parse as block height
    if let Ok(height) = query.parse::<u64>() {
        if let Ok(block_data) = rpc_client.get_block_by_height(height).await {
            return Ok(SearchResult::Block(BlockInfo {
                height: block_data.height,
                hash: block_data.hash,
                prev_hash: block_data.previous_block_hash.unwrap_or_default(),
                merkle_root: Some(block_data.merkle_root),
                timestamp: block_data.time,
                bits: block_data.bits,
                nonce: block_data.nonce,
                version: block_data.version,
                tx_count: block_data.tx.len(),
                size: 0,
                transactions: Vec::new(),
            }));
        }
    }

    // Try to search as block hash or transaction hash
    if let Ok(tx_data) = rpc_client.get_transaction(&query).await {
        let total_value: u64 = tx_data.vout.iter().map(|out| out.value).sum();
        return Ok(SearchResult::Transaction(TransactionInfo {
            hash: query.clone(),
            block_height: None, // RPC TransactionInfo doesn't have block_height
            inputs: tx_data.vin.len(),
            outputs: tx_data.vout.len(),
            total_value,
            timestamp: 0, // RPC TransactionInfo doesn't have timestamp
        }));
    }

    Err(format!(
        "No block or transaction found for query: {}",
        query
    ))
}

#[tauri::command]
pub async fn get_block_message(state: State<'_, AppState>, txid: String) -> Result<String, String> {
    // Get wallet address from default wallet
    let _address = {
        let wallet_manager = state
            .wallet_manager
            .lock()
            .map_err(|e| format!("Failed to lock wallet manager: {}", e))?;
        wallet_manager
            .get_default_wallet()
            .ok_or_else(|| "No default wallet set".to_string())?
            .address
            .clone()
    };

    // CONSTITUTION COMPLIANCE: Use RocksDB indexed lookup (O(log n))
    // Backend is single source of truth (Article XI.1)
    // Direct transaction lookup by txid - avoids O(n*m) scan
    // FIX 2025-12-05: Use .read().await for RwLock access
    let tx = state
        .tx_storage
        .read()
        .await
        .get_transaction(&txid)
        .map_err(|e| format!("Failed to get transaction: {}", e))?
        .ok_or_else(|| "Transaction not found".to_string())?;

    // Check if it's a coinbase transaction
    if !tx.is_coinbase {
        return Err("Not a coinbase transaction".to_string());
    }

    // Get the scriptSig from the first input
    if let Some(first_input) = tx.inputs.first() {
        let script_bytes = &first_input.signature_script;

        // Genesis block format: [timestamp(8)] + [difficulty_target(32)] + [message_length(1)] + [message]
        // Regular mined block format: [message bytes directly]

        if tx.block_height == Some(0) && script_bytes.len() > 41 {
            // Try parsing genesis block format
            let message_len = script_bytes[40] as usize;
            if script_bytes.len() >= 41 + message_len {
                let message_bytes = &script_bytes[41..41 + message_len];
                if let Ok(message) = String::from_utf8(message_bytes.to_vec()) {
                    return Ok(message);
                }
            }
        }

        // Try parsing as direct UTF-8 message (regular mined blocks)
        if let Ok(message) = String::from_utf8(script_bytes.clone()) {
            if !message.is_empty()
                && message
                    .chars()
                    .all(|c| c.is_ascii_graphic() || c.is_whitespace())
            {
                return Ok(message);
            }
        }

        // Fallback: extract printable ASCII characters
        let readable: String = script_bytes
            .iter()
            .filter(|&&b| (32..=126).contains(&b))
            .map(|&b| b as char)
            .collect();

        if !readable.is_empty() {
            return Ok(readable);
        }

        return Ok("[No readable message found]".to_string());
    }

    Err("Failed to extract block message from coinbase transaction".to_string())
}
