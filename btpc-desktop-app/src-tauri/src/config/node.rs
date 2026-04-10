use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub sync_interval_secs: u64,
    pub max_peers: u32,
    pub listen_port: u16,
    pub enable_rpc: bool,
}