//! Test stubs for mining commands
//!
//! These functions provide test-compatible versions of the mining commands
//! that don't require Tauri's AppState.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::mining_thread_pool::MiningThreadPool;

// Re-export MiningStats for tests
pub use crate::mining_thread_pool::MiningStats;

/// Pool mining configuration (test stub mirror)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolMiningConfig {
    pub url: String,
    pub worker: String,
    #[serde(default)]
    pub password: String,
}

/// Mining configuration for test commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningConfig {
    pub enable_cpu: bool,
    pub enable_gpu: bool,
    pub cpu_threads: Option<u32>,
    pub mining_address: String,
    /// Mining mode: "solo" (default) or "pool"
    #[serde(default = "default_mining_mode")]
    pub mining_mode: String,
    /// Pool configuration (only used when mining_mode == "pool")
    #[serde(default)]
    pub pool_config: Option<PoolMiningConfig>,
}

fn default_mining_mode() -> String {
    "solo".to_string()
}

/// Global test mining pool (lazy initialized)
static TEST_MINING_POOL: std::sync::OnceLock<Arc<Mutex<MiningThreadPool>>> = std::sync::OnceLock::new();

fn get_test_pool() -> Arc<Mutex<MiningThreadPool>> {
    TEST_MINING_POOL
        .get_or_init(|| Arc::new(Mutex::new(MiningThreadPool::new(0, 2))))
        .clone()
}

/// Start mining (test stub)
///
/// Starts CPU and/or GPU mining based on configuration.
/// This test version uses a global mining pool.
pub async fn start_mining(config: MiningConfig) -> Result<bool> {
    let pool = get_test_pool();
    let mut pool_lock = pool.lock().await;

    if config.enable_cpu {
        let _threads = pool_lock
            .start_cpu_mining(config.cpu_threads, config.mining_address.clone())
            .await?;
        return Ok(true);
    }

    Ok(true)
}

/// Stop mining (test stub)
///
/// Stops all active mining operations.
pub async fn stop_mining() -> Result<bool> {
    let pool = get_test_pool();
    let mut pool_lock = pool.lock().await;
    pool_lock.stop_all().await?;
    Ok(true)
}

/// Get mining stats (test stub)
pub async fn get_mining_stats() -> Result<crate::mining_thread_pool::MiningStats> {
    let pool = get_test_pool();
    let pool_lock = pool.lock().await;
    Ok(pool_lock.get_stats())
}