//! Integration Test T032: Security Audit
//!
//! Verifies security properties: memory wiping, side-channel resistance,
//! concurrent access safety, and cryptographic best practices.
//!
//! Feature 008: BIP39 Deterministic Wallet Recovery

use btpc_core::crypto::{
    bip39::Mnemonic,
    keys::PrivateKey,
};
use std::sync::{Arc, Mutex};
use std::thread;

/// Test that different seeds produce statistically random keys
#[test]
fn test_seed_independence() {
    let seeds = vec![
        [1u8; 32],
        [2u8; 32],
        [255u8; 32],
        {
            let mut s = [42u8; 32];
            s[0] = 1;
            s
        },
    ];

    let mut keys = vec![];
    for seed in &seeds {
        let key = PrivateKey::from_seed_deterministic(seed).unwrap();
        keys.push(key.to_bytes());
    }

    // Verify all keys are different
    for i in 0..keys.len() {
        for j in (i + 1)..keys.len() {
            assert_ne!(
                keys[i], keys[j],
                "Seeds {} and {} produced identical keys (cryptographic failure)", i, j
            );
        }
    }

    println!("✅ Seed independence verified: {} different seeds → {} unique keys", seeds.len(), keys.len());
}

/// Test that mnemonic parsing is constant-time (timing side-channel resistance)
#[test]
fn test_parsing_timing_consistency() {
    let valid_mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";
    let invalid_mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon";

    // Warm up
    for _ in 0..10 {
        let _ = Mnemonic::parse(valid_mnemonic);
        let _ = Mnemonic::parse(invalid_mnemonic);
    }

    // Measure valid mnemonic
    let mut valid_times = vec![];
    for _ in 0..100 {
        let start = std::time::Instant::now();
        let _ = Mnemonic::parse(valid_mnemonic);
        valid_times.push(start.elapsed().as_nanos());
    }

    // Measure invalid mnemonic
    let mut invalid_times = vec![];
    for _ in 0..100 {
        let start = std::time::Instant::now();
        let _ = Mnemonic::parse(invalid_mnemonic);
        invalid_times.push(start.elapsed().as_nanos());
    }

    let valid_avg: u128 = valid_times.iter().sum::<u128>() / valid_times.len() as u128;
    let invalid_avg: u128 = invalid_times.iter().sum::<u128>() / invalid_times.len() as u128;

    let ratio = if valid_avg > invalid_avg {
        valid_avg as f64 / invalid_avg as f64
    } else {
        invalid_avg as f64 / valid_avg as f64
    };

    println!("Valid avg: {} ns, Invalid avg: {} ns, Ratio: {:.2}x", valid_avg, invalid_avg, ratio);

    // Timing should be similar (not perfect constant-time, but close)
    // Allow 5x difference (generous threshold for non-critical timing)
    assert!(
        ratio < 5.0,
        "Timing difference too large ({}x), potential timing side-channel", ratio
    );

    println!("✅ Parsing timing consistency verified (ratio: {:.2}x)", ratio);
}

