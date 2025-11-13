//! Tauri commands for mining operations
//!
//! Exposes MiningThreadPool functionality to frontend via Tauri IPC.

use crate::mining_thread_pool::{MiningStats, MiningThreadPool};
use anyhow::Result;
use std::sync::{Arc, Mutex};
use tauri::State;

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
) -> Result<bool, String> {
    // Clone config fields for use after lock drop
    let enable_cpu = config.enable_cpu;
    let enable_gpu = config.enable_gpu;
    let cpu_threads = config.cpu_threads;
    let mining_address = config.mining_address.clone();

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

    // Start CPU mining if enabled
    if enable_cpu {
        let cpu_address = mining_address.clone(); // Clone for CPU closure

        // Access mining pool and start CPU mining
        {
            let mut mining_pool_guard = state.mining_pool.write().await;
            if let Some(ref mut pool) = *mining_pool_guard {
                pool.start_cpu_mining(cpu_threads, cpu_address)
                    .await
                    .map_err(|e| format!("Failed to start CPU mining: {}", e))?;
            } else {
                return Err("Mining pool not initialized".to_string());
            }
        }

        // Log mining start to activity buffer
        {
            let mut mining_logs = state.mining_logs.lock()
                .map_err(|_| "Failed to lock mining_logs".to_string())?;
            let threads_str = cpu_threads.map(|t| format!("{} threads", t)).unwrap_or_else(|| "auto threads".to_string());
            mining_logs.add_entry("INFO".to_string(), format!("CPU mining started with {}", threads_str));
        }

        // Start periodic hashrate logging task
        let mining_pool_arc = state.mining_pool.clone();
        let logs_for_logging = state.mining_logs.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
            let mut last_blocks_found = 0u64;
            loop {
                interval.tick().await;

                // Check if mining is still active
                let stats = {
                    let pool_guard = mining_pool_arc.read().await;
                    if let Some(ref pool) = *pool_guard {
                        pool.get_stats()
                    } else {
                        break; // Pool was destroyed, stop logging
                    }
                };

                if !stats.is_mining {
                    break; // Stop logging when mining stops
                }

                // Check if new blocks were found since last check
                if stats.blocks_found > last_blocks_found {
                    let new_blocks = stats.blocks_found - last_blocks_found;
                    if let Ok(mut logs) = logs_for_logging.lock() {
                        for _ in 0..new_blocks {
                            logs.add_entry(
                                "SUCCESS".to_string(),
                                format!(
                                    "Block found! Total blocks mined: {}",
                                    stats.blocks_found
                                )
                            );
                        }
                    }
                    last_blocks_found = stats.blocks_found;
                }

                // Log hashrate update
                if let Ok(mut logs) = logs_for_logging.lock() {
                    let hashrate_display = if stats.total_hashrate >= 1_000_000.0 {
                        format!("{:.2} MH/s", stats.total_hashrate / 1_000_000.0)
                    } else if stats.total_hashrate >= 1_000.0 {
                        format!("{:.2} KH/s", stats.total_hashrate / 1_000.0)
                    } else {
                        format!("{:.2} H/s", stats.total_hashrate)
                    };

                    logs.add_entry(
                        "INFO".to_string(),
                        format!(
                            "Mining active - {} ({} threads) - {} blocks found",
                            hashrate_display,
                            stats.cpu_threads,
                            stats.blocks_found
                        )
                    );
                }
            }
        });
    }

    // Start GPU mining if enabled (non-blocking, fails gracefully)
    if enable_gpu {
        let mining_pool_arc = state.mining_pool.clone();
        let gpu_address = mining_address.clone(); // Clone for GPU closure
        let logs_clone = state.mining_logs.clone();

        // Spawn GPU initialization asynchronously (don't block UI thread)
        tokio::spawn(async move {
            // Access mining pool and start GPU mining
            let gpu_result = {
                let mut pool_guard = mining_pool_arc.write().await;
                if let Some(ref mut pool) = *pool_guard {
                    pool.start_gpu_mining(gpu_address).await
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
                        logs.add_entry("WARN".to_string(), format!("GPU mining unavailable: {}", e));
                    }
                    eprintln!("GPU mining not available: {}", e);
                }
            }
        });

        // Return immediately without waiting for GPU init to complete
        {
            let mut mining_logs = state.mining_logs.lock()
                .map_err(|_| "Failed to lock mining_logs".to_string())?;
            mining_logs.add_entry("INFO".to_string(), "GPU initialization started in background...".to_string());
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
pub async fn stop_mining(state: State<'_, crate::AppState>) -> Result<bool, String> {
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
        let mut mining_logs = state.mining_logs.lock()
            .map_err(|_| "Failed to lock mining_logs".to_string())?;
        mining_logs.add_entry(
            "INFO".to_string(),
            format!(
                "Mining stopped - {} blocks found, {:.2} H/s average, {} seconds uptime",
                final_stats.blocks_found,
                final_stats.total_hashrate,
                final_stats.uptime_seconds
            )
        );
    }

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
            pool_lock.start_cpu_mining(Some(1), "bcrt1qtest".to_string()).await
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