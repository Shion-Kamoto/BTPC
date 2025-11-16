// Test: 100x Consistency Test for Deterministic Key Generation
// Feature 008 - T028
// Ensures that the same BIP39 mnemonic produces IDENTICAL keys across 100 iterations

use btpc_core::crypto::bip39::Mnemonic;
use btpc_core::crypto::keys::PrivateKey;

#[test]
fn test_100x_deterministic_key_generation() {
    // Test mnemonic (BIP39 test vector)
    let mnemonic_str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

    let mnemonic = Mnemonic::parse(mnemonic_str).expect("Failed to parse valid BIP39 mnemonic");

    // Generate seed from mnemonic
    let seed = mnemonic.to_seed("").expect("Failed to generate seed");

    // Generate 100 keys and verify they're ALL IDENTICAL
    let mut reference_key_bytes: Option<[u8; 4000]> = None;
    let mut reference_pubkey_bytes: Option<[u8; 1952]> = None;

    for iteration in 0..100 {
        // Generate key from same seed
        let key = PrivateKey::from_seed_deterministic(&seed)
            .expect(&format!("Failed to generate key on iteration {}", iteration));

        // Get serialized bytes
        let key_bytes = key.to_bytes();
        let pubkey_bytes = key.public_key().to_bytes();

        if iteration == 0 {
            // Store reference on first iteration
            reference_key_bytes = Some(key_bytes.clone());
            reference_pubkey_bytes = Some(pubkey_bytes.clone());
            println!("✓ Iteration {}: Generated reference key ({}bytes private, {}bytes public)",
                iteration, key_bytes.len(), pubkey_bytes.len());
        } else {
            // Verify ALL subsequent iterations match EXACTLY
            assert_eq!(
                &key_bytes,
                reference_key_bytes.as_ref().unwrap(),
                "❌ CRITICAL: Private key mismatch on iteration {}! Determinism BROKEN!",
                iteration
            );

            assert_eq!(
                &pubkey_bytes,
                reference_pubkey_bytes.as_ref().unwrap(),
                "❌ CRITICAL: Public key mismatch on iteration {}! Determinism BROKEN!",
                iteration
            );

            if iteration % 10 == 0 {
                println!("✓ Iteration {}: Byte-identical to reference", iteration);
            }
        }
    }

    println!("\n✅ SUCCESS: All 100 iterations produced IDENTICAL keys!");
    println!("✅ Deterministic key generation is FULLY FUNCTIONAL");
}

#[test]
fn test_different_seeds_produce_different_keys() {
    // Verify that different seeds produce DIFFERENT keys (not stuck on a constant)
    let mnemonic1 = Mnemonic::parse("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art")
        .expect("Failed to parse mnemonic 1");

    let mnemonic2 = Mnemonic::parse("zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo vote")
        .expect("Failed to parse mnemonic 2");

    let seed1 = mnemonic1.to_seed("").expect("Failed to generate seed 1");
    let seed2 = mnemonic2.to_seed("").expect("Failed to generate seed 2");

    let key1 = PrivateKey::from_seed_deterministic(&seed1).expect("Failed to generate key 1");
    let key2 = PrivateKey::from_seed_deterministic(&seed2).expect("Failed to generate key 2");

    assert_ne!(
        key1.to_bytes(),
        key2.to_bytes(),
        "❌ CRITICAL: Different seeds produced IDENTICAL keys! RNG may be broken!"
    );

    assert_ne!(
        key1.public_key().to_bytes(),
        key2.public_key().to_bytes(),
        "❌ CRITICAL: Different seeds produced IDENTICAL public keys!"
    );

    println!("✅ Different mnemonics correctly produce different keys");
}

#[test]
fn test_cross_iteration_consistency_with_passphrase() {
    // Test with BIP39 passphrase (optional 25th word)
    let mnemonic_str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";
    let passphrase = "TREZOR";

    let mnemonic = Mnemonic::parse(mnemonic_str).expect("Failed to parse mnemonic");
    let seed = mnemonic.to_seed(passphrase).expect("Failed to generate seed with passphrase");

    // Generate 10 keys with passphrase
    let mut reference_bytes: Option<[u8; 4000]> = None;

    for iteration in 0..10 {
        let key = PrivateKey::from_seed_deterministic(&seed)
            .expect(&format!("Failed on iteration {} with passphrase", iteration));

        let key_bytes = key.to_bytes();

        if iteration == 0 {
            reference_bytes = Some(key_bytes);
        } else {
            assert_eq!(
                &key_bytes,
                reference_bytes.as_ref().unwrap(),
                "Passphrase-derived key mismatch on iteration {}",
                iteration
            );
        }
    }

    println!("✅ Passphrase-protected keys are also deterministic");
}