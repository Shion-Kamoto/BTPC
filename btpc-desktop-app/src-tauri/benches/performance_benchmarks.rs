//! Performance benchmarks for BTPC Desktop Application
//!
//! These benchmarks measure the performance of critical operations
//! to ensure the application meets performance requirements.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use btpc_desktop_app::security::SecurityManager;
use btpc_desktop_app::utxo_manager::{UTXOManager, UTXO};
use btpc_desktop_app::btpc_integration::BtpcIntegration;
use tempfile::TempDir;
use chrono::Utc;
use std::path::PathBuf;

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
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("password_hash_creation", |b| {
        b.iter(|| {
            rt.block_on(async {
                let (_temp_dir, btpc_home) = setup_test_env();
                let temp_file = btpc_home.join("test_users.json");
                let mut security_manager = SecurityManager::new(temp_file)
                    .expect("Failed to create security manager");

                let result = security_manager.create_user(
                    black_box("testuser".to_string()),
                    black_box("testpassword123!@#".to_string()),
                    black_box("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about".to_string())
                ).await;

                black_box(result)
            })
        })
    });
}

/// Benchmark user authentication
fn bench_user_authentication(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Setup: Create a user first
    let (_temp_dir, btpc_home) = setup_test_env();
    let temp_file = btpc_home.join("test_users.json");
    let mut security_manager = SecurityManager::new(temp_file.clone())
        .expect("Failed to create security manager");

    rt.block_on(async {
        security_manager.create_user(
            "benchuser".to_string(),
            "benchpassword123!@#".to_string(),
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about".to_string()
        ).await.expect("Failed to create user");
    });

    c.bench_function("user_authentication", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut security_manager = SecurityManager::new(temp_file.clone())
                    .expect("Failed to create security manager");

                let result = security_manager.login(
                    black_box("benchuser".to_string()),
                    black_box("benchpassword123!@#".to_string())
                ).await;

                black_box(result)
            })
        })
    });
}

/// Benchmark UTXO operations
fn bench_utxo_operations(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("utxo_operations");

    // Benchmark adding UTXOs
    group.bench_function("add_single_utxo", |b| {
        b.iter(|| {
            rt.block_on(async {
                let (_temp_dir, btpc_home) = setup_test_env();
                let mut utxo_manager = UTXOManager::new(btpc_home)
                    .expect("Failed to create UTXO manager");

                let utxo = UTXO {
                    txid: black_box("test_txid".to_string()),
                    vout: black_box(0),
                    value_credits: black_box(100000000),
                    value_btp: black_box(1.0),
                    address: black_box("test_address".to_string()),
                    block_height: black_box(100),
                    is_coinbase: black_box(false),
                    created_at: Utc::now(),
                    confirmations: black_box(6),
                    spendable: black_box(true),
                };

                let result = utxo_manager.add_utxo(utxo).await;
                black_box(result)
            })
        })
    });

    // Benchmark adding multiple UTXOs
    for count in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::new("add_multiple_utxos", count), count, |b, &count| {
            b.iter(|| {
                rt.block_on(async {
                    let (_temp_dir, btpc_home) = setup_test_env();
                    let mut utxo_manager = UTXOManager::new(btpc_home)
                        .expect("Failed to create UTXO manager");

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
                            confirmations: 6,
                            spendable: true,
                        };

                        let _ = utxo_manager.add_utxo(utxo).await;
                    }

                    black_box(())
                })
            })
        });
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

    c.bench_function("binary_status_check", |b| {
        b.iter(|| {
            let (_temp_dir, btpc_home) = setup_test_env();
            let integration = BtpcIntegration::new(btpc_home);

            let status = integration.get_binary_status();
            black_box(status)
        })
    });
}

/// Benchmark memory usage for large datasets
fn bench_memory_usage(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("memory_usage");
    group.sample_size(10); // Smaller sample size for memory-intensive tests

    // Benchmark memory usage with large UTXO sets
    for count in [1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::new("large_utxo_set", count), count, |b, &count| {
            b.iter(|| {
                rt.block_on(async {
                    let (_temp_dir, btpc_home) = setup_test_env();
                    let mut utxo_manager = UTXOManager::new(btpc_home)
                        .expect("Failed to create UTXO manager");

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
                            confirmations: 6,
                            spendable: true,
                        };

                        let _ = utxo_manager.add_utxo(utxo).await;
                    }

                    // Measure retrieval performance
                    let stats = utxo_manager.get_statistics().await.unwrap();
                    black_box(stats)
                })
            })
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_password_hashing,
    bench_user_authentication,
    bench_utxo_operations,
    bench_btpc_integration,
    bench_memory_usage
);

criterion_main!(benches);