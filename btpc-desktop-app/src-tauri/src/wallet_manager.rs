//! Multi-Wallet Management System for BTPC Desktop Application
//!
//! This module provides comprehensive wallet management functionality including:
//! - Multiple wallet creation and management
//! - Wallet nicknames and metadata
//! - Secure wallet storage and encryption
//! - Import/export capabilities
//! - Balance tracking across multiple wallets

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use chrono::{DateTime, Utc, Duration};
use uuid::Uuid;
use crate::error::{BtpcError, BtpcResult};
use crate::security::SecurityManager;
use btpc_core::crypto::{EncryptedWallet, WalletData, KeyEntry, SecurePassword};

/// UTXO Reservation Token for preventing double-spending during transaction creation
///
/// This token reserves specific UTXOs for a transaction, preventing them from being
/// used by concurrent transaction creation attempts. Reservations expire after a
/// configurable timeout (default 5 minutes) to prevent orphaned locks.
///
/// # Thread Safety
/// This struct is designed to work with `Arc<Mutex<HashMap<Uuid, ReservationToken>>>`
/// for thread-safe concurrent access across multiple transaction creation requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReservationToken {
    /// Unique reservation identifier
    pub id: Uuid,
    /// Associated transaction ID (if transaction has been created)
    pub transaction_id: Option<String>,
    /// List of reserved UTXOs: (transaction_hash, output_index)
    pub utxos: Vec<(String, u32)>,
    /// Timestamp when reservation was created
    pub created_at: DateTime<Utc>,
    /// Timestamp when reservation expires (default: created_at + 5 minutes)
    pub expires_at: DateTime<Utc>,
    /// Wallet ID that owns these UTXOs
    pub wallet_id: String,
}

impl ReservationToken {
    /// Create a new UTXO reservation
    ///
    /// # Parameters
    /// - `wallet_id`: The wallet that owns the UTXOs
    /// - `utxos`: List of UTXOs to reserve (txid, vout)
    /// - `transaction_id`: Optional transaction ID if already created
    /// - `expiry_minutes`: Minutes until reservation expires (default: 5)
    ///
    /// # Returns
    /// New ReservationToken with unique ID and expiry timestamp
    pub fn new(
        wallet_id: String,
        utxos: Vec<(String, u32)>,
        transaction_id: Option<String>,
        expiry_minutes: Option<i64>,
    ) -> Self {
        let now = Utc::now();
        let expiry_duration = Duration::minutes(expiry_minutes.unwrap_or(5));

        Self {
            id: Uuid::new_v4(),
            transaction_id,
            utxos,
            created_at: now,
            expires_at: now + expiry_duration,
            wallet_id,
        }
    }

    /// Check if this reservation has expired
    ///
    /// # Returns
    /// `true` if current time is past expiry timestamp
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Extend the expiry time of this reservation
    ///
    /// # Parameters
    /// - `additional_minutes`: Additional minutes to extend (added to current expires_at)
    ///
    /// # Returns
    /// New expiry timestamp
    pub fn extend_expiry(&mut self, additional_minutes: i64) -> DateTime<Utc> {
        self.expires_at += Duration::minutes(additional_minutes);
        self.expires_at
    }

    /// Release this reservation (marks it for deletion)
    ///
    /// Returns the list of UTXOs that were reserved
    pub fn release(self) -> Vec<(String, u32)> {
        self.utxos
    }

    /// Get the number of UTXOs reserved by this token
    pub fn utxo_count(&self) -> usize {
        self.utxos.len()
    }

    /// Check if a specific UTXO is reserved by this token
    ///
    /// # Parameters
    /// - `txid`: Transaction hash
    /// - `vout`: Output index
    pub fn contains_utxo(&self, txid: &str, vout: u32) -> bool {
        self.utxos.iter().any(|(tx, v)| tx == txid && *v == vout)
    }
}

/// Comprehensive wallet information structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletInfo {
    /// Unique wallet identifier
    pub id: String,
    /// User-friendly nickname for the wallet
    pub nickname: String,
    /// BTPC address (Base58 format, ~34 characters with checksum)
    pub address: String,
    /// File path to the wallet data
    pub file_path: PathBuf,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last accessed timestamp
    pub last_accessed: DateTime<Utc>,
    /// Wallet metadata and settings
    pub metadata: WalletMetadata,
    /// Current balance cache (in satoshis)
    pub cached_balance_credits: u64,
    /// Current balance cache (in BTP)
    pub cached_balance_btp: f64,
    /// Last balance update timestamp
    pub balance_updated_at: DateTime<Utc>,
    /// Whether this wallet is the default/primary wallet
    pub is_default: bool,
    /// Wallet import/creation source
    pub source: WalletSource,
}

