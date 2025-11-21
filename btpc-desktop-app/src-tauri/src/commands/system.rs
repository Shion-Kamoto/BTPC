//! System-related Tauri commands for the BTPC desktop application
//!
//! This module contains commands for:
//! - System status and health checks
//! - Setup and initialization
//! - Logs retrieval
//! - Network configuration
//! - Settings management

use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use tauri::{Emitter, State};

use crate::config::NetworkType;
use crate::error::BtpcError;
use crate::types::{LogInfo, SystemStatus};
use crate::AppState;

// ============================================================================
// Response Types
// ============================================================================

/// Response for get_all_settings command
/// Returns settings as key-value pairs for localStorage sync per Article XI (Backend-First Architecture).
#[derive(Serialize)]
pub struct GetAllSettingsResponse {
    pub success: bool,
    pub data: HashMap<String, String>,
}

// ============================================================================
// System Commands
// ============================================================================

/// Test command to verify Tauri IPC is working
#[tauri::command]
pub async fn test_command() -> Result<String, String> {
    println!("Test command called successfully!");
    Ok("Test command works!".to_string())
}

/// Get the current system status including node, wallet, and mining status
#[tauri::command]
pub async fn get_system_status(state: State<'_, AppState>) -> Result<SystemStatus, String> {
    let mut status = {
        let status_guard = state.status.read().await;
        status_guard.clone()
    };

    // Check node status using ProcessManager
    if state.process_manager.is_running("node") {
        status.node_status = "Running".to_string();
        if let Some(info) = state.process_manager.get_info("node") {
            status.node_pid = Some(info.pid);
        }
    } else {
        status.node_status = "Stopped".to_string();
        status.node_pid = None;
    }

    // Check mining status using mining_processes HashMap
    {
        let mining_processes = state.mining_processes.lock().map_err(|_| {
            BtpcError::mutex_poison("mining_processes", "get_system_status").to_string()
        })?;
        if mining_processes.contains_key("mining") {
            if status.mining_status == "Stopped" {
                status.mining_status = "Running".to_string();
            }
        } else {
            status.mining_status = "Stopped".to_string();
        }
    }

    // Update installation status
    let installation_status = state.btpc.check_installation();
    status.binaries_installed = installation_status.is_complete;

    // Update logs info
    status.logs_available = get_log_info(&state.config.log_dir);

    Ok(status)
}

/// Helper function to get log file information
fn get_log_info(log_dir: &std::path::Path) -> Vec<LogInfo> {
    let log_files = [
        ("Node Output", log_dir.join("node.log")),
        ("Node Errors", log_dir.join("node.err")),
        ("Wallet", log_dir.join("wallet.log")),
        ("Mining", log_dir.join("mining.log")),
    ];

    let mut logs = Vec::new();
    for (name, log_file) in &log_files {
        if log_file.exists() {
            if let Ok(metadata) = fs::metadata(log_file) {
                let size = metadata.len();
                logs.push(LogInfo {
                    name: name.to_string(),
                    path: log_file.display().to_string(),
                    size,
                    lines: 0, // We'll calculate this only when needed
                    recent_entries: Vec::new(),
                });
            }
        }
    }
    logs
}

/// Setup BTPC by installing binaries from build locations
#[tauri::command]
pub async fn setup_btpc(state: State<'_, AppState>) -> Result<String, String> {
    // Try to install binaries from build locations
    match state.btpc.install_binaries_from_build() {
        Ok(installed) => {
            let installation_status = state.btpc.check_installation();

            // Update status
            {
                let mut status = state.status.write().await;
                status.binaries_installed = installation_status.is_complete;
                status.config_exists = state.config.config_dir.join("launcher.toml").exists();
            }

            if installed.is_empty() {
                if installation_status.is_complete {
                    Ok("BTPC is already set up correctly".to_string())
                } else {
                    Err(format!(
                        "BTPC setup incomplete. Missing binaries: {:?}. Please build BTPC first using './build-unified-launcher.sh'",
                        installation_status.missing_required_binaries
                    ))
                }
            } else {
                Ok(format!(
                    "BTPC setup completed. Installed {} binaries: {:?}",
                    installed.len(),
                    installed
                ))
            }
        }
        Err(e) => Err(format!("BTPC setup failed: {}", e)),
    }
}

