//! Wallet serialization and encryption for secure key storage
//!
//! This module provides encrypted wallet file format with proper key serialization.

use std::io::{Read, Write};
use std::path::Path;

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use argon2::{Argon2, PasswordHasher};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::crypto::{keys::KeyError, PrivateKey, PublicKey};

/// Wallet file format version
const WALLET_VERSION: u32 = 1;

/// Salt size for password derivation (16 bytes)
const SALT_SIZE: usize = 16;

/// Nonce size for AES-GCM (12 bytes)
const NONCE_SIZE: usize = 12;

/// Encrypted wallet file format
///
/// Structure:
/// - Magic bytes: "BTPC" (4 bytes)
/// - Version: u32 (4 bytes)
/// - Salt: random bytes for password derivation (16 bytes)
/// - Nonce: random bytes for AES-GCM (12 bytes)
/// - Encrypted data: serialized WalletData + authentication tag
#[derive(Debug)]
pub struct EncryptedWallet {
    /// Version of the wallet format
    pub version: u32,
    /// Salt for password-based key derivation
    pub salt: [u8; SALT_SIZE],
    /// Nonce for AES-GCM encryption
    pub nonce: [u8; NONCE_SIZE],
    /// Encrypted wallet data (includes authentication tag)
    pub encrypted_data: Vec<u8>,
}

/// Wallet data to be encrypted
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletData {
    /// Unique wallet identifier (UUID v4)
    /// Required for backup restoration and wallet identity tracking
    pub wallet_id: String,
    /// Network type (mainnet, testnet, regtest)
    pub network: String,
    /// List of key entries in the wallet
    pub keys: Vec<KeyEntry>,
    /// Creation timestamp
    pub created_at: u64,
    /// Last modified timestamp
    pub modified_at: u64,
}

/// A single key entry in the wallet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyEntry {
    /// User-defined label
    pub label: String,
    /// Private key bytes (4000 bytes for ML-DSA-65)
    pub private_key_bytes: Vec<u8>,
    /// Public key bytes (1952 bytes for ML-DSA-65)
    pub public_key_bytes: Vec<u8>,
    /// Key generation seed (32 bytes)
    /// Required for signing capability after wallet load (T014 fix)
    /// None for legacy keys created before this feature
    #[serde(default)]
    pub seed: Option<Vec<u8>>,
    /// Address string derived from public key
    pub address: String,
    /// Creation timestamp
    pub created_at: u64,
}

/// Secure password wrapper that zeroizes on drop
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct SecurePassword {
    password: Vec<u8>,
}

impl SecurePassword {
    pub fn new(password: String) -> Self {
        SecurePassword {
            password: password.into_bytes(),
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.password
    }
}

impl EncryptedWallet {
    /// Create a new encrypted wallet from wallet data
    ///
    /// # Security
    /// - Uses Argon2id for password-based key derivation (OWASP recommended)
    /// - Uses AES-256-GCM for authenticated encryption
    /// - Random salt and nonce generated for each encryption
    pub fn encrypt(
        wallet_data: &WalletData,
        password: &SecurePassword,
    ) -> Result<Self, WalletError> {
        // Generate random salt for password derivation
        let mut salt = [0u8; SALT_SIZE];
        rand::thread_rng().fill_bytes(&mut salt);

        // Derive encryption key from password using Argon2id
        let encryption_key = Self::derive_key(password.as_bytes(), &salt)?;

        // Generate random nonce for AES-GCM
        let mut nonce = [0u8; NONCE_SIZE];
        rand::thread_rng().fill_bytes(&mut nonce);

        // Serialize wallet data
        let plaintext = bincode::serialize(wallet_data)
            .map_err(|_| WalletError::SerializationFailed)?;

        // Encrypt with AES-256-GCM
        let cipher = Aes256Gcm::new_from_slice(&encryption_key)
            .map_err(|_| WalletError::EncryptionFailed)?;

        // Use direct reference instead of deprecated from_slice()
        let encrypted_data = cipher
            .encrypt((&nonce).into(), plaintext.as_ref())
            .map_err(|_| WalletError::EncryptionFailed)?;

        Ok(EncryptedWallet {
            version: WALLET_VERSION,
            salt,
            nonce,
            encrypted_data,
        })
    }