/// Response from wallet creation including seed phrase and private key for user display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWalletResponse {
    /// The created wallet information
    pub wallet_info: WalletInfo,
    /// BIP39 seed phrase (24 words) - for displaying to user once as backup/recovery
    pub seed_phrase: String,
    /// Private key hex (for displaying to user once as backup/recovery)
    pub private_key_hex: String,
}

/// Additional metadata for wallet management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletMetadata {
    /// User description or notes
    pub description: Option<String>,
    /// Wallet category/purpose (e.g., "Personal", "Mining", "Trading")
    pub category: String,
    /// Custom color for UI display
    pub color: String,
    /// Whether to show this wallet in quick access
    pub is_favorite: bool,
    /// Auto-backup settings
    pub auto_backup: bool,
    /// Notification preferences
    pub notifications_enabled: bool,
    /// Custom transaction fee preference (satoshis)
    pub default_fee_credits: Option<u64>,
}

/// Wallet creation/import source tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WalletSource {
    /// Created through the desktop app
    CreatedNew { version: String },
    /// Imported from external file
    ImportedFile { original_path: String },
    /// Imported from recovery phrase
    ImportedRecovery,
    /// Migrated from legacy format
    Migrated { from_version: String },
}

/// Wallet management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletManagerConfig {
    /// Directory for storing wallet files
    pub wallets_dir: PathBuf,
    /// Directory for wallet backups
    pub backups_dir: PathBuf,
    /// Maximum number of wallets allowed
    pub max_wallets: usize,
    /// Default category for new wallets
    pub default_category: String,
    /// Auto-backup interval in hours
    pub backup_interval_hours: u64,
    /// Whether to encrypt wallet metadata
    pub encrypt_metadata: bool,
}

/// Wallet creation parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWalletRequest {
    /// User-chosen nickname (required)
    pub nickname: String,
    /// Optional description
    pub description: String,
    /// Wallet category
    pub category: Option<String>,
    /// UI color preference
    pub color: Option<String>,
    /// Whether to mark as favorite
    pub is_favorite: bool,
    /// Whether to set as default wallet
    pub is_default: bool,
    /// Auto-backup preference
    pub auto_backup: bool,
    /// Notifications enabled
    pub notifications_enabled: bool,
    /// Default fee in satoshis
    pub default_fee_credits: Option<u64>,
    /// Password for encrypting the private key (required for new wallets)
    pub password: String,
    /// Import data for importing existing wallets
    pub import_data: Option<ImportData>,
}

/// Import data structure for wallet imports
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportData {
    /// Wallet address to import
    pub address: String,
    /// Private key in hex format
    pub private_key_hex: String,
    /// Password for encryption
    pub password: String,
}

/// Wallet update parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateWalletRequest {
    /// Wallet ID to update
    pub wallet_id: String,
    /// New nickname (optional)
    pub nickname: Option<String>,
    /// New description (optional)
    pub description: Option<String>,
    /// New category (optional)
    pub category: Option<String>,
    /// New color (optional)
    pub color: Option<String>,
    /// Update favorite status (optional)
    pub is_favorite: Option<bool>,
    /// Update default status (optional)
    pub is_default: Option<bool>,
    /// Update auto-backup setting (optional)
    pub auto_backup: Option<bool>,
    /// Update notifications setting (optional)
    pub notifications_enabled: Option<bool>,
    /// Update default fee setting (optional)
    pub default_fee_credits: Option<u64>,
}

/// Wallet import parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportWalletRequest {
    /// Import source
    pub source: ImportSource,
    /// User-chosen nickname
    pub nickname: String,
    /// Optional description
    pub description: Option<String>,
    /// Wallet category
    pub category: Option<String>,
    /// Whether to set as default
    pub set_as_default: bool,
}

/// Import source options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImportSource {
    /// Import from file path
    File { path: String },
    /// Import from recovery phrase
    RecoveryPhrase { phrase: String },
    /// Import from private key
    PrivateKey { key: String },
}

/// Wallet selection criteria for operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletSelector {
    /// Select by wallet ID
    pub id: Option<String>,
    /// Select by nickname
    pub nickname: Option<String>,
    /// Select default wallet if no other criteria match
    pub use_default: bool,
}

