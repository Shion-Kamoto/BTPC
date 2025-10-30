//! Cryptography Functions for Authentication
//!
//! This module implements the cryptographic functions for master password security.
//! It follows OWASP recommendations and NIST standards per research.md.
//!
//! # Cryptographic Primitives
//!
//! - **Argon2id**: Password key derivation (64MB, 3 iterations, 4 parallelism)
//! - **AES-256-GCM**: Authenticated encryption (AEAD) for credential storage
//! - **Constant-time comparison**: Timing attack prevention using subtle crate
//! - **Zeroization**: Secure memory clearing for sensitive data
//!
//! # Security Properties
//!
//! - Memory-hard KDF resistant to GPU/ASIC attacks
//! - Authenticated encryption prevents tampering
//! - Constant-time operations prevent timing attacks
//! - Cryptographically secure random number generation
//!
//! # Performance
//!
//! - Argon2id: ~1-2 seconds (acceptable for interactive login per NFR-006)
//! - AES-256-GCM: <10ms (hardware-accelerated on modern CPUs)

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2, ParamsBuilder, Version,
};
use rand::{rngs::OsRng, RngCore};
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

/// OWASP recommended Argon2id parameters for interactive login
/// Memory: 64 MB (65536 KB)
/// Iterations: 3
/// Parallelism: 4
pub const ARGON2_MEMORY_KB: u32 = 65536;
pub const ARGON2_ITERATIONS: u32 = 3;
pub const ARGON2_PARALLELISM: u32 = 4;

/// AES-256-GCM parameters
pub const AES_KEY_SIZE: usize = 32; // 256 bits
pub const AES_NONCE_SIZE: usize = 12; // 96 bits (recommended for GCM)
pub const AES_TAG_SIZE: usize = 16; // 128 bits

/// Salt size for Argon2id
pub const SALT_SIZE: usize = 16; // 128 bits

/// Error type for cryptography operations
#[derive(Debug)]
pub enum CryptoError {
    Argon2Error(String),
    AesGcmError(String),
    RandomGenerationError(String),
}

impl std::fmt::Display for CryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CryptoError::Argon2Error(msg) => write!(f, "Argon2 error: {}", msg),
            CryptoError::AesGcmError(msg) => write!(f, "AES-GCM error: {}", msg),
            CryptoError::RandomGenerationError(msg) => write!(f, "Random generation error: {}", msg),
        }
    }
}

impl std::error::Error for CryptoError {}

// ============================================================================
// T028: Argon2id Key Derivation
// ============================================================================

/// Derives a 32-byte key from a password using Argon2id with OWASP parameters
///
/// # Security
/// - Memory-hard: 64 MB memory cost (resistant to GPU/ASIC attacks)
/// - Time-cost: 3 iterations (balanced security/performance)
/// - Parallelism: 4 threads (optimal for modern CPUs)
///
/// # Performance
/// Expected: ~1-2 seconds on modern hardware (acceptable per NFR-006)
///
/// # Arguments
/// * `password` - User password (zeroized after use)
/// * `salt` - 16-byte cryptographically random salt
///
/// # Returns
/// 32-byte derived key (zeroized after use via Zeroizing wrapper)
pub fn derive_key_argon2id(password: &str, salt: &[u8; SALT_SIZE]) -> Result<Zeroizing<[u8; AES_KEY_SIZE]>, CryptoError> {
    // Configure Argon2id with OWASP parameters
    let params = ParamsBuilder::new()
        .m_cost(ARGON2_MEMORY_KB)
        .t_cost(ARGON2_ITERATIONS)
        .p_cost(ARGON2_PARALLELISM)
        .output_len(AES_KEY_SIZE)
        .build()
        .map_err(|e| CryptoError::Argon2Error(format!("Failed to build Argon2 params: {}", e)))?;

    let argon2 = Argon2::new(
        argon2::Algorithm::Argon2id,
        Version::V0x13,
        params,
    );

    // Wrap password in Zeroizing for automatic cleanup
    let password_zeroizing = Zeroizing::new(password.as_bytes().to_vec());

    // Create SaltString from raw salt bytes
    let salt_string = SaltString::encode_b64(salt)
        .map_err(|e| CryptoError::Argon2Error(format!("Failed to encode salt: {}", e)))?;

    // Derive the key using Argon2id
    let password_hash = argon2
        .hash_password(&password_zeroizing, &salt_string)
        .map_err(|e| CryptoError::Argon2Error(format!("Failed to hash password: {}", e)))?;

    // Extract the raw hash bytes (32 bytes for output_len=32)
    let hash_bytes = password_hash.hash
        .ok_or_else(|| CryptoError::Argon2Error("No hash output".to_string()))?;

    let mut derived_key = [0u8; AES_KEY_SIZE];
    derived_key.copy_from_slice(hash_bytes.as_bytes());

    Ok(Zeroizing::new(derived_key))
}

// ============================================================================
// T029: Generate Cryptographically Random Salt
// ============================================================================

/// Generates a cryptographically secure random 16-byte salt
///
/// # Security
/// Uses OS-provided CSPRNG (ChaCha20 on Linux, CryptGenRandom on Windows)
/// Prevents rainbow table attacks and parallel cracking
///
/// # Returns
/// 16-byte random salt
pub fn generate_random_salt() -> Result<[u8; SALT_SIZE], CryptoError> {
    let mut salt = [0u8; SALT_SIZE];
    OsRng.fill_bytes(&mut salt);
    Ok(salt)
}

// ============================================================================
// T030: AES-256-GCM Encryption
// ============================================================================