    /// Decrypt wallet data
    ///
    /// # Security
    /// - Verifies authentication tag before decrypting (prevents tampering)
    /// - Uses constant-time operations where possible
    pub fn decrypt(&self, password: &SecurePassword) -> Result<WalletData, WalletError> {
        // Check version compatibility
        if self.version != WALLET_VERSION {
            return Err(WalletError::UnsupportedVersion(self.version));
        }

        // Derive encryption key from password
        let encryption_key = Self::derive_key(password.as_bytes(), &self.salt)?;

        // Decrypt with AES-256-GCM
        let cipher = Aes256Gcm::new_from_slice(&encryption_key)
            .map_err(|_| WalletError::DecryptionFailed)?;

        // Use direct reference instead of deprecated from_slice()
        let plaintext = cipher
            .decrypt((&self.nonce).into(), self.encrypted_data.as_ref())
            .map_err(|_| WalletError::DecryptionFailed)?;

        // Deserialize wallet data
        let wallet_data = bincode::deserialize(&plaintext)
            .map_err(|_| WalletError::DeserializationFailed)?;

        Ok(wallet_data)
    }

    /// Save encrypted wallet to file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), WalletError> {
        let mut file = std::fs::File::create(path)
            .map_err(|e| WalletError::IoError(e.to_string()))?;

        // Write magic bytes
        file.write_all(b"BTPC")
            .map_err(|e| WalletError::IoError(e.to_string()))?;

        // Write version
        file.write_all(&self.version.to_le_bytes())
            .map_err(|e| WalletError::IoError(e.to_string()))?;

        // Write salt
        file.write_all(&self.salt)
            .map_err(|e| WalletError::IoError(e.to_string()))?;

        // Write nonce
        file.write_all(&self.nonce)
            .map_err(|e| WalletError::IoError(e.to_string()))?;

        // Write encrypted data length
        let data_len = self.encrypted_data.len() as u32;
        file.write_all(&data_len.to_le_bytes())
            .map_err(|e| WalletError::IoError(e.to_string()))?;

        // Write encrypted data
        file.write_all(&self.encrypted_data)
            .map_err(|e| WalletError::IoError(e.to_string()))?;

        Ok(())
    }

    /// Load encrypted wallet from file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, WalletError> {
        let mut file = std::fs::File::open(path)
            .map_err(|e| WalletError::IoError(e.to_string()))?;

        // Read and verify magic bytes
        let mut magic = [0u8; 4];
        file.read_exact(&mut magic)
            .map_err(|e| WalletError::IoError(e.to_string()))?;
        if &magic != b"BTPC" {
            return Err(WalletError::InvalidFormat);
        }

        // Read version
        let mut version_bytes = [0u8; 4];
        file.read_exact(&mut version_bytes)
            .map_err(|e| WalletError::IoError(e.to_string()))?;
        let version = u32::from_le_bytes(version_bytes);

        // Read salt
        let mut salt = [0u8; SALT_SIZE];
        file.read_exact(&mut salt)
            .map_err(|e| WalletError::IoError(e.to_string()))?;

        // Read nonce
        let mut nonce = [0u8; NONCE_SIZE];
        file.read_exact(&mut nonce)
            .map_err(|e| WalletError::IoError(e.to_string()))?;

        // Read encrypted data length
        let mut len_bytes = [0u8; 4];
        file.read_exact(&mut len_bytes)
            .map_err(|e| WalletError::IoError(e.to_string()))?;
        let data_len = u32::from_le_bytes(len_bytes) as usize;

        // Read encrypted data
        let mut encrypted_data = vec![0u8; data_len];
        file.read_exact(&mut encrypted_data)
            .map_err(|e| WalletError::IoError(e.to_string()))?;

        Ok(EncryptedWallet {
            version,
            salt,
            nonce,
            encrypted_data,
        })
    }

    /// Derive encryption key from password using Argon2id
    ///
    /// # Security Parameters
    /// - Algorithm: Argon2id (resistant to GPU and side-channel attacks)
    /// - Memory: 64 MB (m=65536)
    /// - Iterations: 3 (t=3)
    /// - Parallelism: 4 (p=4)
    /// - Output: 32 bytes (AES-256 key)
    fn derive_key(password: &[u8], salt: &[u8]) -> Result<[u8; 32], WalletError> {
        use argon2::{Algorithm, Params, Version};

        let params = Params::new(
            65536, // 64 MB memory
            3,     // 3 iterations
            4,     // 4 parallel threads
            Some(32), // 32-byte output
        )
        .map_err(|_| WalletError::KeyDerivationFailed)?;

        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

        let mut output = [0u8; 32];
        argon2
            .hash_password_into(password, salt, &mut output)
            .map_err(|_| WalletError::KeyDerivationFailed)?;

        Ok(output)
    }
}