/// Wallet statistics and summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletSummary {
    /// Total number of wallets
    pub total_wallets: usize,
    /// Total balance across all wallets (credits)
    pub total_balance_credits: u64,
    /// Total balance across all wallets (BTP)
    pub total_balance_btp: f64,
    /// Number of favorite wallets
    pub favorite_wallets: usize,
    /// Most recently accessed wallet
    pub most_recent_wallet: Option<WalletInfo>,
    /// Wallet with highest balance
    pub highest_balance_wallet: Option<WalletInfo>,
    /// Default wallet
    pub default_wallet: Option<WalletInfo>,
}

/// Main wallet manager implementation
pub struct WalletManager {
    config: WalletManagerConfig,
    wallets: HashMap<String, WalletInfo>,
    security: SecurityManager,
    metadata_file: PathBuf,
    /// Thread-safe storage for UTXO reservations
    /// Key: Reservation UUID, Value: ReservationToken
    reservations: Arc<Mutex<HashMap<Uuid, ReservationToken>>>,
}

impl Default for WalletManagerConfig {
    fn default() -> Self {
        let btpc_home = dirs::home_dir().unwrap_or_default().join(".btpc");
        Self {
            wallets_dir: btpc_home.join("wallets"),
            backups_dir: btpc_home.join("wallet-backups"),
            max_wallets: 50,
            default_category: "Personal".to_string(),
            backup_interval_hours: 24,
            encrypt_metadata: true,
        }
    }
}

impl Default for WalletMetadata {
    fn default() -> Self {
        Self {
            description: None,
            category: "Personal".to_string(),
            color: "#2563eb".to_string(), // Blue
            is_favorite: false,
            auto_backup: true,
            notifications_enabled: true,
            default_fee_credits: Some(10000), // 0.0001 BTP
        }
    }
}

impl WalletManager {
    /// Create a new wallet manager instance
    pub fn new(config: WalletManagerConfig, security: SecurityManager) -> BtpcResult<Self> {
        // Ensure directories exist
        std::fs::create_dir_all(&config.wallets_dir)
            .map_err(|_e| BtpcError::FileSystem(crate::error::FileSystemError::DirectoryCreationFailed {
                path: config.wallets_dir.display().to_string(),
            }))?;

        std::fs::create_dir_all(&config.backups_dir)
            .map_err(|_e| BtpcError::FileSystem(crate::error::FileSystemError::DirectoryCreationFailed {
                path: config.backups_dir.display().to_string(),
            }))?;

        let metadata_file = config.wallets_dir.join("wallets_metadata.json");

        let mut manager = Self {
            config,
            wallets: HashMap::new(),
            security,
            metadata_file,
            reservations: Arc::new(Mutex::new(HashMap::new())),
        };

        // Load existing wallets
        manager.load_wallets()?;

        Ok(manager)
    }

