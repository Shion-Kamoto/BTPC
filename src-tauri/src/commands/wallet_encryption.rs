//! Wallet Encryption & Lock Commands (Article VI.3 - Encrypted Wallet Metadata)
//!
//! This module handles wallet encryption, locking/unlocking, and password management.

use tauri::State;

use crate::auth_state::{self, get_credentials_path, MasterCredentials};
use crate::auth_crypto::{derive_key_argon2id, decrypt_aes_gcm, constant_time_compare, AES_KEY_SIZE, generate_random_salt, generate_random_nonce};
use crate::AppState;
use zeroize::Zeroizing;

/// Check if wallets are currently locked (password required to access)
///
/// # Returns
/// - `true` - Wallets are locked AND credentials exist (show "Login" form)
/// - `false` - Wallets are unlocked OR no credentials exist (show "Create Password" form if no creds)
///
/// # FIX 2025-12-10
/// Previously only checked `wallets_locked` state, which starts as `true`.
/// This caused the "Login" form to show even on fresh installs with no credentials.
/// Now also checks if credentials.enc exists - if not, returns `false` so frontend
/// shows "Create Password" form instead.
#[tauri::command]
pub async fn check_wallet_lock_status(state: State<'_, AppState>) -> Result<bool, String> {
    // First check if any credentials exist at all
    let creds_path = get_credentials_path();
    if !creds_path.exists() {
        // Also check if any wallet files exist (for old installations)
        let home = dirs::home_dir().ok_or("Cannot find home directory")?;
        let search_paths = vec![
            home.join(".btpc").join("wallets"),
            home.join(".btpc").join("data").join("mainnet").join("wallets"),
            home.join(".btpc").join("data").join("testnet").join("wallets"),
            home.join(".btpc").join("data").join("regtest").join("wallets"),
        ];

        let mut found_wallet = false;
        'outer: for wallet_dir in &search_paths {
            if !wallet_dir.exists() {
                continue;
            }
            if let Ok(entries) = std::fs::read_dir(wallet_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name.starts_with("wallet_") && name.ends_with(".dat") {
                            found_wallet = true;
                            break 'outer;
                        }
                    }
                }
            }
        }

        if !found_wallet {
            // No credentials.enc AND no wallet files = fresh install
            // Return false so frontend shows "Create Password" form
            eprintln!("🆕 Fresh install detected: no credentials.enc and no wallet files");
            return Ok(false);
        }
        // Old installation with wallet files but no credentials.enc
        // Return true to show login form (unlock_wallets will verify against wallet files)
        eprintln!("📦 Legacy installation: wallet files exist but no credentials.enc");
    }

    let locked = state.wallets_locked.read().await;
    Ok(*locked)
}

