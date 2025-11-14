//! BTPC Desktop Application Security Module
//!
//! This module provides secure authentication, session management, and key recovery
//! functionality for the BTPC desktop application.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::{rand_core::OsRng, SaltString};
use aes_gcm::{Aes256Gcm, Nonce, KeyInit};
use aes_gcm::aead::Aead;
use bip39::{Mnemonic, Language};
use rand::Rng;
use sha2::{Sha256, Digest};

// ============================================================================
// Security Configuration
// ============================================================================

const SESSION_TIMEOUT_MINUTES: u64 = 30;
const MAX_LOGIN_ATTEMPTS: u32 = 3;
const LOCKOUT_DURATION_MINUTES: u64 = 15;

// ============================================================================
// Data Structures
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserCredentials {
    pub username: String,
    pub password_hash: String,
    pub salt: String,
    pub recovery_hash: String, // Hash of recovery phrase
    pub created_at: u64,
    pub last_login: Option<u64>,
    pub login_attempts: u32,
    pub locked_until: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSession {
    pub username: String,
    pub session_id: String,
    pub created_at: u64,
    pub last_activity: u64,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryData {
    pub username: String,
    pub encrypted_wallet_key: String,
    pub recovery_phrase: String, // This will be shown only during creation
    pub created_at: u64,
}

// ============================================================================
// Security Manager
// ============================================================================

#[derive(Clone)]
pub struct SecurityManager {
    config_dir: PathBuf,
    sessions: Arc<Mutex<HashMap<String, UserSession>>>,
    lockout_tracker: Arc<Mutex<HashMap<String, (u32, u64)>>>, // (attempts, lockout_until)
}

impl SecurityManager {
    pub fn new(config_dir: PathBuf) -> Self {
        // Ensure security directory exists
        let security_dir = config_dir.join("security");
        if !security_dir.exists() {
            let _ = fs::create_dir_all(&security_dir);
        }

        Self {
            config_dir,
            sessions: Arc::new(Mutex::new(HashMap::new())),
            lockout_tracker: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    // ========================================================================
    // User Management
    // ========================================================================

    /// Create a new user account with recovery phrase
    pub fn create_user(&self, username: &str, password: &str) -> Result<RecoveryData> {
        let credentials_path = self.get_credentials_path(username);

        // Check if user already exists
        if credentials_path.exists() {
            return Err(anyhow!("User '{}' already exists", username));
        }

        // Generate secure password hash with Argon2
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2.hash_password(password.as_bytes(), &salt)
            .map_err(|e| anyhow!("Password hashing failed: {}", e))?
            .to_string();

        // Generate BIP39 mnemonic for recovery
        let entropy = rand::random::<[u8; 32]>();
        let mnemonic = Mnemonic::from_entropy_in(Language::English, &entropy)
            .map_err(|e| anyhow!("Mnemonic generation failed: {}", e))?;

        let recovery_phrase = mnemonic.to_string();
        let recovery_hash = self.hash_recovery_phrase(&recovery_phrase)?;

        // Create user credentials
        let credentials = UserCredentials {
            username: username.to_string(),
            password_hash,
            salt: salt.to_string(),
            recovery_hash,
            created_at: self.current_timestamp(),
            last_login: None,
            login_attempts: 0,
            locked_until: None,
        };

        // Save credentials securely
        self.save_credentials(&credentials)?;

        // Generate encrypted wallet key using recovery phrase as seed
        let wallet_key = self.generate_wallet_key_from_recovery(&recovery_phrase)?;
        let encrypted_wallet_key = self.encrypt_wallet_key(&wallet_key, password)?;

        Ok(RecoveryData {
            username: username.to_string(),
            encrypted_wallet_key,
            recovery_phrase: recovery_phrase.to_string(),
            created_at: self.current_timestamp(),
        })
    }

    /// Authenticate user login
    pub fn authenticate_user(&self, username: &str, password: &str) -> Result<UserSession> {
        // Check for account lockout
        if self.is_account_locked(username)? {
            return Err(anyhow!("Account is temporarily locked due to too many failed attempts"));
        }

        let credentials_path = self.get_credentials_path(username);
        if !credentials_path.exists() {
            self.record_failed_attempt(username);
            return Err(anyhow!("Invalid username or password"));
        }

        // Load and verify credentials
        let mut credentials: UserCredentials = {
            let data = fs::read_to_string(&credentials_path)
                .map_err(|e| anyhow!("Failed to read credentials: {}", e))?;
            serde_json::from_str(&data)
                .map_err(|e| anyhow!("Failed to parse credentials: {}", e))?
        };

        // Verify password using Argon2
        let parsed_hash = PasswordHash::new(&credentials.password_hash)
            .map_err(|e| anyhow!("Invalid password hash: {}", e))?;

        let argon2 = Argon2::default();
        if argon2.verify_password(password.as_bytes(), &parsed_hash).is_err() {
            self.record_failed_attempt(username);
            credentials.login_attempts += 1;
            self.save_credentials(&credentials)?;
            return Err(anyhow!("Invalid username or password"));
        }

        // Reset failed attempts on successful login
        self.clear_failed_attempts(username);
        credentials.login_attempts = 0;
        credentials.last_login = Some(self.current_timestamp());
        self.save_credentials(&credentials)?;

        // Create new session
        let session = self.create_session(username)?;
        Ok(session)
    }

    /// Recover account using mnemonic phrase
    pub fn recover_account(&self, username: &str, recovery_phrase: &str, new_password: &str) -> Result<String> {
        let credentials_path = self.get_credentials_path(username);
        if !credentials_path.exists() {
            return Err(anyhow!("User '{}' does not exist", username));
        }

        // Load existing credentials
        let mut credentials: UserCredentials = {
            let data = fs::read_to_string(&credentials_path)
                .map_err(|e| anyhow!("Failed to read credentials: {}", e))?;
            serde_json::from_str(&data)
                .map_err(|e| anyhow!("Failed to parse credentials: {}", e))?
        };

        // Verify recovery phrase
        let recovery_hash = self.hash_recovery_phrase(recovery_phrase)?;
        if recovery_hash != credentials.recovery_hash {
            return Err(anyhow!("Invalid recovery phrase"));
        }

        // Generate new password hash
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let new_password_hash = argon2.hash_password(new_password.as_bytes(), &salt)
            .map_err(|e| anyhow!("Password hashing failed: {}", e))?
            .to_string();

        // Update credentials
        credentials.password_hash = new_password_hash;
        credentials.salt = salt.to_string();
        credentials.login_attempts = 0;
        credentials.locked_until = None;

        // Save updated credentials
        self.save_credentials(&credentials)?;

        // Clear any existing sessions for security
        self.invalidate_user_sessions(username);

        Ok("Password successfully reset using recovery phrase".to_string())
    }

    // ========================================================================
    // Session Management
    // ========================================================================

    /// Create a new session for authenticated user
    pub fn create_session(&self, username: &str) -> Result<UserSession> {
        let session_id = self.generate_session_id();
        let timestamp = self.current_timestamp();

        let session = UserSession {
            username: username.to_string(),
            session_id: session_id.clone(),
            created_at: timestamp,
            last_activity: timestamp,
            is_active: true,
        };

        // Store session
        {
            let mut sessions = self.sessions.lock().unwrap();
            sessions.insert(session_id, session.clone());
        }

        Ok(session)
    }

    /// Validate session and update activity
    pub fn validate_session(&self, session_id: &str) -> Result<bool> {
        let mut sessions = self.sessions.lock().unwrap();

        if let Some(session) = sessions.get_mut(session_id) {
            let now = self.current_timestamp();
            let session_age = now - session.created_at;

            // Check session timeout
            if session_age > SESSION_TIMEOUT_MINUTES * 60 {
                session.is_active = false;
                return Ok(false);
            }

            // Update last activity
            session.last_activity = now;
            Ok(session.is_active)
        } else {
            Ok(false)
        }
    }

    /// Get session information
    pub fn get_session(&self, session_id: &str) -> Option<UserSession> {
        let sessions = self.sessions.lock().unwrap();
        sessions.get(session_id).cloned()
    }

    /// Invalidate a specific session
    pub fn invalidate_session(&self, session_id: &str) {
        let mut sessions = self.sessions.lock().unwrap();
        sessions.remove(session_id);
    }

    /// Invalidate all sessions for a user
    pub fn invalidate_user_sessions(&self, username: &str) {
        let mut sessions = self.sessions.lock().unwrap();
        sessions.retain(|_, session| session.username != username);
    }

    // ========================================================================
    // Key Recovery and Encryption
    // ========================================================================

    /// Generate wallet key from recovery phrase
    pub fn generate_wallet_key_from_recovery(&self, recovery_phrase: &str) -> Result<String> {
        // Validate mnemonic
        let _mnemonic = Mnemonic::parse_in(Language::English, recovery_phrase)
            .map_err(|e| anyhow!("Invalid recovery phrase: {}", e))?;

        // Generate deterministic wallet key from mnemonic seed
        let mut hasher = Sha256::new();
        hasher.update(recovery_phrase.as_bytes());
        hasher.update(b"BTPC_WALLET_KEY_DERIVATION");
        let wallet_key = format!("{:x}", hasher.finalize());

        Ok(wallet_key)
    }

    /// Encrypt wallet key with user password
    pub fn encrypt_wallet_key(&self, wallet_key: &str, password: &str) -> Result<String> {
        // Generate key from password
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        hasher.update(b"BTPC_WALLET_ENCRYPTION_SALT");
        let key_bytes = hasher.finalize();
        let key_array: &[u8; 32] = key_bytes.as_slice().try_into()
            .map_err(|_| anyhow!("Invalid key length"))?;
        let key = aes_gcm::Key::<Aes256Gcm>::from_slice(key_array);

        // Generate random nonce
        let nonce_bytes: [u8; 12] = rand::thread_rng().gen();
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt wallet key
        let cipher = Aes256Gcm::new(key);
        let ciphertext = cipher.encrypt(nonce, wallet_key.as_bytes())
            .map_err(|e| anyhow!("Encryption failed: {}", e))?;

        // Combine nonce + ciphertext and encode as base64
        let mut encrypted_data = nonce_bytes.to_vec();
        encrypted_data.extend_from_slice(&ciphertext);

        use base64::prelude::*;
        Ok(BASE64_STANDARD.encode(encrypted_data))
    }

    /// Decrypt wallet key with user password
    pub fn decrypt_wallet_key(&self, encrypted_key: &str, password: &str) -> Result<String> {
        // Decode base64
        use base64::prelude::*;
        let encrypted_data = BASE64_STANDARD.decode(encrypted_key)
            .map_err(|e| anyhow!("Invalid encrypted data: {}", e))?;

        if encrypted_data.len() < 12 {
            return Err(anyhow!("Invalid encrypted data length"));
        }

        // Split nonce and ciphertext
        let (nonce_bytes, ciphertext) = encrypted_data.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        // Generate key from password
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        hasher.update(b"BTPC_WALLET_ENCRYPTION_SALT");
        let key_bytes = hasher.finalize();
        let key_array: &[u8; 32] = key_bytes.as_slice().try_into()
            .map_err(|_| anyhow!("Invalid key length"))?;
        let key = aes_gcm::Key::<Aes256Gcm>::from_slice(key_array);

        // Decrypt wallet key
        let cipher = Aes256Gcm::new(key);
        let plaintext = cipher.decrypt(nonce, ciphertext)
            .map_err(|e| anyhow!("Decryption failed: {}", e))?;

        String::from_utf8(plaintext)
            .map_err(|e| anyhow!("Invalid decrypted data: {}", e))
    }

    // ========================================================================
    // Helper Methods
    // ========================================================================

    fn get_credentials_path(&self, username: &str) -> PathBuf {
        self.config_dir.join("security").join(format!("{}.json", username))
    }

    fn save_credentials(&self, credentials: &UserCredentials) -> Result<()> {
        let path = self.get_credentials_path(&credentials.username);
        let data = serde_json::to_string_pretty(credentials)
            .map_err(|e| anyhow!("Failed to serialize credentials: {}", e))?;

        fs::write(path, data)
            .map_err(|e| anyhow!("Failed to save credentials: {}", e))?;

        Ok(())
    }

    fn hash_recovery_phrase(&self, phrase: &str) -> Result<String> {
        let mut hasher = Sha256::new();
        hasher.update(phrase.as_bytes());
        hasher.update(b"BTPC_RECOVERY_SALT");
        Ok(format!("{:x}", hasher.finalize()))
    }

    fn generate_session_id(&self) -> String {
        let random_bytes: [u8; 32] = rand::thread_rng().gen();
        format!("{:x}", Sha256::digest(random_bytes))
    }

    fn current_timestamp(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    fn is_account_locked(&self, username: &str) -> Result<bool> {
        let lockout_tracker = self.lockout_tracker.lock().unwrap();
        if let Some((attempts, lockout_until)) = lockout_tracker.get(username) {
            let now = self.current_timestamp();
            if *attempts >= MAX_LOGIN_ATTEMPTS && *lockout_until > now {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn record_failed_attempt(&self, username: &str) {
        let mut lockout_tracker = self.lockout_tracker.lock().unwrap();
        let now = self.current_timestamp();

        let entry = lockout_tracker.entry(username.to_string()).or_insert((0, 0));
        entry.0 += 1;

        if entry.0 >= MAX_LOGIN_ATTEMPTS {
            entry.1 = now + LOCKOUT_DURATION_MINUTES * 60;
        }
    }

    fn clear_failed_attempts(&self, username: &str) {
        let mut lockout_tracker = self.lockout_tracker.lock().unwrap();
        lockout_tracker.remove(username);
    }

    /// Check if user exists
    pub fn user_exists(&self, username: &str) -> bool {
        self.get_credentials_path(username).exists()
    }

    /// Get list of existing users
    pub fn get_users(&self) -> Result<Vec<String>> {
        let security_dir = self.config_dir.join("security");
        if !security_dir.exists() {
            return Ok(Vec::new());
        }

        let mut users = Vec::new();
        for entry in fs::read_dir(security_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    users.push(stem.to_string());
                }
            }
        }

        Ok(users)
    }
}