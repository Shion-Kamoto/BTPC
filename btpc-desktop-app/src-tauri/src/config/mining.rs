use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningConfig {
    pub enabled: bool,
    pub threads: u32,
    pub target_address: Option<String>,
    pub blocks_to_mine: u32,
    pub mining_interval_ms: u64,
}