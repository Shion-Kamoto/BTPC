//! Wallet Encryption & Lock Commands (Article VI.3 - Encrypted Wallet Metadata)
//!
//! This module handles wallet encryption, locking/unlocking, and password management.

use tauri::State;

use crate::auth_state;
use crate::AppState;

/// Check if wallets are currently locked (password required to access)
#[tauri::command]
pub async fn check_wallet_lock_status(state: State<'_, AppState>) -> Result<bool, String> {
    let locked = state.wallets_locked.read().await;
    Ok(*locked)
}

/// Unlock wallets by loading encrypted metadata with password
#[tauri::command]
pub async fn unlock_wallets(
    password: String,
    state: State<'_, AppState>,
    session: State<'_, std::sync::RwLock<auth_state::SessionState>>,
) -> Result<String, String> {
    // Check if already unlocked
    {
        let locked = state.wallets_locked.read().await;
        if !*locked {
            return Ok("Wallets already unlocked".to_string());
        }
    }

    // Create SecurePassword from string
    let secure_password = btpc_core::crypto::SecurePassword::new(password);

    // Load encrypted wallet metadata
    {
        let mut wallet_manager = state
            .wallet_manager
            .lock()
            .map_err(|e| format!("Failed to lock wallet manager: {}", e))?;
        wallet_manager
            .load_wallets_encrypted(&secure_password)
            .map_err(|e| format!("Failed to decrypt wallets: {}", e))?;
    }

    // Store password in session memory and update lock state
    {
        let mut password_guard = state.wallet_password.write().await;
        *password_guard = Some(secure_password);
    }
    {
        let mut locked_guard = state.wallets_locked.write().await;
        *locked_guard = false;
    }

    // CRITICAL FIX (T011-001): Set SessionState.authenticated = true
    // This ensures check_session (navigation guard) recognizes the user as logged in
    {
        let mut session_state = session.write().unwrap();
        session_state.login(); // Sets authenticated=true, generates session token
    }

    Ok("Wallets unlocked successfully".to_string())
}

/// Lock wallets and clear password from memory
#[tauri::command]
pub async fn lock_wallets(state: State<'_, AppState>) -> Result<String, String> {
    // Clear password from memory (Zeroize will clean it up on drop)
    {
        let mut password_guard = state.wallet_password.write().await;
        *password_guard = None;
    }

    // Set locked state
    {
        let mut locked_guard = state.wallets_locked.write().await;
        *locked_guard = true;
    }

    // Clear wallet metadata from memory
    {
        let mut wallet_manager = state
            .wallet_manager
            .lock()
            .map_err(|e| format!("Failed to lock wallet manager: {}", e))?;
        wallet_manager.clear_wallets();
    }

    Ok("Wallets locked successfully".to_string())
}

/// Change master password for encrypted wallet metadata
#[tauri::command]
pub async fn change_master_password(
    old_password: String,
    new_password: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let old_secure = btpc_core::crypto::SecurePassword::new(old_password);
    let new_secure = btpc_core::crypto::SecurePassword::new(new_password);

    // Verify old password by attempting to load
    {
        let mut wallet_manager = state
            .wallet_manager
            .lock()
            .map_err(|e| format!("Failed to lock wallet manager: {}", e))?;

        // Try loading with old password
        wallet_manager
            .load_wallets_encrypted(&old_secure)
            .map_err(|e| format!("Old password incorrect: {}", e))?;

        // Re-save with new password
        wallet_manager
            .save_wallets_encrypted(&new_secure)
            .map_err(|e| format!("Failed to save with new password: {}", e))?;
    }

    // Update password in session memory
    {
        let mut password_guard = state.wallet_password.write().await;
        *password_guard = Some(new_secure);
    }

    Ok("Master password changed successfully".to_string())
}

/// Migrate plaintext wallet metadata to encrypted format (one-time operation)
#[tauri::command]
pub async fn migrate_to_encrypted(
    password: String,
    state: State<'_, AppState>,
    session: State<'_, std::sync::RwLock<auth_state::SessionState>>,
) -> Result<String, String> {
    use std::path::Path;

    let secure_password = btpc_core::crypto::SecurePassword::new(password);
    let data_dir = &state.config.data_dir;

    let plaintext_path = Path::new(data_dir).join("wallets_metadata.json");
    let encrypted_path = Path::new(data_dir).join("wallets_metadata.dat");

    // Check if already encrypted
    if encrypted_path.exists() {
        return Err("Wallet metadata is already encrypted".to_string());
    }

    // Check if plaintext exists
    if !plaintext_path.exists() {
        return Err("No plaintext wallet metadata found to migrate".to_string());
    }

    // Load plaintext metadata (already done by WalletManager::new())
    // Just save in encrypted format
    {
        let wallet_manager = state
            .wallet_manager
            .lock()
            .map_err(|e| format!("Failed to lock wallet manager: {}", e))?;
        wallet_manager
            .save_wallets_encrypted(&secure_password)
            .map_err(|e| format!("Failed to encrypt wallet metadata: {}", e))?;
    }

    // Verify encrypted file was created successfully
    if !encrypted_path.exists() {
        return Err("Encrypted file was not created".to_string());
    }

    // Backup plaintext file (don't delete, let user do it manually)
    let backup_path = Path::new(data_dir).join("wallets_metadata.json.backup");
    std::fs::copy(&plaintext_path, &backup_path)
        .map_err(|e| format!("Failed to backup plaintext file: {}", e))?;

    // Delete plaintext file
    std::fs::remove_file(&plaintext_path)
        .map_err(|e| format!("Failed to remove plaintext file: {}", e))?;

    // Store password in session and unlock
    {
        let mut password_guard = state.wallet_password.write().await;
        *password_guard = Some(secure_password);
    }
    {
        let mut locked_guard = state.wallets_locked.write().await;
        *locked_guard = false;
    }

    // CRITICAL FIX (T011-001): Set SessionState.authenticated = true
    // This ensures check_session (navigation guard) recognizes the user as logged in
    {
        let mut session_state = session.write().unwrap();
        session_state.login(); // Sets authenticated=true, generates session token
    }

    Ok("Migration successful. Plaintext backed up to wallets_metadata.json.backup".to_string())
}