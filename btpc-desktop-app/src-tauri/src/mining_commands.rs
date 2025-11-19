//! Tauri commands for mining operations
//!
//! Exposes MiningThreadPool functionality to frontend via Tauri IPC.

use crate::mining_thread_pool::{MiningStats, MiningThreadPool};
use anyhow::Result;
use btpc_desktop_app::embedded_node::EmbeddedNode;
use btpc_desktop_app::rpc_client::{BlockTemplate, RpcClientInterface};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, State};
use tokio::sync::RwLock;

// Wrapper to implement RpcClientInterface for the specific type used in AppState
// This bridges the type system between main.rs imports and lib.rs imports
struct EmbeddedNodeRpcWrapper(Arc<RwLock<EmbeddedNode>>);

impl RpcClientInterface for EmbeddedNodeRpcWrapper {
    async fn get_block_template(&self) -> Result<BlockTemplate> {
        let node = self.0.read().await;
        node.get_block_template().await
    }

    async fn submit_block(&self, block_hex: &str) -> Result<String> {
        let mut node = self.0.write().await;
        node.submit_block(block_hex).await
    }
}

/// Global mining pool state (shared across all commands)
pub type MiningHandle = Arc<Mutex<MiningThreadPool>>;

/// Mining configuration from frontend
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MiningConfig {
    pub enable_cpu: bool,
    pub enable_gpu: bool,
    pub cpu_threads: Option<u32>,
    pub mining_address: String,
}

/// Start mining
///
/// # Arguments
/// * `config` - Mining configuration (CPU/GPU, threads, address)
///
/// # Returns
/// * `Ok(true)` - Mining started successfully
/// * `Err(String)` - Failed to start mining
///
/// # Frontend Usage
/// ```javascript
/// const config = {
///   enable_cpu: true,
///   enable_gpu: false,
///   cpu_threads: null, // null = auto (num_cpus - 2)
///   mining_address: 'bcrt1q...'
/// };
/// const started = await invoke('start_mining', { config });
/// console.log('Mining started:', started);
/// ```
#[tauri::command]
pub async fn start_mining(
    state: State<'_, crate::AppState>,
    config: MiningConfig,
    app: tauri::AppHandle,
) -> Result<bool, String> {
    // Clone config fields for use after lock drop
    let enable_cpu = config.enable_cpu;
    let enable_gpu = config.enable_gpu;
    let cpu_threads = config.cpu_threads;
    let mining_address = config.mining_address.clone();

    // REM-C002: Emit mining_started event
    let thermal_limit = state.gpu_temperature_threshold.read().await;
    app.emit("mining_started", serde_json::json!({
        "devices_started": if enable_gpu { 1 } else { 0 }, // TODO: Get actual device count
        "mining_address": mining_address,
        "thermal_limit": *thermal_limit
    })).ok();

    // Get or create mining pool from AppState
    // AppState stores: Arc<RwLock<Option<MiningThreadPool>>>
    // We need to initialize if None, then get a mutable reference
    {
        let mut mining_pool_guard = state.mining_pool.write().await;
        if mining_pool_guard.is_none() {
            // Initialize mining pool on first use
            *mining_pool_guard = Some(MiningThreadPool::new());
        }
    }

    // GPU-ONLY Mining (CPU mining removed - Feature 012)
    // Start GPU mining if enabled (non-blocking, fails gracefully)
    if enable_gpu {
        let mining_pool_arc = state.mining_pool.clone();
        let gpu_address = mining_address.clone(); // Clone for GPU closure
        let logs_clone = state.mining_logs.clone();

        // Create channel for GPU mining logs
        let (log_tx, mut log_rx) = tokio::sync::mpsc::unbounded_channel::<(String, String)>();

        // REM-C002: Create channel for mining events (block_mined, gpu_error)
        let (mining_event_tx, mut mining_event_rx) = tokio::sync::mpsc::unbounded_channel::<crate::mining_thread_pool::MiningEvent>();

        // REM-C002: Spawn task to forward mining events to frontend
        let app_clone = app.clone();
        tokio::spawn(async move {
            while let Some(event) = mining_event_rx.recv().await {
                match &event {
                    crate::mining_thread_pool::MiningEvent::BlockMined { .. } => {
                        app_clone.emit("block_mined", &event).ok();
                    }
                    crate::mining_thread_pool::MiningEvent::ThermalThrottle { .. } => {
                        app_clone.emit("thermal_throttle", &event).ok();
                    }
                    crate::mining_thread_pool::MiningEvent::GpuError { .. } => {
                        app_clone.emit("gpu_error", &event).ok();
                    }
                }
            }
        });

        // Spawn task to forward GPU log events to mining_logs
        let logs_for_receiver = logs_clone.clone();
        let test_tx = log_tx.clone();
        tokio::spawn(async move {
            eprintln!("[GPU LOG RECEIVER] Started waiting for GPU mining log events");
            while let Some((level, message)) = log_rx.recv().await {
                eprintln!("[GPU LOG RECEIVER] Received log: {} - {}", level, message);
                if let Ok(mut logs) = logs_for_receiver.lock() {
                    logs.add_entry(level.clone(), message.clone());
                    eprintln!("[GPU LOG RECEIVER] Added to mining_logs buffer");
                } else {
                    eprintln!("[GPU LOG RECEIVER] FAILED to lock mining_logs");
                }
            }
            eprintln!("[GPU LOG RECEIVER] Channel closed");
        });

        // TEST: Send a test message AFTER spawning receiver
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        eprintln!("[TEST] Sending test message...");
        test_tx
            .send((
                "INFO".to_string(),
                "TEST MESSAGE - Channel created".to_string(),
            ))
            .ok();
        eprintln!("[TEST] Test message sent");

        // Use embedded node for block template requests (Feature 013: Self-contained app)
        // Replaces external RPC client with direct blockchain access
        let embedded_node = state.embedded_node.clone();

        // Spawn GPU initialization asynchronously (don't block UI thread)
        tokio::spawn(async move {
            // Access mining pool and start GPU mining
            let gpu_result = {
                let mut pool_guard = mining_pool_arc.write().await;
                if let Some(ref mut pool) = *pool_guard {
                    // REM-C002: Set mining event channel before starting
                    pool.set_event_channel(mining_event_tx);

                    // Wrap the embedded_node with our RpcClientInterface implementation
                    let rpc_wrapper = Arc::new(EmbeddedNodeRpcWrapper(embedded_node));
                    pool.start_gpu_mining(
                        gpu_address,
                        rpc_wrapper,
                        Some(log_tx),
                        Some(logs_clone.clone()),
                    )
                    .await
                } else {
                    Err(anyhow::anyhow!("Mining pool not initialized"))
                }
            };

            // Log GPU mining result (non-blocking)
            match gpu_result {
                Ok(_) => {
                    if let Ok(mut logs) = logs_clone.lock() {
                        logs.add_entry("INFO".to_string(), "GPU mining started".to_string());
                    }
                }
                Err(e) => {
                    if let Ok(mut logs) = logs_clone.lock() {
                        logs.add_entry(
                            "WARN".to_string(),
                            format!("GPU mining unavailable: {}", e),
                        );
                    }
                    eprintln!("GPU mining not available: {}", e);
                }
            }
        });

        // Return immediately without waiting for GPU init to complete
        {
            let mut mining_logs = state
                .mining_logs
                .lock()
                .map_err(|_| "Failed to lock mining_logs".to_string())?;
            mining_logs.add_entry(
                "INFO".to_string(),
                "GPU initialization started in background...".to_string(),
            );
        }
    }

    Ok(true)
}

