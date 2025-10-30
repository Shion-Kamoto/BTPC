//! BTPC Integration Module
//!
//! This module provides integration with the existing BTPC binaries and functionality.
//! It acts as a bridge between the Tauri frontend and the BTPC blockchain components.

use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::fs;
use btpc_core::crypto::{Address, PrivateKey, EncryptedWallet, WalletData, KeyEntry, SecurePassword};
use btpc_core::Network;

/// Helper functions for binary management and execution
pub struct BtpcIntegration {
    pub btpc_home: PathBuf,
    pub bin_dir: PathBuf,
}

impl BtpcIntegration {
    pub fn new(btpc_home: PathBuf) -> Self {
        let bin_dir = btpc_home.join("bin");
        Self { btpc_home, bin_dir }
    }

    /// Check if a binary exists
    pub fn binary_exists(&self, name: &str) -> bool {
        self.bin_dir.join(name).exists()
    }

    /// Get path to binary
    pub fn binary_path(&self, name: &str) -> PathBuf {
        self.bin_dir.join(name)
    }

    /// Execute a binary with arguments and return output
    pub fn execute_binary(&self, name: &str, args: &[&str]) -> Result<Output> {
        let binary_path = self.binary_path(name);

        println!("Attempting to execute binary: {}", binary_path.display());
        println!("Binary exists: {}", binary_path.exists());
        println!("Arguments: {:?}", args);

        if !binary_path.exists() {
            return Err(anyhow!("Binary '{}' not found at {}", name, binary_path.display()));
        }

        let output = Command::new(&binary_path)
            .args(args)
            .output()?;

        println!("Command executed successfully");
        println!("Exit code: {:?}", output.status.code());
        println!("Stdout length: {}", output.stdout.len());
        println!("Stderr length: {}", output.stderr.len());

        if !output.stderr.is_empty() {
            println!("Stderr: {}", String::from_utf8_lossy(&output.stderr));
        }

        Ok(output)
    }

    /// Check if required BTPC binaries are installed
    pub fn check_installation(&self) -> BtpcInstallationStatus {
        let required_binaries = [
            "btpc-quantum-resistant-chain",
            "btpc_wallet_dilithium",
        ];

        let optional_binaries = [
            "btpc_secure_wallet",
            "btpc_miner",
            "integrated_mining_demo",
        ];

        let mut missing_required = Vec::new();
        let mut missing_optional = Vec::new();
        let mut available = Vec::new();

        for binary in &required_binaries {
            if self.binary_exists(binary) {
                available.push(binary.to_string());
            } else {
                missing_required.push(binary.to_string());
            }
        }

        for binary in &optional_binaries {
            if self.binary_exists(binary) {
                available.push(binary.to_string());
            } else {
                missing_optional.push(binary.to_string());
            }
        }

        BtpcInstallationStatus {
            is_complete: missing_required.is_empty(),
            available_binaries: available,
            missing_required_binaries: missing_required,
            missing_optional_binaries: missing_optional,
            bin_directory_exists: self.bin_dir.exists(),
            btpc_home_exists: self.btpc_home.exists(),
        }
    }

    /// Create wallet by generating ML-DSA keypair with compressed address and encrypted private key
    /// Returns (address, seed_phrase, private_key_hex)
    pub fn create_wallet(&self, wallet_file: &Path, password: &str) -> Result<(String, String, String)> {
        // Ensure wallet directory exists
        if let Some(wallet_dir) = wallet_file.parent() {
            fs::create_dir_all(wallet_dir)?;
        }

        // T015 FIX: Generate deterministic seed for key generation
        // This enables signing capability after wallet load
        use rand::RngCore;
        let mut seed = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut seed);

        // T015 FIX: Generate ML-DSA keypair from seed (enables signing after load)
        let private_key = PrivateKey::from_seed(&seed)
            .map_err(|e| anyhow!("Failed to generate private key from seed: {}", e))?;
        let public_key = private_key.public_key();

        // Create compressed Base58 address (20 bytes hashed, ~34 chars encoded)
        let address = Address::from_public_key(&public_key, Network::Regtest);
        let address_string = address.to_string();

        // Get private key bytes
        let private_key_bytes = private_key.to_bytes();
        let private_key_hex = hex::encode(private_key_bytes);

        // Generate BIP39 mnemonic from the seed (not private key hash)
        // This allows proper wallet recovery from seed phrase
        use bip39::Mnemonic;

        let mnemonic = Mnemonic::from_entropy(&seed)
            .map_err(|e| anyhow!("Failed to generate mnemonic: {}", e))?;
        let seed_phrase = mnemonic.to_string();

        // GREEN PHASE: Use Argon2id encryption (btpc-core's EncryptedWallet)
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // T015 FIX: Create KeyEntry WITH seed for signing capability
        let key_entry = KeyEntry::from_private_key_with_seed(
            &private_key,
            seed,  // ← KEY FIX: Store seed for transaction signing
            "main".to_string(),
            address_string.clone(),
        );

        // T015 FIX: Generate wallet_id for backup restoration
        use uuid::Uuid;
        let wallet_id = Uuid::new_v4().to_string();

        // Create WalletData structure with wallet_id
        let wallet_data = WalletData {
            wallet_id,  // ← KEY FIX: Include wallet_id for backups
            network: "mainnet".to_string(),
            keys: vec![key_entry],
            created_at: now,
            modified_at: now,
        };

        // Encrypt with Argon2id (Article VIII compliance)
        let secure_password = SecurePassword::new(password.to_string());
        let encrypted = EncryptedWallet::encrypt(&wallet_data, &secure_password)
            .map_err(|e| anyhow!("Argon2id encryption failed: {}", e))?;