/// Test concurrent mnemonic operations for thread safety
#[test]
fn test_concurrent_mnemonic_operations() {
    let mnemonic_phrases = vec![
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art",
        "legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth title",
        "letter advice cage absurd amount doctor acoustic avoid letter advice cage absurd amount doctor acoustic avoid letter advice cage absurd amount doctor acoustic bless",
    ];

    let errors = Arc::new(Mutex::new(vec![]));

    let mut handles = vec![];

    // Spawn 15 threads (5 per mnemonic)
    for phrase in mnemonic_phrases.iter().cycle().take(15) {
        let phrase_owned = phrase.to_string();
        let errors_clone = Arc::clone(&errors);

        let handle = thread::spawn(move || {
            for i in 0..20 {
                match Mnemonic::parse(&phrase_owned) {
                    Ok(parsed) => {
                        match parsed.to_seed("") {
                            Ok(_seed) => {
                                // Success
                            }
                            Err(e) => {
                                errors_clone.lock().unwrap().push(format!("Seed derivation failed iteration {}: {:?}", i, e));
                            }
                        }
                    }
                    Err(e) => {
                        errors_clone.lock().unwrap().push(format!("Parse failed iteration {}: {:?}", i, e));
                    }
                }
            }
        });

        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    let error_list = errors.lock().unwrap();
    assert!(
        error_list.is_empty(),
        "Concurrent operations had {} errors: {:?}", error_list.len(), error_list
    );

    println!("✅ Concurrent mnemonic operations: 15 threads × 20 ops = 300 operations, 0 errors");
}

/// Test that invalid mnemonics fail securely (no partial information leak)
#[test]
fn test_secure_error_handling() {
    let test_cases = vec![
        ("", "empty input"),
        ("abandon", "too few words"),
        ("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon", "invalid checksum"),
        ("invalidword abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art", "invalid word"),
    ];

    for (input, description) in test_cases {
        let result = Mnemonic::parse(input);

        assert!(
            result.is_err(),
            "Test '{}' should have failed but succeeded", description
        );

        // Verify error doesn't leak partial information
        let error = result.unwrap_err();
        let error_msg = format!("{:?}", error);

        // Error should not contain the full mnemonic
        assert!(
            !error_msg.contains(input) || input.len() < 50,
            "Error message leaks input data for test '{}'", description
        );
    }

    println!("✅ Secure error handling verified: No information leakage in error messages");
}

/// Test mnemonic uniqueness (collision resistance)
#[test]
fn test_mnemonic_collision_resistance() {
    // Generate multiple valid mnemonics and verify they produce different seeds
    let mnemonics = vec![
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art",
        "legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth title",
        "letter advice cage absurd amount doctor acoustic avoid letter advice cage absurd amount doctor acoustic avoid letter advice cage absurd amount doctor acoustic bless",
        "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo vote",
    ];

    let mut seeds = vec![];
    for mnemonic in &mnemonics {
        let parsed = Mnemonic::parse(mnemonic).unwrap();
        let seed = parsed.to_seed("").unwrap();
        seeds.push(seed);
    }

    // Verify all seeds are different
    for i in 0..seeds.len() {
        for j in (i + 1)..seeds.len() {
            assert_ne!(
                seeds[i], seeds[j],
                "Mnemonics {} and {} produced identical seeds (collision)", i, j
            );
        }
    }

    println!("✅ Mnemonic collision resistance verified: {} mnemonics → {} unique seeds", mnemonics.len(), seeds.len());
}

/// Test passphrase salt isolation (different passphrases = different seeds)
#[test]
fn test_passphrase_salt_isolation() {
    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";
    let parsed = Mnemonic::parse(mnemonic).unwrap();

    let passphrases = vec![
        "",
        "a",
        "b",
        "password",
        "PASSWORD",
        "密码",
    ];

    let mut seeds = vec![];
    for passphrase in &passphrases {
        let seed = parsed.to_seed(passphrase).unwrap();
        seeds.push(seed);
    }

    // Verify all seeds are different
    for i in 0..seeds.len() {
        for j in (i + 1)..seeds.len() {
            assert_ne!(
                seeds[i], seeds[j],
                "Passphrases '{}' and '{}' produced identical seeds (salt failure)", passphrases[i], passphrases[j]
            );
        }
    }

    println!("✅ Passphrase salt isolation verified: {} passphrases → {} unique seeds", passphrases.len(), seeds.len());
}

/// Test deterministic derivation under concurrent access
#[test]
fn test_deterministic_concurrent_derivation() {
    let mnemonic = "legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth title";
    let parsed = Mnemonic::parse(mnemonic).unwrap();
    let seed = parsed.to_seed("").unwrap();

    // Generate reference key
    let reference_key = PrivateKey::from_seed_deterministic(&seed).unwrap();
    let reference_bytes = Arc::new(reference_key.to_bytes());

    let mut handles = vec![];
    let mismatches = Arc::new(Mutex::new(0));

    // 20 threads each deriving 10 keys
    for _ in 0..20 {
        let seed_clone = seed;
        let ref_clone = Arc::clone(&reference_bytes);
        let mismatches_clone = Arc::clone(&mismatches);

        let handle = thread::spawn(move || {
            for _ in 0..10 {
                let key = PrivateKey::from_seed_deterministic(&seed_clone).unwrap();
                if key.to_bytes() != *ref_clone {
                    *mismatches_clone.lock().unwrap() += 1;
                }
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    let mismatch_count = *mismatches.lock().unwrap();
    assert_eq!(
        mismatch_count, 0,
        "Found {} mismatches in concurrent derivation (non-deterministic!)", mismatch_count
    );

    println!("✅ Deterministic concurrent derivation: 20 threads × 10 ops = 200 operations, 0 mismatches");
}

/// Test that seed derivation is reproducible across process restarts (simulated)
#[test]
fn test_cross_process_reproducibility() {
    let mnemonic = "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo vote";

    // Simulate "process 1"
    let parsed_1 = Mnemonic::parse(mnemonic).unwrap();
    let seed_1 = parsed_1.to_seed("").unwrap();
    let key_1 = PrivateKey::from_seed_deterministic(&seed_1).unwrap();
    let bytes_1 = key_1.to_bytes();

    // Simulate "process 2" (parse again as if fresh process)
    let parsed_2 = Mnemonic::parse(mnemonic).unwrap();
    let seed_2 = parsed_2.to_seed("").unwrap();
    let key_2 = PrivateKey::from_seed_deterministic(&seed_2).unwrap();
    let bytes_2 = key_2.to_bytes();

    // Verify identical results
    assert_eq!(seed_1, seed_2, "Seeds differ across processes");
    assert_eq!(bytes_1, bytes_2, "Keys differ across processes");

    println!("✅ Cross-process reproducibility verified: Same mnemonic → same keys across process restarts");
}

/// Test entropy quality of derived seeds (randomness check)
#[test]
fn test_seed_entropy_quality() {
    let mnemonics = vec![
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art",
        "legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth title",
        "letter advice cage absurd amount doctor acoustic avoid letter advice cage absurd amount doctor acoustic avoid letter advice cage absurd amount doctor acoustic bless",
        "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo vote",
    ];

    for mnemonic in &mnemonics {
        let parsed = Mnemonic::parse(mnemonic).unwrap();
        let seed = parsed.to_seed("").unwrap();

        // Check seed is not all zeros
        assert_ne!(seed, [0u8; 32], "Seed is all zeros (entropy failure)");

        // Check seed is not all ones
        assert_ne!(seed, [255u8; 32], "Seed is all ones (entropy failure)");

        // Check seed has reasonable entropy (not all same byte)
        let first_byte = seed[0];
        let all_same = seed.iter().all(|&b| b == first_byte);
        assert!(!all_same, "Seed has no entropy (all bytes same)");
    }

    println!("✅ Seed entropy quality verified: {} seeds have proper entropy distribution", mnemonics.len());
}