/// Encrypts plaintext using AES-256-GCM authenticated encryption
///
/// # Security
/// - AEAD (Authenticated Encryption with Associated Data)
/// - Tamper detection via 16-byte authentication tag
/// - Hardware-accelerated on modern CPUs (AES-NI)
///
/// # Performance
/// Expected: <10ms per encryption operation
///
/// # Arguments
/// * `plaintext` - Data to encrypt
/// * `key` - 32-byte AES-256 key
/// * `nonce` - 12-byte unique nonce (MUST be unique per encryption with same key)
///
/// # Returns
/// Tuple of (ciphertext, 16-byte authentication tag)
pub fn encrypt_aes_gcm(
    plaintext: &[u8],
    key: &[u8; AES_KEY_SIZE],
    nonce: &[u8; AES_NONCE_SIZE],
) -> Result<(Vec<u8>, [u8; AES_TAG_SIZE]), CryptoError> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let nonce_obj = Nonce::from_slice(nonce);

    let ciphertext_with_tag = cipher
        .encrypt(nonce_obj, plaintext)
        .map_err(|e| CryptoError::AesGcmError(format!("Encryption failed: {}", e)))?;

    // AES-GCM returns ciphertext || tag (tag is last 16 bytes)
    let tag_start = ciphertext_with_tag.len().saturating_sub(AES_TAG_SIZE);
    let mut tag = [0u8; AES_TAG_SIZE];
    tag.copy_from_slice(&ciphertext_with_tag[tag_start..]);

    let ciphertext = ciphertext_with_tag[..tag_start].to_vec();

    Ok((ciphertext, tag))
}

// ============================================================================
// T031: AES-256-GCM Decryption
// ============================================================================

/// Decrypts ciphertext using AES-256-GCM authenticated encryption
///
/// # Security
/// - Verifies authentication tag before decryption (prevents tampering)
/// - Returns error if tag verification fails
///
/// # Arguments
/// * `ciphertext` - Encrypted data
/// * `tag` - 16-byte authentication tag
/// * `key` - 32-byte AES-256 key
/// * `nonce` - 12-byte nonce (same as used for encryption)
///
/// # Returns
/// Decrypted plaintext (zeroized after use)
pub fn decrypt_aes_gcm(
    ciphertext: &[u8],
    tag: &[u8; AES_TAG_SIZE],
    key: &[u8; AES_KEY_SIZE],
    nonce: &[u8; AES_NONCE_SIZE],
) -> Result<Zeroizing<Vec<u8>>, CryptoError> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let nonce_obj = Nonce::from_slice(nonce);

    // Reconstruct ciphertext || tag format that aes_gcm expects
    let mut ciphertext_with_tag = ciphertext.to_vec();
    ciphertext_with_tag.extend_from_slice(tag);

    let plaintext = cipher
        .decrypt(nonce_obj, ciphertext_with_tag.as_ref())
        .map_err(|e| CryptoError::AesGcmError(format!("Decryption failed (tampering detected or wrong key): {}", e)))?;

    Ok(Zeroizing::new(plaintext))
}

// ============================================================================
// T032: Generate Cryptographically Random Nonce
// ============================================================================

/// Generates a cryptographically secure random 12-byte nonce for AES-GCM
///
/// # Security
/// - CRITICAL: Nonce MUST be unique for each encryption with the same key
/// - Nonce reuse breaks GCM confidentiality and authenticity
/// - Uses OS-provided CSPRNG
///
/// # Returns
/// 12-byte random nonce
pub fn generate_random_nonce() -> Result<[u8; AES_NONCE_SIZE], CryptoError> {
    let mut nonce = [0u8; AES_NONCE_SIZE];
    OsRng.fill_bytes(&mut nonce);
    Ok(nonce)
}

// ============================================================================
// T033: Constant-Time Comparison
// ============================================================================

/// Compares two byte slices in constant time to prevent timing attacks
///
/// # Security
/// - Uses subtle crate for constant-time equality comparison
/// - Prevents attackers from learning password/hash byte-by-byte via timing
/// - Timing is independent of:
///   - Position of first difference
///   - Number of matching bytes
///   - Length difference (if same length expected)
///
/// # Arguments
/// * `a` - First byte slice
/// * `b` - Second byte slice
///
/// # Returns
/// true if slices are equal, false otherwise (in constant time)
pub fn constant_time_compare(a: &[u8], b: &[u8]) -> bool {
    // If lengths differ, return false (still constant time for same-length comparisons)
    if a.len() != b.len() {
        return false;
    }

    // Use subtle crate for constant-time comparison
    a.ct_eq(b).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_salt_generation() {
        let salt1 = generate_random_salt().unwrap();
        let salt2 = generate_random_salt().unwrap();
        assert_ne!(salt1, salt2, "Salts should be unique");
    }

    #[test]
    fn test_nonce_generation() {
        let nonce1 = generate_random_nonce().unwrap();
        let nonce2 = generate_random_nonce().unwrap();
        assert_ne!(nonce1, nonce2, "Nonces should be unique");
    }

    #[test]
    fn test_constant_time_equal() {
        let a = b"test_hash_32_bytes______________";
        let b = b"test_hash_32_bytes______________";
        assert!(constant_time_compare(a, b));
    }

    #[test]
    fn test_constant_time_not_equal() {
        let a = b"test_hash_32_bytes______________";
        let b = b"different_hash_32_bytes_________";
        assert!(!constant_time_compare(a, b));
    }

    #[test]
    fn test_constant_time_different_length() {
        let a = b"short";
        let b = b"much_longer_string";
        assert!(!constant_time_compare(a, b));
    }
}