//! Authentication State Management
//!
//! This module manages authentication session state and master credentials storage.
//! It implements the data model defined in specs/006-add-application-level/data-model.md.
//!
//! # Features
//!
//! - SessionState: In-memory authentication status (Arc<RwLock>)
//! - MasterCredentials: Encrypted credential file storage
//! - Binary serialization format with magic bytes and versioning
//!
//! # Article XI Compliance
//!
//! - Backend is single source of truth for authentication state (Section 11.1)
//! - No frontend state synchronization issues
//! - Thread-safe Arc<RwLock> for concurrent access

use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// Magic bytes for credentials file format identification
pub const MAGIC_BYTES: [u8; 4] = *b"BTPC";

/// Current file format version
pub const FILE_VERSION: u32 = 1;

/// Error type for state management operations
#[derive(Debug)]
pub enum StateError {
    IoError(std::io::Error),
    SerializationError(String),
    InvalidFormat(String),
    PermissionError(String),
}

impl std::fmt::Display for StateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StateError::IoError(e) => write!(f, "I/O error: {}", e),
            StateError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            StateError::InvalidFormat(msg) => write!(f, "Invalid format: {}", msg),
            StateError::PermissionError(msg) => write!(f, "Permission error: {}", msg),
        }
    }
}

impl std::error::Error for StateError {}

impl From<std::io::Error> for StateError {
    fn from(err: std::io::Error) -> Self {
        StateError::IoError(err)
    }
}

// ============================================================================
// T034: SessionState (In-Memory State)
// ============================================================================

/// In-memory authentication session state
///
/// # Article XI Compliance
/// - Backend Arc<RwLock<SessionState>> is single source of truth (Section 11.1)
/// - Thread-safe for concurrent access
/// - Never serialized to disk or localStorage
///
/// # State Transitions
/// ```text
/// Initial -> Not Authenticated (authenticated=false)
///   ↓ (user logs in successfully)
/// Authenticated (authenticated=true, timestamp set)
///   ↓ (user logs out)
/// Not Authenticated (authenticated=false)
/// ```
#[derive(Debug, Clone)]
pub struct SessionState {
    /// Is user currently logged in?
    pub authenticated: bool,

    /// When did the user log in? (None if not authenticated)
    pub login_timestamp: Option<u64>,

    /// Unique session token (UUID v4, None if not authenticated)
    pub session_token: Option<String>,
}

impl SessionState {
    /// Creates a new unauthenticated session state
    pub fn new() -> Self {
        SessionState {
            authenticated: false,
            login_timestamp: None,
            session_token: None,
        }
    }

    /// Marks the session as authenticated and generates a new session token
    pub fn login(&mut self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.authenticated = true;
        self.login_timestamp = Some(now);
        self.session_token = Some(Uuid::new_v4().to_string());
    }

    /// Marks the session as unauthenticated and clears session data
    pub fn logout(&mut self) {
        self.authenticated = false;
        self.login_timestamp = None;
        self.session_token = None;
    }

    /// Checks if the session is currently authenticated
    pub fn is_authenticated(&self) -> bool {
        self.authenticated
    }

    /// Gets the session token if authenticated
    pub fn get_session_token(&self) -> Option<String> {
        self.session_token.clone()
    }
}

impl Default for SessionState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// T035: MasterCredentials (Persistent Storage)
// ============================================================================

/// Persistent master password credentials stored in encrypted file
///
/// # Binary File Format
/// Per data-model.md specification:
/// ```text
/// [0-3]   Magic bytes (b"BTPC")
/// [4-7]   Version (u32 LE)
/// [8-11]  Argon2 memory KB (u32 LE)
/// [12-15] Argon2 iterations (u32 LE)
/// [16-19] Argon2 parallelism (u32 LE)
/// [20-35] Argon2 salt (16 bytes)
/// [36-39] Encrypted hash length (u32 LE)
/// [40-X]  Encrypted password hash (variable)
/// [X-X+11] AES nonce (12 bytes)
/// [X+12-X+27] AES tag (16 bytes)
/// [X+28-X+35] Created at (u64 LE)
/// [X+36-X+43] Last used at (u64 LE)
/// ```
///
/// # Security
/// - File permissions: 0600 (Unix) or restricted ACL (Windows)
/// - AES-256-GCM encrypted password hash
/// - Argon2id salt stored for verification
#[derive(Debug)]
pub struct MasterCredentials {
    /// File format version
    pub version: u32,

