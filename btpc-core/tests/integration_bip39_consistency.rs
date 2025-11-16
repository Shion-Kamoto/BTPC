//! Integration Test T028: BIP39 100x Consistency Verification
//!
//! Verifies that the same 24-word BIP39 mnemonic produces identical ML-DSA keys
//! across 100 iterations, ensuring deterministic key generation for cross-device recovery.
//!
//! Feature 008: BIP39 Deterministic Wallet Recovery
//! Constitutional Compliance: Article VI.3 (TDD - RED-GREEN-REFACTOR)

use btpc_core::crypto::{
    bip39::Mnemonic,
    keys::PrivateKey,
    shake256_derivation::expand_seed_to_ml_dsa,
};

/// Test official BIP39 test vector produces same keys 100 times
#[test]
fn test_100x_consistency_official_vector() {
    // Official BIP39 test vector (English, 24 words)
    let mnemonic_phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

    let mnemonic = Mnemonic::parse(mnemonic_phrase)
        .expect("Failed to parse official test vector");

    // Generate seed once (this is the BIP39 → 32-byte seed step)
    let seed = mnemonic.to_seed("")
        .expect("Failed to derive seed from mnemonic");

    // Store first iteration results as reference
    let first_ml_dsa_key = PrivateKey::from_seed_deterministic(&seed)
        .expect("Failed to generate ML-DSA key from seed");
    let first_public_key = first_ml_dsa_key.public_key();
    let first_private_bytes = first_ml_dsa_key.to_bytes();
    let first_public_bytes = first_public_key.to_bytes();

    // Verify 100 iterations produce identical keys
    for iteration in 1..=100 {
        // Re-derive key from same seed
        let ml_dsa_key = PrivateKey::from_seed_deterministic(&seed)
            .expect(&format!("Iteration {}: Failed to generate ML-DSA key", iteration));

        let public_key = ml_dsa_key.public_key();
        let private_bytes = ml_dsa_key.to_bytes();
        let public_bytes = public_key.to_bytes();

        // Verify private key bytes are identical
        assert_eq!(
            private_bytes, first_private_bytes,
            "Iteration {}: Private key bytes differ from first iteration", iteration
        );

        // Verify public key bytes are identical
        assert_eq!(
            public_bytes, first_public_bytes,
            "Iteration {}: Public key bytes differ from first iteration", iteration
        );

        // Verify key sizes are correct
        assert_eq!(
            private_bytes.len(), 4000,
            "Iteration {}: ML-DSA private key size should be 4000 bytes", iteration
        );
        assert_eq!(
            public_bytes.len(), 1952,
            "Iteration {}: ML-DSA public key size should be 1952 bytes", iteration
        );
    }

    println!("✅ 100x consistency verified: Same mnemonic → same keys (all 100 iterations)");
}

/// Test random mnemonic produces consistent keys across 50 iterations
#[test]
fn test_50x_consistency_random_mnemonic() {
    // Generate a random valid 24-word mnemonic
    let test_mnemonic = "legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth title";

    let mnemonic = Mnemonic::parse(test_mnemonic)
        .expect("Failed to parse test mnemonic");

    let seed = mnemonic.to_seed("")
        .expect("Failed to derive seed");

    // First iteration
    let first_key = PrivateKey::from_seed_deterministic(&seed)
        .expect("Failed to generate first key");
    let first_bytes = first_key.to_bytes();

    // Verify 50 iterations
    for iteration in 1..=50 {
        let key = PrivateKey::from_seed_deterministic(&seed)
            .expect(&format!("Iteration {}: Key generation failed", iteration));

        assert_eq!(
            key.to_bytes(), first_bytes,
            "Iteration {}: Key bytes differ", iteration
        );
    }

    println!("✅ 50x consistency verified for random mnemonic");
}

