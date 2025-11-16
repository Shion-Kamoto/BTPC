// Test: Cross-Device Recovery Simulation
// Feature 008 - T029
// Simulates recovering the same wallet on two different "devices" (test runs)

use btpc_core::crypto::bip39::Mnemonic;
use btpc_core::crypto::keys::PrivateKey;
use sha2::Digest;

#[test]
fn test_cross_device_recovery_simulation() {
    // Test mnemonic shared between "devices"
    let shared_mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";
    let passphrase = "";

    println!("\n=== Simulating Cross-Device Recovery ===");
    println!("Shared mnemonic: {}...", &shared_mnemonic[..50]);

    // Device 1: Generate wallet
    println!("\n[Device 1] Generating wallet from mnemonic...");
    let mnemonic_device1 = Mnemonic::parse(shared_mnemonic)
        .expect("Device 1 failed to parse mnemonic");
    let seed_device1 = mnemonic_device1.to_seed(passphrase)
        .expect("Device 1 failed to generate seed");
    let key_device1 = PrivateKey::from_seed_deterministic(&seed_device1)
        .expect("Device 1 failed to generate key");

    let privkey_bytes_device1 = key_device1.to_bytes();
    let pubkey_bytes_device1 = key_device1.public_key().to_bytes();
    let address_device1 = format!("{:x}", sha2::Sha256::digest(&pubkey_bytes_device1));

    println!("[Device 1] Address: {}...", &address_device1[..16]);
    println!("[Device 1] Private key size: {} bytes", privkey_bytes_device1.len());
    println!("[Device 1] Public key size: {} bytes", pubkey_bytes_device1.len());

    // Device 2: Recover same wallet (simulating different machine/time/OS)
    println!("\n[Device 2] Recovering wallet from same mnemonic...");
    let mnemonic_device2 = Mnemonic::parse(shared_mnemonic)
        .expect("Device 2 failed to parse mnemonic");
    let seed_device2 = mnemonic_device2.to_seed(passphrase)
        .expect("Device 2 failed to generate seed");
    let key_device2 = PrivateKey::from_seed_deterministic(&seed_device2)
        .expect("Device 2 failed to generate key");

    let privkey_bytes_device2 = key_device2.to_bytes();
    let pubkey_bytes_device2 = key_device2.public_key().to_bytes();
    let address_device2 = format!("{:x}", sha2::Sha256::digest(&pubkey_bytes_device2));

    println!("[Device 2] Address: {}...", &address_device2[..16]);
    println!("[Device 2] Private key size: {} bytes", privkey_bytes_device2.len());
    println!("[Device 2] Public key size: {} bytes", pubkey_bytes_device2.len());

    // Verify EXACT match (byte-identical recovery)
    println!("\n=== Cross-Device Verification ===");

    assert_eq!(
        privkey_bytes_device1,
        privkey_bytes_device2,
        "❌ CRITICAL FAILURE: Private keys DO NOT match across devices!"
    );
    println!("✅ Private keys match (byte-identical)");

    assert_eq!(
        pubkey_bytes_device1,
        pubkey_bytes_device2,
        "❌ CRITICAL FAILURE: Public keys DO NOT match across devices!"
    );
    println!("✅ Public keys match (byte-identical)");

    assert_eq!(
        address_device1,
        address_device2,
        "❌ CRITICAL FAILURE: Addresses DO NOT match across devices!"
    );
    println!("✅ Addresses match");

    println!("\n✅ SUCCESS: Cross-device recovery is FULLY FUNCTIONAL");
    println!("✅ User can recover wallet on ANY device with the same mnemonic");
}

#[test]
fn test_cross_device_recovery_with_passphrase() {
    // Test with BIP39 passphrase (25th word)
    let shared_mnemonic = "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo vote";
    let passphrase = "TREZOR_PASSPHRASE";

    println!("\n=== Testing Cross-Device Recovery with Passphrase ===");

    // Device 1
    let mnemonic1 = Mnemonic::parse(shared_mnemonic).unwrap();
    let seed1 = mnemonic1.to_seed(passphrase).unwrap();
    let key1 = PrivateKey::from_seed_deterministic(&seed1).unwrap();

    // Device 2
    let mnemonic2 = Mnemonic::parse(shared_mnemonic).unwrap();
    let seed2 = mnemonic2.to_seed(passphrase).unwrap();
    let key2 = PrivateKey::from_seed_deterministic(&seed2).unwrap();

    assert_eq!(
        key1.to_bytes(),
        key2.to_bytes(),
        "Passphrase-protected keys don't match across devices"
    );

    println!("✅ Cross-device recovery with passphrase works correctly");
}

#[test]
fn test_wrong_passphrase_produces_different_wallet() {
    // Verify security: wrong passphrase = completely different wallet
    let mnemonic_str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

    let mnemonic = Mnemonic::parse(mnemonic_str).unwrap();

    // Correct passphrase
    let seed_correct = mnemonic.to_seed("correct_password").unwrap();
    let key_correct = PrivateKey::from_seed_deterministic(&seed_correct).unwrap();

    // Wrong passphrase
    let seed_wrong = mnemonic.to_seed("wrong_password").unwrap();
    let key_wrong = PrivateKey::from_seed_deterministic(&seed_wrong).unwrap();

    assert_ne!(
        key_correct.to_bytes(),
        key_wrong.to_bytes(),
        "❌ SECURITY ISSUE: Different passphrases produced SAME wallet!"
    );

    println!("✅ Wrong passphrase correctly produces different wallet (security verified)");
}