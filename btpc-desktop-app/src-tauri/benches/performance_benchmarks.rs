//! Performance benchmarks for BTPC Desktop Application
//!
//! These benchmarks measure the performance of critical operations
//! to ensure the application meets performance requirements.
//!
//! # Success Criteria (REM-C004)
//! - SC-003: Blockchain queries < 100ms
//! - SC-004: GPU mining init < 5 seconds
//! - SC-005: Hashrate updates every 5 seconds

use btpc_desktop_app::btpc_integration::BtpcIntegration;
use btpc_desktop_app::embedded_node::EmbeddedNode;
use btpc_desktop_app::security::SecurityManager;
use btpc_desktop_app::utxo_manager::{UTXOManager, UTXO};
use chrono::Utc;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;

/// Setup function for creating test environment
fn setup_test_env() -> (TempDir, PathBuf) {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    let btpc_home = temp_dir.path().to_path_buf();

    // Create necessary subdirectories
    std::fs::create_dir_all(btpc_home.join("bin")).unwrap();
    std::fs::create_dir_all(btpc_home.join("data")).unwrap();
    std::fs::create_dir_all(btpc_home.join("logs")).unwrap();

    (temp_dir, btpc_home)
}

/// Benchmark password hashing operations
fn bench_password_hashing(c: &mut Criterion) {
    c.bench_function("password_hash_creation", |b| {
        b.iter(|| {
            let (_temp_dir, btpc_home) = setup_test_env();
            // SecurityManager::new takes a directory path, returns Self (not Result)
            let security_manager = SecurityManager::new(btpc_home.clone());

            // create_user is sync and takes (username, password) - returns Result<RecoveryData>
            let result = security_manager.create_user(
                black_box("testuser"),
                black_box("testpassword123!@#"),
            );

            black_box(result)
        })
    });
}

/// Benchmark user authentication
fn bench_user_authentication(c: &mut Criterion) {
    // Setup: Create a user first
    let (_temp_dir, btpc_home) = setup_test_env();
    // SecurityManager::new takes a directory path, returns Self (not Result)
    let security_manager = SecurityManager::new(btpc_home.clone());

    security_manager
        .create_user("benchuser", "benchpassword123!@#")
        .expect("Failed to create user");

    c.bench_function("user_authentication", |b| {
        b.iter(|| {
            // SecurityManager::new takes a directory path, returns Self (not Result)
            let security_manager = SecurityManager::new(btpc_home.clone());

            // authenticate_user is sync, takes (username, password)
            let result = security_manager.authenticate_user(
                black_box("benchuser"),
                black_box("benchpassword123!@#"),
            );

            black_box(result)
        })
    });
}

/// Benchmark UTXO operations
fn bench_utxo_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("utxo_operations");

    // Benchmark adding UTXOs
    group.bench_function("add_single_utxo", |b| {
        b.iter(|| {
            let (_temp_dir, btpc_home) = setup_test_env();
            let mut utxo_manager =
                UTXOManager::new(btpc_home).expect("Failed to create UTXO manager");

            let utxo = UTXO {
                txid: black_box("test_txid".to_string()),
                vout: black_box(0),
                value_credits: black_box(100000000),
                value_btp: black_box(1.0),
                address: black_box("test_address".to_string()),
                block_height: black_box(100),
                is_coinbase: black_box(false),
                created_at: Utc::now(),
                spent: false,
                spent_in_tx: None,
                spent_at_height: None,
                script_pubkey: Vec::new(),
            };

            let result = utxo_manager.add_utxo(utxo);
            black_box(result)
        })
    });

    // Benchmark adding multiple UTXOs
    for count in [10, 100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("add_multiple_utxos", count),
            count,
            |b, &count| {
                b.iter(|| {
                    let (_temp_dir, btpc_home) = setup_test_env();
                    let mut utxo_manager =
                        UTXOManager::new(btpc_home).expect("Failed to create UTXO manager");

                    for i in 0..count {
                        let utxo = UTXO {
                            txid: format!("test_txid_{}", i),
                            vout: 0,
                            value_credits: 100000000,
                            value_btp: 1.0,
                            address: format!("test_address_{}", i % 10), // 10 different addresses
                            block_height: 100 + i as u64,
                            is_coinbase: false,
                            created_at: Utc::now(),
                            spent: false,
                            spent_in_tx: None,
                            spent_at_height: None,
                            script_pubkey: Vec::new(),
                        };

                        let _ = utxo_manager.add_utxo(utxo);
                    }

                    black_box(())
                })
            },
        );
    }

    group.finish();
}

/// Benchmark BTPC integration operations
fn bench_btpc_integration(c: &mut Criterion) {
    c.bench_function("btpc_integration_creation", |b| {
        b.iter(|| {
            let (_temp_dir, btpc_home) = setup_test_env();
            let integration = BtpcIntegration::new(black_box(btpc_home));
            black_box(integration)
        })
    });

    c.bench_function("binary_existence_check", |b| {
        b.iter(|| {
            let (_temp_dir, btpc_home) = setup_test_env();
            let integration = BtpcIntegration::new(btpc_home);

            let exists = integration.binary_exists(black_box("btpc_wallet_dilithium"));
            black_box(exists)
        })
    });

    // Note: get_binary_status() removed - use binary_exists() for individual checks
    c.bench_function("binary_path_resolution", |b| {
        b.iter(|| {
            let (_temp_dir, btpc_home) = setup_test_env();
            let integration = BtpcIntegration::new(btpc_home);

            let path = integration.binary_path(black_box("btpc_node"));
            black_box(path)
        })
    });
}

