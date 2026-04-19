//! Node-related Tauri commands for the BTPC desktop application
//!
//! This module handles node lifecycle management and blockchain synchronization.
//! Feature 013: Uses embedded blockchain node (no external btpc_node binary required).

use std::fs;
use std::io::Write;
use std::sync::atomic::Ordering;
use tauri::State;

use crate::error::BtpcError;
use crate::sync_service::{BlockchainSyncService, SyncConfig, SyncStats};
use crate::AppState;

// ============================================================================
// Node Lifecycle Commands (Embedded Node - Feature 013)
// ============================================================================

#[tauri::command]
pub async fn start_node(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    // Check if node is already active via the embedded node flag
    if state.node_active.load(Ordering::SeqCst) {
        return Ok("Node is already running".to_string());
    }

    // Ensure log directory exists
    fs::create_dir_all(&state.config.log_dir)
        .map_err(|e| format!("Failed to create log dir: {}", e))?;

    // Get active network configuration
    let active_network = state.active_network.read().await.clone();

    // Start embedded node sync with block processing enabled
    // FIX 2026-04-12: Pass Arc reference so event loop can process blocks from peers
    {
        let node_arc = state.embedded_node.clone();
        let mut node = state.embedded_node.write().await;
        node.start_sync_with_block_processing(node_arc)
            .await
            .map_err(|e| format!("Failed to start embedded node sync: {}", e))?;
    }

    // Mark node as active
    state.node_active.store(true, Ordering::SeqCst);

    // Update status (old SystemStatus for backward compatibility)
    {
        let mut status = state.status.write().await;
        status.node_status = "Running".to_string();
        status.node_pid = None; // Embedded node has no separate PID
    }

    // Update NodeStatus via StateManager (Article XI - auto-emits node_status_changed event)
    state
        .node_status
        .update(
            |status| {
                status.running = true;
                status.pid = None; // No separate process - embedded node
                status.network = active_network.to_string();
            },
            &app,
        )
        .map_err(|e| format!("Failed to update node status: {}", e))?;

    println!(
        "📡 Embedded node started — StateManager auto-emitted node_status_changed event: running"
    );

    // Write status to log file
    let log_file = state.config.log_dir.join("node.log");
    let log_message = format!(
        "Embedded node started at {}\nNetwork: {}\nMode: in-process (no external binary)\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        active_network,
    );
    let _ = fs::write(&log_file, log_message);

    // FIX 2026-04-12: Disabled RPC-based BlockchainSyncService.
    // The embedded node does not run a separate RPC server, so the RPC sync
    // service would silently fail every 10 seconds (rpc_client.ping() fails).
    // Block synchronization is now handled by the P2P event loop in
    // start_sync_with_block_processing() which processes BlockReceived and
    // InventoryReceived events from connected peers.
    //
    // The RPC sync service remains available for future use if an external
    // btpc_node with RPC is running alongside the embedded node.
    #[allow(clippy::overly_complex_bool_expr)]
    // RPC sync intentionally disabled; kept for future use
    if false && state.config.node.enable_rpc {
        let fork_id = state.active_network.read().await.fork_id();

        let mut sync_service_guard = state
            .sync_service
            .lock()
            .map_err(|_| BtpcError::mutex_poison("sync_service", "start_node").to_string())?;

        if sync_service_guard.is_none() {
            let sync_config = SyncConfig {
                rpc_host: state.config.rpc.host.clone(),
                rpc_port: state.config.rpc.port,
                poll_interval_secs: 10,
                max_blocks_per_sync: 100,
            };

            let service = BlockchainSyncService::new(
                state.utxo_manager.clone(),
                sync_config,
                Some(app.clone()),
                fork_id,
            );

            match service.start() {
                Ok(_) => {
                    *sync_service_guard = Some(service);
                    println!("🔄 Blockchain sync service auto-started");
                }
                Err(e) => {
                    eprintln!("⚠️ Failed to auto-start blockchain sync: {}", e);
                }
            }
        }
    }

    Ok("Node started successfully (embedded)".to_string())
}

