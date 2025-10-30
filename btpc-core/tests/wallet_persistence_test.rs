//! Integration tests for wallet persistence
//!
//! This tests the full wallet save/load cycle with encryption.

use btpc_core::crypto::{
    EncryptedWallet, KeyEntry, PrivateKey, SecurePassword, WalletData,
};
use std::fs;
use tempfile::tempdir;

#[test]
fn test_wallet_persistence_full_cycle() {
    // Create temporary directory
    let temp_dir = tempdir().unwrap();
    let wallet_path = temp_dir.path().join("test_wallet.dat");

    // Create wallet with keys
    let private_key = PrivateKey::generate_ml_dsa().unwrap();
    let key_entry = KeyEntry::from_private_key(
        &private_key,
        "Main Address".to_string(),
        "btpc_test123".to_string(),
    );

    let wallet_data = WalletData {
        wallet_id: "test-wallet-001".to_string(),
        network: "testnet".to_string(),
        keys: vec![key_entry],
        created_at: 1234567890,
        modified_at: 1234567890,
    };

    let password = SecurePassword::new("super_secure_password_123".to_string());

    // Encrypt and save wallet
    let encrypted_wallet = EncryptedWallet::encrypt(&wallet_data, &password).unwrap();
    encrypted_wallet.save_to_file(&wallet_path).unwrap();

    // Verify file exists
    assert!(wallet_path.exists());
    let file_size = fs::metadata(&wallet_path).unwrap().len();
    assert!(file_size > 100, "Wallet file should contain encrypted data");

    // Load and decrypt wallet
    let loaded_wallet = EncryptedWallet::load_from_file(&wallet_path).unwrap();
    let decrypted_data = loaded_wallet.decrypt(&password).unwrap();

    // Verify data matches
    assert_eq!(decrypted_data.network, "testnet");
    assert_eq!(decrypted_data.keys.len(), 1);
    assert_eq!(decrypted_data.keys[0].label, "Main Address");
    assert_eq!(decrypted_data.keys[0].address, "btpc_test123");

    // Verify key bytes are preserved correctly
    let reconstructed_key = decrypted_data.keys[0].to_private_key().unwrap();
    let reconstructed_pubkey = decrypted_data.keys[0].to_public_key().unwrap();

    // Verify key bytes match
    assert_eq!(reconstructed_key.to_bytes(), private_key.to_bytes());
    assert_eq!(reconstructed_pubkey.to_bytes(), private_key.public_key().to_bytes());
}

#[test]
fn test_wallet_wrong_password_fails() {
    let temp_dir = tempdir().unwrap();
    let wallet_path = temp_dir.path().join("test_wallet.dat");

    let wallet_data = WalletData {
        wallet_id: "test-wallet-002".to_string(),
        network: "mainnet".to_string(),
        keys: vec![],
        created_at: 1111111111,
        modified_at: 1111111111,
    };

    let correct_password = SecurePassword::new("correct_password".to_string());
    let wrong_password = SecurePassword::new("wrong_password".to_string());

    // Encrypt with correct password
    let encrypted_wallet = EncryptedWallet::encrypt(&wallet_data, &correct_password).unwrap();
    encrypted_wallet.save_to_file(&wallet_path).unwrap();

    // Load and try to decrypt with wrong password
    let loaded_wallet = EncryptedWallet::load_from_file(&wallet_path).unwrap();
    let result = loaded_wallet.decrypt(&wrong_password);

    assert!(result.is_err(), "Decryption should fail with wrong password");
}

#[test]
fn test_wallet_file_format() {
    let temp_dir = tempdir().unwrap();
    let wallet_path = temp_dir.path().join("test_wallet.dat");

    let wallet_data = WalletData {
        wallet_id: "test-wallet-003".to_string(),
        network: "regtest".to_string(),
        keys: vec![],
        created_at: 9999999999,
        modified_at: 9999999999,
    };

    let password = SecurePassword::new("test_password".to_string());

    // Save wallet
    let encrypted_wallet = EncryptedWallet::encrypt(&wallet_data, &password).unwrap();
    encrypted_wallet.save_to_file(&wallet_path).unwrap();

    // Read raw file and verify magic bytes
    let file_contents = fs::read(&wallet_path).unwrap();
    assert_eq!(&file_contents[0..4], b"BTPC", "File should start with BTPC magic bytes");

    // Verify version (bytes 4-7, little-endian u32)
    let version = u32::from_le_bytes([
        file_contents[4],
        file_contents[5],
        file_contents[6],
        file_contents[7],
    ]);
    assert_eq!(version, 1, "Wallet version should be 1");
}