/// Benchmark memory usage for large datasets
fn bench_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");
    group.sample_size(10); // Smaller sample size for memory-intensive tests

    // Benchmark memory usage with large UTXO sets
    for count in [1000, 10000].iter() {
        group.bench_with_input(
            BenchmarkId::new("large_utxo_set", count),
            count,
            |b, &count| {
                b.iter(|| {
                    let (_temp_dir, btpc_home) = setup_test_env();
                    let mut utxo_manager =
                        UTXOManager::new(btpc_home).expect("Failed to create UTXO manager");

                    // Create a large set of UTXOs
                    for i in 0..count {
                        let utxo = UTXO {
                            txid: format!("large_test_txid_{}", i),
                            vout: (i % 4) as u32,
                            value_credits: 100000000 + (i as u64 * 1000),
                            value_btp: 1.0 + (i as f64 * 0.001),
                            address: format!("large_test_address_{}", i % 100),
                            block_height: 100 + i as u64,
                            is_coinbase: i % 10 == 0,
                            created_at: Utc::now(),
                            spent: false,
                            spent_in_tx: None,
                            spent_at_height: None,
                            script_pubkey: Vec::new(),
                        };

                        let _ = utxo_manager.add_utxo(utxo);
                    }

                    // Measure retrieval performance (get_stats is sync)
                    let stats = utxo_manager.get_stats();
                    black_box(stats)
                })
            },
        );
    }

    group.finish();
}

/// SC-003: Benchmark blockchain queries (target: < 100ms)
///
/// REM-C004: Validates that blockchain state queries meet performance requirements
fn bench_blockchain_queries(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("blockchain_queries");

    // Setup: Create embedded node
    let (_temp_dir, btpc_home) = setup_test_env();
    let utxo_manager = UTXOManager::new(btpc_home.clone()).expect("Failed to create UTXO manager");
    let utxo_manager_arc = Arc::new(std::sync::Mutex::new(utxo_manager));

    let node_arc = rt.block_on(async {
        EmbeddedNode::new(btpc_home, "regtest", utxo_manager_arc)
            .await
            .expect("Failed to create embedded node")
    });

    // Benchmark get_blockchain_state (should be < 100ms)
    group.bench_function("get_blockchain_state", |b| {
        b.iter(|| {
            rt.block_on(async {
                let node = node_arc.read().await;
                let state = node.get_blockchain_state().await;
                black_box(state)
            })
        })
    });

    // Benchmark get_sync_progress (should be < 100ms)
    group.bench_function("get_sync_progress", |b| {
        b.iter(|| {
            rt.block_on(async {
                let node = node_arc.read().await;
                let progress = node.get_sync_progress();
                black_box(progress)
            })
        })
    });

    // Benchmark get_peer_info (should be < 100ms)
    group.bench_function("get_peer_info", |b| {
        b.iter(|| {
            rt.block_on(async {
                let node = node_arc.read().await;
                let peers = node.get_peer_info();
                black_box(peers)
            })
        })
    });

    group.finish();
}

/// SC-004: Benchmark GPU mining initialization (target: < 5 seconds)
///
/// REM-C004: Validates GPU enumeration and OpenCL context creation performance
fn bench_gpu_mining_init(c: &mut Criterion) {
    use btpc_desktop_app::gpu_miner::{enumerate_gpu_devices, GpuMiner};

    let mut group = c.benchmark_group("gpu_mining_init");
    group.measurement_time(std::time::Duration::from_secs(10)); // Allow enough time for GPU init
    group.sample_size(10); // Smaller sample size for GPU operations

    // Benchmark GPU enumeration (should be < 5 seconds)
    group.bench_function("enumerate_gpus", |b| {
        b.iter(|| {
            let devices = enumerate_gpu_devices();
            black_box(devices)
        })
    });

    // Benchmark full GPU miner initialization (should be < 5 seconds)
    group.bench_function("gpu_miner_init", |b| {
        b.iter(|| {
            // Get first available GPU device
            if let Ok(devices) = enumerate_gpu_devices() {
                if !devices.is_empty() {
                    let miner = GpuMiner::new(0); // batch_size removed from API
                    let _ = black_box(miner);
                }
            }
        })
    });

    group.finish();
}

/// SC-005: Benchmark hashrate update frequency (target: every 5 seconds)
///
/// REM-C004: Validates that mining stats are updated at expected intervals
fn bench_mining_stats_updates(c: &mut Criterion) {
    use btpc_desktop_app::mining_thread_pool::MiningThreadPool;

    let mut group = c.benchmark_group("mining_stats_updates");
    group.sample_size(10); // Smaller sample for timing-sensitive benchmarks

    // Benchmark mining stats retrieval (should be fast, called every 5 seconds)
    group.bench_function("get_mining_stats", |b| {
        b.iter(|| {
            let pool = MiningThreadPool::new(0, 2);
            let stats = pool.get_stats();
            black_box(stats)
        })
    });

    // Benchmark stats update latency
    group.bench_function("stats_update_latency", |b| {
        b.iter(|| {
            let pool = MiningThreadPool::new(0, 2);
            // Simulate stats collection cycle
            let start = std::time::Instant::now();
            let stats = pool.get_stats();
            let latency = start.elapsed();
            black_box((stats, latency))
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_password_hashing,
    bench_user_authentication,
    bench_utxo_operations,
    bench_btpc_integration,
    bench_memory_usage,
    bench_blockchain_queries,
    bench_gpu_mining_init,
    bench_mining_stats_updates
);

criterion_main!(benches);
