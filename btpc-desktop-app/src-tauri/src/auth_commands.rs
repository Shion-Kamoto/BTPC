//! Tauri Commands for Authentication
//!
//! This module implements the Tauri command handlers for authentication operations.
//! It follows the contracts defined in specs/006-add-application-level/contracts/tauri-auth-commands.md.
//!
//! # Commands
//!
//! - `has_master_password`: Check if credentials file exists (first launch detection)
//! - `create_master_password`: Create master password on first launch
//! - `login`: Authenticate user with existing master password
//! - `logout`: End user session
//! - `check_session`: Check if user is currently authenticated (navigation guard)
//!
//! # Events
//!
//! - `session:login`: Emitted after successful authentication
//! - `session:logout`: Emitted after session termination
//!
//! # Article XI Compliance
//!
//! - Backend-first validation (Section 11.2)
//! - Event-driven UI updates (Section 11.3)
//! - No localStorage for authentication state (Section 11.5)
//!
//! # Security
//!
//! - Constant-time password comparison
//! - No password logging (NFR-004)
//! - Consistent error messages to prevent timing attacks

use crate::auth_crypto::{
    constant_time_compare, decrypt_aes_gcm, derive_key_argon2id, encrypt_aes_gcm,
    generate_random_nonce, generate_random_salt, AES_KEY_SIZE, AES_NONCE_SIZE, AES_TAG_SIZE,
    ARGON2_ITERATIONS, ARGON2_MEMORY_KB, ARGON2_PARALLELISM, SALT_SIZE,
};
use crate::auth_state::{get_credentials_path, MasterCredentials, SessionState};
use serde::{Deserialize, Serialize};
use std::sync::RwLock;
use std::time::SystemTime;
use tauri::{AppHandle, Emitter, State};
use zeroize::Zeroizing;

