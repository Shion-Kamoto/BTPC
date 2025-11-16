//! Tauri commands for embedded blockchain node
//!
//! Exposes EmbeddedNode functionality to frontend via Tauri IPC.
//! Commands follow Article XI compliance (backend-first validation).

use crate::embedded_node::{BlockchainState, EmbeddedNode, NodeState, SyncProgress};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tauri::State;

/// Global node state (shared across all commands)
pub type NodeHandle = Arc<RwLock<EmbeddedNode>>;

/// Initialize embedded blockchain node
///
/// # Arguments
/// * `data_path` - Base data directory (e.g., ~/.btpc)
/// * `network` - Network type ("mainnet", "testnet", "regtest")
///
/// # Returns
/// * `Ok(NodeState)` - Node initialized successfully
/// * `Err(String)` - Initialization failed
///
/// # Frontend Usage
/// ```javascript
/// const nodeState = await invoke('init_embedded_node', {
///   dataPath: '~/.btpc',
///   network: 'regtest'
/// });
/// console.log('Node height:', nodeState.current_height);
/// ```
#[tauri::command]
pub async fn init_embedded_node(
    data_path: String,
    network: String,
) -> Result<NodeState, String> {
    // Parse and validate inputs
    let path = std::path::PathBuf::from(shellexpand::tilde(&data_path).to_string());

    // Initialize node
    let node_arc = EmbeddedNode::new(path, &network)
        .await
        .map_err(|e| format!("Failed to initialize node: {}", e))?;

    // Get initial state
    let node = node_arc.read().await;
    let blockchain_state = node
        .get_blockchain_state()
        .await
        .map_err(|e| format!("Failed to get blockchain state: {}", e))?;

    Ok(NodeState {
        network: network.clone(),
        current_height: blockchain_state.current_height,
        is_initialized: true,
        is_syncing: blockchain_state.is_syncing,
    })
}

/// Get current blockchain state
///
/// # Returns
/// * `Ok(BlockchainState)` - Current height, best hash, UTXO count
/// * `Err(String)` - Query failed
///
/// # Performance
/// - Target: <10ms (atomic reads, no locks)
/// - vs RPC: ~50ms (IPC overhead eliminated)
///
/// # Frontend Usage
/// ```javascript
/// const state = await invoke('get_blockchain_state');
/// console.log('Height:', state.current_height);
/// console.log('UTXOs:', state.total_utxos);
/// ```
#[tauri::command]
pub async fn get_blockchain_state(
    node: State<'_, NodeHandle>,
) -> Result<BlockchainState, String> {
    let node_lock = node.read().await;
    node_lock
        .get_blockchain_state()
        .await
        .map_err(|e| format!("Failed to get blockchain state: {}", e))
}

/// Get sync progress
///
/// # Returns
/// * `Ok(SyncProgress)` - Current/target height, peers, sync percentage
/// * `Err(String)` - Query failed
///
/// # Frontend Usage
/// ```javascript
/// const progress = await invoke('get_sync_progress');
/// if (progress.is_syncing) {
///   console.log(`Syncing: ${progress.sync_percentage.toFixed(1)}%`);
///   console.log(`Height: ${progress.current_height}/${progress.target_height}`);
///   console.log(`Peers: ${progress.connected_peers}`);
/// }
/// ```
#[tauri::command]
pub async fn get_sync_progress(
    node: State<'_, NodeHandle>,
) -> Result<SyncProgress, String> {
    let node_lock = node.read().await;
    let progress = node_lock
        .get_sync_progress()
        .map_err(|e| format!("Failed to get sync progress: {}", e))?;

    eprintln!("🔍 get_sync_progress called - is_syncing: {}, percentage: {}, peers: {}",
        progress.is_syncing, progress.sync_percentage, progress.connected_peers);

    Ok(progress)
}

/// Start P2P blockchain sync (optional for testnet/mainnet)
///
/// # Returns
/// * `Ok(())` - Sync started successfully
/// * `Err(String)` - Failed to start sync
///
/// # Frontend Usage
/// ```javascript
/// await invoke('start_blockchain_sync');
/// console.log('P2P sync started');
/// ```
#[tauri::command]
pub async fn start_blockchain_sync(
    node: State<'_, NodeHandle>,
) -> Result<(), String> {
    let mut node_lock = node.write().await;
    node_lock.start_sync().await
        .map_err(|e| format!("Failed to start sync: {}", e))
}

/// Stop P2P blockchain sync
///
/// # Returns
/// * `Ok(())` - Sync stopped successfully
/// * `Err(String)` - Failed to stop sync
#[tauri::command]
pub async fn stop_blockchain_sync(
    node: State<'_, NodeHandle>,
) -> Result<(), String> {
    let mut node_lock = node.write().await;
    node_lock.stop_sync().await
        .map_err(|e| format!("Failed to stop sync: {}", e))
}

/// Gracefully shutdown embedded node
///
/// Follows shutdown sequence from research.md:
/// 1. Stop mining (caller's responsibility)
/// 2. Stop P2P sync
/// 3. Flush mempool
/// 4. Flush RocksDB WAL
/// 5. Zeroize keys
///
/// # Frontend Usage
/// ```javascript
/// await invoke('shutdown_embedded_node');
/// console.log('Node shutdown complete');
/// ```
#[tauri::command]
pub async fn shutdown_embedded_node(
    node: State<'_, NodeHandle>,
) -> Result<(), String> {
    let mut node_lock = node.write().await;
    node_lock.shutdown().await
        .map_err(|e| format!("Failed to shutdown node: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_init_embedded_node_command() {
        // Arrange
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let data_path = temp_dir.path().to_str().unwrap().to_string();

        // Act
        let result = init_embedded_node(data_path, "regtest".to_string()).await;

        // Assert
        assert!(result.is_ok(), "init_embedded_node should succeed");
        let state = result.unwrap();
        assert_eq!(state.network, "regtest");
        assert_eq!(state.current_height, 0);
        assert!(state.is_initialized);
        assert!(!state.is_syncing);
    }

    #[tokio::test]
    async fn test_get_blockchain_state_command() {
        // Arrange
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let node_arc = EmbeddedNode::new(temp_dir.path().to_path_buf(), "regtest")
            .await
            .expect("Node initialization failed");

        // Act
        let node = node_arc.read().await;
        let result = node.get_blockchain_state().await;

        // Assert
        assert!(result.is_ok(), "get_blockchain_state should succeed");
        let state = result.unwrap();
        assert_eq!(state.current_height, 0);
        assert_eq!(state.total_utxos, 0);
    }

    #[tokio::test]
    async fn test_get_sync_progress_command() {
        // Arrange
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let node_arc = EmbeddedNode::new(temp_dir.path().to_path_buf(), "regtest")
            .await
            .expect("Node initialization failed");

        // Act
        let node = node_arc.read().await;
        let result = node.get_sync_progress();

        // Assert
        assert!(result.is_ok(), "get_sync_progress should succeed");
        let progress = result.unwrap();
        assert_eq!(progress.current_height, 0);
        assert!(!progress.is_syncing);
        assert_eq!(progress.connected_peers, 0);
        assert_eq!(progress.sync_percentage, 100.0);
    }
}