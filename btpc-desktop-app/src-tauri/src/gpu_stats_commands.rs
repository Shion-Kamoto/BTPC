// T011-002/T011-003: Tauri GPU Stats Commands
// Feature: 011-frontend-backend-integration
// Contract: Fetch GPU stats from MiningThreadPool (in-process mining)
//
// ARCHITECTURE CHANGE (Feature 010):
// Previously fetched from external btpc_miner HTTP endpoint (Feature 009)
// Now uses in-process MiningThreadPool for embedded mining

use serde::{Deserialize, Serialize};
use tauri::{command, State};
use btpc_desktop_app::gpu_stats_types::GpuStats;

/// Tauri command: Get GPU mining statistics from MiningThreadPool
///
/// Article XI Compliance: Backend fetches data, frontend displays
/// No localStorage/sessionStorage usage - pure backend-first approach
///
/// # T011-003
/// Updated for Feature 011: Frontend-Backend Integration
/// Now uses in-process MiningThreadPool instead of external HTTP endpoint
#[command]
pub async fn get_gpu_stats(state: State<'_, crate::AppState>) -> Result<GpuStats, String> {
    let mining_pool_guard = state.mining_pool.read().await;
    let pool = mining_pool_guard.as_ref()
        .ok_or_else(|| "Mining pool not initialized - start mining first".to_string())?;

    // Check if GPU is available
    if !pool.is_gpu_available() {
        return Err("No GPU devices available".to_string());
    }

    // Get mining stats to extract GPU data
    let stats = pool.get_stats();

    // Get actual GPU device info from MiningThreadPool (Bug Fix: GPU not detected in UI)
    let device_info = pool.get_gpu_device_info(0)
        .ok_or_else(|| "GPU device 0 not found - mining may not have initialized properly".to_string())?;

    // Get total hashes from GPU stats
    let gpu_stats = pool.get_gpu_stats(0);
    let total_hashes = gpu_stats.map(|s| s.total_hashes).unwrap_or(0);

    Ok(GpuStats {
        device_name: device_info.model_name,     // ✅ Real device name from OpenCL
        vendor: device_info.vendor,              // ✅ Real vendor from OpenCL
        compute_units: device_info.compute_units, // ✅ Real compute units from OpenCL
        max_work_group_size: 0,                  // TODO: Add to GpuDevice struct if needed
        global_mem_size: device_info.global_mem_size, // ✅ Real memory size from OpenCL
        local_mem_size: 0,                       // TODO: Add to GpuDevice struct if needed
        max_clock_frequency: device_info.max_clock_frequency, // ✅ Real clock frequency from OpenCL
        hashrate: stats.gpu_hashrate,            // ✅ Available from MiningThreadPool
        total_hashes,                            // ✅ Available from per-GPU stats
        uptime_seconds: stats.uptime_seconds,    // ✅ Available from MiningThreadPool
        temperature: None,                       // TODO: Get GPU temperature via NVML
        power_usage: None,                       // TODO: Get GPU power usage via NVML
    })
}

/// Tauri command: Check if GPU stats are available
///
/// # T011-002
/// Updated for Feature 011: Frontend-Backend Integration
/// Now checks MiningThreadPool instead of external HTTP endpoint
#[command]
pub async fn is_gpu_stats_available(state: State<'_, crate::AppState>) -> Result<bool, String> {
    let mining_pool_guard = state.mining_pool.read().await;
    let pool = match mining_pool_guard.as_ref() {
        Some(p) => p,
        None => return Ok(false), // No mining pool = no GPU devices
    };

    Ok(pool.is_gpu_available())
}

// ============================================================================
// Feature 012: GPU Mining Dashboard with Individual GPU Statistics
// ============================================================================

use crate::gpu_health_monitor::{GpuDevice, GpuHealthMetrics};
use std::collections::HashMap;

/// GPU mining statistics for Feature 012 dashboard (per-GPU granularity)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuMiningStats {
    pub gpu_device_index: u32,
    pub current_hashrate: f64,
    pub lifetime_blocks_found: u64,
    pub mining_uptime: u64,  // seconds
    pub mining_status: String,  // "Active", "Idle", "Error", "Throttled"
    pub energy_efficiency: Option<f64>,   // H/W
    pub thermal_efficiency: Option<f64>,  // H/°C
    pub throttle_percentage: u8,  // 0-100%
}

