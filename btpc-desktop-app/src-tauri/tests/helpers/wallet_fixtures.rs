//! Wallet test fixtures with synthetic UTXOs

use btpc_core::crypto::{PrivateKey, PublicKey};
use std::path::PathBuf;
use tempfile::TempDir;
use uuid::Uuid;

/// Test wallet with known keys and UTXOs
#[derive(Debug, Clone)]
pub struct TestWallet {
    /// Wallet ID (UUID)
    pub id: String,

    /// Wallet file path
    pub path: PathBuf,

    /// BTPC address
    pub address: String,

    /// Password for wallet encryption
    pub password: String,

    /// Private key (for test assertions)
    pub private_key: PrivateKey,

    /// Public key
    pub public_key: PublicKey,

    /// Total balance (sum of UTXO amounts)
    pub balance: u64,

    /// UTXO amounts (for verification)
    pub utxos: Vec<u64>,
}

impl TestWallet {
    /// Create new test wallet with synthetic UTXOs
    pub fn new_with_balance(
        temp_dir: &TempDir,
        name: &str,
        balance: u64,
    ) -> Result<Self, String> {
        Self::new_with_utxos(temp_dir, name, vec![balance])
    }

    /// Create test wallet with specific UTXO amounts
    pub fn new_with_utxos(
        temp_dir: &TempDir,
        name: &str,
        utxo_amounts: Vec<u64>,
    ) -> Result<Self, String> {
        // Generate ML-DSA keypair
        let seed = [0u8; 32]; // Deterministic seed for tests
        let private_key = PrivateKey::from_seed(&seed)
            .map_err(|e| format!("Failed to generate test key: {:?}", e))?;

        let public_key = private_key.public_key();

        // Generate BTPC address from public key
        let address = public_key.to_address()
            .map_err(|e| format!("Failed to generate address: {:?}", e))?;

        // Create wallet file path
        let wallet_id = Uuid::new_v4().to_string();
        let wallet_filename = format!("{}.dat", name);
        let wallet_path = temp_dir.path().join(&wallet_filename);

        // Calculate total balance
        let balance: u64 = utxo_amounts.iter().sum();

        // Create wallet data structure
        let wallet = TestWallet {
            id: wallet_id.clone(),
            path: wallet_path.clone(),
            address: address.clone(),
            password: "test_password_123".to_string(),
            private_key: private_key.clone(),
            public_key,
            balance,
            utxos: utxo_amounts.clone(),
        };

        // Write wallet file (encrypted)
        Self::write_wallet_file(&wallet)?;

        Ok(wallet)
    }

    /// Write encrypted wallet file
    fn write_wallet_file(wallet: &TestWallet) -> Result<(), String> {
        use btpc_core::crypto::wallet_serde::{WalletData, KeyEntry};
        use aes_gcm::{
            aead::{Aead, KeyInit},
            Aes256Gcm, Nonce,
        };
        use argon2::{Argon2, PasswordHasher};
        use std::fs;

        // Derive encryption key from password using Argon2
        let salt = b"test_salt_12345678901234567890"; // 32 bytes
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(wallet.password.as_bytes(), salt.into())
            .map_err(|e| format!("Argon2 hash failed: {:?}", e))?;

        let key_bytes = password_hash.hash
            .ok_or("Hash missing")?
            .as_bytes();

        // Ensure 32 bytes for AES-256
        let mut encryption_key = [0u8; 32];
        encryption_key.copy_from_slice(&key_bytes[0..32]);

        // Create wallet data
        let key_entry = KeyEntry {
            address: wallet.address.clone(),
            private_key_bytes: wallet.private_key.to_bytes().to_vec(),
            public_key_bytes: wallet.public_key.to_bytes().to_vec(),
            seed: Some(vec![0u8; 32]), // Deterministic seed
            label: Some("Test Wallet".to_string()),
        };

        let wallet_data = WalletData {
            wallet_id: wallet.id.clone(),
            version: 1,
            keys: vec![key_entry],
        };

        // Serialize to JSON
        let json_data = serde_json::to_vec(&wallet_data)
            .map_err(|e| format!("JSON serialization failed: {:?}", e))?;

        // Encrypt with AES-256-GCM
        let cipher = Aes256Gcm::new(&encryption_key.into());
        let nonce = Nonce::from_slice(b"unique_nonce"); // 12 bytes
        let ciphertext = cipher.encrypt(nonce, json_data.as_ref())
            .map_err(|e| format!("Encryption failed: {:?}", e))?;

        // Write to file
        fs::write(&wallet.path, ciphertext)
            .map_err(|e| format!("Failed to write wallet file: {:?}", e))?;

        Ok(())
    }

    /// Get wallet as WalletManager-compatible structure
    pub fn to_wallet_info(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "address": self.address,
            "balance": self.balance,
            "path": self.path.to_string_lossy(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_wallet() {
        let temp_dir = TempDir::new().unwrap();
        let wallet = TestWallet::new_with_balance(&temp_dir, "test1", 100_000_000).unwrap();

        assert_eq!(wallet.balance, 100_000_000);
        assert_eq!(wallet.utxos.len(), 1);
        assert!(wallet.path.exists());
    }

    #[test]
    fn test_create_wallet_with_multiple_utxos() {
        let temp_dir = TempDir::new().unwrap();
        let utxos = vec![50_000_000, 30_000_000, 20_000_000];
        let wallet = TestWallet::new_with_utxos(&temp_dir, "test2", utxos).unwrap();

        assert_eq!(wallet.balance, 100_000_000);
        assert_eq!(wallet.utxos.len(), 3);
    }
}