/// Stop mining
///
/// # Returns
/// * `Ok(true)` - Mining stopped successfully
/// * `Err(String)` - Failed to stop mining
///
/// # Frontend Usage
/// ```javascript
/// const stopped = await invoke('stop_mining');
/// console.log('Mining stopped:', stopped);
/// ```
#[tauri::command]
pub async fn stop_mining(state: State<'_, crate::AppState>, app: tauri::AppHandle) -> Result<bool, String> {
    // Get stats before stopping for logging, then stop mining
    let final_stats = {
        let mut mining_pool_guard = state.mining_pool.write().await;
        if mining_pool_guard.is_none() {
            return Err("Mining pool not initialized".to_string());
        }

        if let Some(ref mut pool) = *mining_pool_guard {
            let stats = pool.get_stats();
            pool.stop_all()
                .await
                .map_err(|e| format!("Failed to stop mining: {}", e))?;
            stats
        } else {
            return Err("Mining pool not initialized".to_string());
        }
    };

    // Log mining stop with final stats
    {
        let mut mining_logs = state
            .mining_logs
            .lock()
            .map_err(|_| "Failed to lock mining_logs".to_string())?;
        mining_logs.add_entry(
            "INFO".to_string(),
            format!(
                "Mining stopped - {} blocks found, {:.2} H/s average, {} seconds uptime",
                final_stats.blocks_found, final_stats.total_hashrate, final_stats.uptime_seconds
            ),
        );
    }

    // REM-C002: Emit mining_stopped event
    app.emit("mining_stopped", serde_json::json!({
        "reason": "manual",
        "total_runtime_seconds": final_stats.uptime_seconds,
        "blocks_found": final_stats.blocks_found
    })).ok();

    Ok(true)
}

