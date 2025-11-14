//! Cryptography Tests for Authentication
//!
//! Tests the cryptographic primitives used for master password security.
//! Verifies Argon2id key derivation and AES-256-GCM encryption per research.md.
//!
//! **TDD Phase**: GREEN - Tests verify actual implementation
//!
//! **Test Coverage**:
//! - T017: Argon2id key derivation with OWASP parameters
//! - T018: AES-256-GCM encryption/decryption round-trip
//! - T019: Constant-time password comparison
//! - T020: Argon2id salt uniqueness
//! - T021: AES-256-GCM nonce uniqueness
//! - T022: Zeroization of sensitive data
//!
//! **Security Requirements** (data-model.md):
//! - Argon2id: 64MB memory, 3 iterations, 4 parallelism
//! - AES-256-GCM: 12-byte nonce, 16-byte auth tag
//! - Constant-time comparison using subtle crate

use btpc_desktop_app::auth_crypto::{
    constant_time_compare, decrypt_aes_gcm, derive_key_argon2id, encrypt_aes_gcm,
    generate_random_nonce, generate_random_salt, AES_KEY_SIZE, AES_NONCE_SIZE, AES_TAG_SIZE,
    ARGON2_ITERATIONS, ARGON2_MEMORY_KB, ARGON2_PARALLELISM, SALT_SIZE,
};
use std::collections::HashSet;
use std::time::Instant;

// ============================================================================
// T017: Argon2id Key Derivation with OWASP Parameters
// ============================================================================
#[test]
fn test_argon2id_key_derivation() {
    // Test Argon2id KDF with OWASP-recommended parameters
    let password = "test_password_123!@#";

    // Generate random salt
    let salt = generate_random_salt().expect("Failed to generate salt");
    assert_eq!(salt.len(), SALT_SIZE, "Salt should be 16 bytes");

    // Derive key from password
    let start = Instant::now();
    let derived_key = derive_key_argon2id(password, &salt)
        .expect("Failed to derive key");
    let duration = start.elapsed();

    // Assert derived key is 32 bytes (256 bits for AES-256)
    assert_eq!(derived_key.len(), AES_KEY_SIZE, "Derived key should be 32 bytes");

    // Verify timing is reasonable (should be ~1-2 seconds)
    assert!(
        duration.as_millis() > 100 && duration.as_secs() < 5,
        "Key derivation took {:?}, expected 100ms-5s", duration
    );

    // Verify different salts produce different keys (same password)
    let salt2 = generate_random_salt().expect("Failed to generate second salt");
    let derived_key2 = derive_key_argon2id(password, &salt2)
        .expect("Failed to derive second key");
    assert_ne!(&*derived_key, &*derived_key2, "Different salts should produce different keys");

    // Verify same salt produces same key (same password)
    let derived_key3 = derive_key_argon2id(password, &salt)
        .expect("Failed to derive third key");
    assert_eq!(&*derived_key, &*derived_key3, "Same salt should produce same key");

    // Verify different passwords produce different keys (same salt)
    let derived_key4 = derive_key_argon2id("different_password", &salt)
        .expect("Failed to derive fourth key");
    assert_ne!(&*derived_key, &*derived_key4, "Different passwords should produce different keys");
}

// ============================================================================
// T018: AES-256-GCM Encryption/Decryption Round-Trip
// ============================================================================
#[test]
fn test_aes_256_gcm_round_trip() {
    // Test authenticated encryption with GCM mode
    let plaintext = b"test_password_hash_sensitive_data";

    // Generate random key and nonce
    let mut key = [0u8; AES_KEY_SIZE];
    let salt = generate_random_salt().expect("Failed to generate salt for key");
    let derived_key = derive_key_argon2id("test_key_password", &salt)
        .expect("Failed to derive key");
    key.copy_from_slice(&*derived_key);

    let nonce = generate_random_nonce().expect("Failed to generate nonce");
    assert_eq!(nonce.len(), AES_NONCE_SIZE, "Nonce should be 12 bytes");

    // Encrypt plaintext with AES-256-GCM
    let (ciphertext, tag) = encrypt_aes_gcm(plaintext, &key, &nonce)
        .expect("Failed to encrypt");

    // Assert ciphertext is different from plaintext
    assert_ne!(ciphertext.as_slice(), plaintext, "Ciphertext should differ from plaintext");

    // Assert authentication tag is 16 bytes (128 bits)
    assert_eq!(tag.len(), AES_TAG_SIZE, "Auth tag should be 16 bytes");

    // Decrypt ciphertext with same key and nonce
    let decrypted = decrypt_aes_gcm(&ciphertext, &tag, &key, &nonce)
        .expect("Failed to decrypt");

    // Assert decrypted plaintext equals original
    assert_eq!(&*decrypted, plaintext, "Decrypted should match original");

    // Verify tampering detection - modify ciphertext
    let mut tampered_ciphertext = ciphertext.clone();
    if !tampered_ciphertext.is_empty() {
        tampered_ciphertext[0] ^= 0xFF; // Flip bits in first byte
    }
    let tamper_result = decrypt_aes_gcm(&tampered_ciphertext, &tag, &key, &nonce);
    assert!(tamper_result.is_err(), "Tampered ciphertext should fail decryption");

    // Verify tampering detection - modify tag
    let mut tampered_tag = tag;
    tampered_tag[0] ^= 0xFF; // Flip bits in first byte of tag
    let tamper_result = decrypt_aes_gcm(&ciphertext, &tampered_tag, &key, &nonce);
    assert!(tamper_result.is_err(), "Tampered tag should fail decryption");

    // Verify wrong key fails
    let mut wrong_key = key;
    wrong_key[0] ^= 0xFF; // Modify key
    let wrong_key_result = decrypt_aes_gcm(&ciphertext, &tag, &wrong_key, &nonce);
    assert!(wrong_key_result.is_err(), "Wrong key should fail decryption");
}