#[test]
fn test_wallet_multiple_keys() {
    let temp_dir = tempdir().unwrap();
    let wallet_path = temp_dir.path().join("test_wallet.dat");

    // Create multiple keys
    let key1 = PrivateKey::generate_ml_dsa().unwrap();
    let key2 = PrivateKey::generate_ml_dsa().unwrap();
    let key3 = PrivateKey::generate_ml_dsa().unwrap();

    let keys = vec![
        KeyEntry::from_private_key(&key1, "Address 1".to_string(), "btpc_addr1".to_string()),
        KeyEntry::from_private_key(&key2, "Address 2".to_string(), "btpc_addr2".to_string()),
        KeyEntry::from_private_key(&key3, "Address 3".to_string(), "btpc_addr3".to_string()),
    ];

    let wallet_data = WalletData {
        wallet_id: "test-wallet-004".to_string(),
        network: "testnet".to_string(),
        keys,
        created_at: 5555555555,
        modified_at: 5555555555,
    };

    let password = SecurePassword::new("multi_key_password".to_string());

    // Save and load
    let encrypted_wallet = EncryptedWallet::encrypt(&wallet_data, &password).unwrap();
    encrypted_wallet.save_to_file(&wallet_path).unwrap();

    let loaded_wallet = EncryptedWallet::load_from_file(&wallet_path).unwrap();
    let decrypted_data = loaded_wallet.decrypt(&password).unwrap();

    // Verify all keys are present
    assert_eq!(decrypted_data.keys.len(), 3);
    assert_eq!(decrypted_data.keys[0].label, "Address 1");
    assert_eq!(decrypted_data.keys[1].label, "Address 2");
    assert_eq!(decrypted_data.keys[2].label, "Address 3");

    // Verify each key can be reconstructed
    for key_entry in decrypted_data.keys.iter() {
        let reconstructed_key = key_entry.to_private_key().unwrap();
        let reconstructed_pubkey = key_entry.to_public_key().unwrap();

        // Verify key bytes are preserved
        assert_eq!(reconstructed_key.size(), 4000); // ML-DSA private key size
        assert_eq!(reconstructed_pubkey.size(), 1952); // ML-DSA public key size
    }
}

#[test]
fn test_wallet_update_preserves_data() {
    let temp_dir = tempdir().unwrap();
    let wallet_path = temp_dir.path().join("test_wallet.dat");
    let password = SecurePassword::new("update_test_password".to_string());

    // Create initial wallet with one key
    let key1 = PrivateKey::generate_ml_dsa().unwrap();
    let initial_data = WalletData {
        wallet_id: "test-wallet-005".to_string(),
        network: "mainnet".to_string(),
        keys: vec![KeyEntry::from_private_key(
            &key1,
            "Initial Key".to_string(),
            "btpc_initial".to_string(),
        )],
        created_at: 1000000000,
        modified_at: 1000000000,
    };

    // Save initial wallet
    let encrypted = EncryptedWallet::encrypt(&initial_data, &password).unwrap();
    encrypted.save_to_file(&wallet_path).unwrap();

    // Load, add another key, and save again
    let loaded = EncryptedWallet::load_from_file(&wallet_path).unwrap();
    let mut updated_data = loaded.decrypt(&password).unwrap();

    let key2 = PrivateKey::generate_ml_dsa().unwrap();
    updated_data.keys.push(KeyEntry::from_private_key(
        &key2,
        "New Key".to_string(),
        "btpc_new".to_string(),
    ));
    updated_data.modified_at = 2000000000;

    // Save updated wallet
    let encrypted_updated = EncryptedWallet::encrypt(&updated_data, &password).unwrap();
    encrypted_updated.save_to_file(&wallet_path).unwrap();

    // Load final wallet and verify both keys are present
    let final_loaded = EncryptedWallet::load_from_file(&wallet_path).unwrap();
    let final_data = final_loaded.decrypt(&password).unwrap();

    assert_eq!(final_data.keys.len(), 2);
    assert_eq!(final_data.keys[0].label, "Initial Key");
    assert_eq!(final_data.keys[1].label, "New Key");
    assert_eq!(final_data.created_at, 1000000000);
    assert_eq!(final_data.modified_at, 2000000000);
}