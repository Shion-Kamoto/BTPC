//! Test stubs for embedded node commands
//!
//! These functions provide test-compatible versions of the embedded node commands
//! that don't require Tauri's AppState.

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::embedded_node::EmbeddedNode;
use crate::utxo_manager::UTXOManager;

// Re-export types needed by tests
pub use crate::embedded_node::{BlockchainState, SyncProgress};

/// Node state enum for tests
#[derive(Debug, Clone, PartialEq)]
pub enum NodeState {
    Stopped,
    Starting,
    Running,
    Stopping,
    Error(String),
}

/// Initialize embedded node (test stub)
///
/// Creates a new embedded node instance for testing purposes.
/// Unlike the production version, this doesn't store the node in AppState.
pub async fn init_embedded_node(data_path: String, network: String) -> Result<Arc<RwLock<EmbeddedNode>>> {
    let path = std::path::PathBuf::from(&data_path);

    // Create UTXO manager
    let utxo_manager = UTXOManager::new(path.clone())?;
    let utxo_manager_arc = Arc::new(std::sync::Mutex::new(utxo_manager));

    // Initialize embedded node
    let node = EmbeddedNode::new(path, &network, utxo_manager_arc).await?;

    Ok(node)
}

/// Get blockchain state (test stub)
pub async fn get_blockchain_state(node: &Arc<RwLock<EmbeddedNode>>) -> Result<crate::embedded_node::BlockchainState> {
    let node_read = node.read().await;
    node_read.get_blockchain_state().await
}

/// Get sync progress (test stub)
pub fn get_sync_progress(node: &Arc<RwLock<EmbeddedNode>>) -> Result<crate::embedded_node::SyncProgress> {
    // Use tokio's block_on for sync context
    let rt = tokio::runtime::Handle::try_current()
        .map_err(|_| anyhow::anyhow!("No tokio runtime"))?;

    rt.block_on(async {
        let node_read = node.read().await;
        node_read.get_sync_progress()
    })
}