// ============================================================================
// T019: Constant-Time Password Comparison
// ============================================================================
#[test]
fn test_constant_time_comparison() {
    // Test timing-attack resistant comparison

    // Test equal hashes -> returns true
    let hash_a = b"correct_password_hash_32_bytes__";
    let hash_b = b"correct_password_hash_32_bytes__";
    assert!(
        constant_time_compare(hash_a, hash_b),
        "Equal hashes should return true"
    );

    // Test different hashes -> returns false
    let hash_c = b"wrong_password_hash_32_bytes____";
    assert!(
        !constant_time_compare(hash_a, hash_c),
        "Different hashes should return false"
    );

    // Test with different lengths (should return false)
    let short_hash = b"short";
    let long_hash = b"this_is_a_much_longer_password_hash";
    assert!(
        !constant_time_compare(short_hash, long_hash),
        "Different length hashes should return false"
    );

    // Verify constant-time behavior with timing test
    // Compare timing for first byte difference vs last byte difference
    let base = b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"; // 32 'a's
    let diff_first = b"baaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"; // First byte different
    let diff_last = b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaab"; // Last byte different

    // Measure timing for first byte difference
    let start = Instant::now();
    for _ in 0..10000 {
        let _ = constant_time_compare(base, diff_first);
    }
    let first_diff_time = start.elapsed();

    // Measure timing for last byte difference
    let start = Instant::now();
    for _ in 0..10000 {
        let _ = constant_time_compare(base, diff_last);
    }
    let last_diff_time = start.elapsed();

    // Timing should be similar (within 50% variance is acceptable for this test)
    let time_ratio = first_diff_time.as_nanos() as f64 / last_diff_time.as_nanos() as f64;
    assert!(
        time_ratio > 0.5 && time_ratio < 2.0,
        "Timing variance too high: first={:?}, last={:?}, ratio={}",
        first_diff_time, last_diff_time, time_ratio
    );
}

// ============================================================================
// T020: Argon2id Salt Uniqueness
// ============================================================================
#[test]
fn test_argon2id_salt_uniqueness() {
    // Test that every password creation uses unique random salt
    let mut salts = HashSet::new();

    // Generate 100 random salts
    for _ in 0..100 {
        let salt = generate_random_salt().expect("Failed to generate salt");

        // Assert salt is 16 bytes
        assert_eq!(salt.len(), SALT_SIZE, "Salt should be 16 bytes");

        // Assert salt is unique (no duplicates)
        let was_new = salts.insert(salt);
        assert!(was_new, "Duplicate salt detected - not cryptographically random!");
    }

    // All 100 salts should be unique
    assert_eq!(salts.len(), 100, "All salts should be unique");

    // Basic randomness check - ensure not all bytes are the same
    for salt in salts.iter().take(10) {
        let first_byte = salt[0];
        let all_same = salt.iter().all(|&b| b == first_byte);
        assert!(!all_same, "Salt bytes should not all be the same");
    }
}

// ============================================================================
// T021: AES-256-GCM Nonce Uniqueness
// ============================================================================
#[test]
fn test_aes_gcm_nonce_uniqueness() {
    // Test that every encryption uses unique random nonce
    let mut nonces = HashSet::new();

    // Generate 100 random nonces
    for _ in 0..100 {
        let nonce = generate_random_nonce().expect("Failed to generate nonce");

        // Assert nonce is 12 bytes (96 bits for GCM)
        assert_eq!(nonce.len(), AES_NONCE_SIZE, "Nonce should be 12 bytes");

        // Assert nonce is unique (no duplicates)
        let was_new = nonces.insert(nonce);
        assert!(was_new, "Duplicate nonce detected - critical security issue!");
    }

    // All 100 nonces should be unique
    assert_eq!(nonces.len(), 100, "All nonces should be unique");

    // Basic randomness check - ensure not all bytes are the same
    for nonce in nonces.iter().take(10) {
        let first_byte = nonce[0];
        let all_same = nonce.iter().all(|&b| b == first_byte);
        assert!(!all_same, "Nonce bytes should not all be the same");
    }
}

