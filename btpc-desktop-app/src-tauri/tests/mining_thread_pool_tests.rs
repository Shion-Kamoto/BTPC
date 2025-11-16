//! Contract tests for mining thread pool functionality
//!
//! These tests define the API contract for CPU/GPU mining via thread pools.
//! They are written BEFORE implementation (TDD RED phase) and MUST fail initially.
//!
//! Success criteria from tasks.md:
//! - T006: start_mining/stop_mining commands control thread pool lifecycle
//! - T007: get_mining_stats() returns atomic counter reads for hashrate

use btpc_desktop_app::commands::mining::{
    start_mining, stop_mining, get_mining_stats,
    MiningConfig, MiningStats,
};
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::time::sleep;

/// T006: Contract test for mining start/stop lifecycle
///
/// Verifies that start_mining() spawns background threads and stop_mining()
/// terminates them gracefully. CPU threads should be (num_cpus - 2) with
/// below-normal priority.
///
/// Expected to FAIL: mining module does not exist yet
#[test]
fn test_start_stop_mining() {
    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        // Arrange: Initialize embedded node first
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let data_path = temp_dir.path().to_str().unwrap().to_string();

        // Initialize node (will fail until T010 is complete, but contract defines expectation)
        btpc_desktop_app::commands::embedded_node::init_embedded_node(data_path, "regtest".to_string())
            .await
            .expect("Node initialization required for mining");

        // Act: Start mining with CPU-only configuration
        let mining_config = MiningConfig {
            enable_cpu: true,
            enable_gpu: false,
            cpu_threads: None, // Should default to (num_cpus - 2)
            mining_address: "bcrt1qtest".to_string(),
        };

        let start_result = start_mining(mining_config).await;

        // Assert: Mining started successfully
        assert!(start_result.is_ok(), "start_mining() should succeed");
        assert!(start_result.unwrap(), "Mining should return true when started");

        // Wait briefly to allow mining threads to spawn
        sleep(Duration::from_millis(100)).await;

        // Verify mining is active via stats
        let stats = get_mining_stats().await.expect("get_mining_stats() should succeed");
        assert!(stats.is_mining, "Mining should be active after start_mining()");
        assert!(stats.cpu_threads > 0, "CPU threads should be running");
        assert_eq!(stats.gpu_devices, 0, "GPU should be disabled");

        // Act: Stop mining
        let stop_result = stop_mining().await;

        // Assert: Mining stopped successfully
        assert!(stop_result.is_ok(), "stop_mining() should succeed");
        assert!(stop_result.unwrap(), "Mining should return true when stopped");

        // Verify mining is inactive
        let stats_after = get_mining_stats().await.expect("get_mining_stats() should succeed");
        assert!(!stats_after.is_mining, "Mining should be inactive after stop_mining()");
        assert_eq!(stats_after.cpu_threads, 0, "CPU threads should be 0 after stop");
    });
}

/// T007: Contract test for mining statistics query
///
/// Verifies that get_mining_stats() returns atomic counter reads for hashrate,
/// blocks found, and thread counts. Should complete in <5ms with no IPC overhead.
///
/// Expected to FAIL: get_mining_stats command does not exist yet
#[test]
fn test_get_mining_stats() {
    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        // Arrange: Initialize node and start mining
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let data_path = temp_dir.path().to_str().unwrap().to_string();

        btpc_desktop_app::commands::embedded_node::init_embedded_node(data_path, "regtest".to_string())
            .await
            .expect("Node initialization required");

        let mining_config = MiningConfig {
            enable_cpu: true,
            enable_gpu: false,
            cpu_threads: Some(2), // Explicit thread count for predictable testing
            mining_address: "bcrt1qtest".to_string(),
        };

        start_mining(mining_config).await.expect("Mining start failed");

        // Wait for mining to accumulate some hashes
        sleep(Duration::from_millis(500)).await;

        // Act: Query mining stats
        let result = get_mining_stats().await;

        // Assert: Verify stats structure
        assert!(result.is_ok(), "get_mining_stats() should succeed");

        let stats = result.unwrap();
        assert!(stats.is_mining, "Mining should be active");
        assert_eq!(stats.cpu_threads, 2, "Should have 2 CPU threads");
        assert_eq!(stats.gpu_devices, 0, "GPU should be disabled");
        assert!(stats.total_hashrate > 0.0, "Hashrate should be >0 after 500ms of mining");
        assert_eq!(stats.blocks_found, 0, "Unlikely to find blocks in 500ms on regtest");
        assert!(stats.uptime_seconds > 0, "Uptime should be >0");

        // Verify hashrate components
        assert!(stats.cpu_hashrate > 0.0, "CPU hashrate should be >0");
        assert_eq!(stats.gpu_hashrate, 0.0, "GPU hashrate should be 0 (disabled)");

        // Cleanup
        stop_mining().await.expect("Mining stop failed");
    });
}

/// Additional contract test: Verify mining stats when inactive
///
/// Ensures get_mining_stats() returns valid data even when mining is stopped.
#[test]
fn test_get_mining_stats_inactive() {
    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        // Arrange: Initialize node WITHOUT starting mining
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let data_path = temp_dir.path().to_str().unwrap().to_string();

        btpc_desktop_app::commands::embedded_node::init_embedded_node(data_path, "regtest".to_string())
            .await
            .expect("Node initialization required");

        // Act: Query stats with no active mining
        let result = get_mining_stats().await;

        // Assert: Verify stats show inactive state
        assert!(result.is_ok(), "get_mining_stats() should succeed even when inactive");

        let stats = result.unwrap();
        assert!(!stats.is_mining, "Mining should be inactive");
        assert_eq!(stats.cpu_threads, 0, "CPU threads should be 0");
        assert_eq!(stats.gpu_devices, 0, "GPU devices should be 0");
        assert_eq!(stats.total_hashrate, 0.0, "Hashrate should be 0 when inactive");
        assert_eq!(stats.blocks_found, 0, "Blocks found should be 0");
    });
}

/// Contract test: Verify CPU thread count defaults to (num_cpus - 2)
///
/// Ensures mining respects the constitution's requirement to leave headroom
/// for UI responsiveness.
#[test]
fn test_cpu_thread_count_default() {
    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        // Arrange: Initialize node
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let data_path = temp_dir.path().to_str().unwrap().to_string();

        btpc_desktop_app::commands::embedded_node::init_embedded_node(data_path, "regtest".to_string())
            .await
            .expect("Node initialization required");

        // Act: Start mining with None for cpu_threads (should default)
        let mining_config = MiningConfig {
            enable_cpu: true,
            enable_gpu: false,
            cpu_threads: None, // Test default behavior
            mining_address: "bcrt1qtest".to_string(),
        };

        start_mining(mining_config).await.expect("Mining start failed");

        // Query stats to verify thread count
        let stats = get_mining_stats().await.expect("get_mining_stats() should succeed");

        // Assert: Thread count should be (num_cpus - 2), minimum 1
        let expected_threads = (num_cpus::get() as i32 - 2).max(1) as u32;
        assert_eq!(
            stats.cpu_threads, expected_threads,
            "CPU threads should default to (num_cpus - 2) = {}",
            expected_threads
        );

        // Cleanup
        stop_mining().await.expect("Mining stop failed");
    });
}