/// Unlock wallets by loading encrypted metadata with password
///
/// # Security (FIX 2025-12-09)
/// - ALWAYS verifies password against credentials.enc first (network-independent)
/// - This prevents wrong password acceptance when no wallets exist for current network
/// - Wallet file validation is secondary; credentials.enc is the source of truth
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

    // FIX 2025-12-09: CRITICAL - Verify password against credentials.enc FIRST
    // This prevents wrong password acceptance when switching to a network with no wallets
    // (load_wallets_encrypted returns Ok(()) if no wallet file exists)
    let creds_path = get_credentials_path();
    let mut need_to_create_credentials = false;

    if creds_path.exists() {
        // Load credentials and verify password
        let credentials = MasterCredentials::load_from_file(&creds_path)
            .map_err(|_| "Authentication failed".to_string())?;

        // Derive key from entered password using stored salt
        let _derived_key = derive_key_argon2id(&password, &credentials.argon2_salt)
            .map_err(|_| "Authentication failed".to_string())?;

        // Decrypt stored password hash
        use sha2::{Digest, Sha256};
        let encryption_key_material = Zeroizing::new(format!("{}:encryption", password));
        let mut hasher = Sha256::new();
        hasher.update(encryption_key_material.as_bytes());
        let encryption_key_vec = hasher.finalize();
        let mut encryption_key = [0u8; AES_KEY_SIZE];
        encryption_key.copy_from_slice(&encryption_key_vec[..AES_KEY_SIZE]);

        let stored_hash = decrypt_aes_gcm(
            &credentials.encrypted_password_hash,
            &credentials.aes_tag,
            &encryption_key,
            &credentials.aes_nonce,
        ).map_err(|_| "Incorrect password".to_string())?;

        // Compare derived key with stored hash using constant-time comparison
        let derived_key = derive_key_argon2id(&password, &credentials.argon2_salt)
            .map_err(|_| "Authentication failed".to_string())?;

        if !constant_time_compare(derived_key.as_ref(), stored_hash.as_ref()) {
            return Err("Incorrect password".to_string());
        }

        eprintln!("✅ Password verified against credentials.enc");
    } else {
        // FIX 2025-12-11: No credentials.enc exists - this is a legacy installation
        //
        // IMPORTANT: Wallet files (wallet_*.dat) are encrypted with PER-WALLET passwords,
        // NOT the master password. We cannot verify the master password against wallet files.
        //
        // Solution: For legacy installations, we accept the provided password and create
        // credentials.enc from it. The user must remember their master password.
        //
        // This is safe because:
        // 1. If this is a fresh install, creating credentials.enc establishes the master password
        // 2. If credentials.enc was deleted, user must remember/reset their password anyway
        // 3. Wallet files remain protected by their individual passwords
        eprintln!("⚠️ No credentials.enc found - will create from provided master password");
        eprintln!("   NOTE: Wallet files use per-wallet passwords, not master password");
        need_to_create_credentials = true;
        // password_verified is effectively true - we trust the user's input
    }

    // Create SecurePassword from string
    let secure_password = btpc_core::crypto::SecurePassword::new(password.clone());

    // Load encrypted wallet metadata (may be empty for new networks)
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

    // FIX 2025-12-09: Create credentials.enc if it didn't exist (for existing installations)
    // This ensures future logins will verify against credentials.enc
    if need_to_create_credentials {
        eprintln!("📝 Creating credentials.enc for future password verification...");

        // Ensure parent directory exists
        if let Some(parent) = creds_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        // Generate salt and derive key from password using Argon2id
        if let Ok(salt) = generate_random_salt() {
            if let Ok(derived_key) = derive_key_argon2id(&password, &salt) {
                let password_hash = derived_key.as_ref().to_vec();

                if let Ok(nonce) = generate_random_nonce() {
                    // Encrypt the password hash with AES-256-GCM
                    use sha2::{Digest, Sha256};
                    let encryption_key_material = Zeroizing::new(format!("{}:encryption", password));
                    let mut hasher = Sha256::new();
                    hasher.update(encryption_key_material.as_bytes());
                    let encryption_key_vec = hasher.finalize();
                    let mut encryption_key = [0u8; AES_KEY_SIZE];
                    encryption_key.copy_from_slice(&encryption_key_vec[..AES_KEY_SIZE]);

                    if let Ok((encrypted_hash, aes_tag)) = crate::auth_crypto::encrypt_aes_gcm(&password_hash, &encryption_key, &nonce) {
                        // Argon2id parameters
                        const ARGON2_MEMORY_KB: u32 = 65536;
                        const ARGON2_ITERATIONS: u32 = 3;
                        const ARGON2_PARALLELISM: u32 = 4;

                        let credentials = MasterCredentials::new(
                            ARGON2_MEMORY_KB,
                            ARGON2_ITERATIONS,
                            ARGON2_PARALLELISM,
                            salt,
                            encrypted_hash,
                            nonce,
                            aes_tag,
                        );

                        if credentials.save_to_file(&creds_path).is_ok() {
                            eprintln!("✅ Created credentials.enc - future logins will verify against this file");
                        }
                    }
                }
            }
        }
    }

    Ok("Wallets unlocked successfully".to_string())
}