impl KeyEntry {
    /// Create a new key entry from a private key
    pub fn from_private_key(
        private_key: &PrivateKey,
        label: String,
        address: String,
    ) -> Self {
        KeyEntry {
            label,
            private_key_bytes: private_key.to_bytes().to_vec(),
            public_key_bytes: private_key.public_key().to_bytes().to_vec(),
            seed: None,  // T014: Seed must be set separately via from_private_key_with_seed()
            address,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

    /// Create a new key entry from a private key WITH seed (ENABLES SIGNING)
    ///
    /// This is the preferred method for creating key entries that support signing
    /// after wallet load. The seed enables keypair regeneration for signing operations.
    ///
    /// # Arguments
    /// * `private_key` - The private key to store
    /// * `seed` - The 32-byte seed used to generate this key
    /// * `label` - User-defined label for the key
    /// * `address` - Address string derived from the public key
    pub fn from_private_key_with_seed(
        private_key: &PrivateKey,
        seed: [u8; 32],
        label: String,
        address: String,
    ) -> Self {
        KeyEntry {
            label,
            private_key_bytes: private_key.to_bytes().to_vec(),
            public_key_bytes: private_key.public_key().to_bytes().to_vec(),
            seed: Some(seed.to_vec()),  // T014: Store seed for signing capability
            address,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

    /// Reconstruct a private key from this entry
    ///
    /// # Signing Capability (T014 FIX)
    /// If this key entry has a seed stored, the reconstructed key WILL support signing
    /// via on-demand keypair regeneration. This fixes the transaction signing bug.
    ///
    /// # Legacy Keys
    /// Keys without a seed (created before this feature) can still be used for:
    /// - Deriving the public key
    /// - Generating addresses
    /// - Verifying signatures (via the public key)
    ///
    /// But they CANNOT sign new transactions.
    pub fn to_private_key(&self) -> Result<PrivateKey, WalletError> {
        // T014 FIX: Use from_key_pair_bytes_with_seed() when seed is available
        if let Some(seed_vec) = &self.seed {
            // Validate seed size
            if seed_vec.len() != 32 {
                return Err(WalletError::KeyReconstruction(
                    crate::crypto::keys::KeyError::InvalidKeyData
                ));
            }

            // Convert Vec<u8> to [u8; 32]
            let mut seed = [0u8; 32];
            seed.copy_from_slice(&seed_vec[..32]);

            // Reconstruct with seed - ENABLES SIGNING!
            PrivateKey::from_key_pair_bytes_with_seed(
                &self.private_key_bytes,
                &self.public_key_bytes,
                seed
            )
            .map_err(WalletError::KeyReconstruction)
        } else {
            // Legacy key without seed - cannot sign
            PrivateKey::from_key_pair_bytes(&self.private_key_bytes, &self.public_key_bytes)
                .map_err(WalletError::KeyReconstruction)
        }
    }

    /// Get the public key
    pub fn to_public_key(&self) -> Result<PublicKey, WalletError> {
        PublicKey::from_bytes(&self.public_key_bytes)
            .map_err(WalletError::KeyReconstruction)
    }
}

/// Wallet-related errors
#[derive(Debug, Clone)]
pub enum WalletError {
    /// Encryption failed
    EncryptionFailed,
    /// Decryption failed (wrong password or corrupted data)
    DecryptionFailed,
    /// Serialization failed
    SerializationFailed,
    /// Deserialization failed
    DeserializationFailed,
    /// Key derivation failed
    KeyDerivationFailed,
    /// Key reconstruction failed
    KeyReconstruction(KeyError),
    /// Unsupported wallet version
    UnsupportedVersion(u32),
    /// Invalid wallet file format
    InvalidFormat,
    /// I/O error
    IoError(String),
}

impl std::fmt::Display for WalletError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WalletError::EncryptionFailed => write!(f, "Wallet encryption failed"),
            WalletError::DecryptionFailed => {
                write!(f, "Wallet decryption failed (wrong password or corrupted file)")
            }
            WalletError::SerializationFailed => write!(f, "Wallet serialization failed"),
            WalletError::DeserializationFailed => write!(f, "Wallet deserialization failed"),
            WalletError::KeyDerivationFailed => write!(f, "Password key derivation failed"),
            WalletError::KeyReconstruction(e) => write!(f, "Key reconstruction failed: {}", e),
            WalletError::UnsupportedVersion(v) => {
                write!(f, "Unsupported wallet version: {}", v)
            }
            WalletError::InvalidFormat => write!(f, "Invalid wallet file format"),
            WalletError::IoError(e) => write!(f, "Wallet I/O error: {}", e),
        }
    }
}

impl std::error::Error for WalletError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::PrivateKey;

