//! Security-related Tauri commands
//!
//! This module handles user authentication, session management, and wallet key decryption.
//! All security operations use Argon2 password hashing and AES-256-GCM encryption.

use tauri::State;

use crate::error::BtpcError;
use crate::AppState;
use btpc_desktop_app::security::{RecoveryData, UserSession};

/// Create a new user account with password
/// Returns recovery data (mnemonic phrase) for account recovery
#[tauri::command]
pub async fn create_user(
    state: State<'_, AppState>,
    username: String,
    password: String,
) -> Result<RecoveryData, String> {
    state
        .security
        .create_user(&username, &password)
        .map_err(|e| format!("Failed to create user: {}", e))
}

/// Authenticate user and create a new session
/// Returns session info on successful login
#[tauri::command]
pub async fn login_user(
    state: State<'_, AppState>,
    username: String,
    password: String,
) -> Result<UserSession, String> {
    match state.security.authenticate_user(&username, &password) {
        Ok(session) => {
            // Store current session
            {
                let mut current_session = state.current_session.lock().map_err(|_| {
                    BtpcError::mutex_poison("current_session", "login_user").to_string()
                })?;
                *current_session = Some(session.session_id.clone());
            }
            Ok(session)
        }
        Err(e) => Err(format!("Login failed: {}", e)),
    }
}

/// Logout the current user and invalidate session
#[tauri::command]
pub async fn logout_user(state: State<'_, AppState>) -> Result<String, String> {
    let session_id = {
        let mut current_session = state
            .current_session
            .lock()
            .map_err(|_| BtpcError::mutex_poison("current_session", "logout_user").to_string())?;
        current_session.take()
    };

    if let Some(session_id) = session_id {
        state.security.invalidate_session(&session_id);
        Ok("Successfully logged out".to_string())
    } else {
        Err("No active session".to_string())
    }
}

/// Recover account using mnemonic phrase
/// Allows setting a new password after recovery
#[tauri::command]
pub async fn recover_account(
    state: State<'_, AppState>,
    username: String,
    recovery_phrase: String,
    new_password: String,
) -> Result<String, String> {
    state
        .security
        .recover_account(&username, &recovery_phrase, &new_password)
        .map_err(|e| format!("Recovery failed: {}", e))
}

/// Check if current session is valid
/// Returns true if session exists and hasn't expired
#[tauri::command]
pub async fn check_security_session(state: State<'_, AppState>) -> Result<bool, String> {
    let session_id = {
        let current_session = state.current_session.lock().map_err(|_| {
            BtpcError::mutex_poison("current_session", "check_security_session").to_string()
        })?;
        current_session.clone()
    };

    if let Some(session_id) = session_id {
        state
            .security
            .validate_session(&session_id)
            .map_err(|e| format!("Session validation failed: {}", e))
    } else {
        Ok(false)
    }
}

/// Get information about the current session
/// Returns None if no active session
#[tauri::command]
pub async fn get_session_info(state: State<'_, AppState>) -> Result<Option<UserSession>, String> {
    let session_id = {
        let current_session = state.current_session.lock().map_err(|_| {
            BtpcError::mutex_poison("current_session", "get_session_info").to_string()
        })?;
        current_session.clone()
    };

    if let Some(session_id) = session_id {
        Ok(state.security.get_session(&session_id))
    } else {
        Ok(None)
    }
}

/// Get list of all registered usernames
#[tauri::command]
pub async fn get_users(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    state
        .security
        .get_users()
        .map_err(|e| format!("Failed to get users: {}", e))
}

/// Check if a user with the given username exists
#[tauri::command]
pub async fn user_exists(state: State<'_, AppState>, username: String) -> Result<bool, String> {
    Ok(state.security.user_exists(&username))
}

/// Decrypt an encrypted wallet key using password
/// Used for accessing encrypted wallet data
#[tauri::command]
pub async fn decrypt_wallet_key(
    state: State<'_, AppState>,
    encrypted_key: String,
    password: String,
) -> Result<String, String> {
    state
        .security
        .decrypt_wallet_key(&encrypted_key, &password)
        .map_err(|e| format!("Decryption failed: {}", e))
}
