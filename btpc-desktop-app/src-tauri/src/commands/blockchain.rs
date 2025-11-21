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

/// Convert compact difficulty bits to a human-readable difficulty value
///
/// # Format
/// Compact bits format (4 bytes): 0xAABBCCDD
/// - AA: exponent (number of bytes)
/// - BBCCDD: mantissa (coefficient)
///
/// # Network Baselines
/// - Mainnet/Testnet: 0x1d00ffff = difficulty 1
/// - Regtest: 0x207fffff = difficulty 1 (easier target for testing)
///
/// # Examples
/// - 0x1d00ffff = difficulty 1 (mainnet minimum)
/// - 0x207fffff = difficulty 1 (regtest minimum)
/// - 0x1b0404cb = difficulty ~16,307
///
/// # Returns
/// Difficulty as a floating-point number (difficulty 1 = easiest for that network)
pub fn bits_to_difficulty(bits: u32) -> f64 {
    // Special case: if bits is 0 or invalid, return 1
    if bits == 0 {
        return 1.0;
    }

    // Regtest minimum difficulty (very easy, for testing)
    const REGTEST_MIN_BITS: u32 = 0x207fffff;
    // Mainnet/Testnet minimum difficulty (Bitcoin standard)
    const MAINNET_MIN_BITS: u32 = 0x1d00ffff;

    // Determine baseline based on the bits value
    // If bits indicates an easier target than mainnet minimum, use regtest baseline
    let baseline_bits = if bits >= 0x20000000 {
        // Exponent >= 0x20 indicates regtest-level easy difficulty
        REGTEST_MIN_BITS
    } else {
        MAINNET_MIN_BITS
    };

    // Extract exponent and mantissa from compact format
    let base_exponent = (baseline_bits >> 24) as i32;
    let base_mantissa = (baseline_bits & 0x00ffffff) as f64;

    let current_exponent = (bits >> 24) as i32;
    let current_mantissa = (bits & 0x00ffffff) as f64;

    // Difficulty = baseline_target / current_target
    // target = mantissa * 256^(exponent - 3)
    // So: difficulty = (base_mantissa * 256^(base_exp - 3)) / (current_mantissa * 256^(current_exp - 3))
    //              = (base_mantissa / current_mantissa) * 256^(base_exp - current_exp)

    let mantissa_ratio = base_mantissa / current_mantissa;
    let exponent_diff = base_exponent - current_exponent;
    let scale_factor = 256_f64.powi(exponent_diff);

    let difficulty = mantissa_ratio * scale_factor;

    // Ensure minimum difficulty is 1.0
    difficulty.max(1.0)
}