    #[test]
    fn test_wallet_encryption_decryption() {
        let wallet_data = WalletData {
            wallet_id: "test-wallet-001".to_string(),
            network: "mainnet".to_string(),
            keys: vec![],
            created_at: 1234567890,
            modified_at: 1234567890,
        };

        let password = SecurePassword::new("test_password_123".to_string());

        // Encrypt
        let encrypted = EncryptedWallet::encrypt(&wallet_data, &password).unwrap();

        // Decrypt
        let decrypted = encrypted.decrypt(&password).unwrap();

        assert_eq!(decrypted.network, wallet_data.network);
        assert_eq!(decrypted.created_at, wallet_data.created_at);
    }

    #[test]
    fn test_wallet_wrong_password() {
        let wallet_data = WalletData {
            wallet_id: "test-wallet-002".to_string(),
            network: "mainnet".to_string(),
            keys: vec![],
            created_at: 1234567890,
            modified_at: 1234567890,
        };

        let password = SecurePassword::new("correct_password".to_string());
        let wrong_password = SecurePassword::new("wrong_password".to_string());

        let encrypted = EncryptedWallet::encrypt(&wallet_data, &password).unwrap();

        // Should fail with wrong password
        assert!(encrypted.decrypt(&wrong_password).is_err());
    }

    #[test]
    fn test_wallet_with_keys() {
        let private_key = PrivateKey::generate_ml_dsa().unwrap();
        let public_key = private_key.public_key();

        let key_entry = KeyEntry::from_private_key(
            &private_key,
            "Test Key".to_string(),
            "btpc_test_address".to_string(),
        );

        let wallet_data = WalletData {
            wallet_id: "test-wallet-003".to_string(),
            network: "testnet".to_string(),
            keys: vec![key_entry],
            created_at: 1234567890,
            modified_at: 1234567890,
        };

        let password = SecurePassword::new("secure_password_456".to_string());

        // Encrypt
        let encrypted = EncryptedWallet::encrypt(&wallet_data, &password).unwrap();

        // Decrypt
        let decrypted = encrypted.decrypt(&password).unwrap();

        assert_eq!(decrypted.keys.len(), 1);
        assert_eq!(decrypted.keys[0].label, "Test Key");
        assert_eq!(decrypted.keys[0].address, "btpc_test_address");
    }

    #[test]
    fn test_wallet_file_save_load() {
        let wallet_data = WalletData {
            wallet_id: "test-wallet-004".to_string(),
            network: "regtest".to_string(),
            keys: vec![],
            created_at: 9876543210,
            modified_at: 9876543210,
        };

        let password = SecurePassword::new("file_test_password".to_string());

        // Encrypt
        let encrypted = EncryptedWallet::encrypt(&wallet_data, &password).unwrap();

        // Save to file
        let temp_file = "/tmp/test_wallet.dat";
        encrypted.save_to_file(temp_file).unwrap();

        // Load from file
        let loaded = EncryptedWallet::load_from_file(temp_file).unwrap();

        // Decrypt
        let decrypted = loaded.decrypt(&password).unwrap();

        assert_eq!(decrypted.network, wallet_data.network);
        assert_eq!(decrypted.created_at, wallet_data.created_at);

        // Cleanup
        std::fs::remove_file(temp_file).ok();
    }

    #[test]
    fn test_wallet_tampering_detection() {
        let wallet_data = WalletData {
            wallet_id: "test-wallet-005".to_string(),
            network: "mainnet".to_string(),
            keys: vec![],
            created_at: 1111111111,
            modified_at: 1111111111,
        };

        let password = SecurePassword::new("tamper_test".to_string());

        let mut encrypted = EncryptedWallet::encrypt(&wallet_data, &password).unwrap();

        // Tamper with encrypted data
        if !encrypted.encrypted_data.is_empty() {
            encrypted.encrypted_data[0] ^= 0xFF;
        }

        // Should fail due to authentication tag mismatch
        assert!(encrypted.decrypt(&password).is_err());
    }

    // T004: Test that WalletData includes wallet_id field for backup restoration
    // This test WILL FAIL TO COMPILE until we add wallet_id field to WalletData struct
    #[test]
    fn test_wallet_backup_includes_wallet_id() {
        let wallet_id = "550e8400-e29b-41d4-a716-446655440000";

        let wallet_data = WalletData {
            wallet_id: wallet_id.to_string(), // ← COMPILE ERROR: field doesn't exist yet
            network: "mainnet".to_string(),
            keys: vec![],
            created_at: 1234567890,
            modified_at: 1234567890,
        };

        let password = SecurePassword::new("backup_test_password".to_string());

        // Encrypt wallet
        let encrypted = EncryptedWallet::encrypt(&wallet_data, &password).unwrap();

        // Decrypt and verify wallet_id is preserved
        let decrypted = encrypted.decrypt(&password).unwrap();

        assert_eq!(decrypted.wallet_id, wallet_id);
    }
}