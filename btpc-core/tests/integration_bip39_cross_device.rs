//! Integration Test T029: Cross-Device Recovery Simulation
//!
//! Simulates wallet creation on Device A and recovery on Device B,
//! verifying that the same mnemonic produces identical keys/addresses.
//!
//! Feature 008: BIP39 Deterministic Wallet Recovery
//! Constitutional Compliance: Article VI.3 (TDD)

use btpc_core::crypto::{
    bip39::Mnemonic,
    keys::PrivateKey,
    Address,
};
use std::collections::HashMap;

/// Simulate Device A: Create wallet and export mnemonic
struct DeviceA {
    mnemonic: Mnemonic,
    private_key: PrivateKey,
    address: String,
}

impl DeviceA {
    fn create_wallet(mnemonic_phrase: &str, passphrase: &str) -> Self {
        // Parse mnemonic
        let mnemonic = Mnemonic::parse(mnemonic_phrase)
            .expect("Device A: Failed to parse mnemonic");

        // Derive seed
        let seed = mnemonic.to_seed(passphrase)
            .expect("Device A: Failed to derive seed");

        // Generate keys
        let private_key = PrivateKey::from_seed_deterministic(&seed)
            .expect("Device A: Failed to generate keys");

        // Derive address
        let public_key = private_key.public_key();
        let address = Address::from_public_key(&public_key, btpc_core::Network::Regtest);

        DeviceA {
            mnemonic,
            private_key,
            address: address.to_string(),
        }
    }

    fn export_mnemonic(&self) -> String {
        // In real app, user writes down 24 words
        format!("{:?}", self.mnemonic)
    }
}

/// Simulate Device B: Import mnemonic and recover wallet
struct DeviceB {
    private_key: PrivateKey,
    address: String,
}

impl DeviceB {
    fn recover_wallet(mnemonic_phrase: &str, passphrase: &str) -> Self {
        // Parse imported mnemonic
        let mnemonic = Mnemonic::parse(mnemonic_phrase)
            .expect("Device B: Failed to parse mnemonic");

        // Derive seed (same as Device A)
        let seed = mnemonic.to_seed(passphrase)
            .expect("Device B: Failed to derive seed");

        // Generate keys (should match Device A)
        let private_key = PrivateKey::from_seed_deterministic(&seed)
            .expect("Device B: Failed to generate keys");

        // Derive address (should match Device A)
        let public_key = private_key.public_key();
        let address = Address::from_public_key(&public_key, btpc_core::Network::Regtest);

        DeviceB {
            private_key,
            address: address.to_string(),
        }
    }
}

/// Test basic cross-device recovery
#[test]
fn test_cross_device_recovery_basic() {
    let mnemonic_phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";
    let passphrase = "";

    // Device A: Create wallet
    let device_a = DeviceA::create_wallet(mnemonic_phrase, passphrase);
    println!("Device A created wallet: {}", device_a.address);

    // User writes down mnemonic (simulated by reusing same phrase)
    let exported_mnemonic = mnemonic_phrase;

    // Device B: Recover wallet from mnemonic
    let device_b = DeviceB::recover_wallet(exported_mnemonic, passphrase);
    println!("Device B recovered wallet: {}", device_b.address);

    // Verify keys match
    assert_eq!(
        device_a.private_key.to_bytes(),
        device_b.private_key.to_bytes(),
        "Private keys should match across devices"
    );

    assert_eq!(
        device_a.private_key.public_key().to_bytes(),
        device_b.private_key.public_key().to_bytes(),
        "Public keys should match across devices"
    );

    // Verify addresses match (most important for user)
    assert_eq!(
        device_a.address, device_b.address,
        "Addresses should match across devices"
    );

    println!("✅ Cross-device recovery successful: Same address on both devices");
}

/// Test cross-device recovery with passphrase
#[test]
fn test_cross_device_recovery_with_passphrase() {
    let mnemonic_phrase = "legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth title";
    let passphrase = "my-secure-passphrase-2025";

    // Device A: Create wallet with passphrase
    let device_a = DeviceA::create_wallet(mnemonic_phrase, passphrase);

    // Device B: Recover with same passphrase
    let device_b = DeviceB::recover_wallet(mnemonic_phrase, passphrase);

    // Verify match
    assert_eq!(device_a.address, device_b.address);
    assert_eq!(
        device_a.private_key.to_bytes(),
        device_b.private_key.to_bytes()
    );

    println!("✅ Cross-device recovery with passphrase successful");
}

