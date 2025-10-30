//! Cryptographic primitives for BTPC
//!
//! This module provides quantum-resistant cryptography using ML-DSA (Dilithium)
//! and SHA-512 hashing throughout the system.

use std::fmt;

use serde::{Deserialize, Serialize};

pub mod address;
pub mod hash;
pub mod keys;
pub mod script;
pub mod signatures;
pub mod wallet_serde;

pub use address::{Address, AddressError};
pub use hash::{Hash, HashError};
pub use keys::{KeyError, PrivateKey, PublicKey};
pub use script::{Script, ScriptError};
pub use signatures::{Signature, SignatureError};
pub use wallet_serde::{
    EncryptedWallet, KeyEntry, SecurePassword, WalletData, WalletError,
};

/// Cryptographic constants for BTPC
pub mod constants {
    /// ML-DSA-65 private key size in bytes (pqc_dilithium implementation)
    /// Note: FIPS 204 specifies 4032, but pqc_dilithium uses 4000 bytes internally
    pub const ML_DSA_PRIVATE_KEY_SIZE: usize = pqc_dilithium::SECRETKEYBYTES;

    /// ML-DSA-65 public key size in bytes (per FIPS 204 specification)
    pub const ML_DSA_PUBLIC_KEY_SIZE: usize = pqc_dilithium::PUBLICKEYBYTES;

    /// ML-DSA-65 signature size in bytes (pqc_dilithium implementation)
    /// Note: FIPS 204 specifies 3309, but pqc_dilithium uses 3293 bytes
    pub const ML_DSA_SIGNATURE_SIZE: usize = pqc_dilithium::SIGNBYTES;

    /// SHA-512 hash size in bytes
    pub const SHA512_HASH_SIZE: usize = 64;

    /// Address size in bytes
    pub const ADDRESS_SIZE: usize = 20;
}

/// Error types for cryptographic operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CryptoError {
    /// Hash computation error
    Hash(HashError),
    /// Key generation or usage error
    Key(KeyError),
    /// Signature creation or verification error
    Signature(SignatureError),
    /// Address creation or validation error
    Address(AddressError),
    /// Script execution or validation error
    Script(ScriptError),
    /// Invalid input data
    InvalidInput(String),
    /// Internal cryptographic library error
    Internal(String),
}

impl fmt::Display for CryptoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CryptoError::Hash(e) => write!(f, "Hash error: {}", e),
            CryptoError::Key(e) => write!(f, "Key error: {}", e),
            CryptoError::Signature(e) => write!(f, "Signature error: {}", e),
            CryptoError::Address(e) => write!(f, "Address error: {}", e),
            CryptoError::Script(e) => write!(f, "Script error: {}", e),
            CryptoError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            CryptoError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for CryptoError {}

impl From<HashError> for CryptoError {
    fn from(err: HashError) -> Self {
        CryptoError::Hash(err)
    }
}

impl From<KeyError> for CryptoError {
    fn from(err: KeyError) -> Self {
        CryptoError::Key(err)
    }
}

impl From<SignatureError> for CryptoError {
    fn from(err: SignatureError) -> Self {
        CryptoError::Signature(err)
    }
}

impl From<AddressError> for CryptoError {
    fn from(err: AddressError) -> Self {
        CryptoError::Address(err)
    }
}

impl From<ScriptError> for CryptoError {
    fn from(err: ScriptError) -> Self {
        CryptoError::Script(err)
    }
}

/// Result type for cryptographic operations
pub type CryptoResult<T> = Result<T, CryptoError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants_are_correct() {
        use constants::*;

        // Verify ML-DSA-65 parameter sizes (pqc_dilithium implementation)
        // Note: Slightly different from FIPS 204 spec due to internal format
        assert_eq!(ML_DSA_PRIVATE_KEY_SIZE, 4000); // pqc_dilithium SECRETKEYBYTES
        assert_eq!(ML_DSA_PUBLIC_KEY_SIZE, 1952);  // Matches FIPS 204
        assert_eq!(ML_DSA_SIGNATURE_SIZE, 3293);   // pqc_dilithium SIGNBYTES

        // Verify SHA-512 hash size
        assert_eq!(SHA512_HASH_SIZE, 64);

        // Verify address size (RIPEMD-160)
        assert_eq!(ADDRESS_SIZE, 20);
    }

    #[test]
    fn test_error_conversions() {
        let hash_error = HashError::InvalidLength;
        let crypto_error: CryptoError = hash_error.into();

        match crypto_error {
            CryptoError::Hash(HashError::InvalidLength) => (),
            _ => panic!("Error conversion failed"),
        }
    }
}
