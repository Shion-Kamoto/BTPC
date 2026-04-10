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
        }
    }
}