/// Get mining statistics
///
/// # Returns
/// * `Ok(MiningStats)` - Current hashrate, blocks found, uptime
/// * `Err(String)` - Failed to get stats
///
/// # Performance
/// - Target: <5ms (atomic reads, no locks)
///
/// # Frontend Usage
/// ```javascript
/// const stats = await invoke('get_mining_stats');
/// console.log('Hashrate:', stats.total_hashrate.toFixed(2), 'H/s');
/// console.log('CPU threads:', stats.cpu_threads);
/// console.log('Blocks found:', stats.blocks_found);
/// console.log('Uptime:', stats.uptime_seconds, 's');
/// ```
#[tauri::command]
pub async fn get_mining_stats(state: State<'_, crate::AppState>) -> Result<MiningStats, String> {
    let mining_pool_guard = state.mining_pool.read().await;
    if mining_pool_guard.is_none() {
        // Return default stats if mining hasn't started yet
        return Ok(MiningStats {
            is_mining: false,
            cpu_threads: 0,
            gpu_devices: 0,
            total_hashrate: 0.0,
            cpu_hashrate: 0.0,
            gpu_hashrate: 0.0,
            blocks_found: 0,
            uptime_seconds: 0,
        });
    }

    if let Some(ref pool) = *mining_pool_guard {
        Ok(pool.get_stats())
    } else {
        // Should never happen, but return default stats for safety
        Ok(MiningStats {
            is_mining: false,
            cpu_threads: 0,
            gpu_devices: 0,
            total_hashrate: 0.0,
            cpu_hashrate: 0.0,
            gpu_hashrate: 0.0,
            blocks_found: 0,
            uptime_seconds: 0,
        })
    }
}

/// Mining session information (historical record)
///
/// REM-C003 2025-11-19: Added for get_mining_history command
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MiningSession {
    pub session_id: String,
    pub device_id: u32,
    pub device_name: String,
    pub started_at: u64,
    pub stopped_at: Option<u64>,
    pub duration_seconds: u64,
    pub total_hashes: u64,
    pub average_hashrate: f64,
    pub peak_temperature: u32,
    pub throttle_events: u32,
    pub blocks_found: u32,
    pub rewards_btpc: f64,
}

/// Response structure for get_mining_history command
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MiningHistoryResponse {
    pub sessions: Vec<MiningSession>,
    pub total_blocks_found: u32,
    pub total_rewards_btpc: f64,
}

