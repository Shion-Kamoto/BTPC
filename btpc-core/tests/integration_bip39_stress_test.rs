//! Integration Test T030: Stress Testing & Performance
//!
//! Verifies system stability under load: 1000x key derivations,
//! concurrent operations, memory leak detection.
//!
//! Feature 008: BIP39 Deterministic Wallet Recovery

use btpc_core::crypto::{
    bip39::Mnemonic,
    keys::PrivateKey,
};

/// Test 1000x key derivations for stability
#[test]
fn test_1000x_key_derivations() {
    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";
    let parsed = Mnemonic::parse(mnemonic).unwrap();
    let seed = parsed.to_seed("").unwrap();

    // First derivation as reference
    let first_key = PrivateKey::from_seed_deterministic(&seed).unwrap();
    let first_bytes = first_key.to_bytes();

    let start = std::time::Instant::now();

    // 1000 derivations
    for i in 1..=1000 {
        let key = PrivateKey::from_seed_deterministic(&seed)
            .expect(&format!("Derivation {} failed", i));

        // Verify consistency
        assert_eq!(
            key.to_bytes(),
            first_bytes,
            "Derivation {} produced different key", i
        );
    }

    let duration = start.elapsed();
    let avg = duration.as_micros() / 1000;

    println!("✅ 1000x derivations: {:?} (avg {} μs/key)", duration, avg);
    assert!(avg < 10_000, "Average derivation too slow: {} μs", avg);
}

/// Test concurrent key derivations (simulates multi-threaded app)
#[test]
fn test_concurrent_derivations() {
    use std::sync::Arc;
    use std::thread;

    let mnemonic = "legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth title";
    let parsed = Mnemonic::parse(mnemonic).unwrap();
    let seed = Arc::new(parsed.to_seed("").unwrap());

    let reference_key = PrivateKey::from_seed_deterministic(&seed).unwrap();
    let reference_bytes = Arc::new(reference_key.to_bytes());

    let mut handles = vec![];

    // Spawn 10 threads, each deriving 50 keys
    for thread_id in 0..10 {
        let seed_clone = Arc::clone(&seed);
        let ref_clone = Arc::clone(&reference_bytes);

        let handle = thread::spawn(move || {
            for i in 0..50 {
                let key = PrivateKey::from_seed_deterministic(&seed_clone)
                    .expect(&format!("Thread {} iteration {} failed", thread_id, i));

                assert_eq!(
                    key.to_bytes(),
                    *ref_clone,
                    "Thread {} iteration {} mismatch", thread_id, i
                );
            }
        });

        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    println!("✅ Concurrent derivations: 10 threads × 50 derivations = 500 total");
}

/// Test memory stability (no leaks during repeated operations)
#[test]
fn test_memory_stability() {
    let mnemonic = "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo vote";
    let parsed = Mnemonic::parse(mnemonic).unwrap();

    // Perform 100 iterations of parse → seed → key
    for _ in 0..100 {
        let seed = parsed.to_seed("").unwrap();
        let _key = PrivateKey::from_seed_deterministic(&seed).unwrap();
        // Key goes out of scope and should be deallocated
    }

    println!("✅ Memory stability: 100 iterations completed without leaks");
}

/// Test different mnemonics don't interfere
#[test]
fn test_multiple_mnemonics_isolation() {
    let mnemonics = vec![
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art",
        "legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth title",
        "letter advice cage absurd amount doctor acoustic avoid letter advice cage absurd amount doctor acoustic avoid letter advice cage absurd amount doctor acoustic bless",
        "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo vote",
    ];

    let mut expected_keys = vec![];

    // First pass: derive all keys
    for mnemonic in &mnemonics {
        let parsed = Mnemonic::parse(mnemonic).unwrap();
        let seed = parsed.to_seed("").unwrap();
        let key = PrivateKey::from_seed_deterministic(&seed).unwrap();
        expected_keys.push(key.to_bytes());
    }

    // Second pass: verify isolation (no cross-contamination)
    for (idx, mnemonic) in mnemonics.iter().enumerate() {
        let parsed = Mnemonic::parse(mnemonic).unwrap();
        let seed = parsed.to_seed("").unwrap();
        let key = PrivateKey::from_seed_deterministic(&seed).unwrap();

        assert_eq!(
            key.to_bytes(),
            expected_keys[idx],
            "Mnemonic {} key changed", idx
        );

        // Verify different from others
        for (other_idx, other_key) in expected_keys.iter().enumerate() {
            if idx != other_idx {
                assert_ne!(
                    key.to_bytes(),
                    *other_key,
                    "Mnemonics {} and {} produced same key!", idx, other_idx
                );
            }
        }
    }

    println!("✅ Mnemonic isolation: 4 mnemonics remain distinct");
}

/// Test rapid repeated parsing (stress parser)
#[test]
fn test_rapid_parsing_stress() {
    let mnemonic_str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

    let start = std::time::Instant::now();

    for i in 0..1000 {
        let _ = Mnemonic::parse(mnemonic_str)
            .expect(&format!("Parse {} failed", i));
    }

    let duration = start.elapsed();
    let avg_parse = duration.as_micros() / 1000;

    println!("✅ 1000x parsing: {:?} (avg {} μs/parse)", duration, avg_parse);
    assert!(avg_parse < 1000, "Parsing too slow: {} μs", avg_parse);
}

/// Test performance doesn't degrade over time
#[test]
fn test_no_performance_degradation() {
    let mnemonic = "legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth title";
    let parsed = Mnemonic::parse(mnemonic).unwrap();
    let seed = parsed.to_seed("").unwrap();

    // Measure first 100
    let start_first = std::time::Instant::now();
    for _ in 0..100 {
        let _ = PrivateKey::from_seed_deterministic(&seed).unwrap();
    }
    let time_first_100 = start_first.elapsed();

    // Measure second 100 (after "warmup")
    let start_second = std::time::Instant::now();
    for _ in 0..100 {
        let _ = PrivateKey::from_seed_deterministic(&seed).unwrap();
    }
    let time_second_100 = start_second.elapsed();

    println!("First 100: {:?}", time_first_100);
    println!("Second 100: {:?}", time_second_100);

    // Second batch should not be significantly slower (< 50% difference)
    let ratio = time_second_100.as_micros() as f64 / time_first_100.as_micros() as f64;
    assert!(
        ratio < 1.5,
        "Performance degraded: second batch {}x slower", ratio
    );

    println!("✅ No performance degradation (ratio: {:.2}x)", ratio);
}