/// Combined GPU dashboard data for event emission (Feature 012)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuDashboardData {
    pub devices: Vec<GpuDevice>,
    pub stats: HashMap<u32, GpuMiningStats>,
    pub health: HashMap<u32, GpuHealthMetrics>,
    pub temperature_threshold: f32,
}

/// Enumerate all available GPUs on the system (Feature 012)
///
/// Uses OpenCL device enumeration with sysinfo fallback for cross-platform compatibility.
/// Returns GPU device information for dashboard display.
///
/// # Article XI Compliance
/// - Backend is source of truth for GPU enumeration
/// - No frontend polling - called on-demand or via event triggers
///
/// # Returns
/// - `Ok(Vec<GpuDevice>)` - List of detected GPUs (may be empty if no GPUs)
/// - `Err(String)` - Error message if enumeration failed
///
/// # Implementation (T014)
/// Delegates to gpu_health_monitor::enumerate_gpus() service function
#[command]
pub async fn enumerate_gpus() -> Result<Vec<GpuDevice>, String> {
    // Delegate to gpu_health_monitor service (T013)
    crate::gpu_health_monitor::enumerate_gpus()
}

/// Get current mining statistics for all GPUs or a specific GPU (Feature 012)
///
/// Queries MiningThreadPool for per-GPU mining statistics and combines with health metrics.
///
/// # Article XI Compliance
/// - Stats queried from backend MiningThreadPool (single source of truth)
/// - No frontend polling - called on-demand or via event triggers
///
/// # Arguments
/// * `state` - Tauri AppState containing MiningThreadPool
/// * `gpu_device_index` - Optional GPU device index (None = all GPUs)
///
/// # Returns
/// * `Ok(HashMap<u32, GpuMiningStats>)` - GPU mining stats by device index
/// * `Err(String)` - Error message if mining pool not initialized
///
/// # Implementation (T017)
/// Converts PerGpuStats from MiningThreadPool to GpuMiningStats for frontend
#[command]
pub async fn get_gpu_mining_stats(
    state: State<'_, crate::AppState>,
    gpu_device_index: Option<u32>,
) -> Result<HashMap<u32, GpuMiningStats>, String> {
    // Get MiningThreadPool from AppState
    let mining_pool_guard = state.mining_pool.read().await;
    let pool = mining_pool_guard.as_ref()
        .ok_or_else(|| "Mining pool not initialized".to_string())?;

    // Get per-GPU stats from MiningThreadPool
    let per_gpu_stats = if let Some(device_index) = gpu_device_index {
        // Get stats for specific GPU
        match pool.get_gpu_stats(device_index) {
            Some(stats) => {
                let mut map = HashMap::new();
                map.insert(device_index, stats);
                map
            }
            None => {
                return Err(format!("GPU device {} not found or not mining", device_index));
            }
        }
    } else {
        // Get stats for all GPUs
        pool.get_all_gpu_stats()
    };

    // Get GPU health metrics for efficiency calculations
    let health_metrics = match get_gpu_health_metrics(None).await {
        Ok(metrics) => metrics,
        Err(_) => HashMap::new(), // Sensors may not be available
    };

    // Get temperature threshold for throttle status
    let temp_threshold = {
        let threshold = state.gpu_temperature_threshold.read().await;
        *threshold
    };

    // Convert PerGpuStats to GpuMiningStats
    let mut result = HashMap::new();
    for (device_index, stats) in per_gpu_stats {
        // Get health data for this GPU
        let health = health_metrics.get(&device_index);

        // Determine mining status
        let mining_status = if stats.current_hashrate > 0.0 {
            "Active".to_string()
        } else {
            "Idle".to_string()
        };

        // Calculate thermal throttle percentage (100% = no throttling)
        let throttle_percentage = if let Some(h) = health {
            if let Some(temp) = h.temperature {
                if temp >= temp_threshold {
                    // Reduce throttle by 10% per degree over threshold
                    let degrees_over = temp - temp_threshold;
                    let reduction = (degrees_over * 10.0).min(100.0) as u8;
                    100u8.saturating_sub(reduction)
                } else {
                    100u8 // No throttling
                }
            } else {
                100u8 // Unknown temperature, assume no throttling
            }
        } else {
            100u8 // No health data, assume no throttling
        };

        // Calculate energy efficiency (hashes per watt)
        let energy_efficiency = health
            .and_then(|h| h.power_consumption)
            .filter(|&power| power > 0.0)
            .map(|power| stats.current_hashrate / power as f64);

        // Calculate thermal efficiency (hashes per degree Celsius)
        let thermal_efficiency = health
            .and_then(|h| h.temperature)
            .filter(|&temp| temp > 0.0)
            .map(|temp| stats.current_hashrate / temp as f64);

        result.insert(device_index, GpuMiningStats {
            gpu_device_index: device_index,
            current_hashrate: stats.current_hashrate,
            lifetime_blocks_found: stats.blocks_found,
            mining_uptime: stats.mining_uptime,
            mining_status,
            energy_efficiency,
            thermal_efficiency,
            throttle_percentage,
        });
    }

    Ok(result)
}

