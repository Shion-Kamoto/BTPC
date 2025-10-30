//! Integration test: Concurrent transactions
//!
//! This test verifies proper handling of concurrent transaction attempts:
//! 1. UTXO locking prevents double-spending
//! 2. Reservation system works under concurrency
//! 3. Multiple transactions can be processed safely
//! 4. Race conditions are prevented
//! 5. Deadlock prevention

use btpc_desktop_app::utxo_manager::UTXOManager;
use std::sync::Arc;
use std::thread;
use tempfile::TempDir;

#[cfg(test)]
mod tests {
    use super::*;

    /// Test: Two threads cannot reserve same UTXO
    #[test]
    fn test_concurrent_utxo_reservation_conflict() {
        // Given: Shared wallet with one UTXO
        let temp_dir = TempDir::new().unwrap();
        let wallet = Arc::new(UTXOManager::new(temp_dir.path().to_path_buf()).unwrap());

        let utxo_key = "concurrent_test:0".to_string();

        // When: Two threads try to reserve same UTXO simultaneously
        let wallet1 = Arc::clone(&wallet);
        let utxo_key1 = utxo_key.clone();
        let handle1 = thread::spawn(move || {
            wallet1.reserve_utxos(vec![utxo_key1], Some("thread1_tx".to_string()))
        });

        let wallet2 = Arc::clone(&wallet);
        let utxo_key2 = utxo_key.clone();
        let handle2 = thread::spawn(move || {
            wallet2.reserve_utxos(vec![utxo_key2], Some("thread2_tx".to_string()))
        });

        // Then: One should succeed, one should fail
        let result1 = handle1.join().unwrap();
        let result2 = handle2.join().unwrap();

        let success_count = result1.is_ok() as u32 + result2.is_ok() as u32;
        let failure_count = result1.is_err() as u32 + result2.is_err() as u32;

        assert_eq!(success_count, 1, "Exactly one reservation should succeed");
        assert_eq!(failure_count, 1, "Exactly one reservation should fail");

        println!("✅ Concurrent reservation conflict prevented:");
        println!("  Thread 1: {}", if result1.is_ok() { "SUCCESS" } else { "FAILED" });
        println!("  Thread 2: {}", if result2.is_ok() { "SUCCESS" } else { "FAILED" });

        // Cleanup successful reservation
        if let Ok(token) = result1 {
            wallet.release_reservation(&token.id).unwrap();
        } else if let Ok(token) = result2 {
            wallet.release_reservation(&token.id).unwrap();
        }
    }

    /// Test: Multiple transactions with different UTXOs succeed
    #[test]
    fn test_concurrent_different_utxos() {
        // Given: Shared wallet with multiple UTXOs
        let temp_dir = TempDir::new().unwrap();
        let wallet = Arc::new(UTXOManager::new(temp_dir.path().to_path_buf()).unwrap());

        let utxo_keys = vec![
            "utxo_1:0".to_string(),
            "utxo_2:0".to_string(),
            "utxo_3:0".to_string(),
        ];

        // When: Three threads reserve different UTXOs simultaneously
        let mut handles = vec![];

        for (i, utxo_key) in utxo_keys.iter().enumerate() {
            let wallet_clone = Arc::clone(&wallet);
            let utxo = utxo_key.clone();
            let tx_id = format!("tx_{}", i);

            let handle = thread::spawn(move || {
                wallet_clone.reserve_utxos(vec![utxo], Some(tx_id))
            });

            handles.push(handle);
        }

        // Then: All reservations should succeed (different UTXOs)
        let mut results = vec![];
        for handle in handles {
            results.push(handle.join().unwrap());
        }

        let success_count = results.iter().filter(|r| r.is_ok()).count();
        assert_eq!(success_count, 3, "All 3 reservations should succeed with different UTXOs");

        println!("✅ Concurrent reservations of different UTXOs:");
        for (i, result) in results.iter().enumerate() {
            println!("  Thread {}: {}", i, if result.is_ok() { "SUCCESS" } else { "FAILED" });
        }

        // Cleanup
        for result in results {
            if let Ok(token) = result {
                wallet.release_reservation(&token.id).unwrap();
            }
        }
    }