/// Get log files with recent entries
#[tauri::command]
pub async fn get_logs(state: State<'_, AppState>) -> Result<Vec<LogInfo>, String> {
    let log_files = [
        ("Node Output", state.config.log_dir.join("node.log")),
        ("Node Errors", state.config.log_dir.join("node.err")),
        ("Wallet", state.config.log_dir.join("wallet.log")),
        ("Mining", state.config.log_dir.join("mining.log")),
    ];

    let mut logs = Vec::new();

    for (name, log_file) in &log_files {
        if log_file.exists() {
            match fs::metadata(log_file) {
                Ok(metadata) => {
                    let size = metadata.len();

                    // Read recent entries
                    let recent_entries = match fs::read_to_string(log_file) {
                        Ok(content) => {
                            if content.trim().is_empty() {
                                vec![format!("No content in {} yet", name)]
                            } else {
                                let lines: Vec<&str> = content.lines().collect();
                                let recent_lines = if lines.len() > 15 {
                                    &lines[lines.len() - 15..]
                                } else {
                                    &lines
                                };
                                let mut entries: Vec<String> =
                                    recent_lines.iter().map(|s| s.to_string()).collect();

                                // Add a header for better context
                                if !entries.is_empty() {
                                    entries.insert(
                                        0,
                                        format!(
                                            "=== Last {} lines from {} ===",
                                            entries.len(),
                                            name
                                        ),
                                    );
                                }
                                entries
                            }
                        }
                        Err(_) => vec!["Error reading log file".to_string()],
                    };

                    logs.push(LogInfo {
                        name: name.to_string(),
                        path: log_file.display().to_string(),
                        size,
                        lines: recent_entries.len(),
                        recent_entries,
                    });
                }
                Err(_) => {
                    logs.push(LogInfo {
                        name: name.to_string(),
                        path: log_file.display().to_string(),
                        size: 0,
                        lines: 0,
                        recent_entries: vec!["Unable to access log file".to_string()],
                    });
                }
            }
        }
    }

    Ok(logs)
}

/// Get current network configuration
#[tauri::command]
pub async fn get_network_config(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let active_network = state.active_network.read().await.clone();
    let active_rpc_port = *state.active_rpc_port.read().await;
    let active_p2p_port = *state.active_p2p_port.read().await;

    Ok(serde_json::json!({
        "network": active_network.to_string(),
        "rpc_port": active_rpc_port,
        "p2p_port": active_p2p_port,
        "rpc_host": state.config.rpc.host,
    }))
}

/// Save network configuration (network type, RPC port, P2P port)
#[tauri::command]
pub async fn save_network_config(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    network: String,
    rpc_port: u16,
    p2p_port: u16,
) -> Result<String, String> {
    // Validate network type
    let network_type = match network.as_str() {
        "mainnet" => NetworkType::Mainnet,
        "testnet" => NetworkType::Testnet,
        "regtest" => NetworkType::Regtest,
        _ => return Err(format!("Invalid network type: {}", network)),
    };

    // Validate ports
    if rpc_port < 1024 {
        return Err("RPC port must be >= 1024".to_string());
    }
    if p2p_port < 1024 {
        return Err("P2P port must be >= 1024".to_string());
    }
    if rpc_port == p2p_port {
        return Err("RPC and P2P ports must be different".to_string());
    }

    // Check if node is running
    let node_running = state.process_manager.is_running("node");

    if node_running {
        return Err(
            "Cannot change network settings while node is running. Please stop the node first."
                .to_string(),
        );
    }

    // Update active network configuration
    {
        let mut active_network = state.active_network.write().await;
        *active_network = network_type;
    }
    {
        let mut active_rpc_port = state.active_rpc_port.write().await;
        *active_rpc_port = rpc_port;
    }
    {
        let mut active_p2p_port = state.active_p2p_port.write().await;
        *active_p2p_port = p2p_port;
    }

    // Emit event to notify all pages about network config change
    let event_payload = serde_json::json!({
        "network": network,
        "rpc_port": rpc_port,
        "p2p_port": p2p_port,
    });

    if let Err(e) = app.emit("network-config-changed", event_payload) {
        eprintln!("Failed to emit network-config-changed event: {}", e);
    } else {
        println!(
            "Emitted network-config-changed event: {} (RPC: {}, P2P: {})",
            network, rpc_port, p2p_port
        );
    }

    Ok(format!(
        "Network settings saved successfully: {} (RPC: {}, P2P: {}). Changes will be applied when the node is started.",
        network, rpc_port, p2p_port
    ))
}

/// Get all application settings from backend
///
/// Returns settings as key-value pairs for localStorage sync per Article XI (Backend-First Architecture).
/// Currently returns minimal settings since BTPC doesn't have persistent configuration storage yet.
///
/// # Frontend Usage
/// ```javascript
/// const result = await window.invoke('get_all_settings');
/// if (result.success) {
///     for (const [key, value] of Object.entries(result.data)) {
///         localStorage.setItem(key, value);
///     }
/// }
/// ```
#[tauri::command]
pub async fn get_all_settings(state: State<'_, AppState>) -> Result<GetAllSettingsResponse, String> {
    // Load settings from RocksDB (Backend-First Architecture per Article XI)
    let settings = state
        .settings_storage
        .load_all_settings()
        .map_err(|e| format!("Failed to load settings: {}", e))?;

    Ok(GetAllSettingsResponse {
        success: true,
        data: settings,
    })
}