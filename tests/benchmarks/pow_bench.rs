//! Performance benchmarks for proof-of-work operations
//!
//! Tests SHA-512 proof-of-work mining and validation performance.

use btpc_core::{
    blockchain::{Block, BlockHeader},
    crypto::Hash,
    consensus::{pow::ProofOfWork, difficulty::DifficultyTarget, DifficultyAdjustment},
};

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::time::{Duration, Instant};

/// Create a test block header for mining
fn create_test_header() -> BlockHeader {
    BlockHeader {
        version: 1,
        prev_hash: Hash::hash(b"previous_block"),
        merkle_root: Hash::hash(b"merkle_root"),
        timestamp: 1640995200, // Jan 1, 2022
        bits: 0x207fffff, // Easy difficulty for testing
        nonce: 0, // Will be set during mining
    }
}

/// Create a test block for PoW validation
fn create_test_block() -> Block {
    let header = create_test_header();
    let transactions = vec![btpc_core::blockchain::Transaction::create_test_coinbase(5000000000)];

    Block { header, transactions }
}

/// Benchmark SHA-512 hash calculation
fn bench_sha512_hashing(c: &mut Criterion) {
    let mut group = c.benchmark_group("sha512_hashing");

    let data_sizes = vec![32, 64, 128, 256, 512, 1024];

    for size in data_sizes {
        let data = vec![0u8; size];

        group.bench_with_input(
            BenchmarkId::new("hash_data", size),
            &size,
            |b, _| {
                b.iter(|| {
                    black_box(Hash::hash(black_box(&data)))
                });
            },
        );
    }

    group.finish();
}

/// Benchmark block header hashing for PoW
fn bench_header_hashing(c: &mut Criterion) {
    let mut group = c.benchmark_group("header_hashing");

    let header = create_test_header();

    group.bench_function("block_header", |b| {
        b.iter(|| {
            black_box(header.hash())
        });
    });

    group.finish();
}

/// Benchmark PoW target checking
fn bench_pow_target_check(c: &mut Criterion) {
    let mut group = c.benchmark_group("pow_target_check");

    let difficulties = vec![
        0x207fffff, // Very easy
        0x1d00ffff, // Easy
        0x1b0404cb, // Medium
        0x1a44b9f2, // Hard
    ];

    for bits in difficulties {
        let target = DifficultyTarget::from_bits(bits);
        let header = BlockHeader {
            version: 1,
            prev_hash: Hash::zero(),
            merkle_root: Hash::zero(),
            timestamp: 1640995200,
            bits,
            nonce: 12345,
        };
        let hash = header.hash();

        group.bench_with_input(
            BenchmarkId::new("check_target", format!("{:08x}", bits)),
            &bits,
            |b, _| {
                b.iter(|| {
                    black_box(target.check_hash(black_box(&hash)))
                });
            },
        );
    }

    group.finish();
}

/// Benchmark PoW validation
fn bench_pow_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("pow_validation");

    let mut block = create_test_block();

    // Set a valid nonce (in practice this would be found by mining)
    block.header.nonce = 123456;

    let target = DifficultyTarget::from_bits(block.header.bits);

    group.bench_function("validate_block_pow", |b| {
        b.iter(|| {
            black_box(ProofOfWork::validate_block_pow(
                black_box(&block),
                black_box(&target)
            )).expect("PoW validation failed")
        });
    });

    group.finish();
}

/// Benchmark difficulty target conversion
fn bench_difficulty_conversion(c: &mut Criterion) {
    let mut group = c.benchmark_group("difficulty_conversion");

    let bits_values = vec![0x207fffff, 0x1d00ffff, 0x1b0404cb, 0x1a44b9f2];

    for bits in bits_values {
        group.bench_with_input(
            BenchmarkId::new("from_bits", format!("{:08x}", bits)),
            &bits,
            |b, _| {
                b.iter(|| {
                    black_box(DifficultyTarget::from_bits(black_box(bits)))
                });
            },
        );
    }

    // Benchmark target to bits conversion
    let targets: Vec<DifficultyTarget> = bits_values.iter()
        .map(|&bits| DifficultyTarget::from_bits(bits))
        .collect();

    for (i, target) in targets.iter().enumerate() {
        group.bench_with_input(
            BenchmarkId::new("to_bits", i),
            &i,
            |b, _| {
                b.iter(|| {
                    black_box(target.to_bits())
                });
            },
        );
    }

    group.finish();
}

