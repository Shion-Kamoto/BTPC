//! Tauri commands for embedded blockchain node
//!
//! Exposes EmbeddedNode functionality to frontend via Tauri IPC.
//! Commands follow Article XI compliance (backend-first validation).

use anyhow::Result;
use btpc_desktop_app::embedded_node::{BlockchainState, EmbeddedNode, NodeState, SyncProgress};
use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

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
pub async fn init_embedded_node(data_path: String, network: String) -> Result<NodeState, String> {
    // Parse and validate inputs
    let path = std::path::PathBuf::from(shellexpand::tilde(&data_path).to_string());

    // Create UTXO manager for this node instance
    // NOTE: In production, the node is initialized in main.rs with a shared UTXO manager
    // This command exists for compatibility but should not be used directly
    let utxo_manager = btpc_desktop_app::utxo_manager::UTXOManager::new(path.clone())
        .map_err(|e| format!("Failed to create UTXO manager: {}", e))?;
    let utxo_manager_arc = std::sync::Arc::new(std::sync::Mutex::new(utxo_manager));

    // Initialize node
    let node_arc = EmbeddedNode::new(path, &network, utxo_manager_arc)
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
pub async fn get_blockchain_state(node: State<'_, NodeHandle>) -> Result<BlockchainState, String> {
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
pub async fn get_sync_progress(node: State<'_, NodeHandle>) -> Result<SyncProgress, String> {
    let node_lock = node.read().await;
    let progress = node_lock
        .get_sync_progress()
        .map_err(|e| format!("Failed to get sync progress: {}", e))?;

    eprintln!(
        "🔍 get_sync_progress called - is_syncing: {}, percentage: {}, peers: {}",
        progress.is_syncing, progress.sync_percentage, progress.connected_peers
    );

    Ok(progress)
}

/// Start P2P blockchain sync (optional for testnet/mainnet)
///
/// # Returns
/// * `Ok(())` - Sync started successfully
/// * `Err(String)` - Failed to start sync (including insufficient disk space)
///
/// # FR-058 Disk Space Check
/// Sync is paused/prevented if available space is below 5GB threshold.
///
/// # Frontend Usage
/// ```javascript
/// await invoke('start_embedded_blockchain_sync');
/// console.log('P2P sync started');
/// ```
#[tauri::command]
pub async fn start_embedded_blockchain_sync(
    node: State<'_, NodeHandle>,
    state: State<'_, crate::AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    use btpc_desktop_app::disk_space_monitor::{DiskSpaceAlertLevel, DiskSpaceMonitor};
    use tauri::Emitter;

    // FR-058: Check disk space before starting sync
    // Sync is paused if available space is below 5GB threshold
    let disk_check = state.disk_space_monitor.check().await;

    if let Ok(disk_info) = &disk_check {
        let alert_level = state.disk_space_monitor.get_alert_level().await;

        if alert_level == DiskSpaceAlertLevel::SyncPaused
            || alert_level == DiskSpaceAlertLevel::MiningPrevented
        {
            let formatted_space = DiskSpaceMonitor::format_bytes(disk_info.available_bytes);

            // Emit disk space event to frontend
            app.emit("disk:sync_paused", serde_json::json!({
                "available_bytes": disk_info.available_bytes,
                "available_formatted": formatted_space,
                "threshold_bytes": btpc_desktop_app::disk_space_monitor::THRESHOLD_PAUSE_SYNC_BYTES,
                "message": format!("Sync paused: only {} available (minimum 5GB required)", formatted_space)
            })).ok();

            return Err(format!(
                "Insufficient disk space for blockchain sync: {} available (minimum 5GB required)",
                formatted_space
            ));
        }

        // Emit warning if disk space is getting low
        if alert_level == DiskSpaceAlertLevel::Warning {
            let formatted_space = DiskSpaceMonitor::format_bytes(disk_info.available_bytes);
            app.emit("disk:space_warning", serde_json::json!({
                "available_bytes": disk_info.available_bytes,
                "available_formatted": formatted_space,
                "message": format!("Low disk space warning: {} available", formatted_space)
            })).ok();
        }
    }

    let mut node_lock = node.write().await;
    node_lock
        .start_sync()
        .await
        .map_err(|e| format!("Failed to start sync: {}", e))
}

/// Stop P2P blockchain sync
///
/// # Returns
/// * `Ok(())` - Sync stopped successfully
/// * `Err(String)` - Failed to stop sync
#[tauri::command]
pub async fn stop_embedded_blockchain_sync(node: State<'_, NodeHandle>) -> Result<(), String> {
    let mut node_lock = node.write().await;
    node_lock
        .stop_sync()
        .await
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
pub async fn shutdown_embedded_node(node: State<'_, NodeHandle>) -> Result<(), String> {
    let mut node_lock = node.write().await;
    node_lock
        .shutdown()
        .await
        .map_err(|e| format!("Failed to shutdown node: {}", e))
}

/// Get P2P peer information
///
/// # Returns
/// * `Ok(GetPeerInfoResponse)` - List of connected peers
/// * `Err(String)` - Query failed
///
/// # Note
/// Currently returns empty list as full P2P implementation is pending.
/// This provides graceful degradation for frontend network status displays.
///
/// # Frontend Usage
/// ```javascript
/// const peerInfo = await invoke('get_peer_info');
/// console.log('Connected peers:', peerInfo.total_peers);
/// console.log('Peers:', peerInfo.peers);
/// ```
///
/// # REM-C001
/// Added 2025-11-19 to complete node-api.md contract
#[tauri::command]
pub async fn get_peer_info(node: State<'_, NodeHandle>) -> Result<GetPeerInfoResponse, String> {
    let node_lock = node.read().await;
    let peers = node_lock.get_peer_info();

    Ok(GetPeerInfoResponse {
        total_peers: peers.len(),
        peers,
    })
}

/// Response structure for get_peer_info command
#[derive(Debug, serde::Serialize)]
pub struct GetPeerInfoResponse {
    pub peers: Vec<btpc_desktop_app::embedded_node::PeerInfo>,
    pub total_peers: usize,
}

/// Add simulated peer for testing P2P display
///
/// # Arguments
/// * `count` - Number of simulated peers to add (default: 1)
///
/// # Returns
/// * `Ok(usize)` - Number of peers after adding
///
/// # Note
/// This is for testing/demo purposes only. Real P2P connections will
/// replace this when the P2P network layer is fully integrated.
///
/// # Frontend Usage
/// ```javascript
/// // Add 3 simulated peers for testing
/// const peerCount = await invoke('add_simulated_peers', { count: 3 });
/// console.log('Total peers:', peerCount);
/// ```
#[tauri::command]
pub async fn add_simulated_peers(
    node: State<'_, NodeHandle>,
    count: Option<u32>,
) -> Result<usize, String> {
    let node_lock = node.read().await;
    let add_count = count.unwrap_or(1).min(10); // Cap at 10 for safety

    for _ in 0..add_count {
        node_lock.add_simulated_peer();
    }

    Ok(node_lock.get_peer_count() as usize)
}

/// Clear all connected peers
///
/// # Returns
/// * `Ok(())` - Peers cleared successfully
///
/// # Frontend Usage
/// ```javascript
/// await invoke('clear_peers');
/// ```
#[tauri::command]
pub async fn clear_peers(node: State<'_, NodeHandle>) -> Result<(), String> {
    let node_lock = node.read().await;
    node_lock.clear_peers();
    Ok(())
}

/// Connect to a peer by address (Real P2P connection)
///
/// # Arguments
/// * `address` - Peer address in format "IP:port" (e.g., "192.168.1.100:8333")
///
/// # Returns
/// * `Ok(PeerConnectionResult)` - Connection result with peer info
/// * `Err(String)` - Connection failed
///
/// # Frontend Usage
/// ```javascript
/// try {
///     const result = await invoke('connect_to_peer', { address: '192.168.1.100:8333' });
///     console.log('Connected to peer:', result.address);
/// } catch (e) {
///     console.error('Connection failed:', e);
/// }
/// ```
#[tauri::command]
pub async fn connect_to_peer(
    node: State<'_, NodeHandle>,
    address: String,
) -> Result<PeerConnectionResult, String> {
    use btpc_core::network::{
        NetworkConfig, NetworkAddress, ServiceFlags, VersionMessage,
        ProtocolCodec, PeerConnection,
    };
    use tokio::net::TcpStream;
    use std::net::SocketAddr;

    // Parse address
    let socket_addr: SocketAddr = address.parse()
        .map_err(|e| format!("Invalid address '{}': {}", address, e))?;

    eprintln!("🔗 Connecting to peer: {}", socket_addr);

    // Connect via TCP with 10 second timeout
    let stream = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        TcpStream::connect(socket_addr)
    ).await
        .map_err(|_| format!("Connection to {} timed out", socket_addr))?
        .map_err(|e| format!("Failed to connect to {}: {}", socket_addr, e))?;

    eprintln!("✅ TCP connected to {}", socket_addr);

    // Get network config from node
    let node_lock = node.read().await;
    let network = node_lock.network();
    let current_height = node_lock.get_sync_progress()
        .map(|p| p.current_height as u32)
        .unwrap_or(0);

    // Create protocol codec with network-specific magic bytes
    let config = match network {
        btpc_core::Network::Mainnet => NetworkConfig::mainnet(),
        btpc_core::Network::Testnet => NetworkConfig::testnet(),
        btpc_core::Network::Regtest => NetworkConfig::testnet(), // Use testnet config for regtest
    };
    let magic_bytes = config.magic_bytes();
    let codec = ProtocolCodec::new(magic_bytes);
    let mut connection = PeerConnection::new(stream, codec);

    // Create our version message
    let our_version = VersionMessage::new(
        ServiceFlags::NETWORK,
        NetworkAddress::default(),
        NetworkAddress::default(),
        format!("/btpc-desktop:{}/", env!("CARGO_PKG_VERSION")),
        current_height,
        true,
    );

    // Perform handshake with 30 second timeout
    let handshake_result = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        connection.handshake(our_version)
    ).await
        .map_err(|_| format!("Handshake with {} timed out", socket_addr))?
        .map_err(|e| format!("Handshake with {} failed: {}", socket_addr, e))?;

    eprintln!("✅ Handshake complete with {} (height: {}, version: {})",
        socket_addr, handshake_result.start_height, handshake_result.user_agent);

    // Add peer to tracking
    let peer_version = handshake_result.user_agent.clone();
    let peer_height = handshake_result.start_height as u64;

    node_lock.add_peer(
        address.clone(),
        peer_version.clone(),
        peer_height,
    );

    // Measure ping latency (send ping, wait for pong)
    let ping_start = std::time::Instant::now();
    let ping_nonce: u64 = rand::random();

    if let Err(e) = connection.send_message(&btpc_core::network::Message::Ping(ping_nonce)).await {
        eprintln!("⚠️ Failed to send ping: {}", e);
    } else {
        // Wait for pong with 5 second timeout
        match tokio::time::timeout(
            std::time::Duration::from_secs(5),
            connection.receive_message()
        ).await {
            Ok(Ok(btpc_core::network::Message::Pong(_))) => {
                let ping_ms = ping_start.elapsed().as_millis() as u64;
                node_lock.update_peer_ping(&address, ping_ms);
                eprintln!("📍 Ping: {}ms", ping_ms);
            }
            _ => {
                eprintln!("⚠️ No pong received");
            }
        }
    }

    // TODO: Spawn a background task to maintain this connection
    // For now, the connection will close after this function returns
    // The peer info is still tracked and displayed

    Ok(PeerConnectionResult {
        address,
        version: peer_version,
        height: peer_height,
        success: true,
        message: "Connected successfully".to_string(),
    })
}

/// Result of peer connection attempt
#[derive(Debug, serde::Serialize)]
pub struct PeerConnectionResult {
    pub address: String,
    pub version: String,
    pub height: u64,
    pub success: bool,
    pub message: String,
}

/// Get disk space information (FR-058)
///
/// # Returns
/// * `Ok(DiskSpaceResponse)` - Current disk space status
/// * `Err(String)` - Query failed
///
/// # Frontend Usage
/// ```javascript
/// const diskInfo = await invoke('get_disk_space_info');
/// console.log('Available:', diskInfo.available_formatted);
/// console.log('Alert level:', diskInfo.alert_level);
/// if (diskInfo.can_mine && diskInfo.can_sync) {
///   console.log('All operations allowed');
/// }
/// ```
#[tauri::command]
pub async fn get_disk_space_info(
    state: State<'_, crate::AppState>,
) -> Result<DiskSpaceResponse, String> {
    use btpc_desktop_app::disk_space_monitor::{DiskSpaceAlertLevel, DiskSpaceMonitor};

    let disk_info = state.disk_space_monitor.check().await
        .map_err(|e| format!("Failed to check disk space: {}", e))?;
    let alert_level = state.disk_space_monitor.get_alert_level().await;

    Ok(DiskSpaceResponse {
        available_bytes: disk_info.available_bytes,
        total_bytes: disk_info.total_bytes,
        used_bytes: disk_info.used_bytes,
        available_formatted: DiskSpaceMonitor::format_bytes(disk_info.available_bytes),
        total_formatted: DiskSpaceMonitor::format_bytes(disk_info.total_bytes),
        used_formatted: DiskSpaceMonitor::format_bytes(disk_info.used_bytes),
        usage_percentage: disk_info.usage_percent,
        alert_level: format!("{:?}", alert_level),
        can_sync: !matches!(
            alert_level,
            DiskSpaceAlertLevel::SyncPaused | DiskSpaceAlertLevel::MiningPrevented
        ),
        can_mine: !matches!(alert_level, DiskSpaceAlertLevel::MiningPrevented),
        partition_path: disk_info.partition.clone(),
    })
}

/// Response structure for get_disk_space_info command
#[derive(Debug, serde::Serialize)]
pub struct DiskSpaceResponse {
    pub available_bytes: u64,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_formatted: String,
    pub total_formatted: String,
    pub used_formatted: String,
    pub usage_percentage: f64,
    pub alert_level: String,
    pub can_sync: bool,
    pub can_mine: bool,
    pub partition_path: String,
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
        let utxo_manager =
            btpc_desktop_app::utxo_manager::UTXOManager::new(temp_dir.path().to_path_buf())
                .expect("Failed to create UTXO manager");
        let utxo_manager_arc = std::sync::Arc::new(std::sync::Mutex::new(utxo_manager));
        let node_arc =
            EmbeddedNode::new(temp_dir.path().to_path_buf(), "regtest", utxo_manager_arc)
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
        let utxo_manager =
            btpc_desktop_app::utxo_manager::UTXOManager::new(temp_dir.path().to_path_buf())
                .expect("Failed to create UTXO manager");
        let utxo_manager_arc = std::sync::Arc::new(std::sync::Mutex::new(utxo_manager));
        let node_arc =
            EmbeddedNode::new(temp_dir.path().to_path_buf(), "regtest", utxo_manager_arc)
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
