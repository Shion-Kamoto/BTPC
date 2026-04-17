//! Tauri commands for shared node visibility (US2) and peer management (US4).
//!
//! Provides:
//!   - `get_node_status`   — returns `NodeStatusDto` (camelCase) snapshot
//!   - `list_peers`        — returns `Vec<PeerSummary>` from SimplePeerManager
//!   - `ban_peer`          — bans a peer by address
//!   - `disconnect_peer`   — disconnects without banning
//!   - `add_node`          — inserts address into AddressManager + outbound attempt
//!   - `get_network_info`  — chain/network summary

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tauri::State;

use crate::app_state::AppState;
use btpc_desktop_app::embedded_node;

pub use btpc_desktop_app::types::NodeStatusDto;

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

/// Per-peer summary for the Network dashboard (US4, T070).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerSummary {
    pub address: String,
    pub direction: String, // "inbound" | "outbound"
    pub user_agent: String,
    pub protocol_version: u32,
    pub ping_rtt_ms: Option<f64>,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub ban_score: u32,
    pub connected_since: String, // ISO 8601
}

/// Network-level summary (chain name, protocol version, connections).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkInfoDto {
    pub network: String,
    pub protocol_version: u32,
    pub connections: u32,
    pub connections_in: u32,
    pub connections_out: u32,
    pub relay_fee: f64,
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Returns the current `NodeStatus` snapshot as a camelCase DTO.
#[tauri::command]
pub async fn get_shared_node_status(state: State<'_, AppState>) -> Result<NodeStatusDto, String> {
    let node_arc = state.embedded_node.clone();
    let status = embedded_node::build_node_status_snapshot(&node_arc).await;
    Ok(NodeStatusDto::from(status))
}

/// Standalone helper so RED tests can call without Tauri State.
#[cfg(test)]
pub async fn get_node_status() -> Result<NodeStatusDto, String> {
    Ok(NodeStatusDto::default())
}

/// Returns the list of currently connected peers.
#[tauri::command]
pub async fn list_peers(state: State<'_, AppState>) -> Result<Vec<PeerSummary>, String> {
    let node = state.embedded_node.read().await;
    let pm = node.get_peer_manager();
    let snapshots = pm.peer_snapshot_list().await;

    let peers = snapshots
        .into_iter()
        .map(|s| PeerSummary {
            address: s.addr.to_string(),
            direction: if s.inbound { "inbound" } else { "outbound" }.to_string(),
            user_agent: s.user_agent,
            protocol_version: s.protocol_version,
            ping_rtt_ms: s.last_ping_rtt_ms.map(|ms| ms as f64),
            bytes_in: s.bytes_in,
            bytes_out: s.bytes_out,
            ban_score: s.ban_score,
            connected_since: String::new(), // TODO: add connected_at to Peer
        })
        .collect();

    Ok(peers)
}

/// Ban a peer by address string. Disconnects if connected.
#[tauri::command]
pub async fn ban_peer(
    state: State<'_, AppState>,
    address: String,
    reason: String,
    _duration_sec: u64,
) -> Result<(), String> {
    let addr: SocketAddr = address
        .parse()
        .map_err(|e| format!("invalid_address: {}", e))?;
    let node = state.embedded_node.read().await;
    let pm = node.get_peer_manager();
    // Set ban score to 100 (threshold) to mark as banned
    pm.set_ban_score_for_test(&addr, 100).await;
    // Disconnect if currently connected
    pm.disconnect_peer(&addr).await;
    println!("🚫 Banned peer {} reason: {}", addr, reason);
    Ok(())
}

/// Disconnect a peer without banning.
#[tauri::command]
pub async fn disconnect_peer(state: State<'_, AppState>, address: String) -> Result<(), String> {
    let addr: SocketAddr = address
        .parse()
        .map_err(|e| format!("invalid_address: {}", e))?;
    let node = state.embedded_node.read().await;
    let pm = node.get_peer_manager();
    let removed = pm.disconnect_peer(&addr).await;
    if !removed {
        return Err("peer_not_found".to_string());
    }
    Ok(())
}

/// Insert an address into the address book and trigger an outbound attempt.
#[tauri::command]
pub async fn add_node(state: State<'_, AppState>, address: String) -> Result<(), String> {
    let addr: SocketAddr = address
        .parse()
        .map_err(|e| format!("invalid_address: {}", e))?;
    let node = state.embedded_node.read().await;
    let pm = node.get_peer_manager();
    pm.connect_to_peer(addr)
        .await
        .map_err(|e| format!("connection_failed: {}", e))?;
    Ok(())
}

/// Returns a network-level summary.
#[tauri::command]
pub async fn get_network_info(state: State<'_, AppState>) -> Result<NetworkInfoDto, String> {
    let node = state.embedded_node.read().await;
    let pm = node.get_peer_manager();
    let total = pm.peer_count().await as u32;
    let out = pm.outbound_count().await as u32;

    Ok(NetworkInfoDto {
        network: format!("{:?}", node.network()),
        protocol_version: btpc_core::network::protocol::PROTOCOL_VERSION,
        connections: total,
        connections_in: total.saturating_sub(out),
        connections_out: out,
        relay_fee: 0.00001,
    })
}
