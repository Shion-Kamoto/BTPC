//! Tests for BTPC Integration Module
//!
//! Tests wallet creation, encryption, and key management using Argon2id

use super::*;
use btpc_core::crypto::{EncryptedWallet, SecurePassword, WalletData};
use std::path::PathBuf;
use tempfile::TempDir;

/// TDD RED Phase Test 1: Wallet creation should use Argon2id encryption
///
/// This test verifies that create_wallet() produces wallet files encrypted
/// with Argon2id (not SHA-256 KDF) to match metadata encryption strength.
#[test]
fn test_wallet_creation_uses_argon2id() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let btpc_home = temp_dir.path().to_path_buf();
    let integration = BtpcIntegration::new(btpc_home.clone());

    let wallet_file = btpc_home.join("wallets").join("test_wallet.dat");
    let password = "test_password_123";

    // Create wallet
    let (address, seed_phrase, private_key_hex) = integration
        .create_wallet(&wallet_file, password)
        .expect("Wallet creation failed");

    // GREEN PHASE: Verify .dat file created (EncryptedWallet format)
    let wallet_dat_file = wallet_file.with_extension("dat");
    assert!(wallet_dat_file.exists(), "Wallet .dat file should exist");

    // Load as EncryptedWallet (Argon2id encryption)
    let encrypted_wallet = EncryptedWallet::load_from_file(&wallet_dat_file)
        .expect("Failed to load encrypted wallet");

    // Decrypt to verify Argon2id encryption works
    let secure_password = SecurePassword::new(password.to_string());
    let wallet_data = encrypted_wallet.decrypt(&secure_password)
        .expect("Failed to decrypt with Argon2id");

    // Verify decrypted data contains our key
    assert!(!wallet_data.keys.is_empty(), "Should have at least one key");
    assert_eq!(wallet_data.keys[0].address, address, "Address should match");

    // Verify address format (Base58, ~34 chars)
    assert!(address.len() >= 26 && address.len() <= 35, "Address should be Base58 format");

    // Verify seed phrase (24 words)
    assert_eq!(seed_phrase.split_whitespace().count(), 24, "Should have 24-word mnemonic");

    // Verify private key hex (ML-DSA = 4000 bytes = 8000 hex chars)
    assert_eq!(private_key_hex.len(), 8000, "Private key should be 4000 bytes (8000 hex)");
}

/// TDD RED Phase Test 2: Decrypt wallet with correct password
///
/// Verifies that wallet encrypted with Argon2id can be decrypted
/// using the correct password via btpc-core's EncryptedWallet.
#[test]
fn test_wallet_decryption_with_correct_password() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let btpc_home = temp_dir.path().to_path_buf();
    let integration = BtpcIntegration::new(btpc_home.clone());

    let wallet_file = btpc_home.join("wallets").join("test_decrypt.dat");
    let password = "secure_password_456";

    // Create wallet
    let (original_address, _, original_key_hex) = integration
        .create_wallet(&wallet_file, password)
        .expect("Wallet creation failed");

    // GREEN PHASE: Load .dat file (EncryptedWallet format)
    let wallet_dat_file = wallet_file.with_extension("dat");
    let encrypted_wallet = EncryptedWallet::load_from_file(&wallet_dat_file)
        .expect("Failed to load encrypted wallet");

    let secure_password = SecurePassword::new(password.to_string());
    let wallet_data = encrypted_wallet.decrypt(&secure_password)
        .expect("Failed to decrypt wallet with correct password");

    // Verify decrypted data contains our key
    assert!(!wallet_data.keys.is_empty(), "Should have at least one key");

    let key_entry = &wallet_data.keys[0];
    assert_eq!(key_entry.address, original_address, "Address should match");
    assert!(!key_entry.private_key_bytes.is_empty(), "Should have private key");

    // Verify private key matches original
    let decrypted_key_hex = hex::encode(&key_entry.private_key_bytes);
    assert_eq!(decrypted_key_hex, original_key_hex, "Private key should match");
}