/// Test that wrong passphrase produces different wallet
#[test]
fn test_cross_device_recovery_wrong_passphrase() {
    let mnemonic_phrase = "letter advice cage absurd amount doctor acoustic avoid letter advice cage absurd amount doctor acoustic avoid letter advice cage absurd amount doctor acoustic bless";
    let correct_passphrase = "correct-passphrase";
    let wrong_passphrase = "wrong-passphrase";

    // Device A: Create with correct passphrase
    let device_a = DeviceA::create_wallet(mnemonic_phrase, correct_passphrase);

    // Device B: Recover with WRONG passphrase
    let device_b = DeviceB::recover_wallet(mnemonic_phrase, wrong_passphrase);

    // Keys should NOT match (security feature)
    assert_ne!(
        device_a.private_key.to_bytes(),
        device_b.private_key.to_bytes(),
        "Wrong passphrase should produce different keys"
    );

    assert_ne!(
        device_a.address, device_b.address,
        "Wrong passphrase should produce different address"
    );

    println!("✅ Wrong passphrase correctly produces different wallet");
}

/// Test multiple wallets from same mnemonic (with different passphrases)
#[test]
fn test_cross_device_multi_wallet() {
    let mnemonic_phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

    // Create multiple wallets with different passphrases
    let passphrases = vec!["", "wallet1", "wallet2", "wallet3"];
    let mut device_a_wallets = HashMap::new();

    for passphrase in &passphrases {
        let wallet = DeviceA::create_wallet(mnemonic_phrase, passphrase);
        device_a_wallets.insert(*passphrase, wallet);
    }

    // Recover all wallets on Device B
    for passphrase in &passphrases {
        let recovered = DeviceB::recover_wallet(mnemonic_phrase, passphrase);
        let original = device_a_wallets.get(passphrase).unwrap();

        assert_eq!(
            original.address, recovered.address,
            "Passphrase '{}' wallet should match", passphrase
        );
    }

    println!("✅ Multi-wallet recovery successful (4 wallets from 1 mnemonic)");
}

/// Test recovery across 10 simulated devices
#[test]
fn test_cross_device_recovery_10_devices() {
    let mnemonic_phrase = "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo vote";
    let passphrase = "";

    // Device 1: Create original wallet
    let device_1 = DeviceA::create_wallet(mnemonic_phrase, passphrase);
    let reference_address = device_1.address.clone();
    let reference_private_key = device_1.private_key.to_bytes();

    // Devices 2-10: Recover wallet
    for device_num in 2..=10 {
        let recovered = DeviceB::recover_wallet(mnemonic_phrase, passphrase);

        assert_eq!(
            recovered.address, reference_address,
            "Device {} address mismatch", device_num
        );
        assert_eq!(
            recovered.private_key.to_bytes(), reference_private_key,
            "Device {} private key mismatch", device_num
        );
    }

    println!("✅ 10-device recovery successful: All devices produced same wallet");
}

/// Test recovery after "lost device" scenario
#[test]
fn test_lost_device_recovery_scenario() {
    // User creates wallet on Phone
    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";
    let phone = DeviceA::create_wallet(mnemonic, "");

    println!("Phone wallet created: {}", phone.address);

    // Phone gets lost/broken
    // User buys new Laptop and recovers using written mnemonic
    let laptop = DeviceB::recover_wallet(mnemonic, "");

    println!("Laptop recovered wallet: {}", laptop.address);

    // Verify user can access same funds
    assert_eq!(
        phone.address, laptop.address,
        "User should have same wallet on laptop after recovery"
    );

    // User can now spend funds from laptop
    assert_eq!(
        phone.private_key.to_bytes(),
        laptop.private_key.to_bytes(),
        "Private key needed for spending should match"
    );

    println!("✅ Lost device recovery scenario successful");
}

/// Test recovery timing consistency
#[test]
fn test_recovery_timing_consistency() {
    let mnemonic = "legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth title";

    // Measure Device A creation time
    let start_a = std::time::Instant::now();
    let device_a = DeviceA::create_wallet(mnemonic, "");
    let time_a = start_a.elapsed();

    // Measure Device B recovery time
    let start_b = std::time::Instant::now();
    let device_b = DeviceB::recover_wallet(mnemonic, "");
    let time_b = start_b.elapsed();

    println!("Device A creation: {:?}", time_a);
    println!("Device B recovery: {:?}", time_b);

    // Both should be fast (< 100ms)
    assert!(time_a.as_millis() < 100, "Wallet creation too slow");
    assert!(time_b.as_millis() < 100, "Wallet recovery too slow");

    // Verify they match
    assert_eq!(device_a.address, device_b.address);

    println!("✅ Recovery timing consistent and fast");
}