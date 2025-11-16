//! GPU Statistics Types
//!
//! Shared types for GPU statistics used by both Tauri commands (main.rs)
//! and tests (lib.rs). This module is part of the library to enable testing.

use serde::{Deserialize, Serialize};

/// GPU statistics structure for frontend display (T011-003)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GpuStats {
    pub device_name: String,
    pub vendor: String,
    pub compute_units: u32,
    pub max_work_group_size: usize,
    pub global_mem_size: u64,
    pub local_mem_size: u64,
    pub max_clock_frequency: u32,
    pub hashrate: f64,
    pub total_hashes: u64,
    pub uptime_seconds: u64,
    pub temperature: Option<f32>,
    pub power_usage: Option<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_stats_serialization() {
        let stats = GpuStats {
            device_name: "Test GPU".to_string(),
            vendor: "Test Vendor".to_string(),
            compute_units: 16,
            max_work_group_size: 256,
            global_mem_size: 8_589_934_592,
            local_mem_size: 32768,
            max_clock_frequency: 1500,
            hashrate: 1000.0,
            total_hashes: 50000,
            uptime_seconds: 3600,
            temperature: Some(65.0),
            power_usage: Some(150.0),
        };

        // Serialize to JSON
        let json = serde_json::to_string(&stats).unwrap();

        // Deserialize back
        let deserialized: GpuStats = serde_json::from_str(&json).unwrap();

        assert_eq!(stats, deserialized);
    }

    #[test]
    fn test_gpu_stats_optional_fields() {
        let stats = GpuStats {
            device_name: "Test GPU".to_string(),
            vendor: "Test Vendor".to_string(),
            compute_units: 16,
            max_work_group_size: 256,
            global_mem_size: 8_589_934_592,
            local_mem_size: 32768,
            max_clock_frequency: 1500,
            hashrate: 1000.0,
            total_hashes: 50000,
            uptime_seconds: 3600,
            temperature: None,
            power_usage: None,
        };

        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("\"temperature\":null"));
        assert!(json.contains("\"power_usage\":null"));
    }
}