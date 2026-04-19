use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatus {
    pub node_status: String,
    pub node_pid: Option<u32>,
    pub wallet_balance: String,
    pub mining_status: String,
    pub binaries_installed: bool,
    pub config_exists: bool,
    pub logs_available: Vec<LogInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogInfo {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub lines: usize,
    pub recent_entries: Vec<String>,
}

// Article XI-compliant state structures for StateManager
// These are serializable and suitable for automatic event emission

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStatus {
    pub running: bool,
    pub pid: Option<u32>,
    pub block_height: u64,
    pub peer_count: u32,
    pub sync_progress: f64, // 0.0 to 1.0
    pub network: String,    // "mainnet", "testnet", "regtest"
    pub difficulty: f64,    // Mining difficulty (1.0 = minimum)
    // --- 003-testnet-p2p-hardening: extended shared-status fields ---
    pub tip_hash: Option<String>,
    pub headers_height: u64,
    pub is_syncing: bool,
    pub last_block_time: Option<u64>, // unix seconds
    pub peer_count_in: u32,
    pub peer_count_out: u32,
    pub mempool_size: u32,
    pub ban_count: u32,
    pub generated_at: u64, // unix seconds when snapshot was built
}

impl Default for NodeStatus {
    fn default() -> Self {
        Self {
            running: false,
            pid: None,
            block_height: 0,
            peer_count: 0,
            sync_progress: 0.0,
            network: "mainnet".to_string(),
            difficulty: 1.0,
            tip_hash: None,
            headers_height: 0,
            is_syncing: false,
            last_block_time: None,
            peer_count_in: 0,
            peer_count_out: 0,
            mempool_size: 0,
            ban_count: 0,
            generated_at: 0,
        }
    }
}

/// camelCase DTO for the JS/UI layer. Built via `From<NodeStatus>`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeStatusDto {
    pub running: bool,
    pub block_height: u64,
    pub peer_count: u32,
    pub peer_count_in: u32,
    pub peer_count_out: u32,
    pub sync_progress: f64,
    pub network: String,
    pub difficulty: f64,
    pub tip_hash: String,
    pub headers_height: u64,
    pub is_syncing: bool,
    pub last_block_time: Option<u64>,
    pub mempool_size: u32,
    pub ban_count: u32,
    pub generated_at: u64,
}

impl From<NodeStatus> for NodeStatusDto {
    fn from(ns: NodeStatus) -> Self {
        NodeStatusDto {
            running: ns.running,
            block_height: ns.block_height,
            peer_count: ns.peer_count,
            peer_count_in: ns.peer_count_in,
            peer_count_out: ns.peer_count_out,
            sync_progress: ns.sync_progress,
            network: ns.network,
            difficulty: ns.difficulty,
            tip_hash: ns.tip_hash.unwrap_or_default(),
            headers_height: ns.headers_height,
            is_syncing: ns.is_syncing,
            last_block_time: ns.last_block_time,
            mempool_size: ns.mempool_size,
            ban_count: ns.ban_count,
            generated_at: ns.generated_at,
        }
    }
}

#[cfg(test)]
mod red_phase_dto_tests {
    //! T101 RED-phase — `NodeStatusDto` + `From<NodeStatus>` do not yet exist.
    //!
    //! DTO must serialize with camelCase keys for the JS layer. GREEN impl
    //! in Phase 4 (US2) introduces `NodeStatusDto` and the conversion.

    use super::NodeStatus;
    use crate::types::status::NodeStatusDto;

    #[test]
    fn from_node_status_preserves_all_fields() {
        let ns = NodeStatus {
            running: true,
            block_height: 42,
            headers_height: 50,
            peer_count: 4,
            peer_count_in: 1,
            peer_count_out: 3,
            is_syncing: true,
            tip_hash: Some("deadbeef".to_string()),
            last_block_time: Some(1_700_000_000),
            mempool_size: 7,
            ban_count: 2,
            generated_at: 1_700_000_100,
            ..NodeStatus::default()
        };
        let dto: NodeStatusDto = NodeStatusDto::from(ns.clone());
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["blockHeight"], 42);
        assert_eq!(json["headersHeight"], 50);
        assert_eq!(json["peerCountIn"], 1);
        assert_eq!(json["peerCountOut"], 3);
        assert_eq!(json["isSyncing"], true);
        assert_eq!(json["tipHash"], "deadbeef");
        assert_eq!(json["lastBlockTime"], 1_700_000_000u64);
        assert_eq!(json["mempoolSize"], 7);
        assert_eq!(json["banCount"], 2);
        assert_eq!(json["generatedAt"], 1_700_000_100u64);
    }

    #[test]
    fn dto_never_emits_snake_case() {
        let dto = NodeStatusDto::from(NodeStatus::default());
        let s = serde_json::to_string(&dto).unwrap();
        assert!(!s.contains("block_height"));
        assert!(!s.contains("peer_count_in"));
        assert!(!s.contains("is_syncing"));
    }
}
