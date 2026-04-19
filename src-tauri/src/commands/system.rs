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
///
/// Note: Testnet and Regtest are developer-only networks. The frontend controls
/// access via Developer Mode (10x click on version number). This backend guard
/// provides defense-in-depth by rejecting dev networks if bypassed in UI.
#[tauri::command]
pub async fn save_network_config(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    network: String,
    rpc_port: u16,
    p2p_port: u16,
    developer_mode: Option<bool>,
) -> Result<String, String> {
    // Validate network type
    let network_type = match network.as_str() {
        "mainnet" => NetworkType::Mainnet,
        "testnet" => {
            // Testnet requires developer mode
            if developer_mode != Some(true) {
                return Err("Testnet is only available in Developer Mode. Enable Developer Mode in Settings first.".to_string());
            }
            NetworkType::Testnet
        }
        "regtest" => {
            // Regtest requires developer mode
            if developer_mode != Some(true) {
                return Err("Regtest is only available in Developer Mode. Enable Developer Mode in Settings first.".to_string());
            }
            NetworkType::Regtest
        }
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
        *active_network = network_type.clone();
    }
    {
        let mut active_rpc_port = state.active_rpc_port.write().await;
        *active_rpc_port = rpc_port;
    }
    {
        let mut active_p2p_port = state.active_p2p_port.write().await;
        *active_p2p_port = p2p_port;
    }

    // FIX 2025-12-01: Update embedded node's network to match active_network
    // This ensures UTXOs created during mining have the correct network prefix
    // to match wallet addresses (critical for balance display)
    {
        let core_network: btpc_core::Network = network_type.clone().into();
        let mut embedded_node = state.embedded_node.write().await;
        embedded_node.update_network(core_network);
        println!("Updated embedded node network to: {}", network);
    }

    // FIX 2025-12-05: Reinitialize all network-specific data stores at runtime
    // This eliminates the need for app restart when switching networks
    // Reinitializes: tx_storage, utxo_manager, wallet_manager, address_book_manager
    if let Err(e) = state.reinitialize_for_network(&network_type).await {
        eprintln!(
            "⚠️ Failed to reinitialize data stores for network '{}': {}",
            network, e
        );
        // Don't fail the command - user can still restart manually
    } else {
        println!("✅ All data stores reinitialized for network '{}'", network);
    }

    // FIX 2025-12-01: Persist network to settings storage so it loads on restart
    if let Err(e) = state.settings_storage.save_setting("network", &network) {
        eprintln!("Failed to persist network to settings: {}", e);
    }
    if let Err(e) = state
        .settings_storage
        .save_setting("rpc_port", &rpc_port.to_string())
    {
        eprintln!("Failed to persist rpc_port to settings: {}", e);
    }
    if let Err(e) = state
        .settings_storage
        .save_setting("p2p_port", &p2p_port.to_string())
    {
        eprintln!("Failed to persist p2p_port to settings: {}", e);
    }

    // Emit event to notify all pages about network config change
    // FIX 2025-12-05: restart_required changed to false since we now reinitialize at runtime
    let event_payload = serde_json::json!({
        "network": network,
        "rpc_port": rpc_port,
        "p2p_port": p2p_port,
        "restart_required": false,  // No longer required - data stores reinitialized at runtime
    });

    if let Err(e) = app.emit("network-config-changed", event_payload) {
        eprintln!("Failed to emit network-config-changed event: {}", e);
    } else {
        println!(
            "Emitted network-config-changed event: {} (RPC: {}, P2P: {})",
            network, rpc_port, p2p_port
        );
    }

    // FIX 2025-12-05: Network switching now works without restart
    Ok(format!(
        "Network changed to {}. All data stores reinitialized. RPC: {}, P2P: {}",
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
pub async fn get_all_settings(
    state: State<'_, AppState>,
) -> Result<GetAllSettingsResponse, String> {
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

/// Debug command to dump the current persistence state
///
/// Returns detailed information about:
/// - Active network configuration
/// - File paths being used
/// - UTXO count and data
/// - Wallet count and addresses
/// - Settings storage contents
///
/// This is useful for debugging persistence issues.
#[tauri::command]
pub async fn debug_dump_persistence_state(state: State<'_, AppState>) -> Result<String, String> {
    let mut output = String::new();

    output.push_str("=== BTPC PERSISTENCE STATE DEBUG DUMP ===\n\n");

    // 1. Active network
    let active_network = state.active_network.read().await;
    output.push_str(&format!("📡 Active Network: {:?}\n", *active_network));
    output.push_str(&format!("📁 Base Data Dir: {:?}\n", state.config.data_dir));

    let network_str = match *active_network {
        NetworkType::Mainnet => "mainnet",
        NetworkType::Testnet => "testnet",
        NetworkType::Regtest => "regtest",
    };
    let network_data_dir = state.config.data_dir.join(network_str);
    output.push_str(&format!("📁 Network Data Dir: {:?}\n\n", network_data_dir));
    drop(active_network);

    // 2. Settings from RocksDB
    output.push_str("=== SETTINGS STORAGE ===\n");
    match state.settings_storage.load_all_settings() {
        Ok(settings) => {
            output.push_str(&format!("Total settings: {}\n", settings.len()));
            for (key, value) in &settings {
                output.push_str(&format!("  {} = {}\n", key, value));
            }
        }
        Err(e) => {
            output.push_str(&format!("ERROR loading settings: {}\n", e));
        }
    }
    output.push('\n');

    // 3. UTXO Manager state
    output.push_str("=== UTXO MANAGER ===\n");
    match state.utxo_manager.lock() {
        Ok(utxo_manager) => {
            let stats = utxo_manager.get_stats();
            output.push_str(&format!(
                "UTXO file: {:?}\n",
                utxo_manager.get_utxo_file_path()
            ));
            output.push_str(&format!("Total UTXOs: {}\n", stats.total_utxos));
            output.push_str(&format!("Unspent UTXOs: {}\n", stats.unspent_utxos));
            output.push_str(&format!("Total value: {:.8} BTP\n", stats.total_value_btp));
        }
        Err(e) => {
            output.push_str(&format!("ERROR locking UTXO manager: {}\n", e));
        }
    }
    output.push('\n');

    // 4. Wallet Manager state
    output.push_str("=== WALLET MANAGER ===\n");
    match state.wallet_manager.lock() {
        Ok(wallet_manager) => {
            let wallets = wallet_manager.list_wallets();
            output.push_str(&format!("Total wallets: {}\n", wallets.len()));
            for wallet in &wallets {
                output.push_str(&format!(
                    "  - {} ({}...)\n",
                    wallet.nickname,
                    &wallet.address[..20.min(wallet.address.len())]
                ));
            }
        }
        Err(e) => {
            output.push_str(&format!("ERROR locking wallet manager: {}\n", e));
        }
    }
    output.push('\n');

    // 5. Embedded node state
    output.push_str("=== EMBEDDED NODE ===\n");
    let embedded_node = state.embedded_node.read().await;
    match embedded_node.get_blockchain_state().await {
        Ok(blockchain_state) => {
            output.push_str(&format!("Network: {:?}\n", embedded_node.get_network()));
            output.push_str(&format!(
                "Current height: {}\n",
                blockchain_state.current_height
            ));
            output.push_str(&format!(
                "Difficulty bits: 0x{:08x}\n",
                blockchain_state.difficulty_bits
            ));
        }
        Err(e) => {
            output.push_str(&format!("ERROR getting blockchain state: {}\n", e));
        }
    }
    drop(embedded_node);
    output.push('\n');

    // 6. File system check
    output.push_str("=== FILE SYSTEM CHECK ===\n");
    let utxo_file = network_data_dir.join("wallet").join("wallet_utxos.json");
    let wallet_metadata = network_data_dir
        .join("wallets")
        .join("wallets_metadata.json");
    let settings_db = state.config.data_dir.join("settings");

    output.push_str(&format!(
        "UTXO file exists: {} ({:?})\n",
        utxo_file.exists(),
        utxo_file
    ));
    output.push_str(&format!(
        "Wallet metadata exists: {} ({:?})\n",
        wallet_metadata.exists(),
        wallet_metadata
    ));
    output.push_str(&format!(
        "Settings DB exists: {} ({:?})\n",
        settings_db.exists(),
        settings_db
    ));

    output.push_str("\n=== END DEBUG DUMP ===\n");

    // Also print to terminal
    eprintln!("{}", output);

    Ok(output)
}

/// Diagnostic command to dump block timestamps for difficulty adjustment analysis
/// Shows timestamps at key block heights (1, 2015, 2016, 4031, 4032, 6047, 6048)
#[tauri::command]
pub async fn debug_dump_difficulty_timestamps(
    state: State<'_, AppState>,
) -> Result<String, BtpcError> {
    let mut output = String::new();
    output.push_str("=== DIFFICULTY ADJUSTMENT TIMESTAMP ANALYSIS ===\n\n");

    let embedded_node = state.embedded_node.read().await;
    let database = embedded_node.get_database();

    // Get current height and difficulty from blockchain state
    let blockchain_state = embedded_node
        .get_blockchain_state()
        .await
        .map_err(|e| BtpcError::Application(format!("Failed to get blockchain state: {}", e)))?;
    let current_height = blockchain_state.current_height;
    output.push_str(&format!("Current blockchain height: {}\n", current_height));
    output.push_str(&format!("Network: {:?}\n\n", embedded_node.get_network()));

    // Key heights for difficulty adjustment analysis
    let key_heights: Vec<u32> = vec![
        0,    // Genesis
        1,    // First mined block (used instead of genesis for first period)
        2015, // End of first period
        2016, // First adjustment
        4031, // End of second period
        4032, // Second adjustment
        6047, // End of third period
        6048, // Third adjustment
    ];

    output.push_str("=== BLOCK TIMESTAMPS ===\n");
    output.push_str("Height  | Timestamp (Unix)   | Human-Readable (UTC)\n");
    output.push_str("--------|--------------------|-----------------------\n");

    for height in key_heights {
        if height as u64 > current_height {
            output.push_str(&format!("{:7} | (not yet mined)\n", height));
            continue;
        }

        match database.get_block(height) {
            Ok(Some(block)) => {
                let timestamp = block.header.timestamp;
                // Convert to human-readable
                let datetime = chrono::DateTime::from_timestamp(timestamp as i64, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| "Invalid timestamp".to_string());
                output.push_str(&format!("{:7} | {:18} | {}\n", height, timestamp, datetime));
            }
            Ok(None) => {
                output.push_str(&format!("{:7} | (block not found in database)\n", height));
            }
            Err(e) => {
                output.push_str(&format!("{:7} | ERROR: {}\n", height, e));
            }
        }
    }

    // Calculate timespans for each period
    output.push_str("\n=== ADJUSTMENT PERIOD ANALYSIS ===\n");
    output.push_str("Expected timespan: 1,209,600 seconds (14 days)\n");
    output.push_str("4x clamp range: 302,400 - 4,838,400 seconds (3.5 - 56 days)\n\n");

    let periods = vec![
        (1, 2015, "Period 1 (blocks 1-2015)"), // First period uses block 1, not genesis
        (2016, 4031, "Period 2 (blocks 2016-4031)"),
        (4032, 6047, "Period 3 (blocks 4032-6047)"),
    ];

    for (start_height, end_height, period_name) in periods {
        if end_height as u64 > current_height {
            output.push_str(&format!("{}: Not enough blocks yet\n", period_name));
            continue;
        }

        let start_block = database.get_block(start_height);
        let end_block = database.get_block(end_height);

        match (start_block, end_block) {
            (Ok(Some(start)), Ok(Some(end))) => {
                let timespan = end.header.timestamp.saturating_sub(start.header.timestamp);
                let expected: u64 = 1_209_600;
                let ratio = timespan as f64 / expected as f64;
                let clamped_ratio = ratio.clamp(0.25, 4.0);
                let days = timespan as f64 / 86400.0;

                output.push_str(&format!("{}\n", period_name));
                output.push_str(&format!(
                    "  Start: block {} @ {}\n",
                    start_height, start.header.timestamp
                ));
                output.push_str(&format!(
                    "  End:   block {} @ {}\n",
                    end_height, end.header.timestamp
                ));
                output.push_str(&format!(
                    "  Timespan: {} seconds ({:.2} days)\n",
                    timespan, days
                ));
                output.push_str(&format!(
                    "  Ratio: {:.4} (clamped: {:.4})\n",
                    ratio, clamped_ratio
                ));
                output.push_str(&format!(
                    "  Expected adjustment: {:.1}%\n\n",
                    (1.0 - clamped_ratio) * 100.0
                ));
            }
            _ => {
                output.push_str(&format!("{}: Could not retrieve blocks\n\n", period_name));
            }
        }
    }

    // Show current difficulty (from blockchain_state we retrieved earlier)
    output.push_str("=== CURRENT DIFFICULTY ===\n");
    let current_bits = blockchain_state.difficulty_bits;
    let exponent = current_bits >> 24;
    let mantissa = current_bits & 0x00ffffff;
    output.push_str(&format!(
        "Compact bits: 0x{:08x} (decimal: {})\n",
        current_bits, current_bits
    ));
    output.push_str(&format!("Exponent: {} (0x{:02x})\n", exponent, exponent));
    output.push_str(&format!("Mantissa: {} (0x{:06x})\n", mantissa, mantissa));

    output.push_str("\n=== END DIFFICULTY ANALYSIS ===\n");

    // Also print to terminal
    eprintln!("{}", output);

    Ok(output)
}