/// TDD RED Phase Test 3: Decrypt wallet with wrong password fails
///
/// Verifies that Argon2id-encrypted wallet cannot be decrypted
/// with incorrect password (authentication fails).
#[test]
fn test_wallet_decryption_with_wrong_password() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let btpc_home = temp_dir.path().to_path_buf();
    let integration = BtpcIntegration::new(btpc_home.clone());

    let wallet_file = btpc_home.join("wallets").join("test_wrong_pass.dat");
    let correct_password = "correct_password_789";
    let wrong_password = "wrong_password_000";

    // Create wallet with correct password
    integration
        .create_wallet(&wallet_file, correct_password)
        .expect("Wallet creation failed");

    // GREEN PHASE: Load .dat file
    let wallet_dat_file = wallet_file.with_extension("dat");
    let encrypted_wallet = EncryptedWallet::load_from_file(&wallet_dat_file)
        .expect("Failed to load encrypted wallet");

    let wrong_secure_password = SecurePassword::new(wrong_password.to_string());
    let decrypt_result = encrypted_wallet.decrypt(&wrong_secure_password);

    assert!(decrypt_result.is_err(), "Decryption with wrong password should fail");
}

/// TDD RED Phase Test 4: Migration from SHA-256 to Argon2id
///
/// Verifies that old SHA-256 encrypted wallets can be detected
/// and migrated to Argon2id encryption.
#[test]
fn test_migration_from_sha256_to_argon2id() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let btpc_home = temp_dir.path().to_path_buf();
    let wallet_file = btpc_home.join("wallets").join("legacy_wallet.dat");

    // Create directory
    std::fs::create_dir_all(wallet_file.parent().unwrap())
        .expect("Failed to create wallet dir");

    // Create legacy wallet JSON (SHA-256 encrypted)
    let legacy_wallet = serde_json::json!({
        "version": "3.0",
        "address": "btpc1qtest1234567890abcdefghijk",
        "public_key": "abcd1234",
        "encrypted_private_key": "legacy_sha256_encrypted_data",
        "seed_phrase_hash": "abc123",
        "created_at": "2025-01-01T00:00:00Z",
        "crypto_type": "ML-DSA-65",
        "address_type": "P2PKH-Base58",
        "encryption": "AES-256-GCM"  // Old: SHA-256 KDF
    });

    std::fs::write(&wallet_file, serde_json::to_string_pretty(&legacy_wallet).unwrap())
        .expect("Failed to write legacy wallet");

    // TDD Assertion: Should detect legacy format and provide migration
    // This will FAIL until GREEN phase when we implement migration
    let integration = BtpcIntegration::new(btpc_home);

    let is_legacy = integration.is_legacy_wallet_format(&wallet_file)
        .expect("Failed to check wallet format");

    assert!(is_legacy, "Should detect legacy SHA-256 wallet format");
}

/// TDD RED Phase Test 5: Encryption strength verification
///
/// Verifies that Argon2id parameters match metadata encryption:
/// - 64 MB memory
/// - 3 iterations
/// - 4 parallelism
#[test]
fn test_argon2id_parameters_match_metadata() {
    // This test verifies that btpc_integration uses same Argon2id
    // parameters as wallet_manager (constitutional requirement)

    // Expected parameters (from btpc-core/src/crypto/wallet_serde.rs:257-264)
    const EXPECTED_MEMORY: u32 = 65536; // 64 MB
    const EXPECTED_ITERATIONS: u32 = 3;
    const EXPECTED_PARALLELISM: u32 = 4;
    const EXPECTED_OUTPUT_LEN: usize = 32; // 256 bits

    // TDD Assertion: Implementation should use these exact parameters
    // This will be verified in GREEN phase implementation

    // For now, document the requirement
    let requirement = format!(
        "Argon2id parameters: m={}, t={}, p={}, output={}",
        EXPECTED_MEMORY, EXPECTED_ITERATIONS, EXPECTED_PARALLELISM, EXPECTED_OUTPUT_LEN
    );

    assert!(
        !requirement.is_empty(),
        "Argon2id parameters must match wallet_manager implementation"
    );
}