    /// Test: Reserve, release, reserve in concurrent threads
    #[test]
    fn test_concurrent_reserve_release_cycle() {
        // Given: Shared wallet
        let temp_dir = TempDir::new().unwrap();
        let wallet = Arc::new(UTXOManager::new(temp_dir.path().to_path_buf()).unwrap());

        let utxo_key = "cycle_test:0".to_string();
        let iterations = 10;

        // When: Multiple threads repeatedly reserve and release same UTXO
        let mut handles = vec![];

        for thread_id in 0..3 {
            let wallet_clone = Arc::clone(&wallet);
            let utxo = utxo_key.clone();

            let handle = thread::spawn(move || {
                let mut success_count = 0;

                for _ in 0..iterations {
                    // Try to reserve
                    if let Ok(token) = wallet_clone.reserve_utxos(
                        vec![utxo.clone()],
                        Some(format!("thread_{}_tx", thread_id)),
                    ) {
                        success_count += 1;

                        // Simulate some work
                        thread::sleep(std::time::Duration::from_micros(100));

                        // Release
                        wallet_clone.release_reservation(&token.id).unwrap();
                    }
                }

                success_count
            });

            handles.push(handle);
        }

        // Then: All threads should get some successful reservations
        let mut total_successes = 0;
        for handle in handles {
            let count = handle.join().unwrap();
            total_successes += count;
            assert!(count > 0, "Each thread should succeed at least once");
        }

        println!("✅ Concurrent reserve-release cycle:");
        println!("  Total successful reservations: {}", total_successes);
        println!("  Expected around: {}", iterations * 3);

        // Verify UTXO is released after all threads complete
        let final_reservation = wallet.reserve_utxos(
            vec![utxo_key],
            Some("final_check".to_string()),
        );
        assert!(final_reservation.is_ok(), "UTXO should be available after all threads complete");
    }

    /// Test: Check for deadlocks with multiple UTXOs
    #[test]
    fn test_no_deadlock_multiple_utxos() {
        // Given: Shared wallet with two UTXOs
        let temp_dir = TempDir::new().unwrap();
        let wallet = Arc::new(UTXOManager::new(temp_dir.path().to_path_buf()).unwrap());

        let utxo_a = "utxo_a:0".to_string();
        let utxo_b = "utxo_b:0".to_string();

        // When: Two threads try to reserve in opposite order
        // Thread 1: A then B
        let wallet1 = Arc::clone(&wallet);
        let utxo_a1 = utxo_a.clone();
        let utxo_b1 = utxo_b.clone();
        let handle1 = thread::spawn(move || {
            let res_a = wallet1.reserve_utxos(vec![utxo_a1.clone()], Some("thread1_a".to_string()));
            thread::sleep(std::time::Duration::from_millis(10));
            let res_b = wallet1.reserve_utxos(vec![utxo_b1.clone()], Some("thread1_b".to_string()));

            (res_a, res_b)
        });

        // Thread 2: B then A
        let wallet2 = Arc::clone(&wallet);
        let utxo_a2 = utxo_a.clone();
        let utxo_b2 = utxo_b.clone();
        let handle2 = thread::spawn(move || {
            let res_b = wallet2.reserve_utxos(vec![utxo_b2.clone()], Some("thread2_b".to_string()));
            thread::sleep(std::time::Duration::from_millis(10));
            let res_a = wallet2.reserve_utxos(vec![utxo_a2.clone()], Some("thread2_a".to_string()));

            (res_b, res_a)
        });

        // Then: Both threads should complete (no deadlock)
        let (res1_a, res1_b) = handle1.join().expect("Thread 1 should complete without deadlock");
        let (res2_b, res2_a) = handle2.join().expect("Thread 2 should complete without deadlock");

        println!("✅ No deadlock detected:");
        println!("  Thread 1 - UTXO A: {}, UTXO B: {}",
            if res1_a.is_ok() { "OK" } else { "ERR" },
            if res1_b.is_ok() { "OK" } else { "ERR" }
        );
        println!("  Thread 2 - UTXO B: {}, UTXO A: {}",
            if res2_b.is_ok() { "OK" } else { "ERR" },
            if res2_a.is_ok() { "OK" } else { "ERR" }
        );

        // Cleanup
        for result in [res1_a, res1_b, res2_b, res2_a] {
            if let Ok(token) = result {
                wallet.release_reservation(&token.id).unwrap();
            }
        }
    }

