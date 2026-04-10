//! Node-related Tauri commands for the BTPC desktop application
//!
//! This module handles node lifecycle management and blockchain synchronization.

use std::fs;
use std::io::Write;
use tauri::State;

use crate::error::BtpcError;
use crate::sync_service::{BlockchainSyncService, SyncConfig, SyncStats};
use crate::AppState;

// ============================================================================
// Node Lifecycle Commands
// ============================================================================

#[tauri::command]
pub async fn start_node(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    // Check if node is already running using ProcessManager
    if state.process_manager.is_running("node") {
        return Ok("Node is already running".to_string());
    }

    let bin_path = state.config.btpc_home.join("bin").join("btpc_node");

    if !bin_path.exists() {
        return Err("Node binary not found. Please run setup first.".to_string());
    }

    let data_dir = state.config.data_dir.join("desktop-node");
    let log_file = state.config.log_dir.join("node.log");
    let err_file = state.config.log_dir.join("node.err");

    // Ensure directories exist
    fs::create_dir_all(&data_dir).map_err(|e| format!("Failed to create data dir: {}", e))?;
    fs::create_dir_all(&state.config.log_dir)
        .map_err(|e| format!("Failed to create log dir: {}", e))?;

    // Get active network configuration
    let active_network = state.active_network.read().await.clone();
    let active_p2p_port = *state.active_p2p_port.read().await;
    let active_rpc_port = *state.active_rpc_port.read().await;

    let listen_addr = format!("127.0.0.1:{}", active_p2p_port);
    let args = vec![
        "--network".to_string(),
        active_network.to_string(),
        "--datadir".to_string(),
        data_dir.to_string_lossy().to_string(),
        "--rpcport".to_string(),
        active_rpc_port.to_string(),
        "--rpcbind".to_string(),
        "127.0.0.1".to_string(),
        "--listen".to_string(),
        listen_addr,
    ];

    // Use ProcessManager for detached process (survives page navigation)
    let process_info = state.process_manager.start_detached(
        "node".to_string(),
        bin_path.to_string_lossy().to_string(),
        args,
        Some(log_file.clone()),
        Some(err_file),
    )?;

    // Update status (old SystemStatus for backward compatibility)
    {
        let mut status = state.status.write().await;
        status.node_status = "Running".to_string();
        status.node_pid = Some(process_info.pid);
    }

    // Update NodeStatus via StateManager (Article XI - auto-emits node_status_changed event)
    state
        .node_status
        .update(
            |status| {
                status.running = true;
                status.pid = Some(process_info.pid);
                status.network = active_network.to_string();
            },
            &app,
        )
        .map_err(|e| format!("Failed to update node status: {}", e))?;

    println!("📡 StateManager auto-emitted node_status_changed event: running");

    // Write initial status to log file since node runs silently
    let initial_log_message = format!("Node started successfully at {} (PID: {})\nListening and synchronizing blockchain data...\nNetwork: {}\nRPC Port: {}\nP2P Port: {}\nData directory: {}\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        process_info.pid,
        active_network,
        active_rpc_port,
        active_p2p_port,
        data_dir.display()
    );
    let _ = fs::write(&log_file, initial_log_message);

    // Auto-start blockchain synchronization service if RPC is enabled
    if state.config.node.enable_rpc {
        // Give the node a moment to start up before attempting sync
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        let mut sync_service_guard = state
            .sync_service
            .lock()
            .map_err(|_| BtpcError::mutex_poison("sync_service", "start_node").to_string())?;

        // Only start if not already running
        if sync_service_guard.is_none() {
            let sync_config = SyncConfig {
                rpc_host: state.config.rpc.host.clone(),
                rpc_port: state.config.rpc.port,
                poll_interval_secs: 10,
                max_blocks_per_sync: 100,
            };

            let service = BlockchainSyncService::new(state.utxo_manager.clone(), sync_config, Some(app.clone()));

            match service.start() {
                Ok(_) => {
                    *sync_service_guard = Some(service);
                    println!("🔄 Blockchain sync service auto-started");
                }
                Err(e) => {
                    eprintln!("⚠️ Failed to auto-start blockchain sync: {}", e);
                    // Don't fail node startup if sync fails to start
                }
            }
        }
    }

    Ok(format!(
        "Node started successfully (PID: {})",
        process_info.pid
    ))
}

#[tauri::command]
pub async fn stop_node(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    state.process_manager.kill("node")?;

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

    println!("📡 StateManager auto-emitted node_status_changed event: stopped");

    // Append stop message to log file
    let log_file = state.config.log_dir.join("node.log");
    let stop_message = format!(
        "Node stopped at {} by user request\n",
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
    let running = state.process_manager.is_running("node");

    let (pid, status_str) = if running {
        let info = state.process_manager.get_info("node");
        (info.map(|i| i.pid), "running")
    } else {
        (None, "stopped")
    };

    Ok(serde_json::json!({
        "is_running": running,
        "running": running,  // Keep both for compatibility
        "status": status_str,
        "pid": pid
    }))
}

// ============================================================================
// Blockchain Sync Commands
// ============================================================================

#[tauri::command]
pub async fn start_blockchain_sync(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<String, String> {
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
        let service = BlockchainSyncService::new(state.utxo_manager.clone(), sync_config, Some(app.clone()));

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