/// Benchmark mining simulation (finding nonce)
fn bench_mining_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("mining_simulation");
    group.measurement_time(Duration::from_secs(10));

    // Use very easy difficulty for benchmarking
    let mut header = create_test_header();
    header.bits = 0x207fffff; // Very easy

    let target = DifficultyTarget::from_bits(header.bits);

    group.bench_function("find_nonce_easy", |b| {
        b.iter(|| {
            let mut test_header = header.clone();
            let mut nonce = 0u32;

            // Simulate mining (limit iterations for benchmarking)
            for _ in 0..1000 {
                test_header.nonce = nonce;
                let hash = test_header.hash();
                if target.check_hash(&hash) {
                    break;
                }
                nonce += 1;
            }

            black_box(nonce)
        });
    });

    group.finish();
}

/// Benchmark difficulty adjustment calculation
fn bench_difficulty_adjustment(c: &mut Criterion) {
    let mut group = c.benchmark_group("difficulty_adjustment");

    let adjustment = DifficultyAdjustment::new(
        600, // 10 minute target
        2016, // Adjust every 2016 blocks
    );

    let scenarios = vec![
        (1200, "slow_blocks"), // Blocks took twice as long
        (300, "fast_blocks"),  // Blocks took half as long
        (600, "normal_blocks"), // Blocks on time
    ];

    for (actual_time, scenario) in scenarios {
        group.bench_with_input(
            BenchmarkId::new("calculate", scenario),
            &actual_time,
            |b, &time| {
                b.iter(|| {
                    black_box(adjustment.calculate_next_difficulty(
                        black_box(0x1d00ffff), // Current difficulty
                        black_box(time)         // Actual time taken
                    ))
                });
            },
        );
    }

    group.finish();
}

/// Stress test mining operations
fn bench_stress_test(c: &mut Criterion) {
    let mut group = c.benchmark_group("stress_test");
    group.measurement_time(Duration::from_secs(5));

    let header = create_test_header();
    let target = DifficultyTarget::from_bits(header.bits);

    group.bench_function("rapid_hashing", |b| {
        b.iter(|| {
            let mut test_header = header.clone();
            for nonce in 0..1000 {
                test_header.nonce = nonce;
                black_box(test_header.hash());
            }
        });
    });

    group.bench_function("rapid_target_checking", |b| {
        let hashes: Vec<Hash> = (0..1000)
            .map(|i| Hash::hash(&i.to_le_bytes()))
            .collect();

        b.iter(|| {
            for hash in &hashes {
                black_box(target.check_hash(black_box(hash)));
            }
        });
    });

    group.finish();
}

/// Benchmark PoW throughput
fn bench_pow_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput");
    group.measurement_time(Duration::from_secs(5));

    let header = create_test_header();

    // Hashes per second
    group.bench_function("hashes_per_second", |b| {
        b.iter_custom(|iters| {
            let mut test_header = header.clone();
            let start = Instant::now();
            for i in 0..iters {
                test_header.nonce = i as u32;
                black_box(test_header.hash());
            }
            start.elapsed()
        });
    });

    // Target checks per second
    let target = DifficultyTarget::from_bits(header.bits);
    let test_hash = header.hash();

    group.bench_function("target_checks_per_second", |b| {
        b.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                black_box(target.check_hash(black_box(&test_hash)));
            }
            start.elapsed()
        });
    });

    group.finish();
}

/// Performance requirement validation
fn test_pow_performance_requirements() {
    println!("Testing PoW performance requirements...");

    // Test header hashing performance
    let header = create_test_header();

    let start = Instant::now();
    for _ in 0..10000 {
        header.hash();
    }
    let hash_time = start.elapsed();
    let avg_hash_time = hash_time / 10000;

    println!("Average hash time: {:?}", avg_hash_time);
    assert!(
        avg_hash_time < Duration::from_micros(100),
        "Hash calculation too slow: {:?}",
        avg_hash_time
    );

    // Test target checking performance
    let target = DifficultyTarget::from_bits(0x1d00ffff);
    let test_hash = Hash::hash(b"test");

    let start = Instant::now();
    for _ in 0..10000 {
        target.check_hash(&test_hash);
    }
    let check_time = start.elapsed();
    let avg_check_time = check_time / 10000;

    println!("Average target check time: {:?}", avg_check_time);
    assert!(
        avg_check_time < Duration::from_micros(10),
        "Target checking too slow: {:?}",
        avg_check_time
    );

    // Test PoW validation performance
    let mut block = create_test_block();
    block.header.nonce = 123456;
    let target = DifficultyTarget::from_bits(block.header.bits);

    let start = Instant::now();
    for _ in 0..1000 {
        ProofOfWork::validate_block_pow(&block, &target).expect("PoW validation failed");
    }
    let validation_time = start.elapsed();
    let avg_validation_time = validation_time / 1000;

    println!("Average PoW validation time: {:?}", avg_validation_time);
    assert!(
        avg_validation_time < Duration::from_micros(500),
        "PoW validation too slow: {:?}",
        avg_validation_time
    );

    println!("✅ All PoW performance requirements met!");
}