/// Get historical mining statistics
///
/// # Arguments
/// * `start_time` - Optional Unix timestamp for filtering (default: session start)
/// * `end_time` - Optional Unix timestamp for filtering (default: now)
/// * `device_id` - Optional device filter (default: all devices)
///
/// # Returns
/// * `Ok(MiningHistoryResponse)` - Mining sessions with aggregated statistics
/// * `Err(String)` - Query failed
///
/// # Note
/// Currently returns synthesized history from persistent GPU stats.
/// Full session tracking with RocksDB will be implemented in future version.
/// This provides graceful degradation - frontend can display lifetime statistics.
///
/// # Frontend Usage
/// ```javascript
/// const history = await invoke('get_mining_history', {
///   start_time: 1699900000,
///   end_time: 1700000000,
///   device_id: null  // All devices
/// });
/// console.log('Total blocks found:', history.total_blocks_found);
/// console.log('Total rewards:', history.total_rewards_btpc, 'BTPC');
/// console.log('Sessions:', history.sessions.length);
/// ```
///
/// # REM-C003
/// Added 2025-11-19 to complete mining-api.md contract
#[tauri::command]
pub async fn get_mining_history(
    state: State<'_, crate::AppState>,
    start_time: Option<u64>,
    end_time: Option<u64>,
    device_id: Option<u32>,
) -> Result<MiningHistoryResponse, String> {
    // Validate time range
    if let (Some(start), Some(end)) = (start_time, end_time) {
        if start > end {
            return Err("Invalid time range: start_time > end_time".to_string());
        }
    }

    // Load persistent GPU stats from disk
    // Get data directory from app config
    let data_dir = state.config.data_dir.clone();

    let persistence = crate::gpu_stats_persistence::GpuStatsPersistence::new(data_dir);
    let all_gpu_stats = persistence.get_all_stats();

    // Filter by device_id if specified
    let filtered_stats: Vec<_> = if let Some(device_id) = device_id {
        all_gpu_stats
            .into_iter()
            .filter(|(id, _)| *id == device_id)
            .collect()
    } else {
        all_gpu_stats.into_iter().collect()
    };

    // Convert persistent stats to mining sessions
    // Note: Since we don't have detailed session history yet, we create a single
    // "lifetime" session per GPU representing all mining activity
    let mut sessions = Vec::new();
    let mut total_blocks_found = 0u32;
    let mut total_rewards_btpc = 0.0f64;

    for (device_idx, gpu_stats) in filtered_stats {
        // Parse timestamps (first_seen and last_updated are ISO 8601 strings)
        let started_at = chrono::DateTime::parse_from_rfc3339(&gpu_stats.first_seen)
            .ok()
            .map(|dt| dt.timestamp() as u64)
            .unwrap_or(0);

        let stopped_at = chrono::DateTime::parse_from_rfc3339(&gpu_stats.last_updated)
            .ok()
            .map(|dt| dt.timestamp() as u64);

        // Calculate average hashrate
        let average_hashrate = if gpu_stats.total_uptime > 0 {
            gpu_stats.total_hashes as f64 / gpu_stats.total_uptime as f64
        } else {
            0.0
        };

        // Block reward: 32.375 BTPC per block (from mining-api.md)
        let blocks_found = gpu_stats.lifetime_blocks_found as u32;
        let rewards = blocks_found as f64 * 32.375;

        total_blocks_found += blocks_found;
        total_rewards_btpc += rewards;

        // Create synthetic session representing lifetime stats
        let session = MiningSession {
            session_id: format!("gpu-{}-lifetime", device_idx),
            device_id: device_idx,
            device_name: format!("GPU {}", device_idx), // TODO: Get actual device name from GPU enumeration
            started_at,
            stopped_at,
            duration_seconds: gpu_stats.total_uptime,
            total_hashes: gpu_stats.total_hashes,
            average_hashrate,
            peak_temperature: 0, // TODO: Track peak temperature in future
            throttle_events: 0,  // TODO: Track throttle events in future
            blocks_found,
            rewards_btpc: rewards,
        };

        // Apply time range filter if specified
        let include_session = match (start_time, end_time) {
            (Some(start), Some(end)) => {
                // Include if session overlaps with time range
                let session_start = session.started_at;
                let session_end = session.stopped_at.unwrap_or(chrono::Utc::now().timestamp() as u64);

                // Sessions overlap if: session_start <= end && session_end >= start
                session_start <= end && session_end >= start
            }
            (Some(start), None) => session.started_at >= start,
            (None, Some(end)) => {
                let session_end = session.stopped_at.unwrap_or(chrono::Utc::now().timestamp() as u64);
                session_end <= end
            }
            (None, None) => true, // No filter, include all
        };

        if include_session {
            sessions.push(session);
        }
    }

    Ok(MiningHistoryResponse {
        sessions,
        total_blocks_found,
        total_rewards_btpc,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_start_stop_mining_command() {
        // Arrange
        let pool = Arc::new(Mutex::new(MiningThreadPool::new()));
        let config = MiningConfig {
            enable_cpu: true,
            enable_gpu: false,
            cpu_threads: Some(1),
            mining_address: "bcrt1qtest".to_string(),
        };

        // Act: Start mining
        let start_result = {
            let mut pool_lock = pool.lock().expect("mutex poisoned");
            pool_lock
                .start_cpu_mining(Some(1), "bcrt1qtest".to_string())
                .await
        };

        // Assert: Mining started
        assert!(start_result.is_ok(), "Mining should start successfully");

        // Wait briefly
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Get stats
        let stats = {
            let pool_lock = pool.lock().expect("mutex poisoned");
            pool_lock.get_stats()
        };
        assert!(stats.is_mining);

        // Act: Stop mining
        let stop_result = {
            let mut pool_lock = pool.lock().expect("mutex poisoned");
            pool_lock.stop_all().await
        };

        // Assert: Mining stopped
        assert!(stop_result.is_ok(), "Mining should stop successfully");
    }

    #[tokio::test]
    async fn test_get_mining_stats_command() {
        // Arrange
        let pool = Arc::new(Mutex::new(MiningThreadPool::new()));

        // Act
        let stats = {
            let pool_lock = pool.lock().expect("mutex poisoned");
            pool_lock.get_stats()
        };

        // Assert
        assert!(!stats.is_mining);
        assert_eq!(stats.cpu_threads, 0);
        assert_eq!(stats.total_hashrate, 0.0);
    }
}