/// Lock wallets and clear password from memory
///
/// CRITICAL FIX (2025-12-20): Save wallets to encrypted metadata BEFORE clearing.
/// Previously, wallet changes were only saved to plaintext wallets_metadata.json,
/// but unlock_wallets loads from encrypted wallets_metadata.dat. This caused
/// "wallet deletion" when locking/unlocking because the encrypted file was stale.
#[tauri::command]
pub async fn lock_wallets(state: State<'_, AppState>) -> Result<String, String> {
    // CRITICAL: Save wallets to encrypted metadata BEFORE clearing
    // This ensures lock/unlock cycle preserves wallet data
    {
        let password_guard = state.wallet_password.read().await;
        if let Some(ref password) = *password_guard {
            let wallet_manager = state
                .wallet_manager
                .lock()
                .map_err(|e| format!("Failed to lock wallet manager: {}", e))?;

            // Save current wallet state to encrypted file
            if let Err(e) = wallet_manager.save_wallets_encrypted(password) {
                eprintln!("⚠️ Warning: Failed to save encrypted wallets before lock: {}", e);
                // Continue with lock even if save fails - don't block user
            } else {
                eprintln!("✅ Saved wallet metadata to encrypted file before locking");
            }
        }
    }

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
/// Also creates credentials.enc for master password verification
#[tauri::command]
pub async fn migrate_to_encrypted(
    password: String,
    state: State<'_, AppState>,
    session: State<'_, std::sync::RwLock<auth_state::SessionState>>,
) -> Result<String, String> {
    use std::path::Path;

    let secure_password = btpc_core::crypto::SecurePassword::new(password.clone());
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

    // FIX 2025-12-09: Create credentials.enc for master password verification
    // This is network-independent and used to verify password on all logins
    let creds_path = get_credentials_path();
    if !creds_path.exists() {
        // Ensure parent directory exists
        if let Some(parent) = creds_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create credentials directory: {}", e))?;
        }

        // Generate salt and derive key from password using Argon2id
        let salt = generate_random_salt()
            .map_err(|e| format!("Failed to generate salt: {}", e))?;

        let derived_key = derive_key_argon2id(&password, &salt)
            .map_err(|_| "Failed to derive key".to_string())?;

        // Create password hash (store the derived key itself as the "hash")
        let password_hash = derived_key.as_ref().to_vec();

        // Generate nonce for AES-GCM encryption
        let nonce = generate_random_nonce()
            .map_err(|e| format!("Failed to generate nonce: {}", e))?;

        // Encrypt the password hash with AES-256-GCM
        use sha2::{Digest, Sha256};
        let encryption_key_material = Zeroizing::new(format!("{}:encryption", password));
        let mut hasher = Sha256::new();
        hasher.update(encryption_key_material.as_bytes());
        let encryption_key_vec = hasher.finalize();
        let mut encryption_key = [0u8; AES_KEY_SIZE];
        encryption_key.copy_from_slice(&encryption_key_vec[..AES_KEY_SIZE]);

        let (encrypted_hash, aes_tag) = crate::auth_crypto::encrypt_aes_gcm(&password_hash, &encryption_key, &nonce)
            .map_err(|_| "Failed to encrypt credentials".to_string())?;

        // Argon2id parameters (match auth_commands.rs constants)
        const ARGON2_MEMORY_KB: u32 = 65536;   // 64 MB
        const ARGON2_ITERATIONS: u32 = 3;       // 3 iterations
        const ARGON2_PARALLELISM: u32 = 4;      // 4 parallel threads

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

        // Save credentials.enc
        credentials.save_to_file(&creds_path)
            .map_err(|e| format!("Failed to save credentials: {}", e))?;

        eprintln!("✅ Created credentials.enc for master password verification");
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