        // Save as .dat file (EncryptedWallet format)
        let wallet_dat_file = wallet_file.with_extension("dat");
        encrypted.save_to_file(&wallet_dat_file)
            .map_err(|e| anyhow!("Failed to save encrypted wallet: {}", e))?;

        // Return address, seed phrase, and hex key for display
        Ok((address_string, seed_phrase, private_key_hex))
    }

    // Legacy SHA-256 encryption methods removed (replaced by Argon2id EncryptedWallet)
    // See SESSION_SUMMARY_2025-10-19_ARGON2ID_GREEN_PHASE.md for migration details

    /// Get wallet balance
    pub fn get_wallet_balance(&self, wallet_file: &Path) -> Result<String> {
        if !self.binary_exists("btpc_wallet") {
            return Err(anyhow!("Wallet binary not found"));
        }

        if !wallet_file.exists() {
            return Err(anyhow!("Wallet file not found: {}", wallet_file.display()));
        }

        let output = self.execute_binary("btpc_wallet", &[
            "balance",
            "--file",
            &wallet_file.display().to_string(),
        ])?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(anyhow!("Balance check failed: {}", String::from_utf8_lossy(&output.stderr)))
        }
    }

    /// Get wallet address from wallet file
    pub fn get_wallet_address(&self, wallet_file: &Path) -> Result<String> {
        if !wallet_file.exists() {
            return Err(anyhow!("Wallet file not found: {}", wallet_file.display()));
        }

        // Read wallet JSON file
        let wallet_content = fs::read_to_string(wallet_file)?;
        let wallet_data: serde_json::Value = serde_json::from_str(&wallet_content)?;

        // Extract address
        let address = wallet_data["address"]
            .as_str()
            .ok_or_else(|| anyhow!("Address not found in wallet file"))?
            .to_string();

        Ok(address)
    }

    /// Setup directories for BTPC
    pub fn setup_directories(&self, log_dir: &Path, data_dir: &Path, config_dir: &Path) -> Result<()> {
        let dirs = [
            &self.btpc_home,
            log_dir,
            data_dir,
            config_dir,
            &data_dir.join("node"),
            &data_dir.join("wallet"),
        ];

        for dir in dirs {
            if !dir.exists() {
                fs::create_dir_all(dir)?;
            }
        }

        Ok(())
    }

    /// Copy binaries from build locations to BTPC bin directory
    pub fn install_binaries_from_build(&self) -> Result<Vec<String>> {
        fs::create_dir_all(&self.bin_dir)?;

        let mut installed = Vec::new();

        // Check build locations
        let build_locations = [
            PathBuf::from("/home/bob/BTPC/core/target/release"),
            PathBuf::from("/home/bob/BTPC/core/target/debug"),
            PathBuf::from("/home/bob/BTPC/target/release"),
            PathBuf::from("/home/bob/BTPC/unified-launcher/target/release"),
        ];

        let target_binaries = [
            "btpc-quantum-resistant-chain",
            "btpc_wallet_dilithium",
            "btpc_secure_wallet",
            "btpc_miner",
            "integrated_mining_demo",
            "mine_send_wallet",
        ];

        for location in &build_locations {
            if !location.exists() {
                continue;
            }

            for binary in &target_binaries {
                let src = location.join(binary);
                let dest = self.bin_dir.join(binary);

                if src.exists() && !dest.exists() {
                    match fs::copy(&src, &dest) {
                        Ok(_) => {
                            // Make executable on Unix
                            #[cfg(unix)]
                            {
                                use std::os::unix::fs::PermissionsExt;
                                let mut perms = fs::metadata(&dest)?.permissions();
                                perms.set_mode(perms.mode() | 0o755);
                                fs::set_permissions(&dest, perms)?;
                            }

                            installed.push(binary.to_string());
                        }
                        Err(e) => {
                            eprintln!("Failed to copy {}: {}", binary, e);
                        }
                    }
                }
            }
        }

        Ok(installed)
    }

    /// Get system information
    #[allow(dead_code)]
    pub fn get_system_info(&self) -> SystemInfo {
        SystemInfo {
            btpc_home: self.btpc_home.clone(),
            bin_dir: self.bin_dir.clone(),
            installation_status: self.check_installation(),
            rust_version: get_rust_version(),
            platform: get_platform_info(),
        }
    }

    /// Check if wallet file uses legacy SHA-256 encryption format
    ///
    /// Returns true if wallet uses old "AES-256-GCM" (SHA-256 KDF)
    /// Returns false if wallet uses new "Argon2id-AES-256-GCM"
    pub fn is_legacy_wallet_format(&self, wallet_file: &Path) -> Result<bool> {
        if !wallet_file.exists() {
            return Err(anyhow!("Wallet file not found"));
        }

        let content = fs::read_to_string(wallet_file)?;
        let wallet_json: serde_json::Value = serde_json::from_str(&content)?;

        // Check encryption field
        match wallet_json["encryption"].as_str() {
            Some("AES-256-GCM") => Ok(true), // Legacy SHA-256 KDF
            Some("Argon2id-AES-256-GCM") => Ok(false), // New Argon2id
            _ => Ok(true), // Unknown = assume legacy
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct BtpcInstallationStatus {
    pub is_complete: bool,
    pub available_binaries: Vec<String>,
    pub missing_required_binaries: Vec<String>,
    pub missing_optional_binaries: Vec<String>,
    pub bin_directory_exists: bool,
    pub btpc_home_exists: bool,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SystemInfo {
    pub btpc_home: PathBuf,
    pub bin_dir: PathBuf,
    pub installation_status: BtpcInstallationStatus,
    pub rust_version: Option<String>,
    pub platform: String,
}

fn get_rust_version() -> Option<String> {
    Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                None
            }
        })
}

fn get_platform_info() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

#[cfg(test)]
mod tests;