#[tauri::command]
pub async fn stop_node(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    // Check if node is active
    if !state.node_active.load(Ordering::SeqCst) {
        // Node is not running, but update state to ensure consistency
        state
            .node_status
            .update(
                |status| {
                    status.running = false;
                    status.pid = None;
                    status.block_height = 0;
                    status.peer_count = 0;
                    status.sync_progress = 0.0;
                },
                &app,
            )
            .map_err(|e| format!("Failed to update node status: {}", e))?;
        return Ok("Node is already stopped".to_string());
    }

    // Stop embedded node sync
    {
        let mut node = state.embedded_node.write().await;
        node.stop_sync()
            .await
            .map_err(|e| format!("Failed to stop embedded node sync: {}", e))?;
    }

    // Mark node as inactive
    state.node_active.store(false, Ordering::SeqCst);

    // Stop blockchain sync service if running
    {
        let mut sync_service_guard = state
            .sync_service
            .lock()
            .map_err(|_| BtpcError::mutex_poison("sync_service", "stop_node").to_string())?;
        if let Some(service) = sync_service_guard.as_ref() {
            service.stop();
            *sync_service_guard = None;
            println!("🛑 Blockchain sync service stopped");
        }
    }

    // Update status (old SystemStatus for backward compatibility)
    {
        let mut status = state.status.write().await;
        status.node_status = "Stopped".to_string();
        status.node_pid = None;
    }

    // Update NodeStatus via StateManager (Article XI - auto-emits node_status_changed event)
    state
        .node_status
        .update(
            |status| {
                status.running = false;
                status.pid = None;
                status.block_height = 0;
                status.peer_count = 0;
                status.sync_progress = 0.0;
            },
            &app,
        )
        .map_err(|e| format!("Failed to update node status: {}", e))?;

    println!(
        "📡 Embedded node stopped — StateManager auto-emitted node_status_changed event: stopped"
    );

    // Append stop message to log file
    let log_file = state.config.log_dir.join("node.log");
    let stop_message = format!(
        "Embedded node stopped at {} by user request\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    );
    let _ = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
        .and_then(|mut f| f.write_all(stop_message.as_bytes()));

    Ok("Node stopped successfully".to_string())
}

#[tauri::command]
pub async fn get_node_status(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let running = state.node_active.load(Ordering::SeqCst);

    // Get additional info from embedded node if running
    // FIX 2026-04-12: Use get_peer_count() directly for accurate P2P peer count
    // instead of get_sync_progress().connected_peers which may not update.
    let (block_height, peer_count) = if running {
        let node = state.embedded_node.read().await;
        let height = node
            .get_blockchain_state()
            .await
            .map(|s| s.current_height)
            .unwrap_or(0);
        let peers = node.get_peer_count();
        (height, peers)
    } else {
        (0, 0)
    };

    Ok(serde_json::json!({
        "is_running": running,
        "running": running,
        "status": if running { "running" } else { "stopped" },
        "pid": null,
        "block_height": block_height,
        "peer_count": peer_count
    }))
}

// ============================================================================
// Node Settings Commands
// ============================================================================

#[tauri::command]
pub async fn set_auto_connect(state: State<'_, AppState>, enabled: bool) -> Result<String, String> {
    state
        .settings_storage
        .save_setting("auto_connect_node", if enabled { "true" } else { "false" })
        .map_err(|e| format!("Failed to save setting: {}", e))?;
    Ok(format!("Auto-connect set to {}", enabled))
}

// ============================================================================
// Blockchain Sync Commands
// ============================================================================

#[tauri::command]
pub async fn start_blockchain_sync(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    // Get fork_id before acquiring sync_service lock (avoid holding MutexGuard across await)
    let fork_id = state.active_network.read().await.fork_id();

    // Create sync service if not already created
    {
        let mut sync_service_guard = state.sync_service.lock().map_err(|_| {
            BtpcError::mutex_poison("sync_service", "start_blockchain_sync").to_string()
        })?;

        if sync_service_guard.is_some() {
            return Err("Blockchain sync is already running".to_string());
        }

        // Create sync config from app config
        let sync_config = SyncConfig {
            rpc_host: state.config.rpc.host.clone(),
            rpc_port: state.config.rpc.port,
            poll_interval_secs: 10,
            max_blocks_per_sync: 100,
        };

        // Create new sync service
        let service = BlockchainSyncService::new(
            state.utxo_manager.clone(),
            sync_config,
            Some(app.clone()),
            fork_id,
        );

        // Start the service
        service
            .start()
            .map_err(|e| format!("Failed to start sync service: {}", e))?;

        *sync_service_guard = Some(service);
    }

    Ok("Blockchain synchronization started successfully".to_string())
}

#[tauri::command]
pub async fn stop_blockchain_sync(state: State<'_, AppState>) -> Result<String, String> {
    let mut sync_service_guard = state
        .sync_service
        .lock()
        .map_err(|_| BtpcError::mutex_poison("sync_service", "stop_blockchain_sync").to_string())?;

    if let Some(service) = sync_service_guard.as_ref() {
        service.stop();
        *sync_service_guard = None;
        Ok("Blockchain synchronization stopped successfully".to_string())
    } else {
        Err("Blockchain sync is not running".to_string())
    }
}

#[tauri::command]
pub async fn get_sync_stats(state: State<'_, AppState>) -> Result<SyncStats, String> {
    let sync_service_guard = state
        .sync_service
        .lock()
        .map_err(|_| BtpcError::mutex_poison("sync_service", "get_sync_stats").to_string())?;

    if let Some(service) = sync_service_guard.as_ref() {
        Ok(service.get_stats())
    } else {
        // Return default stats if sync service is not running
        Ok(SyncStats::default())
    }
}

#[tauri::command]
pub async fn trigger_manual_sync(_state: State<'_, AppState>) -> Result<String, String> {
    // Manual sync is automatically handled by the background sync service
    // This command is kept for future manual sync trigger implementation
    Ok("Manual sync will be triggered in the next sync iteration".to_string())
}

#[tauri::command]
pub async fn get_address_balance_from_node(
    state: State<'_, AppState>,
    address: String,
) -> Result<u64, String> {
    // Create a temporary RPC client to query the node
    use btpc_desktop_app::rpc_client::RpcClient;
    let rpc_client = RpcClient::new(&state.config.rpc.host, state.config.rpc.port);

    rpc_client
        .get_address_balance(&address)
        .await
        .map_err(|e| format!("Failed to get balance from node: {}", e))
}
