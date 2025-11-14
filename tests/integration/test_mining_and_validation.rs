// Mining and Validation Integration Tests
// These tests MUST FAIL initially to ensure TDD compliance

use btpc_core::blockchain::{Block, BlockHeader, Transaction};
use btpc_core::consensus::{Consensus, DifficultyTarget, ProofOfWork};
use btpc_core::crypto::{Hash, PrivateKey};
use btpc_core::mining::{Miner, MiningPool, MiningConfig};
use btpc_core::storage::UTXOSet;

#[cfg(test)]
mod mining_validation_tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::{Duration, Instant};
    use tempfile::tempdir;

    #[test]
    fn test_mining_pool_coordination() {
        // Integration: Multiple miners coordinating through mining pool
        // Scenario: 3 miners → pool → shared work distribution → rewards

        let temp_dir = tempdir().unwrap();
        let pool_config = MiningConfig {
            network: btpc_core::Network::Regtest,
            reward_address: PrivateKey::generate_ml_dsa().unwrap().public_key().to_address(),
            max_workers: 3,
            difficulty_target: DifficultyTarget::from_bits(0x207fffff),
        };

        let pool = MiningPool::new(pool_config, temp_dir.path()).unwrap();
        let pool_arc = Arc::new(Mutex::new(pool));

        // Create 3 miners
        let miners: Vec<_> = (0..3).map(|i| {
            let miner_key = PrivateKey::generate_ml_dsa().unwrap();
            Miner::new(
                format!("miner_{}", i),
                miner_key,
                pool_arc.clone(),
            ).unwrap()
        }).collect();

        // Start mining for 5 seconds
        let mining_handles: Vec<_> = miners.into_iter().map(|mut miner| {
            thread::spawn(move || {
                let start = Instant::now();
                let mut blocks_found = 0;

                while start.elapsed() < Duration::from_secs(5) {
                    if let Ok(block) = miner.mine_single_attempt() {
                        blocks_found += 1;
                        // Submit block to pool
                        miner.submit_block(block).unwrap();
                    }
                }

                blocks_found
            })
        }).collect();

        // Wait for mining to complete
        let block_counts: Vec<_> = mining_handles.into_iter()
            .map(|handle| handle.join().unwrap())
            .collect();

        let total_blocks: u32 = block_counts.iter().sum();
        assert!(total_blocks > 0, "Mining pool must find at least one block");

        // Verify pool state
        let pool_guard = pool_arc.lock().unwrap();
        let pool_stats = pool_guard.get_statistics();

        assert_eq!(pool_stats.total_blocks_found, total_blocks, "Pool must track all found blocks");
        assert_eq!(pool_stats.active_miners, 3, "Pool must track all miners");
        assert!(pool_stats.total_hashrate > 0.0, "Pool must calculate total hashrate");

        // Verify reward distribution
        for (i, miner_blocks) in block_counts.iter().enumerate() {
            let miner_rewards = pool_guard.get_miner_rewards(format!("miner_{}", i)).unwrap();
            let expected_rewards = *miner_blocks as u64 * btpc_core::consensus::RewardCalculator::calculate_block_reward(1);
            assert_eq!(miner_rewards.total_earned, expected_rewards, "Miner rewards must match blocks found");
        }
    }

    #[test]
    fn test_difficulty_adjustment_under_varying_hashrate() {
        // Integration: Difficulty adjustment responding to hashrate changes
        // Scenario: Simulate hashrate spikes and drops over time

        let temp_dir = tempdir().unwrap();
        let mut consensus = Consensus::new(temp_dir.path(), btpc_core::Network::Regtest).unwrap();

        let mut block_times = Vec::new();
        let target_block_time = Duration::from_secs(600); // 10 minutes

        // Phase 1: Normal hashrate (baseline)
        let normal_difficulty = DifficultyTarget::from_bits(0x207fffff);
        consensus.set_difficulty_target(normal_difficulty);

        for height in 1..=2016 {
            let start_time = Instant::now();

            // Simulate normal mining time
            let block = consensus.mine_block_at_height(height).unwrap();
            let mining_time = start_time.elapsed();

            block_times.push(mining_time);

            // Add some variance (±20%)
            let variance = (height % 5) as f64 * 0.04 - 0.08; // -8% to +8%
            let adjusted_time = mining_time.as_secs_f64() * (1.0 + variance);
            block_times.last_mut().unwrap().clone_from(&Duration::from_secs_f64(adjusted_time));
        }

        // Calculate average time for first period
        let avg_time_1: Duration = block_times.iter().sum::<Duration>() / block_times.len() as u32;

        // Trigger difficulty adjustment
        let total_time_1: Duration = block_times.iter().sum();
        let new_difficulty_1 = consensus.adjust_difficulty(total_time_1).unwrap();

        if avg_time_1 < target_block_time {
            assert!(new_difficulty_1.is_harder_than(&normal_difficulty), "Difficulty must increase when blocks are too fast");
        } else {
            assert!(new_difficulty_1.is_easier_than(&normal_difficulty), "Difficulty must decrease when blocks are too slow");
        }

        // Phase 2: High hashrate (2x speed)
        block_times.clear();
        consensus.set_difficulty_target(new_difficulty_1);

        for height in 2017..=4032 {
            let start_time = Instant::now();
            let block = consensus.mine_block_at_height(height).unwrap();
            let mining_time = start_time.elapsed();

            // Simulate 2x hashrate (half the time)
            let adjusted_time = Duration::from_secs_f64(mining_time.as_secs_f64() * 0.5);
            block_times.push(adjusted_time);
        }

        // Should increase difficulty significantly
        let total_time_2: Duration = block_times.iter().sum();
        let new_difficulty_2 = consensus.adjust_difficulty(total_time_2).unwrap();
        assert!(new_difficulty_2.is_harder_than(&new_difficulty_1), "Difficulty must increase with higher hashrate");

        // Phase 3: Low hashrate (0.5x speed)
        block_times.clear();
        consensus.set_difficulty_target(new_difficulty_2);

        for height in 4033..=6048 {
            let start_time = Instant::now();
            let block = consensus.mine_block_at_height(height).unwrap();
            let mining_time = start_time.elapsed();

            // Simulate half hashrate (double the time)
            let adjusted_time = Duration::from_secs_f64(mining_time.as_secs_f64() * 2.0);
            block_times.push(adjusted_time);
        }

        // Should decrease difficulty
        let total_time_3: Duration = block_times.iter().sum();
        let new_difficulty_3 = consensus.adjust_difficulty(total_time_3).unwrap();
        assert!(new_difficulty_3.is_easier_than(&new_difficulty_2), "Difficulty must decrease with lower hashrate");

        // Verify bounds are respected (max 4x change)
        let max_increase = new_difficulty_2.multiply_difficulty(4.0);
        let max_decrease = new_difficulty_2.divide_difficulty(4.0);

        assert!(new_difficulty_3 >= max_decrease, "Difficulty decrease must be bounded");
        assert!(new_difficulty_2 <= max_increase, "Difficulty increase must be bounded");
    }

    #[test]
    fn test_block_validation_edge_cases() {
        // Integration: Block validation handling edge cases and attacks
        // Security: Prevent various block-based attacks

        let temp_dir = tempdir().unwrap();
        let mut consensus = Consensus::new(temp_dir.path(), btpc_core::Network::Regtest).unwrap();
        let utxo_set = UTXOSet::new();

        // Test 1: Block with invalid proof-of-work
        let mut invalid_pow_block = Block::create_test_block();
        invalid_pow_block.header.nonce = 0; // Wrong nonce
        let validation_result = consensus.validate_block(&invalid_pow_block, &utxo_set);
        assert!(validation_result.is_err(), "Block with invalid PoW must be rejected");

        // Test 2: Block with timestamp too far in future
        let mut future_block = Block::create_test_block();
        future_block.header.timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() + 7300; // 2+ hours in future
        assert!(consensus.validate_block(&future_block, &utxo_set).is_err(), "Future block must be rejected");

        // Test 3: Block with invalid merkle root
        let mut invalid_merkle_block = Block::create_test_block();
        invalid_merkle_block.header.merkle_root = Hash::random(); // Wrong merkle root
        assert!(consensus.validate_block(&invalid_merkle_block, &utxo_set).is_err(), "Invalid merkle root must be rejected");

        // Test 4: Block exceeding size limit
        let oversized_block = Block::create_oversized_test_block(); // >1MB
        assert!(consensus.validate_block(&oversized_block, &utxo_set).is_err(), "Oversized block must be rejected");

        // Test 5: Block with no coinbase transaction
        let mut no_coinbase_block = Block::create_test_block();
        no_coinbase_block.transactions = vec![
            Transaction::create_test_transfer(1000000, Hash::random())
        ]; // No coinbase
        assert!(consensus.validate_block(&no_coinbase_block, &utxo_set).is_err(), "Block without coinbase must be rejected");

        // Test 6: Block with multiple coinbase transactions
        let mut multi_coinbase_block = Block::create_test_block();
        multi_coinbase_block.transactions.push(
            Transaction::coinbase(1000000, Hash::random())
        ); // Second coinbase
        assert!(consensus.validate_block(&multi_coinbase_block, &utxo_set).is_err(), "Multiple coinbase transactions must be rejected");

        // Test 7: Block with excessive coinbase reward
        let mut excessive_reward_block = Block::create_test_block();
        excessive_reward_block.transactions[0] = Transaction::coinbase(
            5000000000, // 50 BTPC - too much for height 1
            Hash::random()
        );
        assert!(consensus.validate_block(&excessive_reward_block, &utxo_set).is_err(), "Excessive coinbase reward must be rejected");

        // Test 8: Valid block (should pass all checks)
        let valid_block = consensus.mine_valid_block(&utxo_set).unwrap();
        assert!(consensus.validate_block(&valid_block, &utxo_set).is_ok(), "Valid block must pass validation");
    }

    #[test]
    fn test_concurrent_mining_race_conditions() {
        // Integration: Multiple miners working simultaneously without race conditions
        // Concurrency: Thread-safe mining operations

        let temp_dir = tempdir().unwrap();
        let consensus = Arc::new(Mutex::new(Consensus::new(temp_dir.path(), btpc_core::Network::Regtest).unwrap()));

        let found_blocks = Arc::new(Mutex::new(Vec::new()));
        let mut mining_handles = Vec::new();

        // Start 5 concurrent miners
        for miner_id in 0..5 {
            let consensus_clone = Arc::clone(&consensus);
            let found_blocks_clone = Arc::clone(&found_blocks);

            let handle = thread::spawn(move || {
                let miner_key = PrivateKey::generate_ml_dsa().unwrap();
                let mut miner = Miner::new(
                    format!("concurrent_miner_{}", miner_id),
                    miner_key,
                    consensus_clone.clone(),
                ).unwrap();

                let start = Instant::now();
                let mut local_blocks = Vec::new();

                // Mine for 3 seconds
                while start.elapsed() < Duration::from_secs(3) {
                    if let Ok(block) = miner.mine_single_attempt() {
                        // Validate block before adding
                        let consensus_guard = consensus_clone.lock().unwrap();
                        let utxo_set = consensus_guard.get_utxo_set();

                        if consensus_guard.validate_block(&block, &utxo_set).is_ok() {
                            local_blocks.push(block);
                        }
                    }
                }

                // Add found blocks to shared collection
                let mut found_guard = found_blocks_clone.lock().unwrap();
                found_guard.extend(local_blocks.clone());
                drop(found_guard);

                local_blocks.len()
            });

            mining_handles.push(handle);
        }

        // Wait for all miners to complete
        let block_counts: Vec<_> = mining_handles.into_iter()
            .map(|handle| handle.join().unwrap())
            .collect();

        let total_blocks_found: usize = block_counts.iter().sum();
        assert!(total_blocks_found > 0, "Concurrent mining must find blocks");

        // Verify no race conditions in block storage
        let found_blocks_guard = found_blocks.lock().unwrap();
        assert_eq!(found_blocks_guard.len(), total_blocks_found, "All found blocks must be properly stored");

        // Verify all blocks are unique (no duplicates from race conditions)
        let mut block_hashes = std::collections::HashSet::new();
        for block in found_blocks_guard.iter() {
            let block_hash = block.hash();
            assert!(block_hashes.insert(block_hash), "Each block must have unique hash (no race condition duplicates)");
        }

        // Verify all blocks are valid
        let consensus_guard = consensus.lock().unwrap();
        let utxo_set = consensus_guard.get_utxo_set();

        for block in found_blocks_guard.iter() {
            assert!(consensus_guard.validate_block(block, &utxo_set).is_ok(), "All concurrent blocks must be valid");
        }
    }

    #[test]
    fn test_mining_performance_optimization() {
        // Integration: Mining performance optimization and monitoring
        // Performance: Achieve target hashrate and efficiency

        let temp_dir = tempdir().unwrap();
        let mining_config = MiningConfig {
            network: btpc_core::Network::Regtest,
            reward_address: PrivateKey::generate_ml_dsa().unwrap().public_key().to_address(),
            max_workers: num_cpus::get(),
            difficulty_target: DifficultyTarget::from_bits(0x207fffff),
        };

        let mut miner = Miner::new_optimized("performance_test", mining_config).unwrap();

        // Baseline performance test
        let baseline_start = Instant::now();
        let mut baseline_hashes = 0u64;

        while baseline_start.elapsed() < Duration::from_secs(2) {
            baseline_hashes += miner.perform_hash_attempt();
        }

        let baseline_hashrate = baseline_hashes as f64 / baseline_start.elapsed().as_secs_f64();
        assert!(baseline_hashrate > 1000.0, "Baseline hashrate must be >1000 H/s");

        // Optimized performance test
        miner.enable_optimizations().unwrap();

        let optimized_start = Instant::now();
        let mut optimized_hashes = 0u64;

        while optimized_start.elapsed() < Duration::from_secs(2) {
            optimized_hashes += miner.perform_hash_attempt();
        }

        let optimized_hashrate = optimized_hashes as f64 / optimized_start.elapsed().as_secs_f64();

        // Optimizations should improve performance by at least 10%
        let improvement_ratio = optimized_hashrate / baseline_hashrate;
        assert!(improvement_ratio > 1.1, "Optimizations must improve hashrate by >10%");

        // Test mining efficiency (blocks found per time)
        let efficiency_start = Instant::now();
        let mut blocks_found = 0;

        while efficiency_start.elapsed() < Duration::from_secs(5) && blocks_found < 3 {
            if miner.mine_single_attempt().is_ok() {
                blocks_found += 1;
            }
        }

        let efficiency_time = efficiency_start.elapsed();
        let blocks_per_minute = (blocks_found as f64 / efficiency_time.as_secs_f64()) * 60.0;

        assert!(blocks_found > 0, "Mining must find at least one block in 5 seconds");
        assert!(blocks_per_minute > 5.0, "Mining efficiency must be >5 blocks per minute on regtest");

        // Monitor resource usage
        let cpu_usage = miner.get_cpu_usage().unwrap();
        let memory_usage = miner.get_memory_usage().unwrap();

        assert!(cpu_usage > 0.0 && cpu_usage <= 100.0, "CPU usage must be valid percentage");
        assert!(memory_usage < 100 * 1024 * 1024, "Memory usage must be <100MB for mining");

        // Test graceful shutdown
        let shutdown_start = Instant::now();
        miner.shutdown().unwrap();
        let shutdown_time = shutdown_start.elapsed();

        assert!(shutdown_time < Duration::from_secs(1), "Mining shutdown must be quick");
    }
}

// Note: These tests WILL FAIL initially because the implementation doesn't exist yet.
// This is the correct TDD approach - write tests first, then implement to make them pass.