    /// Argon2id parameters
    pub argon2_memory_kb: u32,
    pub argon2_iterations: u32,
    pub argon2_parallelism: u32,
    pub argon2_salt: [u8; 16],

    /// AES-256-GCM encrypted password hash
    pub encrypted_password_hash: Vec<u8>,
    pub aes_nonce: [u8; 12],
    pub aes_tag: [u8; 16],

    /// Metadata
    pub created_at: u64,
    pub last_used_at: u64,
}

impl MasterCredentials {
    /// Creates new master credentials with current timestamp
    pub fn new(
        argon2_memory_kb: u32,
        argon2_iterations: u32,
        argon2_parallelism: u32,
        argon2_salt: [u8; 16],
        encrypted_password_hash: Vec<u8>,
        aes_nonce: [u8; 12],
        aes_tag: [u8; 16],
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        MasterCredentials {
            version: FILE_VERSION,
            argon2_memory_kb,
            argon2_iterations,
            argon2_parallelism,
            argon2_salt,
            encrypted_password_hash,
            aes_nonce,
            aes_tag,
            created_at: now,
            last_used_at: now,
        }
    }

    /// Updates the last_used_at timestamp to current time
    pub fn update_last_used(&mut self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.last_used_at = now;
    }

    // ========================================================================
    // T036: Binary Serialization/Deserialization
    // ========================================================================

    /// Serializes credentials to binary format per data-model.md specification
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // [0-3] Magic bytes
        bytes.extend_from_slice(&MAGIC_BYTES);

        // [4-7] Version (u32 LE)
        bytes.extend_from_slice(&self.version.to_le_bytes());

        // [8-11] Argon2 memory KB (u32 LE)
        bytes.extend_from_slice(&self.argon2_memory_kb.to_le_bytes());

        // [12-15] Argon2 iterations (u32 LE)
        bytes.extend_from_slice(&self.argon2_iterations.to_le_bytes());

        // [16-19] Argon2 parallelism (u32 LE)
        bytes.extend_from_slice(&self.argon2_parallelism.to_le_bytes());

        // [20-35] Argon2 salt (16 bytes)
        bytes.extend_from_slice(&self.argon2_salt);

        // [36-39] Encrypted hash length (u32 LE)
        bytes.extend_from_slice(&(self.encrypted_password_hash.len() as u32).to_le_bytes());

        // [40-X] Encrypted password hash (variable)
        bytes.extend_from_slice(&self.encrypted_password_hash);

        // [X-X+11] AES nonce (12 bytes)
        bytes.extend_from_slice(&self.aes_nonce);

        // [X+12-X+27] AES tag (16 bytes)
        bytes.extend_from_slice(&self.aes_tag);

        // [X+28-X+35] Created at (u64 LE)
        bytes.extend_from_slice(&self.created_at.to_le_bytes());

        // [X+36-X+43] Last used at (u64 LE)
        bytes.extend_from_slice(&self.last_used_at.to_le_bytes());

