//! Performance benchmarks for block validation
//!
//! Tests block validation performance to ensure constitutional requirements
//! are met (<10ms block validation).

use btpc_core::{
    blockchain::{Block, BlockHeader, Transaction, TransactionInput, TransactionOutput, OutPoint},
    crypto::{Hash, Script, PrivateKey},
    consensus::{BlockValidator, StorageBlockValidator, pow::ProofOfWork, DifficultyTarget},
    storage::{BlockchainDb, UtxoDb, database::{Database, DatabaseConfig}},
};

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::{sync::Arc, time::{Duration, Instant}};
use tempfile::TempDir;

/// Create a test block with specified number of transactions
fn create_test_block_with_txs(num_transactions: usize) -> Block {
    let mut transactions = Vec::new();

    // Add coinbase transaction
    let coinbase = Transaction {
        version: 1,
        inputs: vec![TransactionInput {
            prev_out: OutPoint {
                txid: Hash::zero(),
                vout: 0xffffffff,
            },
            script_sig: Script::new(b"coinbase".to_vec()),
            sequence: 0xffffffff,
        }],
        outputs: vec![TransactionOutput {
            value: 5000000000, // 50 BTPC
            script_pubkey: Script::new(b"miner_address".to_vec()),
        }],
        lock_time: 0,
    };
    transactions.push(coinbase);

    // Add regular transactions
    for i in 0..num_transactions {
        let tx = Transaction {
            version: 1,
            inputs: vec![TransactionInput {
                prev_out: OutPoint {
                    txid: Hash::hash(&i.to_le_bytes()),
                    vout: 0,
                },
                script_sig: Script::new(vec![0x47; 100]), // Mock signature script
                sequence: 0xffffffff,
            }],
            outputs: vec![
                TransactionOutput {
                    value: 1000000000, // 10 BTPC
                    script_pubkey: Script::new(format!("recipient_{}", i).as_bytes().to_vec()),
                },
                TransactionOutput {
                    value: 500000000, // 5 BTPC change
                    script_pubkey: Script::new(format!("change_{}", i).as_bytes().to_vec()),
                },
            ],
            lock_time: 0,
        };
        transactions.push(tx);
    }

    let header = BlockHeader {
        version: 1,
        prev_hash: Hash::hash(b"previous_block"),
        merkle_root: Hash::hash(b"merkle_root"), // Simplified
        timestamp: 1640995200, // Jan 1, 2022
        bits: 0x207fffff, // Easy difficulty
        nonce: 12345,
    };

    Block { header, transactions }
}

/// Create storage-aware validator for testing
async fn create_storage_validator() -> (StorageBlockValidator, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_config = DatabaseConfig::test();
    let database = Arc::new(Database::open(temp_dir.path(), db_config).expect("Failed to open database"));

    let blockchain_db = Arc::new(BlockchainDb::new(database.clone()));
    let utxo_db = Arc::new(UtxoDb::new(database));

    let validator = StorageBlockValidator::new(
        blockchain_db as Arc<dyn btpc_core::storage::BlockchainDatabase + Send + Sync>,
        utxo_db as Arc<dyn btpc_core::storage::UTXODatabase + Send + Sync>,
    );

    (validator, temp_dir)
}

/// Benchmark basic block validation (stateless)
fn bench_basic_block_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("basic_block_validation");

    let transaction_counts = vec![1, 10, 50, 100, 500, 1000];

    for tx_count in transaction_counts {
        let block = create_test_block_with_txs(tx_count);
        let validator = BlockValidator::new();

        group.bench_with_input(
            BenchmarkId::new("stateless", tx_count),
            &tx_count,
            |b, _| {
                b.iter(|| {
                    black_box(validator.validate_block(black_box(&block)))
                        .expect("Block validation failed")
                });
            },
        );
    }

    group.finish();
}

/// Benchmark storage-aware block validation
fn bench_storage_block_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("storage_block_validation");

    let transaction_counts = vec![1, 10, 50, 100];

    let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");

    for tx_count in transaction_counts {
        let block = create_test_block_with_txs(tx_count);

        group.bench_with_input(
            BenchmarkId::new("with_storage", tx_count),
            &tx_count,
            |b, _| {
                b.to_async(&rt).iter(|| async {
                    let (validator, _temp_dir) = create_storage_validator().await;
                    black_box(validator.validate_block_with_context(black_box(&block)).await)
                        .expect("Storage block validation failed")
                });
            },
        );
    }

    group.finish();
}