criterion_group!(
    benches,
    bench_sha512_hashing,
    bench_header_hashing,
    bench_pow_target_check,
    bench_pow_validation,
    bench_difficulty_conversion,
    bench_mining_simulation,
    bench_difficulty_adjustment,
    bench_stress_test,
    bench_pow_throughput
);
criterion_main!(benches);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_requirements() {
        test_pow_performance_requirements();
    }

    #[test]
    fn test_difficulty_target_consistency() {
        let bits = 0x1d00ffff;
        let target = DifficultyTarget::from_bits(bits);
        let converted_bits = target.to_bits();

        // Should be able to round-trip
        assert_eq!(bits, converted_bits, "Difficulty target conversion inconsistent");
    }

    #[test]
    fn test_pow_validation_consistency() {
        let mut block = create_test_block();
        block.header.nonce = 123456;
        let target = DifficultyTarget::from_bits(block.header.bits);

        // Multiple validations should give same result
        for _ in 0..10 {
            let result = ProofOfWork::validate_block_pow(&block, &target);
            assert!(result.is_ok(), "PoW validation should be consistent");
        }
    }

    #[test]
    fn test_hash_determinism() {
        let header = create_test_header();

        // Same header should always produce same hash
        let hash1 = header.hash();
        let hash2 = header.hash();

        assert_eq!(hash1, hash2, "Hash calculation should be deterministic");
    }

    #[test]
    fn test_nonce_increment_changes_hash() {
        let mut header = create_test_header();

        header.nonce = 0;
        let hash1 = header.hash();

        header.nonce = 1;
        let hash2 = header.hash();

        assert_ne!(hash1, hash2, "Different nonces should produce different hashes");
    }

    #[test]
    fn test_target_boundary_conditions() {
        // Test with maximum difficulty (lowest target)
        let max_difficulty_target = DifficultyTarget::from_bits(0x1b000000);
        let zero_hash = Hash::zero();

        // Zero hash should meet any target
        assert!(max_difficulty_target.check_hash(&zero_hash));

        // Test with minimum difficulty (highest target)
        let min_difficulty_target = DifficultyTarget::from_bits(0x207fffff);
        let max_hash = Hash::from_bytes([0xff; 32]);

        // Max hash should not meet minimum difficulty
        assert!(!min_difficulty_target.check_hash(&max_hash));
    }

    #[test]
    fn test_difficulty_adjustment_bounds() {
        let adjustment = DifficultyAdjustment::new(600, 2016);

        // Test extreme cases
        let very_slow = adjustment.calculate_next_difficulty(0x1d00ffff, 2400); // 4x slower
        let very_fast = adjustment.calculate_next_difficulty(0x1d00ffff, 150);  // 4x faster

        // Difficulty should be bounded (max 4x change)
        assert!(very_slow > 0x1c000000, "Difficulty decrease should be bounded");
        assert!(very_fast < 0x1e000000, "Difficulty increase should be bounded");
    }

    #[test]
    fn test_mining_simulation_finds_solution() {
        let mut header = create_test_header();
        header.bits = 0x207fffff; // Very easy difficulty

        let target = DifficultyTarget::from_bits(header.bits);
        let mut found_solution = false;

        // Should find a solution within reasonable iterations
        for nonce in 0..100000 {
            header.nonce = nonce;
            let hash = header.hash();
            if target.check_hash(&hash) {
                found_solution = true;
                break;
            }
        }

        assert!(found_solution, "Should find PoW solution with easy difficulty");
    }

    #[test]
    fn test_performance_regression() {
        // Ensure PoW operations don't regress in performance
        let header = create_test_header();

        let start = Instant::now();
        for _ in 0..1000 {
            header.hash();
        }
        let total_time = start.elapsed();

        let avg_time = total_time / 1000;
        println!("Average hash time: {:?}", avg_time);

        // Should be very fast
        assert!(avg_time < Duration::from_micros(200), "Hash performance regression detected");
    }
}