        bytes
    }

    /// Deserializes credentials from binary format with validation
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, StateError> {
        if bytes.len() < 44 {
            return Err(StateError::InvalidFormat(
                "File too short (minimum 44 bytes)".to_string(),
            ));
        }

        // [0-3] Magic bytes
        if &bytes[0..4] != &MAGIC_BYTES {
            return Err(StateError::InvalidFormat("Invalid magic bytes".to_string()));
        }

        // [4-7] Version
        let version = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        if version != FILE_VERSION {
            return Err(StateError::InvalidFormat(format!(
                "Unsupported version: {}",
                version
            )));
        }

        // [8-11] Argon2 memory KB
        let argon2_memory_kb = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);

        // [12-15] Argon2 iterations
        let argon2_iterations = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);

        // [16-19] Argon2 parallelism
        let argon2_parallelism = u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);

        // [20-35] Argon2 salt
        let mut argon2_salt = [0u8; 16];
        argon2_salt.copy_from_slice(&bytes[20..36]);

        // [36-39] Encrypted hash length
        let hash_len = u32::from_le_bytes([bytes[36], bytes[37], bytes[38], bytes[39]]) as usize;

        // Validate remaining length
        if bytes.len() < 40 + hash_len + 12 + 16 + 8 + 8 {
            return Err(StateError::InvalidFormat(
                "File truncated (incomplete data)".to_string(),
            ));
        }

        // [40-X] Encrypted password hash
        let encrypted_password_hash = bytes[40..40 + hash_len].to_vec();

        // [X-X+11] AES nonce
        let mut aes_nonce = [0u8; 12];
        aes_nonce.copy_from_slice(&bytes[40 + hash_len..40 + hash_len + 12]);

        // [X+12-X+27] AES tag
        let mut aes_tag = [0u8; 16];
        aes_tag.copy_from_slice(&bytes[40 + hash_len + 12..40 + hash_len + 12 + 16]);

        // [X+28-X+35] Created at
        let created_at_start = 40 + hash_len + 12 + 16;
        let created_at = u64::from_le_bytes([
            bytes[created_at_start],
            bytes[created_at_start + 1],
            bytes[created_at_start + 2],
            bytes[created_at_start + 3],
            bytes[created_at_start + 4],
            bytes[created_at_start + 5],
            bytes[created_at_start + 6],
            bytes[created_at_start + 7],
        ]);

        // [X+36-X+43] Last used at
        let last_used_at_start = created_at_start + 8;
        let last_used_at = u64::from_le_bytes([
            bytes[last_used_at_start],
            bytes[last_used_at_start + 1],
            bytes[last_used_at_start + 2],
            bytes[last_used_at_start + 3],
            bytes[last_used_at_start + 4],
            bytes[last_used_at_start + 5],
            bytes[last_used_at_start + 6],
            bytes[last_used_at_start + 7],
        ]);

        Ok(MasterCredentials {
            version,
            argon2_memory_kb,
            argon2_iterations,
            argon2_parallelism,
            argon2_salt,
            encrypted_password_hash,
            aes_nonce,
            aes_tag,
            created_at,
            last_used_at,
        })
    }

    /// Saves credentials to file with proper permissions (0600 on Unix)
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), StateError> {
        let path = path.as_ref();

        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Serialize to bytes
        let bytes = self.to_bytes();

        // Write to file
        let mut file = File::create(path)?;
        file.write_all(&bytes)?;
        file.sync_all()?;

        // Set file permissions to 0600 (owner read/write only) on Unix
        #[cfg(unix)]
        {
            let mut permissions = file.metadata()?.permissions();
            permissions.set_mode(0o600);
            fs::set_permissions(path, permissions)?;
        }

        // On Windows, the file inherits %APPDATA% directory ACLs which should
        // already be restricted to the current user

        Ok(())
    }

    /// Loads credentials from file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, StateError> {
        let mut file = File::open(path.as_ref())?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;

        Self::from_bytes(&bytes)
    }

    /// Checks if credentials file exists
    pub fn exists<P: AsRef<Path>>(path: P) -> bool {
        path.as_ref().exists()
    }
}

/// Gets the default credentials file path (~/.btpc/credentials.enc)
pub fn get_credentials_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".btpc").join("credentials.enc")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_state_lifecycle() {
        let mut state = SessionState::new();
        assert!(!state.is_authenticated());
        assert!(state.get_session_token().is_none());

        state.login();
        assert!(state.is_authenticated());
        assert!(state.get_session_token().is_some());

        state.logout();
        assert!(!state.is_authenticated());
        assert!(state.get_session_token().is_none());
    }

    #[test]
    fn test_credentials_serialization_round_trip() {
        let creds = MasterCredentials::new(
            65536,
            3,
            4,
            [1u8; 16],
            vec![42u8; 32],
            [2u8; 12],
            [3u8; 16],
        );

        let bytes = creds.to_bytes();
        let loaded = MasterCredentials::from_bytes(&bytes).unwrap();

        assert_eq!(loaded.version, creds.version);
        assert_eq!(loaded.argon2_memory_kb, creds.argon2_memory_kb);
        assert_eq!(loaded.argon2_salt, creds.argon2_salt);
        assert_eq!(loaded.encrypted_password_hash, creds.encrypted_password_hash);
    }

    #[test]
    fn test_credentials_invalid_magic_bytes() {
        let mut bytes = vec![0u8; 100];
        bytes[0..4].copy_from_slice(b"FAKE");

        let result = MasterCredentials::from_bytes(&bytes);
        assert!(result.is_err());
    }
}