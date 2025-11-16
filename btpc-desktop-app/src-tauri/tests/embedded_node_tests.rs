//! Contract tests for embedded blockchain node functionality
//!
//! These tests define the API contract for the embedded btpc-core node.
//! They are written BEFORE implementation (TDD RED phase) and MUST fail initially.
//!
//! Success criteria from tasks.md:
//! - T003: init_embedded_node() command returns NodeState with height 0
//! - T004: get_blockchain_state() query completes in <10ms
//! - T005: get_sync_progress() returns SyncProgress with is_syncing flag

use btpc_desktop_app::commands::embedded_node::{
    init_embedded_node, get_blockchain_state, get_sync_progress,
    NodeState, BlockchainState, SyncProgress,
};
use std::time::Instant;
use tokio::runtime::Runtime;

/// T003: Contract test for embedded node initialization
///
/// Verifies that init_embedded_node() can be invoked successfully and returns
/// a valid NodeState with height 0 for a fresh blockchain.
///
/// Expected to FAIL: embedded_node module does not exist yet
#[test]
fn test_init_embedded_node() {
    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        // Arrange: Create temporary data directory for test
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let data_path = temp_dir.path().to_str().unwrap().to_string();

        // Act: Initialize embedded node
        let result = init_embedded_node(data_path, "regtest".to_string()).await;

        // Assert: Verify successful initialization
        assert!(result.is_ok(), "init_embedded_node() should succeed");

        let node_state = result.unwrap();
        assert_eq!(node_state.network, "regtest", "Network should match regtest");
        assert_eq!(node_state.current_height, 0, "Fresh blockchain should have height 0");
        assert!(node_state.is_initialized, "Node should be marked as initialized");
        assert!(!node_state.is_syncing, "Fresh node should not be syncing initially");
    });
}

/// T004: Contract test for blockchain state query performance
///
/// Verifies that get_blockchain_state() completes in <10ms (vs ~50ms for RPC).
/// This validates the performance improvement from eliminating IPC overhead.
///
/// Expected to FAIL: get_blockchain_state command does not exist yet
#[test]
fn test_get_blockchain_state_performance() {
    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        // Arrange: Initialize embedded node
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let data_path = temp_dir.path().to_str().unwrap().to_string();

        init_embedded_node(data_path.clone(), "regtest".to_string())
            .await
            .expect("Node initialization failed");

        // Act: Query blockchain state and measure time
        let start = Instant::now();
        let result = get_blockchain_state().await;
        let elapsed = start.elapsed();

        // Assert: Verify performance target <10ms
        assert!(result.is_ok(), "get_blockchain_state() should succeed");
        assert!(
            elapsed.as_millis() < 10,
            "Query took {}ms, expected <10ms",
            elapsed.as_millis()
        );

        // Verify returned data structure
        let state = result.unwrap();
        assert_eq!(state.current_height, 0, "Height should be 0 for fresh blockchain");
        assert!(state.best_block_hash.len() > 0, "Best block hash should be populated");
        assert_eq!(state.total_utxos, 0, "Fresh blockchain should have 0 UTXOs");
    });
}

/// T005: Contract test for sync progress query
///
/// Verifies that get_sync_progress() returns SyncProgress with is_syncing flag.
/// For a fresh node with no peers, should report not syncing.
///
/// Expected to FAIL: get_sync_progress command does not exist yet
#[test]
fn test_get_sync_progress() {
    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        // Arrange: Initialize embedded node
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let data_path = temp_dir.path().to_str().unwrap().to_string();

        init_embedded_node(data_path, "regtest".to_string())
            .await
            .expect("Node initialization failed");

        // Act: Query sync progress
        let result = get_sync_progress().await;

        // Assert: Verify sync progress data
        assert!(result.is_ok(), "get_sync_progress() should succeed");

        let progress = result.unwrap();
        assert!(!progress.is_syncing, "Fresh node should not be syncing");
        assert_eq!(progress.current_height, 0, "Current height should be 0");
        assert_eq!(progress.target_height, 0, "Target height should be 0 with no peers");
        assert_eq!(progress.connected_peers, 0, "Should have 0 connected peers initially");
        assert!(progress.sync_percentage >= 0.0 && progress.sync_percentage <= 100.0,
                "Sync percentage should be in range [0, 100]");
    });
}

/// Additional contract test: Verify node shutdown
///
/// Ensures graceful shutdown with proper resource cleanup order:
/// Mining → P2P → Mempool → RocksDB WAL → Key zeroization
#[test]
fn test_embedded_node_shutdown() {
    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        // Arrange: Initialize embedded node
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let data_path = temp_dir.path().to_str().unwrap().to_string();

        init_embedded_node(data_path, "regtest".to_string())
            .await
            .expect("Node initialization failed");

        // Act: Shutdown node (will be implemented in shutdown_embedded_node command)
        // For now, just verify initialization worked
        let state = get_blockchain_state().await;

        // Assert: Node is operational before shutdown
        assert!(state.is_ok(), "Node should be operational before shutdown");
    });
}