    /// Test: Concurrent cleanup of expired reservations
    #[test]
    fn test_concurrent_cleanup() {
        // Given: Shared wallet with reservations
        let temp_dir = TempDir::new().unwrap();
        let wallet = Arc::new(UTXOManager::new(temp_dir.path().to_path_buf()).unwrap());

        // Create some reservations
        for i in 0..5 {
            let _ = wallet.reserve_utxos(
                vec![format!("utxo_{}:0", i)],
                Some(format!("tx_{}", i)),
            );
        }

        // When: Multiple threads run cleanup simultaneously
        let mut handles = vec![];

        for _ in 0..5 {
            let wallet_clone = Arc::clone(&wallet);
            let handle = thread::spawn(move || {
                wallet_clone.cleanup_expired_reservations();
            });
            handles.push(handle);
        }

        // Then: All cleanups should complete without panic
        for handle in handles {
            handle.join().expect("Cleanup thread should complete without panic");
        }

        println!("✅ Concurrent cleanup completed without errors");
    }

    /// Test: High concurrency stress test
    #[test]
    fn test_high_concurrency_stress() {
        // Given: Shared wallet
        let temp_dir = TempDir::new().unwrap();
        let wallet = Arc::new(UTXOManager::new(temp_dir.path().to_path_buf()).unwrap());

        let num_threads = 20;
        let operations_per_thread = 50;

        // When: Many threads perform many operations
        let mut handles = vec![];

        for thread_id in 0..num_threads {
            let wallet_clone = Arc::clone(&wallet);

            let handle = thread::spawn(move || {
                let mut operations = 0;

                for op_id in 0..operations_per_thread {
                    let utxo_key = format!("stress_{}:0", thread_id % 10); // Share some UTXOs

                    // Try to reserve
                    if let Ok(token) = wallet_clone.reserve_utxos(
                        vec![utxo_key],
                        Some(format!("t{}_op{}", thread_id, op_id)),
                    ) {
                        operations += 1;

                        // Small delay to simulate work
                        thread::sleep(std::time::Duration::from_micros(10));

                        // Release
                        let _ = wallet_clone.release_reservation(&token.id);
                    }

                    // Occasionally run cleanup
                    if op_id % 10 == 0 {
                        wallet_clone.cleanup_expired_reservations();
                    }
                }

                operations
            });

            handles.push(handle);
        }

        // Then: All threads should complete
        let mut total_operations = 0;
        for (i, handle) in handles.into_iter().enumerate() {
            let ops = handle.join().expect(&format!("Thread {} should complete", i));
            total_operations += ops;
        }

        println!("✅ High concurrency stress test:");
        println!("  Threads: {}", num_threads);
        println!("  Total successful operations: {}", total_operations);
        println!("  All threads completed without panic");

        assert!(total_operations > 0, "Should have some successful operations");
    }

    /// Test: Race condition in balance check
    #[test]
    fn test_balance_check_race_condition() {
        // Given: Shared wallet with funds
        let temp_dir = TempDir::new().unwrap();
        let mut wallet_setup = UTXOManager::new(temp_dir.path().to_path_buf()).unwrap();
        let address = "btpc1qracetest000000000000000000000000000000".to_string();

        wallet_setup.add_coinbase_utxo(
            "single_utxo".to_string(),
            0,
            100_000_000, // 1 BTPC
            address.clone(),
            1,
        ).unwrap();

        drop(wallet_setup); // Drop mutable reference

        let wallet = Arc::new(UTXOManager::new(temp_dir.path().to_path_buf()).unwrap());

        // When: Two threads check balance simultaneously
        let wallet1 = Arc::clone(&wallet);
        let addr1 = address.clone();
        let handle1 = thread::spawn(move || {
            wallet1.get_balance(&addr1).0
        });

        let wallet2 = Arc::clone(&wallet);
        let addr2 = address.clone();
        let handle2 = thread::spawn(move || {
            wallet2.get_balance(&addr2).0
        });

        // Then: Both should get consistent balance
        let balance1 = handle1.join().unwrap();
        let balance2 = handle2.join().unwrap();

        assert_eq!(balance1, balance2, "Balance checks should be consistent");
        assert_eq!(balance1, 100_000_000, "Balance should be correct");

        println!("✅ Concurrent balance checks are consistent: {} credits", balance1);
    }
}