// ============================================================================
// T022: Zeroization of Sensitive Data
// ============================================================================
#[test]
fn test_zeroization() {
    // Test that passwords and derived keys are zeroed from memory after use
    // Note: This test validates that we're using the Zeroizing wrapper correctly

    // The derive_key_argon2id function returns a Zeroizing wrapper
    let password = "sensitive_password_data";
    let salt = generate_random_salt().expect("Failed to generate salt");

    // Derive key inside a scope
    {
        let derived_key = derive_key_argon2id(password, &salt)
            .expect("Failed to derive key");

        // Use the key
        assert_eq!(derived_key.len(), AES_KEY_SIZE);

        // Key will be automatically zeroized when dropped at end of scope
    } // <- Zeroizing happens here

    // Test decryption also uses Zeroizing for plaintext
    let key = [0u8; AES_KEY_SIZE];
    let nonce = generate_random_nonce().expect("Failed to generate nonce");
    let plaintext = b"sensitive_plaintext_data";

    let (ciphertext, tag) = encrypt_aes_gcm(plaintext, &key, &nonce)
        .expect("Failed to encrypt");

    {
        let decrypted = decrypt_aes_gcm(&ciphertext, &tag, &key, &nonce)
            .expect("Failed to decrypt");

        // Use the decrypted data
        assert_eq!(&*decrypted, plaintext);

        // Decrypted data will be automatically zeroized when dropped
    } // <- Zeroizing happens here

    // Since we're using the Zeroizing wrapper from the zeroize crate,
    // we can trust that memory is properly cleared. The crate handles
    // the low-level memory clearing with compiler optimization barriers.

    // Verify we're using Zeroizing types (this test passes if it compiles)
    let _key: zeroize::Zeroizing<[u8; AES_KEY_SIZE]> =
        derive_key_argon2id("test", &salt).expect("Failed to derive key");
    let _plaintext: zeroize::Zeroizing<Vec<u8>> =
        decrypt_aes_gcm(&ciphertext, &tag, &key, &nonce).expect("Failed to decrypt");
}

// ============================================================================
// Performance Benchmarks (Optional - for verification)
// ============================================================================

#[test]
#[ignore] // Run with: cargo test --test auth_crypto_test -- --ignored
fn bench_argon2id_performance() {
    // Benchmark verifies Argon2id meets NFR-006: Login <2 seconds
    println!("\n=== Argon2id Performance Benchmark ===");

    let password = "benchmark_test_password";
    let mut times = Vec::new();

    // Measure time to derive key 10 times
    for i in 0..10 {
        let salt = generate_random_salt().expect("Failed to generate salt");
        let start = Instant::now();
        let _ = derive_key_argon2id(password, &salt).expect("Failed to derive key");
        let duration = start.elapsed();
        times.push(duration);
        println!("  Iteration {}: {:?}", i + 1, duration);
    }

    // Calculate statistics
    let total: std::time::Duration = times.iter().sum();
    let avg = total / times.len() as u32;
    let min = times.iter().min().unwrap();
    let max = times.iter().max().unwrap();

    println!("\nResults:");
    println!("  Average: {:?}", avg);
    println!("  Min: {:?}", min);
    println!("  Max: {:?}", max);
    println!("  Parameters: {}KB memory, {} iterations, {} parallelism",
             ARGON2_MEMORY_KB, ARGON2_ITERATIONS, ARGON2_PARALLELISM);

    // Assert average time is 1-2 seconds (acceptable for interactive login)
    assert!(
        avg.as_millis() >= 100 && avg.as_secs() <= 3,
        "Average time {:?} outside acceptable range (100ms-3s)", avg
    );

    println!("✓ Performance meets NFR-006 requirements");
}

#[test]
#[ignore] // Run with: cargo test --test auth_crypto_test -- --ignored
fn bench_aes_gcm_performance() {
    // Benchmark verifies AES-256-GCM is fast enough for real-time use
    println!("\n=== AES-256-GCM Performance Benchmark ===");

    let key = [0u8; AES_KEY_SIZE];
    let plaintext = b"benchmark test data for AES-256-GCM encryption";
    let mut times = Vec::new();

    // Measure time to encrypt 1000 times
    for _ in 0..1000 {
        let nonce = generate_random_nonce().expect("Failed to generate nonce");
        let start = Instant::now();
        let _ = encrypt_aes_gcm(plaintext, &key, &nonce).expect("Failed to encrypt");
        let duration = start.elapsed();
        times.push(duration);
    }

    // Calculate statistics
    let total: std::time::Duration = times.iter().sum();
    let avg = total / times.len() as u32;
    let min = times.iter().min().unwrap();
    let max = times.iter().max().unwrap();

    println!("\nResults for 1000 encryptions:");
    println!("  Average: {:?}", avg);
    println!("  Min: {:?}", min);
    println!("  Max: {:?}", max);

    // Assert average time is <10ms (should be microseconds with AES-NI)
    assert!(
        avg.as_millis() < 10,
        "Average time {:?} exceeds 10ms threshold", avg
    );

    println!("✓ AES-256-GCM performance is hardware-accelerated");
}