/// Calculate effective difficulty based on actual block mining rate
///
/// This calculates what the difficulty SHOULD be if the network had proper
/// difficulty adjustment, based on how fast blocks are actually being mined.
///
/// Formula: effective_difficulty = (target_time / actual_time) * base_difficulty
///
/// If blocks are being mined faster than the 10-minute target, difficulty is higher.
async fn calculate_effective_difficulty(
    node: &btpc_desktop_app::embedded_node::EmbeddedNode,
    current_height: u64,
) -> Result<f64, String> {
    // Need at least 10 blocks to calculate meaningful difficulty
    if current_height < 10 {
        eprintln!("DEBUG: Height {} < 10, returning 1.0", current_height);
        return Ok(1.0);
    }

    // Try different sample sizes, starting large and working down
    let sample_sizes: [u64; 4] = [100, 50, 20, 10];

    let db = node.get_database();

    // Find the end block by searching backwards from current_height
    let mut end_block = None;
    let mut end_height = 0u64;
    for h in (0..current_height).rev().take(50) {
        if let Ok(Some(block)) = db.get_block(h as u32) {
            end_block = Some(block);
            end_height = h;
            break;
        }
    }

    let end_block = end_block.ok_or_else(|| "End block not found".to_string())?;

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

    let start_block = start_block.ok_or_else(|| {
        "No start block found for difficulty calculation".to_string()
    })?;

    eprintln!("DEBUG: Calculating difficulty for height {}, using {} block sample",
              current_height, actual_sample_size);

    // Calculate actual time for these blocks
    let start_ts = start_block.header.timestamp;
    let end_ts = end_block.header.timestamp;
    let actual_time_seconds = end_ts.saturating_sub(start_ts);

    eprintln!("DEBUG: Start timestamp: {}, End timestamp: {}, Time diff: {} seconds",
              start_ts, end_ts, actual_time_seconds);

    // Target time: 10 minutes per block = 600 seconds
    const TARGET_BLOCK_TIME_SECONDS: u64 = 600;
    let expected_time_seconds = actual_sample_size * TARGET_BLOCK_TIME_SECONDS;

    // If actual time is very small (less than 1 second per block), use minimum
    if actual_time_seconds < actual_sample_size {
        // Blocks mined extremely fast - very high difficulty
        let ratio = expected_time_seconds as f64 / actual_sample_size as f64;
        eprintln!("DEBUG: Blocks mined very fast, effective difficulty: {}", ratio);
        return Ok(ratio.max(1.0));
    }

    // Calculate effective difficulty: how much harder mining should be
    // If blocks take 1 second instead of 600, difficulty should be 600x higher
    let effective_difficulty = expected_time_seconds as f64 / actual_time_seconds as f64;

    eprintln!("DEBUG: Expected time: {}s, Actual time: {}s, Effective difficulty: {:.2}",
              expected_time_seconds, actual_time_seconds, effective_difficulty);

    // Ensure minimum of 1.0
    Ok(effective_difficulty.max(1.0))
}

// ============================================================================
// Tauri Commands
// ============================================================================

#[tauri::command]
pub async fn get_blockchain_info(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    eprintln!("=== get_blockchain_info CALLED ===");

    // Feature 013: Use embedded node instead of external RPC (self-contained app)
    let node = state.embedded_node.read().await;
    eprintln!("DEBUG: Got node lock");

    // Get blockchain state from embedded node
    let blockchain_state = node
        .get_blockchain_state()
        .await
        .map_err(|e| format!("Failed to get blockchain state: {}", e))?;

    eprintln!("DEBUG: blockchain_state.current_height = {}", blockchain_state.current_height);

    // Get sync progress for connection count
    let sync_progress = node
        .get_sync_progress()
        .map_err(|e| format!("Failed to get sync progress: {}", e))?;

    // Get network type
    let network = node.get_network();

    // Calculate effective difficulty based on actual mining rate
    // This shows what the difficulty SHOULD be if we had proper adjustment
    let difficulty = match calculate_effective_difficulty(&node, blockchain_state.current_height).await {
        Ok(diff) => {
            eprintln!("DEBUG: Effective difficulty calculated: {:.2}", diff);
            diff
        }
        Err(e) => {
            eprintln!("DEBUG: Failed to calculate effective difficulty: {}, falling back to bits_to_difficulty", e);
            bits_to_difficulty(blockchain_state.difficulty_bits)
        }
    };

    // Return blockchain info in the format the frontend expects
    Ok(serde_json::json!({
        "blocks": blockchain_state.current_height,
        "height": blockchain_state.current_height,  // Alias for compatibility
        "headers": blockchain_state.current_height,  // In embedded node, headers == blocks
        "chain": network,
        "difficulty": difficulty,  // Human-readable difficulty (1.0 for regtest)
        "difficulty_bits": blockchain_state.difficulty_bits,  // Raw bits value for debugging
        "best_block_hash": blockchain_state.best_block_hash,
        "bestblockhash": blockchain_state.best_block_hash,  // Alias for compatibility
        "connections": sync_progress.connected_peers,
        "node_offline": false,  // Embedded node is always "online" when app is running
    }))
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
    let tx = state
        .tx_storage
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