/// Benchmark proof-of-work validation
fn bench_pow_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("pow_validation");

    let mut block = create_test_block_with_txs(10);

    // Set a valid nonce for the block (this would normally be found by mining)
    block.header.nonce = 123456;

    let target = DifficultyTarget::from_bits(block.header.bits);

    group.bench_function("sha512_pow", |b| {
        b.iter(|| {
            black_box(ProofOfWork::validate_block_pow(black_box(&block), black_box(&target)))
                .expect("PoW validation failed")
        });
    });

    group.finish();
}

/// Benchmark merkle root calculation
fn bench_merkle_root_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("merkle_root");

    let transaction_counts = vec![1, 10, 50, 100, 500, 1000, 5000];

    for tx_count in transaction_counts {
        let block = create_test_block_with_txs(tx_count);

        group.bench_with_input(
            BenchmarkId::new("calculate", tx_count),
            &tx_count,
            |b, _| {
                b.iter(|| {
                    black_box(btpc_core::blockchain::merkle::calculate_merkle_root(
                        black_box(&block.transactions)
                    )).expect("Merkle root calculation failed")
                });
            },
        );
    }

    group.finish();
}

/// Benchmark transaction validation within blocks
fn bench_transaction_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("transaction_validation");

    let transaction_counts = vec![1, 10, 50, 100, 500];

    for tx_count in transaction_counts {
        let block = create_test_block_with_txs(tx_count);

        group.bench_with_input(
            BenchmarkId::new("all_transactions", tx_count),
            &tx_count,
            |b, _| {
                b.iter(|| {
                    for tx in black_box(&block.transactions) {
                        black_box(tx.validate_structure())
                            .expect("Transaction validation failed");
                    }
                });
            },
        );
    }

    group.finish();
}

/// Constitutional compliance test for block validation
fn test_constitutional_block_validation_performance() {
    println!("Testing constitutional block validation performance requirements...");

    // Test with typical block size (100 transactions)
    let block = create_test_block_with_txs(100);
    let validator = BlockValidator::new();

    let start = Instant::now();
    validator.validate_block(&block).expect("Block validation failed");
    let validation_time = start.elapsed();

    println!("Block validation time (100 tx): {:?}", validation_time);
    assert!(
        validation_time < Duration::from_millis(10),
        "Block validation took {:?}, exceeds 10ms constitutional requirement",
        validation_time
    );

    // Test with large block (1000 transactions)
    let large_block = create_test_block_with_txs(1000);

    let start = Instant::now();
    validator.validate_block(&large_block).expect("Large block validation failed");
    let large_validation_time = start.elapsed();

    println!("Large block validation time (1000 tx): {:?}", large_validation_time);
    assert!(
        large_validation_time < Duration::from_millis(50),
        "Large block validation took {:?}, exceeds reasonable limits",
        large_validation_time
    );

    println!("✅ Constitutional block validation performance requirements met!");
}

/// Benchmark block header validation
fn bench_header_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("header_validation");

    let header = BlockHeader {
        version: 1,
        prev_hash: Hash::hash(b"previous_block"),
        merkle_root: Hash::hash(b"merkle_root"),
        timestamp: 1640995200,
        bits: 0x207fffff,
        nonce: 12345,
    };

    let validator = BlockValidator::new();

    group.bench_function("header_only", |b| {
        b.iter(|| {
            black_box(validator.validate_header(black_box(&header)))
                .expect("Header validation failed")
        });
    });

    group.finish();
}

/// Benchmark block size validation
fn bench_block_size_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("block_size_validation");

    let transaction_counts = vec![1, 100, 500, 1000, 2000];

    for tx_count in transaction_counts {
        let block = create_test_block_with_txs(tx_count);
        let validator = BlockValidator::new();

        group.bench_with_input(
            BenchmarkId::new("size_check", tx_count),
            &tx_count,
            |b, _| {
                b.iter(|| {
                    black_box(validator.validate_block_size(black_box(&block)))
                        .expect("Block size validation failed")
                });
            },
        );
    }

    group.finish();
}