/// Get current health metrics for all GPUs or a specific GPU (Feature 012)
///
/// Polls GPU sensors for temperature, fan speed, power, memory, and clock data.
/// Uses NVML for NVIDIA GPUs, fallback to sysinfo for AMD/Intel.
///
/// # Article XI Compliance
/// - Health metrics polled from GPU sensors (backend source of truth)
/// - No frontend polling - called on-demand or via event triggers
///
/// # Arguments
/// * `gpu_device_index` - Optional GPU device index (None = all GPUs)
///
/// # Returns
/// * `Ok(HashMap<u32, GpuHealthMetrics>)` - GPU health metrics by device index
/// * `Err(String)` - Error message if GPU polling failed
///
/// # Implementation (T018)
/// Delegates to gpu_health_monitor::poll_gpu_health() for sensor data
#[command]
pub async fn get_gpu_health_metrics(
    gpu_device_index: Option<u32>,
) -> Result<HashMap<u32, GpuHealthMetrics>, String> {
    use crate::gpu_health_monitor;

    if let Some(device_index) = gpu_device_index {
        // Poll specific GPU
        match gpu_health_monitor::poll_gpu_health(device_index) {
            Ok(metrics) => {
                let mut map = HashMap::new();
                map.insert(device_index, metrics);
                Ok(map)
            }
            Err(e) => Err(format!("Failed to poll GPU {}: {}", device_index, e)),
        }
    } else {
        // Poll all GPUs
        let metrics = gpu_health_monitor::poll_all_gpu_health();

        if metrics.is_empty() {
            return Err("No GPU health data available - GPUs may not be detected or sensors unavailable".to_string());
        }

        // Convert Vec to HashMap
        let mut map = HashMap::new();
        for metric in metrics {
            map.insert(metric.gpu_device_index, metric);
        }
        Ok(map)
    }
}

/// Set GPU temperature warning threshold (Feature 012)
///
/// Validates and stores GPU temperature threshold for thermal throttling.
///
/// # Article XI Compliance (Section 11.2)
/// - Backend validates FIRST before saving
/// - Range validation: 60-95°C
/// - Single source of truth in AppState
///
/// # Arguments
/// * `state` - Tauri AppState
/// * `threshold` - New temperature threshold in °C
///
/// # Returns
/// * `Ok(f32)` - Updated threshold value (echoed back for confirmation)
/// * `Err(String)` - Validation error if threshold out of range
///
/// # Implementation (T019)
/// Validates threshold range and stores in AppState
#[command]
pub async fn set_temperature_threshold(
    state: State<'_, crate::AppState>,
    threshold: f32,
) -> Result<f32, String> {
    // Validate threshold range: 60-95°C
    const MIN_THRESHOLD: f32 = 60.0;
    const MAX_THRESHOLD: f32 = 95.0;

    if threshold < MIN_THRESHOLD || threshold > MAX_THRESHOLD {
        return Err(format!(
            "Temperature threshold must be between {:.0}°C and {:.0}°C (received: {:.1}°C)",
            MIN_THRESHOLD, MAX_THRESHOLD, threshold
        ));
    }

    // Round to 1 decimal place for consistency
    let rounded_threshold = (threshold * 10.0).round() / 10.0;

    // Store in AppState
    let mut threshold_guard = state.gpu_temperature_threshold.write().await;
    *threshold_guard = rounded_threshold;

    Ok(rounded_threshold)
}