// ============================================================================
// Response Types (Per Contract Specification)
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct CreatePasswordResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LogoutResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CheckSessionResponse {
    pub authenticated: bool,
    pub session_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HasMasterPasswordResponse {
    pub exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionLoginEvent {
    pub session_token: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionLogoutEvent {
    pub timestamp: u64,
}

// ============================================================================
// T037: has_master_password Command
// ============================================================================

/// Checks if master password exists (first launch detection)
///
/// # Contract
/// - Read-only operation
/// - Checks ~/.btpc/credentials.enc file existence
/// - No side effects
///
/// # Returns
/// `{ exists: boolean }`
#[tauri::command(rename_all = "snake_case")]
pub fn has_master_password() -> HasMasterPasswordResponse {
    let creds_path = get_credentials_path();
    HasMasterPasswordResponse {
        exists: MasterCredentials::exists(&creds_path),
    }
}

// ============================================================================
// T038: create_master_password Command
// ============================================================================

/// Creates master password on first launch
///
/// # Contract
/// - Input: `{ password: string, password_confirm: string }`
/// - Validation: password.length >= 8, password == password_confirm
/// - Side Effects: Creates ~/.btpc/credentials.enc, emits session:login, updates SessionState
///
/// # Security
/// - Argon2id KDF (64MB, 3 iter, 4 par)
/// - AES-256-GCM encryption of password hash
/// - File permissions: 0600 (Unix)
///
/// # Returns
/// Success: `{ success: true, message: "Master password created successfully" }`
/// Error: `{ success: false, error: "PASSWORD_TOO_SHORT" | "PASSWORDS_DONT_MATCH" | "CREDENTIALS_ALREADY_EXIST" | "ENCRYPTION_FAILED" }`
#[tauri::command(rename_all = "snake_case")]
pub fn create_master_password(
    app: AppHandle,
    session: State<RwLock<SessionState>>,
    password: String,
    password_confirm: String,
) -> CreatePasswordResponse {
    // Validation: password length >= 8
    if password.len() < 8 {
        return CreatePasswordResponse {
            success: false,
            message: None,
            error: Some("PASSWORD_TOO_SHORT".to_string()),
        };
    }

    // Validation: password == password_confirm
    if password != password_confirm {
        return CreatePasswordResponse {
            success: false,
            message: None,
            error: Some("PASSWORDS_DONT_MATCH".to_string()),
        };
    }

    // Check if credentials already exist
    let creds_path = get_credentials_path();
    if MasterCredentials::exists(&creds_path) {
        return CreatePasswordResponse {
            success: false,
            message: None,
            error: Some("CREDENTIALS_ALREADY_EXIST".to_string()),
        };
    }

    // Generate salt and derive key from password using Argon2id
    let salt = match generate_random_salt() {
        Ok(s) => s,
        Err(_) => {
            return CreatePasswordResponse {
                success: false,
                message: None,
                error: Some("ENCRYPTION_FAILED".to_string()),
            }
        }
    };

    let derived_key = match derive_key_argon2id(&password, &salt) {
        Ok(k) => k,
        Err(_) => {
            return CreatePasswordResponse {
                success: false,
                message: None,
                error: Some("ENCRYPTION_FAILED".to_string()),
            }
        }
    };

    // Create password hash (we'll store the derived key itself as the "hash")
    let password_hash = derived_key.as_ref().to_vec();

    // Generate nonce for AES-GCM encryption
    let nonce = match generate_random_nonce() {
        Ok(n) => n,
        Err(_) => {
            return CreatePasswordResponse {
                success: false,
                message: None,
                error: Some("ENCRYPTION_FAILED".to_string()),
            }
        }
    };

    // Encrypt the password hash with AES-256-GCM
    // We use a simple key derivation: SHA-256 hash of the password as encryption key
    use sha2::{Digest, Sha256};
    let encryption_key_material = Zeroizing::new(format!("{}:encryption", password));
    let mut hasher = Sha256::new();
    hasher.update(encryption_key_material.as_bytes());
    let encryption_key_vec = hasher.finalize();
    let mut encryption_key = [0u8; AES_KEY_SIZE];
    encryption_key.copy_from_slice(&encryption_key_vec[..AES_KEY_SIZE]);

    let (encrypted_hash, aes_tag) = match encrypt_aes_gcm(&password_hash, &encryption_key, &nonce)
    {
        Ok(result) => result,
        Err(_) => {
            return CreatePasswordResponse {
                success: false,
                message: None,
                error: Some("ENCRYPTION_FAILED".to_string()),
            }
        }
    };

    // Create MasterCredentials struct
    let credentials = MasterCredentials::new(
        ARGON2_MEMORY_KB,
        ARGON2_ITERATIONS,
        ARGON2_PARALLELISM,
        salt,
        encrypted_hash,
        nonce,
        aes_tag,
    );

    // Save to file
    if let Err(_) = credentials.save_to_file(&creds_path) {
        return CreatePasswordResponse {
            success: false,
            message: None,
            error: Some("ENCRYPTION_FAILED".to_string()),
        };
    }

    // Update session state (login)
    {
        let mut state = session.write().unwrap();
        state.login();

        // Emit session:login event
        let timestamp = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let event = SessionLoginEvent {
            session_token: state.get_session_token().unwrap(),
            timestamp,
        };
        let _ = app.emit("session:login", event);
    }

    CreatePasswordResponse {
        success: true,
        message: Some("Master password created successfully".to_string()),
        error: None,
    }
}

// ============================================================================
// T039: login Command
// ============================================================================

/// Authenticates user with existing master password
///
/// # Contract
/// - Input: `{ password: string }`
/// - Validation: credentials.enc exists, password matches stored hash (constant-time)
/// - Side Effects: Emits session:login, updates SessionState, updates MasterCredentials.last_used_at
///
/// # Security
/// - Constant-time password comparison
/// - All errors return "AUTHENTICATION_FAILED" (timing attack prevention)
///
/// # Returns
/// Success: `{ success: true, message: "Login successful" }`
/// Error: `{ success: false, error: "AUTHENTICATION_FAILED" }`
#[tauri::command(rename_all = "snake_case")]
pub fn login(
    app: AppHandle,
    session: State<RwLock<SessionState>>,
    password: String,
) -> LoginResponse {
    let creds_path = get_credentials_path();

    // Load credentials from file
    let mut credentials = match MasterCredentials::load_from_file(&creds_path) {
        Ok(c) => c,
        Err(_) => {
            // Return generic error (don't reveal file doesn't exist)
            return LoginResponse {
                success: false,
                message: None,
                error: Some("AUTHENTICATION_FAILED".to_string()),
            };
        }
    };

    // Derive key from entered password using stored salt
    let derived_key = match derive_key_argon2id(&password, &credentials.argon2_salt) {
        Ok(k) => k,
        Err(_) => {
            return LoginResponse {
                success: false,
                message: None,
                error: Some("AUTHENTICATION_FAILED".to_string()),
            };
        }
    };

    // Decrypt stored password hash
    use sha2::{Digest, Sha256};
    let encryption_key_material = Zeroizing::new(format!("{}:encryption", password));
    let mut hasher = Sha256::new();
    hasher.update(encryption_key_material.as_bytes());
    let encryption_key_vec = hasher.finalize();
    let mut encryption_key = [0u8; AES_KEY_SIZE];
    encryption_key.copy_from_slice(&encryption_key_vec[..AES_KEY_SIZE]);

    let stored_hash = match decrypt_aes_gcm(
        &credentials.encrypted_password_hash,
        &credentials.aes_tag,
        &encryption_key,
        &credentials.aes_nonce,
    ) {
        Ok(h) => h,
        Err(_) => {
            return LoginResponse {
                success: false,
                message: None,
                error: Some("AUTHENTICATION_FAILED".to_string()),
            };
        }
    };

    // Compare derived key with stored hash using constant-time comparison
    if !constant_time_compare(derived_key.as_ref(), stored_hash.as_ref()) {
        return LoginResponse {
            success: false,
            message: None,
            error: Some("AUTHENTICATION_FAILED".to_string()),
        };
    }

    // Password matches! Update last_used_at and save
    credentials.update_last_used();
    let _ = credentials.save_to_file(&creds_path); // Ignore errors on metadata update

    // Update session state (login)
    {
        let mut state = session.write().unwrap();
        state.login();

        // Emit session:login event
        let timestamp = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let event = SessionLoginEvent {
            session_token: state.get_session_token().unwrap(),
            timestamp,
        };
        let _ = app.emit("session:login", event);
    }

    LoginResponse {
        success: true,
        message: Some("Login successful".to_string()),
        error: None,
    }
}

// ============================================================================
// T040: logout Command
// ============================================================================

/// Ends user session
///
/// # Contract
/// - No input
/// - Always succeeds
/// - Side Effects: Emits session:logout, updates SessionState (authenticated=false)
///
/// # Returns
/// `{ success: true, message: "Logged out successfully" }`
#[tauri::command(rename_all = "snake_case")]
pub fn logout(app: AppHandle, session: State<RwLock<SessionState>>) -> LogoutResponse {
    // Update session state (logout)
    {
        let mut state = session.write().unwrap();
        state.logout();

        // Emit session:logout event
        let timestamp = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let event = SessionLogoutEvent { timestamp };
        let _ = app.emit("session:logout", event);
    }

    LogoutResponse {
        success: true,
        message: "Logged out successfully".to_string(),
    }
}

// ============================================================================
// T041: check_session Command
// ============================================================================

/// Checks if user is currently authenticated (navigation guard)
///
/// # Contract
/// - No input
/// - Read-only operation (<50ms performance requirement)
/// - Reads Arc<RwLock<SessionState>> in-memory state
///
/// # Returns
/// `{ authenticated: boolean, session_token: string | null }`
#[tauri::command(rename_all = "snake_case")]
pub fn check_session(session: State<RwLock<SessionState>>) -> CheckSessionResponse {
    let state = session.read().unwrap();
    CheckSessionResponse {
        authenticated: state.is_authenticated(),
        session_token: state.get_session_token(),
    }
}