/// Test different passphrases produce different seeds (but each is deterministic)
#[test]
fn test_passphrase_determinism() {
    let mnemonic_phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";
    let mnemonic = Mnemonic::parse(mnemonic_phrase).unwrap();

    // Derive seeds with different passphrases
    let seed_empty = mnemonic.to_seed("").unwrap();
    let seed_trezor = mnemonic.to_seed("TREZOR").unwrap();
    let seed_custom = mnemonic.to_seed("my-secure-passphrase").unwrap();

    // Generate keys from each seed
    let key_empty = PrivateKey::from_seed_deterministic(&seed_empty).unwrap();
    let key_trezor = PrivateKey::from_seed_deterministic(&seed_trezor).unwrap();
    let key_custom = PrivateKey::from_seed_deterministic(&seed_custom).unwrap();

    // Verify different passphrases produce different keys
    assert_ne!(
        key_empty.to_bytes(), key_trezor.to_bytes(),
        "Empty and TREZOR passphrases should produce different keys"
    );
    assert_ne!(
        key_empty.to_bytes(), key_custom.to_bytes(),
        "Empty and custom passphrases should produce different keys"
    );
    assert_ne!(
        key_trezor.to_bytes(), key_custom.to_bytes(),
        "TREZOR and custom passphrases should produce different keys"
    );

    // But each passphrase is deterministic (verify 10x)
    for _ in 0..10 {
        let key_empty_again = PrivateKey::from_seed_deterministic(&seed_empty).unwrap();
        assert_eq!(key_empty.to_bytes(), key_empty_again.to_bytes());

        let key_trezor_again = PrivateKey::from_seed_deterministic(&seed_trezor).unwrap();
        assert_eq!(key_trezor.to_bytes(), key_trezor_again.to_bytes());
    }

    println!("✅ Passphrase determinism verified: Different passphrases → different keys (but each is deterministic)");
}

/// Test SHAKE256 seed expansion is deterministic
#[test]
fn test_shake256_expansion_determinism() {
    let test_seed: [u8; 32] = [42u8; 32]; // Arbitrary 32-byte seed

    // Expand seed 100 times and verify identical results
    let first_expanded = expand_seed_to_ml_dsa(&test_seed)
        .expect("Failed to expand seed");

    for iteration in 1..=100 {
        let expanded = expand_seed_to_ml_dsa(&test_seed)
            .expect(&format!("Iteration {}: Failed to expand seed", iteration));

        assert_eq!(
            expanded, first_expanded,
            "Iteration {}: SHAKE256 expansion produced different result", iteration
        );
    }

    println!("✅ SHAKE256 expansion determinism verified (100 iterations)");
}

/// Test end-to-end consistency: Mnemonic → Seed → SHAKE256 → ML-DSA keys
#[test]
fn test_e2e_determinism_chain() {
    let mnemonic_phrase = "letter advice cage absurd amount doctor acoustic avoid letter advice cage absurd amount doctor acoustic avoid letter advice cage absurd amount doctor acoustic bless";

    // Repeat entire chain 25 times
    let mut reference_private_bytes: Option<[u8; 4000]> = None;
    let mut reference_public_bytes: Option<[u8; 1952]> = None;

    for iteration in 0..25 {
        // Full chain: Parse → Seed → Expand → Generate Key
        let mnemonic = Mnemonic::parse(mnemonic_phrase)
            .expect(&format!("Iteration {}: Failed to parse mnemonic", iteration));

        let seed = mnemonic.to_seed("")
            .expect(&format!("Iteration {}: Failed to derive seed", iteration));

        let ml_dsa_key = PrivateKey::from_seed_deterministic(&seed)
            .expect(&format!("Iteration {}: Failed to generate ML-DSA key", iteration));

        let private_bytes = ml_dsa_key.to_bytes();
        let public_bytes = ml_dsa_key.public_key().to_bytes();

        if iteration == 0 {
            // Store reference
            reference_private_bytes = Some(private_bytes);
            reference_public_bytes = Some(public_bytes);
        } else {
            // Verify matches reference
            assert_eq!(
                private_bytes, reference_private_bytes.unwrap(),
                "Iteration {}: End-to-end chain produced different private key", iteration
            );
            assert_eq!(
                public_bytes, reference_public_bytes.unwrap(),
                "Iteration {}: End-to-end chain produced different public key", iteration
            );
        }
    }

    println!("✅ End-to-end determinism verified: Mnemonic → Seed → SHAKE256 → ML-DSA (25 iterations)");
}

/// Performance benchmark: Measure key derivation speed
#[test]
fn test_key_derivation_performance() {
    let mnemonic_phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";
    let mnemonic = Mnemonic::parse(mnemonic_phrase).unwrap();
    let seed = mnemonic.to_seed("").unwrap();

    let start = std::time::Instant::now();

    // Generate 100 keys
    for _ in 0..100 {
        let _ = PrivateKey::from_seed_deterministic(&seed)
            .expect("Key generation failed");
    }

    let duration = start.elapsed();
    let avg_per_key = duration.as_micros() / 100;

    println!("⚡ Performance: 100 key derivations in {:?} (avg: {} μs/key)", duration, avg_per_key);

    // Sanity check: Should be reasonably fast (< 100ms per key on modern hardware)
    assert!(
        avg_per_key < 100_000,
        "Key derivation too slow: {} μs/key (expected < 100,000 μs)", avg_per_key
    );
}