/// Stress test with rapid block validation
fn bench_stress_test(c: &mut Criterion) {
    let mut group = c.benchmark_group("stress_test");
    group.measurement_time(Duration::from_secs(10));

    let blocks: Vec<Block> = (0..100)
        .map(|i| create_test_block_with_txs(10 + i % 50))
        .collect();

    let validator = BlockValidator::new();

    group.bench_function("rapid_validation", |b| {
        b.iter(|| {
            for block in &blocks {
                black_box(validator.validate_block(black_box(block)))
                    .expect("Block validation failed");
            }
        });
    });

    group.finish();
}

/// Benchmark validation throughput
fn bench_validation_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput");
    group.measurement_time(Duration::from_secs(5));

    let block = create_test_block_with_txs(100);
    let validator = BlockValidator::new();

    group.bench_function("blocks_per_second", |b| {
        b.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                black_box(validator.validate_block(black_box(&block)))
                    .expect("Block validation failed");
            }
            start.elapsed()
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_basic_block_validation,
    bench_storage_block_validation,
    bench_pow_validation,
    bench_merkle_root_calculation,
    bench_transaction_validation,
    bench_header_validation,
    bench_block_size_validation,
    bench_stress_test,
    bench_validation_throughput
);
criterion_main!(benches);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constitutional_requirements() {
        test_constitutional_block_validation_performance();
    }

    #[test]
    fn test_block_creation_consistency() {
        let block1 = create_test_block_with_txs(10);
        let block2 = create_test_block_with_txs(10);

        // Blocks should have consistent structure
        assert_eq!(block1.transactions.len(), block2.transactions.len());
        assert_eq!(block1.transactions.len(), 11); // 10 + 1 coinbase

        // Each block should validate
        let validator = BlockValidator::new();
        assert!(validator.validate_block(&block1).is_ok());
        assert!(validator.validate_block(&block2).is_ok());
    }

    #[test]
    fn test_large_block_handling() {
        // Test with very large block (stress test)
        let large_block = create_test_block_with_txs(5000);
        let validator = BlockValidator::new();

        let start = Instant::now();
        let result = validator.validate_block(&large_block);
        let validation_time = start.elapsed();

        assert!(result.is_ok(), "Large block validation failed");
        println!("Large block (5000 tx) validation time: {:?}", validation_time);

        // Should complete within reasonable time (even if not constitutional requirement)
        assert!(validation_time < Duration::from_millis(500), "Large block validation too slow");
    }

    #[test]
    fn test_empty_block_validation() {
        // Test block with only coinbase transaction
        let empty_block = create_test_block_with_txs(0);
        let validator = BlockValidator::new();

        assert_eq!(empty_block.transactions.len(), 1); // Just coinbase
        assert!(validator.validate_block(&empty_block).is_ok());
    }

    #[test]
    fn test_validation_consistency() {
        let block = create_test_block_with_txs(50);
        let validator = BlockValidator::new();

        // Multiple validations should give consistent results
        for _ in 0..10 {
            assert!(validator.validate_block(&block).is_ok());
        }
    }

    #[tokio::test]
    async fn test_storage_validation_basic() {
        let block = create_test_block_with_txs(5);
        let (validator, _temp_dir) = create_storage_validator().await;

        // Genesis block should validate
        let mut genesis_block = block;
        genesis_block.header.prev_hash = Hash::zero();

        let result = validator.validate_block_with_context(&genesis_block).await;
        assert!(result.is_ok(), "Genesis block should validate: {:?}", result);
    }

    #[test]
    fn test_performance_regression() {
        // Ensure validation performance doesn't regress
        let block = create_test_block_with_txs(100);
        let validator = BlockValidator::new();

        let start = Instant::now();
        for _ in 0..100 {
            validator.validate_block(&block).expect("Validation failed");
        }
        let total_time = start.elapsed();

        let avg_time = total_time / 100;
        println!("Average validation time: {:?}", avg_time);

        // Should average well under constitutional requirement
        assert!(avg_time < Duration::from_millis(5), "Performance regression detected");
    }
}