/// Get current GPU temperature warning threshold (Feature 012)
///
/// # Article XI Compliance
/// - Backend is single source of truth
///
/// # Arguments
/// * `state` - Tauri AppState
///
/// # Returns
/// * `Ok(f32)` - Current temperature threshold in °C
///
/// # Implementation (T019)
/// Reads threshold from AppState
#[command]
pub async fn get_temperature_threshold(
    state: State<'_, crate::AppState>,
) -> Result<f32, String> {
    let threshold = state.gpu_temperature_threshold.read().await;
    Ok(*threshold)
}

/// Get complete dashboard data (Feature 012)
///
/// Combines all GPU data sources into a single response:
/// - GPU device enumeration
/// - Per-GPU mining statistics
/// - GPU health metrics (temperature, fan, power, etc.)
/// - Current temperature threshold
///
/// # Article XI Compliance
/// - Single backend query aggregates all data sources
/// - No multiple frontend queries required
/// - Single source of truth for dashboard display
///
/// # Arguments
/// * `state` - Tauri AppState
///
/// # Returns
/// * `Ok(GpuDashboardData)` - Complete GPU dashboard data
/// * `Err(String)` - Error if data aggregation failed
///
/// # Implementation (T021)
/// Aggregates data from gpu_health_monitor, MiningThreadPool, and AppState
#[command]
pub async fn get_gpu_dashboard_data(
    state: State<'_, crate::AppState>,
) -> Result<GpuDashboardData, String> {
    use crate::gpu_health_monitor;

    // 1. Enumerate GPU devices
    let devices = gpu_health_monitor::enumerate_gpus()
        .map_err(|e| format!("Failed to enumerate GPUs: {}", e))?;

    // 2. Get per-GPU mining statistics
    let mining_stats = match get_gpu_mining_stats(state.clone(), None).await {
        Ok(stats) => stats,
        Err(_) => HashMap::new(), // Mining may not be active - return empty
    };

    // 3. Get GPU health metrics
    let health_metrics = match get_gpu_health_metrics(None).await {
        Ok(metrics) => metrics,
        Err(_) => HashMap::new(), // Sensors may not be available - return empty
    };

    // 4. Get current temperature threshold
    let temperature_threshold = {
        let threshold = state.gpu_temperature_threshold.read().await;
        *threshold
    };

    Ok(GpuDashboardData {
        devices,
        stats: mining_stats,
        health: health_metrics,
        temperature_threshold,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test GpuStats struct serialization/deserialization (Feature 011)
    #[test]
    fn test_gpu_stats_serde() {
        let stats = GpuStats {
            device_name: "Test GPU".to_string(),
            vendor: "Test Vendor".to_string(),
            compute_units: 64,
            max_work_group_size: 256,
            global_mem_size: 17179869184,
            local_mem_size: 65536,
            max_clock_frequency: 2105,
            hashrate: 342.5,
            total_hashes: 20550000000,
            uptime_seconds: 60,
            temperature: None,
            power_usage: None,
        };

        // Serialize to JSON
        let json = serde_json::to_string(&stats).expect("Failed to serialize");

        // Deserialize back
        let deserialized: GpuStats =
            serde_json::from_str(&json).expect("Failed to deserialize");

        // Verify fields match
        assert_eq!(deserialized.device_name, "Test GPU");
        assert_eq!(deserialized.compute_units, 64);
        assert_eq!(deserialized.hashrate, 342.5);
    }

    /// Test GpuMiningStats serialization (Feature 012)
    #[test]
    fn test_gpu_mining_stats_serialization() {
        let stats = GpuMiningStats {
            gpu_device_index: 0,
            current_hashrate: 25000.0,
            lifetime_blocks_found: 42,
            mining_uptime: 3600,
            mining_status: "Active".to_string(),
            energy_efficiency: Some(250.0),
            thermal_efficiency: Some(15.0),
            throttle_percentage: 100,
        };

        let json = serde_json::to_string(&stats).unwrap();
        let deserialized: GpuMiningStats = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.gpu_device_index, 0);
        assert_eq!(deserialized.lifetime_blocks_found, 42);
    }
}