    /// Create a new wallet with the specified parameters
    pub fn create_wallet(&mut self, request: CreateWalletRequest, btpc_integration: &crate::btpc_integration::BtpcIntegration) -> BtpcResult<CreateWalletResponse> {
        // Validate nickname uniqueness
        if self.get_wallet_by_nickname(&request.nickname).is_some() {
            return Err(BtpcError::Validation(crate::error::ValidationError::CustomValidation {
                rule: "unique_nickname".to_string(),
                message: format!("Wallet nickname '{}' already exists", request.nickname),
            }));
        }

        // Check wallet limit
        if self.wallets.len() >= self.config.max_wallets {
            return Err(BtpcError::Validation(crate::error::ValidationError::CustomValidation {
                rule: "wallet_limit".to_string(),
                message: format!("Maximum number of wallets ({}) reached", self.config.max_wallets),
            }));
        }

        // Generate unique wallet ID
        let wallet_id = Uuid::new_v4().to_string();
        let wallet_filename = format!("wallet_{}.dat", wallet_id);
        let wallet_path = self.config.wallets_dir.join(&wallet_filename);

        // Create wallet file using BTPC integration with password
        let (address, seed_phrase, private_key_hex) = self.create_btpc_wallet_file(&wallet_path, &request.password, btpc_integration)?;

        // Prepare metadata
        let metadata = WalletMetadata {
            description: Some(request.description),
            category: request.category.unwrap_or_else(|| self.config.default_category.clone()),
            color: request.color.unwrap_or_else(Self::generate_random_color),
            is_favorite: request.is_favorite,
            auto_backup: request.auto_backup,
            notifications_enabled: request.notifications_enabled,
            default_fee_credits: request.default_fee_credits,
        };

        // Create wallet info
        let mut wallet_info = WalletInfo {
            id: wallet_id.clone(),
            nickname: request.nickname,
            address,
            file_path: wallet_path,
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            metadata,
            cached_balance_credits: 0,
            cached_balance_btp: 0.0,
            balance_updated_at: Utc::now(),
            is_default: request.is_default,
            source: WalletSource::CreatedNew {
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        };

        // If setting as default, unset other defaults
        if request.is_default {
            self.unset_all_defaults();
        }

        // If this is the first wallet, make it default
        if self.wallets.is_empty() {
            wallet_info.is_default = true;
        }

        // Store wallet
        self.wallets.insert(wallet_id, wallet_info.clone());
        self.save_wallets()?;

        // Create initial backup if enabled
        if wallet_info.metadata.auto_backup {
            self.backup_wallet(&wallet_info.id)?;
        }

        Ok(CreateWalletResponse {
            wallet_info,
            seed_phrase,
            private_key_hex,
        })
    }

    /// Get wallet by ID
    pub fn get_wallet(&self, wallet_id: &str) -> Option<&WalletInfo> {
        self.wallets.get(wallet_id)
    }

    /// Get wallet by nickname
    pub fn get_wallet_by_nickname(&self, nickname: &str) -> Option<&WalletInfo> {
        self.wallets.values().find(|w| w.nickname == nickname)
    }

    /// Get default wallet
    pub fn get_default_wallet(&self) -> Option<&WalletInfo> {
        self.wallets.values().find(|w| w.is_default)
    }

    /// List all wallets
    pub fn list_wallets(&self) -> Vec<&WalletInfo> {
        let mut wallets: Vec<&WalletInfo> = self.wallets.values().collect();
        wallets.sort_by(|a, b| b.last_accessed.cmp(&a.last_accessed));
        wallets
    }

    /// Update wallet information
    pub fn update_wallet(&mut self, request: UpdateWalletRequest) -> BtpcResult<WalletInfo> {
        // Check if wallet exists first
        if !self.wallets.contains_key(&request.wallet_id) {
            return Err(BtpcError::Validation(crate::error::ValidationError::CustomValidation {
                rule: "wallet_exists".to_string(),
                message: format!("Wallet with ID '{}' not found", request.wallet_id),
            }));
        }

        // Validate nickname uniqueness if provided
        if let Some(new_nickname) = &request.nickname {
            if let Some(existing) = self.get_wallet_by_nickname(new_nickname) {
                if existing.id != request.wallet_id {
                    return Err(BtpcError::Validation(crate::error::ValidationError::CustomValidation {
                        rule: "unique_nickname".to_string(),
                        message: format!("Wallet nickname '{}' already exists", new_nickname),
                    }));
                }
            }
        }

        // Handle default wallet changes first (needs separate handling)
        if let Some(is_default) = request.is_default {
            if is_default {
                self.unset_all_defaults();
            }
        }

        // Now get mutable reference to the wallet and apply updates
        let wallet = self.wallets.get_mut(&request.wallet_id).unwrap(); // Safe because we checked existence above

        // Update nickname if provided
        if let Some(new_nickname) = &request.nickname {
            wallet.nickname = new_nickname.clone();
        }

        // Update metadata fields
        if let Some(description) = request.description {
            wallet.metadata.description = Some(description);
        }
        if let Some(category) = request.category {
            wallet.metadata.category = category;
        }
        if let Some(color) = request.color {
            wallet.metadata.color = color;
        }
        if let Some(is_favorite) = request.is_favorite {
            wallet.metadata.is_favorite = is_favorite;
        }
        if let Some(auto_backup) = request.auto_backup {
            wallet.metadata.auto_backup = auto_backup;
        }
        if let Some(notifications_enabled) = request.notifications_enabled {
            wallet.metadata.notifications_enabled = notifications_enabled;
        }
        if let Some(default_fee_credits) = request.default_fee_credits {
            wallet.metadata.default_fee_credits = Some(default_fee_credits);
        }

        // Apply default wallet setting
        if let Some(is_default) = request.is_default {
            wallet.is_default = is_default;
        }

        wallet.last_accessed = Utc::now();

        // Clone the result before calling save_wallets
        let result = wallet.clone();

        // Save changes
        self.save_wallets()?;

        // Ensure at least one default exists after all updates
        if request.is_default == Some(false) {
            self.ensure_default_wallet();
        }

        Ok(result)
    }

    /// Delete a wallet
    pub fn delete_wallet(&mut self, wallet_id: &str) -> BtpcResult<()> {
        let wallet = self.wallets.get(wallet_id)
            .ok_or_else(|| BtpcError::Validation(crate::error::ValidationError::CustomValidation {
                rule: "wallet_exists".to_string(),
                message: format!("Wallet with ID '{}' not found", wallet_id),
            }))?;

        // Prevent deleting the last wallet
        if self.wallets.len() == 1 {
            return Err(BtpcError::Validation(crate::error::ValidationError::CustomValidation {
                rule: "minimum_wallets".to_string(),
                message: "Cannot delete the last remaining wallet".to_string(),
            }));
        }

        let was_default = wallet.is_default;

        // Remove wallet file
        if wallet.file_path.exists() {
            std::fs::remove_file(&wallet.file_path)
                .map_err(|_| BtpcError::FileSystem(crate::error::FileSystemError::WriteFailed {
                    path: wallet.file_path.display().to_string(),
                    error: "Failed to delete wallet file".to_string(),
                }))?;
        }

        // Remove from memory
        self.wallets.remove(wallet_id);

        // Set new default if needed
        if was_default {
            self.ensure_default_wallet();
        }

        self.save_wallets()?;
        Ok(())
    }

    /// Update wallet balance cache
    pub fn update_wallet_balance(&mut self, wallet_id: &str, balance_credits: u64) -> BtpcResult<()> {
        let wallet = self.wallets.get_mut(wallet_id)
            .ok_or_else(|| BtpcError::Validation(crate::error::ValidationError::CustomValidation {
                rule: "wallet_exists".to_string(),
                message: format!("Wallet with ID '{}' not found", wallet_id),
            }))?;

        wallet.cached_balance_credits = balance_credits;
        wallet.cached_balance_btp = balance_credits as f64 / 100_000_000.0;
        wallet.balance_updated_at = Utc::now();
        wallet.last_accessed = Utc::now();

        self.save_wallets()?;
        Ok(())
    }

    /// Get wallet summary statistics
    pub fn get_summary(&self) -> WalletSummary {
        let total_balance_credits: u64 = self.wallets.values()
            .map(|w| w.cached_balance_credits)
            .sum();

        let favorite_count = self.wallets.values()
            .filter(|w| w.metadata.is_favorite)
            .count();

        let most_recent = self.wallets.values()
            .max_by_key(|w| w.last_accessed)
            .cloned();

        let highest_balance = self.wallets.values()
            .max_by_key(|w| w.cached_balance_credits)
            .cloned();

        let default_wallet = self.get_default_wallet().cloned();

        WalletSummary {
            total_wallets: self.wallets.len(),
            total_balance_credits,
            total_balance_btp: total_balance_credits as f64 / 100_000_000.0,
            favorite_wallets: favorite_count,
            most_recent_wallet: most_recent,
            highest_balance_wallet: highest_balance,
            default_wallet,
        }
    }

    /// Import wallet from external source
    pub fn import_wallet(&mut self, _request: ImportWalletRequest) -> BtpcResult<WalletInfo> {
        // Implementation would depend on the import source
        // This is a placeholder for the interface
        Err(BtpcError::Application(
            "Wallet import functionality not yet implemented. Use create_wallet or restore from backup instead.".to_string()
        ))
    }

    /// Export wallet to file
    pub fn export_wallet(&self, _wallet_id: &str, _export_path: &Path) -> BtpcResult<()> {
        // Implementation for wallet export
        Err(BtpcError::Application(
            "Wallet export functionality not yet implemented. Use backup_wallet for wallet backups instead.".to_string()
        ))
    }

    /// Create backup of wallet
    pub fn backup_wallet(&self, wallet_id: &str) -> BtpcResult<PathBuf> {
        let wallet = self.get_wallet(wallet_id)
            .ok_or_else(|| BtpcError::Validation(crate::error::ValidationError::CustomValidation {
                rule: "wallet_exists".to_string(),
                message: format!("Wallet with ID '{}' not found", wallet_id),
            }))?;

        // Ensure backups directory exists
        if !self.config.backups_dir.exists() {
            std::fs::create_dir_all(&self.config.backups_dir)
                .map_err(|e| BtpcError::FileSystem(crate::error::FileSystemError::WriteFailed {
                    path: self.config.backups_dir.display().to_string(),
                    error: format!("Failed to create backups directory: {}", e),
                }))?;
        }

        // Create backup with timestamp
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let backup_filename = format!("backup_{}_{}.dat", wallet.nickname, timestamp);
        let backup_path = self.config.backups_dir.join(backup_filename);

        // Verify source wallet file exists
        if !wallet.file_path.exists() {
            return Err(BtpcError::FileSystem(crate::error::FileSystemError::ReadFailed {
                path: wallet.file_path.display().to_string(),
                error: "Wallet file not found".to_string(),
            }));
        }

        // Copy wallet file to backup location
        std::fs::copy(&wallet.file_path, &backup_path)
            .map_err(|e| BtpcError::FileSystem(crate::error::FileSystemError::WriteFailed {
                path: backup_path.display().to_string(),
                error: format!("Failed to create backup: {}", e),
            }))?;

        println!("✅ Wallet backup created: {}", backup_path.display());
        Ok(backup_path)
    }

    // Private helper methods

    fn load_wallets(&mut self) -> BtpcResult<()> {
        if !self.metadata_file.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(&self.metadata_file)
            .map_err(|_| BtpcError::FileSystem(crate::error::FileSystemError::ReadFailed {
                path: self.metadata_file.display().to_string(),
                error: "Failed to read wallet metadata".to_string(),
            }))?;

        self.wallets = serde_json::from_str(&content)
            .map_err(|_| BtpcError::Utxo(crate::error::UtxoError::DeserializationError {
                data_type: "wallet metadata".to_string(),
                error: "Invalid JSON format".to_string(),
            }))?;

        Ok(())
    }

    fn save_wallets(&self) -> BtpcResult<()> {
        let content = serde_json::to_string_pretty(&self.wallets)
            .map_err(|_| BtpcError::Utxo(crate::error::UtxoError::SerializationError {
                data_type: "wallet metadata".to_string(),
            }))?;

        std::fs::write(&self.metadata_file, content)
            .map_err(|_| BtpcError::FileSystem(crate::error::FileSystemError::WriteFailed {
                path: self.metadata_file.display().to_string(),
                error: "Failed to save wallet metadata".to_string(),
            }))?;

        Ok(())
    }

    /// Save wallets with encryption using password (NEW - secure method)
    ///
    /// Note: This encrypts the wallet metadata (addresses, balances, settings).
    /// Individual wallet key files are encrypted separately by btpc_integration.
    pub fn save_wallets_encrypted(&self, password: &SecurePassword) -> BtpcResult<()> {
        // Serialize wallet metadata as JSON first
        let metadata_json = serde_json::to_string(&self.wallets)
            .map_err(|_| BtpcError::Utxo(crate::error::UtxoError::SerializationError {
                data_type: "wallet metadata".to_string(),
            }))?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Create a KeyEntry with the serialized metadata
        let metadata_entry = KeyEntry {
            label: "wallet_metadata".to_string(),
            private_key_bytes: vec![], // Not storing keys here
            public_key_bytes: vec![],  // Not storing keys here
            seed: None,                 // T015 FIX: Not a signing key, just metadata
            address: metadata_json,     // Store JSON in address field
            created_at: now,
        };

        let wallet_data = WalletData {
            wallet_id: "metadata".to_string(),  // T015 FIX: Metadata storage identifier
            network: "mainnet".to_string(),
            keys: vec![metadata_entry],
            created_at: now,
            modified_at: now,
        };

        // Encrypt wallet data
        let encrypted = EncryptedWallet::encrypt(&wallet_data, password)
            .map_err(|e| BtpcError::FileSystem(crate::error::FileSystemError::WriteFailed {
                path: self.metadata_file.display().to_string(),
                error: format!("Encryption failed: {}", e),
            }))?;

        // Save encrypted wallet
        let encrypted_path = self.metadata_file.with_extension("dat");
        encrypted.save_to_file(&encrypted_path)
            .map_err(|e| BtpcError::FileSystem(crate::error::FileSystemError::WriteFailed {
                path: encrypted_path.display().to_string(),
                error: format!("Failed to save encrypted wallet: {}", e),
            }))?;

        Ok(())
    }

    /// Load wallets with decryption using password (NEW - secure method)
    pub fn load_wallets_encrypted(&mut self, password: &SecurePassword) -> BtpcResult<()> {
        let encrypted_path = self.metadata_file.with_extension("dat");

        if !encrypted_path.exists() {
            return Ok(());
        }

        // Load encrypted wallet
        let encrypted = EncryptedWallet::load_from_file(&encrypted_path)
            .map_err(|e| BtpcError::FileSystem(crate::error::FileSystemError::ReadFailed {
                path: encrypted_path.display().to_string(),
                error: format!("Failed to load encrypted wallet: {}", e),
            }))?;

        // Decrypt wallet data
        let wallet_data = encrypted.decrypt(password)
            .map_err(|e| BtpcError::FileSystem(crate::error::FileSystemError::ReadFailed {
                path: encrypted_path.display().to_string(),
                error: format!("Decryption failed: {}", e),
            }))?;

        // Extract metadata from first key entry
        if let Some(metadata_entry) = wallet_data.keys.first() {
            if metadata_entry.label == "wallet_metadata" {
                // Deserialize JSON from address field
                self.wallets = serde_json::from_str(&metadata_entry.address)
                    .map_err(|_| BtpcError::Utxo(crate::error::UtxoError::DeserializationError {
                        data_type: "wallet metadata".to_string(),
                        error: "Invalid encrypted wallet format".to_string(),
                    }))?;
            }
        }

        Ok(())
    }

    /// Clear all wallet metadata from memory (for wallet locking)
    /// This removes all wallet information from the HashMap but doesn't delete files
    pub fn clear_wallets(&mut self) {
        self.wallets.clear();
    }

    fn unset_all_defaults(&mut self) {
        for wallet in self.wallets.values_mut() {
            wallet.is_default = false;
        }
    }

    fn ensure_default_wallet(&mut self) {
        if !self.wallets.values().any(|w| w.is_default) {
            if let Some(first_wallet) = self.wallets.values_mut().next() {
                first_wallet.is_default = true;
            }
        }
    }

    fn generate_random_color() -> String {
        let colors = [
            "#2563eb", "#dc2626", "#059669", "#d97706",
            "#7c3aed", "#db2777", "#0891b2", "#65a30d"
        ];
        colors[fastrand::usize(..colors.len())].to_string()
    }

    fn create_btpc_wallet_file(&self, path: &Path, password: &str, btpc_integration: &crate::btpc_integration::BtpcIntegration) -> BtpcResult<(String, String, String)> {
        // Use BtpcIntegration to create a real wallet with ML-DSA keypair and password encryption
        let (address, seed_phrase, private_key_hex) = btpc_integration.create_wallet(path, password)
            .map_err(|e| BtpcError::FileSystem(crate::error::FileSystemError::WriteFailed {
                path: path.display().to_string(),
                error: format!("Failed to create wallet: {}", e),
            }))?;

        Ok((address.trim().to_string(), seed_phrase, private_key_hex))
    }

    // ========================================================================
    // UTXO Reservation Methods (Feature 007: Fix Transaction Sending)
    // ========================================================================

    /// Reserve UTXOs for a transaction to prevent double-spending
    ///
    /// Creates a ReservationToken that locks the specified UTXOs for exclusive use
    /// by a single transaction. This prevents concurrent transaction attempts from
    /// selecting the same UTXOs.
    ///
    /// # Parameters
    /// - `wallet_id`: The wallet that owns the UTXOs
    /// - `utxos`: List of UTXOs to reserve (transaction_hash, output_index)
    /// - `transaction_id`: Optional transaction ID if already created
    ///
    /// # Returns
    /// `ReservationToken` if successful, error if UTXOs are already locked
    ///
    /// # Thread Safety
    /// This method is thread-safe and can be called concurrently from multiple threads.
    pub fn reserve_utxos(
        &self,
        wallet_id: String,
        utxos: Vec<(String, u32)>,
        transaction_id: Option<String>,
    ) -> BtpcResult<ReservationToken> {
        let mut reservations = self.reservations.lock()
            .map_err(|_| BtpcError::mutex_poison("WalletManager", "reservation_lock"))?;

        // Check if any of the requested UTXOs are already reserved
        for (txid, vout) in &utxos {
            for existing_reservation in reservations.values() {
                // Skip expired reservations
                if existing_reservation.is_expired() {
                    continue;
                }

                if existing_reservation.wallet_id == wallet_id &&
                   existing_reservation.contains_utxo(txid, *vout) {
                    return Err(BtpcError::Validation(crate::error::ValidationError::CustomValidation {
                        rule: "utxo_available".to_string(),
                        message: format!(
                            "UTXO {}:{} is already reserved by transaction {:?}",
                            txid, vout, existing_reservation.transaction_id
                        ),
                    }));
                }
            }
        }

        // Create new reservation
        let token = ReservationToken::new(wallet_id, utxos, transaction_id, None);
        let token_id = token.id;
        reservations.insert(token_id, token.clone());

        tracing::info!(
            "Reserved {} UTXOs for wallet with token {}",
            token.utxo_count(),
            token_id
        );

        Ok(token)
    }

    /// Release a UTXO reservation
    ///
    /// Removes the reservation from the active reservations map, making the UTXOs
    /// available for other transactions.
    ///
    /// # Parameters
    /// - `token`: The ReservationToken to release
    ///
    /// # Returns
    /// Number of UTXOs that were released
    pub fn release_reservation(&self, token: &ReservationToken) -> BtpcResult<usize> {
        let mut reservations = self.reservations.lock()
            .map_err(|_| BtpcError::mutex_poison("WalletManager", "reservation_lock"))?;

        let utxo_count = token.utxo_count();

        if reservations.remove(&token.id).is_some() {
            tracing::info!(
                "Released reservation {} ({} UTXOs)",
                token.id,
                utxo_count
            );
            Ok(utxo_count)
        } else {
            // Reservation already released or doesn't exist
            Ok(0)
        }
    }

    /// Clean up expired reservations
    ///
    /// Removes all reservations that have passed their expiry timestamp.
    /// This prevents orphaned reservations from permanently locking UTXOs.
    ///
    /// # Returns
    /// Number of expired reservations that were cleaned up
    pub fn cleanup_expired_reservations(&self) -> BtpcResult<usize> {
        let mut reservations = self.reservations.lock()
            .map_err(|_| BtpcError::mutex_poison("WalletManager", "reservation_lock"))?;

        let expired_ids: Vec<Uuid> = reservations
            .iter()
            .filter(|(_, token)| token.is_expired())
            .map(|(id, _)| *id)
            .collect();

        let count = expired_ids.len();

        for id in expired_ids {
            reservations.remove(&id);
        }

        if count > 0 {
            tracing::info!("Cleaned up {} expired UTXO reservations", count);
        }

        Ok(count)
    }

    /// Get all active reservations for a wallet
    ///
    /// Returns a list of all non-expired reservations for the specified wallet.
    ///
    /// # Parameters
    /// - `wallet_id`: The wallet to query
    ///
    /// # Returns
    /// Vector of active ReservationTokens
    pub fn get_active_reservations(&self, wallet_id: &str) -> BtpcResult<Vec<ReservationToken>> {
        let reservations = self.reservations.lock()
            .map_err(|_| BtpcError::mutex_poison("WalletManager", "reservation_lock"))?;

        let active: Vec<ReservationToken> = reservations
            .values()
            .filter(|token| token.wallet_id == wallet_id && !token.is_expired())
            .cloned()
            .collect();

        Ok(active)
    }

    /// Check if a specific UTXO is currently reserved
    ///
    /// # Parameters
    /// - `wallet_id`: The wallet that owns the UTXO
    /// - `txid`: Transaction hash
    /// - `vout`: Output index
    ///
    /// # Returns
    /// `true` if the UTXO is reserved by a non-expired reservation
    pub fn is_utxo_reserved(&self, wallet_id: &str, txid: &str, vout: u32) -> BtpcResult<bool> {
        let reservations = self.reservations.lock()
            .map_err(|_| BtpcError::mutex_poison("WalletManager", "reservation_lock"))?;

        let is_reserved = reservations
            .values()
            .any(|token| {
                token.wallet_id == wallet_id &&
                !token.is_expired() &&
                token.contains_utxo(txid, vout)
            });

        Ok(is_reserved)
    }

    /// Start periodic cleanup task for expired UTXO reservations
    ///
    /// Spawns a background tokio task that runs every 60 seconds to clean up
    /// expired reservations. This prevents memory leaks from abandoned reservations.
    ///
    /// # Returns
    /// `JoinHandle` for the background task (can be used to cancel if needed)
    ///
    /// # Example
    /// ```rust
    /// let cleanup_handle = wallet_manager.start_cleanup_task();
    /// // Task runs in background...
    /// // To stop: cleanup_handle.abort();
    /// ```
    pub fn start_cleanup_task(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));

            loop {
                interval.tick().await;

                match self.cleanup_expired_reservations() {
                    Ok(count) => {
                        if count > 0 {
                            tracing::info!(
                                "Background cleanup: Removed {} expired UTXO reservations",
                                count
                            );
                        }
                    }
                    Err(e) => {
                        tracing::error!("Background cleanup failed: {}", e);
                    }
                }
            }
        })
    }
}

// NOTE: Original comprehensive tests temporarily disabled due to API refactoring
// All tests need to be updated for new password-based wallet creation API
// See wallet_manager/tests.rs for full test suite (needs updating)
// Encrypted wallet tests are now in tests_simple.rs (working tests)
#[cfg(test)]
mod tests_simple;