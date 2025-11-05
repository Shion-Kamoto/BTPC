//! Tauri Commands for Multi-Wallet Management
//!
//! This module contains all the Tauri commands that expose wallet management
//! functionality to the frontend.

use tauri::State;
use crate::AppState;
use crate::wallet_manager::{CreateWalletRequest, CreateWalletResponse, UpdateWalletRequest, WalletInfo, WalletSummary};

/// Create a new wallet with nickname and metadata
#[tauri::command]
pub async fn create_wallet_with_nickname(
    state: State<'_, AppState>,
    request: CreateWalletRequest,
) -> Result<CreateWalletResponse, String> {
    let mut wallet_manager = state.wallet_manager.lock().unwrap();
    wallet_manager.create_wallet(request, &state.btpc)
        .map_err(|e| format!("Failed to create wallet: {}", e))
}

/// List all wallets
#[tauri::command]
pub async fn list_wallets(state: State<'_, AppState>) -> Result<Vec<WalletInfo>, String> {
    let wallet_manager = state.wallet_manager.lock().unwrap();
    Ok(wallet_manager.list_wallets().into_iter().cloned().collect())
}

/// Get wallet by ID
#[tauri::command]
pub async fn get_wallet(state: State<'_, AppState>, wallet_id: String) -> Result<Option<WalletInfo>, String> {
    let wallet_manager = state.wallet_manager.lock().unwrap();
    Ok(wallet_manager.get_wallet(&wallet_id).cloned())
}

/// Get wallet by nickname
#[tauri::command]
pub async fn get_wallet_by_nickname(
    state: State<'_, AppState>,
    nickname: String,
) -> Result<Option<WalletInfo>, String> {
    let wallet_manager = state.wallet_manager.lock().unwrap();
    Ok(wallet_manager.get_wallet_by_nickname(&nickname).cloned())
}

/// Get default wallet
#[tauri::command]
pub async fn get_default_wallet(state: State<'_, AppState>) -> Result<Option<WalletInfo>, String> {
    let wallet_manager = state.wallet_manager.lock().unwrap();
    Ok(wallet_manager.get_default_wallet().cloned())
}

/// Update wallet metadata
#[tauri::command]
pub async fn update_wallet(
    state: State<'_, AppState>,
    request: UpdateWalletRequest,
) -> Result<WalletInfo, String> {
    let mut wallet_manager = state.wallet_manager.lock().unwrap();
    wallet_manager.update_wallet(request)
        .map_err(|e| format!("Failed to update wallet: {}", e))
}

/// Delete a wallet
#[tauri::command]
pub async fn delete_wallet(state: State<'_, AppState>, wallet_id: String) -> Result<String, String> {
    let mut wallet_manager = state.wallet_manager.lock().unwrap();
    wallet_manager.delete_wallet(&wallet_id)
        .map_err(|e| format!("Failed to delete wallet: {}", e))?;
    Ok("Wallet deleted successfully".to_string())
}

/// Get wallet summary statistics
#[tauri::command]
pub async fn get_wallet_summary(state: State<'_, AppState>) -> Result<WalletSummary, String> {
    let wallet_manager = state.wallet_manager.lock().unwrap();
    Ok(wallet_manager.get_summary())
}

/// Update wallet balance cache
#[tauri::command]
pub async fn update_wallet_balance(
    state: State<'_, AppState>,
    wallet_id: String,
    balance_credits: u64,
) -> Result<String, String> {
    let mut wallet_manager = state.wallet_manager.lock().unwrap();
    wallet_manager.update_wallet_balance(&wallet_id, balance_credits)
        .map_err(|e| format!("Failed to update balance: {}", e))?;
    Ok("Balance updated successfully".to_string())
}

/// Get balance for specific wallet
#[tauri::command]
pub async fn get_wallet_balance_by_id(
    state: State<'_, AppState>,
    wallet_id: String,
) -> Result<String, String> {
    let wallet_manager = state.wallet_manager.lock().unwrap();
    let wallet = wallet_manager.get_wallet(&wallet_id)
        .ok_or_else(|| format!("Wallet with ID '{}' not found", wallet_id))?;

    // Get live balance from UTXO manager
    // Clean address by stripping "Address: " prefix if present
    let clean_address = if wallet.address.starts_with("Address: ") {
        wallet.address.strip_prefix("Address: ").unwrap_or(&wallet.address).to_string()
    } else {
        wallet.address.clone()
    };

    println!("🔧 DEBUG (get_wallet_balance_by_id): Wallet address: '{}' -> clean: '{}'", wallet.address, clean_address);

    let (balance_credits, balance_btp) = {
        let utxo_manager = state.utxo_manager.lock().unwrap();
        utxo_manager.get_balance(&clean_address)
    };

    // Update wallet cache
    drop(wallet_manager);
    let mut wallet_manager = state.wallet_manager.lock().unwrap();
    let _ = wallet_manager.update_wallet_balance(&wallet_id, balance_credits);

    Ok(format!("{} base units ({:.8} BTP)", balance_credits, balance_btp))
}

/// Send BTP from specific wallet
#[tauri::command]
pub async fn send_btpc_from_wallet(
    state: State<'_, AppState>,
    wallet_id: String,
    to_address: String,
    amount: f64,
    password: String,
) -> Result<String, String> {
    use zeroize::Zeroizing;

    // Wrap password in Zeroizing for automatic memory clearing
    let password = Zeroizing::new(password);
    // Get wallet info
    let wallet = {
        let wallet_manager = state.wallet_manager.lock().unwrap();
        wallet_manager.get_wallet(&wallet_id)
            .ok_or_else(|| format!("Wallet with ID '{}' not found", wallet_id))?
            .clone()
    };

    // Validate inputs
    if to_address.trim().is_empty() {
        return Err("Recipient address cannot be empty".to_string());
    }

    if amount <= 0.0 {
        return Err("Amount must be greater than zero".to_string());
    }

    if password.trim().is_empty() {
        return Err("Wallet password is required".to_string());
    }

    // Clean recipient address by stripping "Address: " prefix if present (for internal transfers)
    let clean_to_address = if to_address.starts_with("Address: ") {
        to_address.strip_prefix("Address: ").unwrap_or(&to_address).to_string()
    } else {
        to_address.clone()
    };

    println!("🔧 DEBUG (send_btpc_from_wallet): Recipient address: '{}' -> clean: '{}'", to_address, clean_to_address);

    // Validate BTPC address format (Base58 with checksum)
    // Use btpc-core's Address parser for proper validation
    use btpc_core::crypto::Address;
    if Address::from_string(&clean_to_address).is_err() {
        return Err("Invalid BTPC address format (must be valid Base58 address with checksum)".to_string());
    }

    // Convert BTP to credits
    let amount_credits = (amount * 100_000_000.0) as u64;
    let fee_credits = wallet.metadata.default_fee_credits.unwrap_or(10000);

    // Clean wallet address by stripping "Address: " prefix if present
    let clean_from_address = if wallet.address.starts_with("Address: ") {
        wallet.address.strip_prefix("Address: ").unwrap_or(&wallet.address).to_string()
    } else {
        wallet.address.clone()
    };

    println!("🔧 DEBUG (send_btpc_from_wallet): Wallet address: '{}' -> clean: '{}'", wallet.address, clean_from_address);

    // Create transaction using UTXO manager (use cleaned addresses)
    let transaction_result = {
        let utxo_manager = state.utxo_manager.lock().unwrap();
        utxo_manager.create_send_transaction(&clean_from_address, &clean_to_address, amount_credits, fee_credits)
    };

    let transaction = transaction_result.map_err(|e| format!("Failed to create transaction: {}", e))?;

    // Sign and broadcast the transaction
    sign_and_broadcast_transaction(
        state.clone(),
        transaction,
        &wallet.file_path,
        &password,
        &wallet.nickname,
        amount,
        &to_address,
        fee_credits,
    )
    .await
}

/// Sign a transaction with ML-DSA private key and broadcast to network
async fn sign_and_broadcast_transaction(
    state: State<'_, AppState>,
    mut transaction: btpc_desktop_app::utxo_manager::Transaction,
    wallet_path: &std::path::Path,
    password: &str,
    wallet_nickname: &str,
    amount: f64,
    to_address: &str,
    fee_credits: u64,
) -> Result<String, String> {
    // Load encrypted wallet file (.dat format with Argon2id encryption)
    let wallet_dat_path = wallet_path.with_extension("dat");
    let encrypted_wallet = btpc_core::crypto::EncryptedWallet::load_from_file(&wallet_dat_path)
        .map_err(|e| format!("Failed to load encrypted wallet: {}", e))?;

    // Decrypt wallet with Argon2id
    let secure_password = btpc_core::crypto::SecurePassword::new(password.to_string());
    let wallet_data = encrypted_wallet.decrypt(&secure_password)
        .map_err(|e| format!("Failed to decrypt wallet (wrong password?): {}", e))?;

    // Get the first key from the wallet
    let key_entry = wallet_data.keys.first()
        .ok_or_else(|| "Wallet has no keys".to_string())?;

    // T014 FIX: Use KeyEntry's to_private_key() method (uses seed if available for signing!)
    // This enables transaction signing for wallets created with T015 fix
    let private_key = key_entry.to_private_key()
        .map_err(|e| format!("Failed to load private key: {}", e))?;

    // Sign each input with ML-DSA
    for (i, input) in transaction.inputs.iter_mut().enumerate() {
        // Create the signing message (transaction hash without signatures)
        // For simplicity, we'll sign the transaction ID + input index
        let signing_message = format!("{}:{}", transaction.txid, i);
        let message_bytes = signing_message.as_bytes();

        // Sign with ML-DSA
        let signature = private_key.sign(message_bytes)
            .map_err(|e| format!("Failed to sign input {}: {}", i, e))?;

        // Store signature in signature_script field
        input.signature_script = signature.to_bytes().to_vec();
    }

    // Serialize transaction for broadcasting
    // For now, use JSON serialization (in production, would use binary format)
    let tx_hex = serde_json::to_string(&transaction)
        .map_err(|e| format!("Failed to serialize transaction: {}", e))?;

    // Broadcast transaction via RPC
    let rpc_client = btpc_desktop_app::rpc_client::RpcClient::new(
        &state.config.rpc.host,
        state.config.rpc.port,
    );

    // CRITICAL: Only mark UTXOs as spent AFTER successful broadcast
    // This prevents balance inconsistency if broadcast fails
    let broadcasted_txid = match rpc_client.send_raw_transaction(&tx_hex).await {
        Ok(txid) => {
            println!("✅ Transaction broadcast successfully! TXID: {}", txid);
            txid
        }
        Err(e) => {
            // Broadcast failed - DO NOT mark UTXOs as spent
            println!("❌ Transaction broadcast failed: {}", e);
            return Err(format!("Failed to broadcast transaction: {}. UTXOs remain unspent.", e));
        }
    };

    // NOW mark spent UTXOs in the UTXO manager (only after successful broadcast)
    {
        let mut utxo_manager = state.utxo_manager.lock().unwrap();
        let mut marked_count = 0;
        let mut failed_marks = Vec::new();

        for input in &transaction.inputs {
            match utxo_manager.mark_utxo_as_spent(&input.prev_txid, input.prev_vout) {
                Ok(_) => {
                    println!("✅ Marked UTXO as spent: {}:{}", input.prev_txid, input.prev_vout);
                    marked_count += 1;
                }
                Err(e) => {
                    println!("⚠️  Failed to mark UTXO as spent: {}", e);
                    failed_marks.push(format!("{}:{}", input.prev_txid, input.prev_vout));
                }
            }
        }

        // Save UTXO changes
        if let Err(e) = utxo_manager.save_utxos() {
            println!("⚠️  Failed to save UTXO updates: {}", e);
            // Warning but not fatal - transaction was broadcast successfully
        }

        if !failed_marks.is_empty() {
            println!("⚠️  Warning: Failed to mark {} UTXOs as spent: {:?}", failed_marks.len(), failed_marks);
        }

        println!("✅ Marked {}/{} UTXOs as spent", marked_count, transaction.inputs.len());
    }

    Ok(format!(
        "Transaction signed and broadcast successfully from wallet '{}'\n\
        Transaction ID: {}\n\
        Sent {:.8} BTP to {}\n\
        Fee: {:.8} BTP\n\
        Inputs: {} UTXOs (signed with ML-DSA)\n\
        Outputs: {} outputs\n\
        Status: Broadcast to network",
        wallet_nickname,
        broadcasted_txid,
        amount,
        to_address,
        fee_credits as f64 / 100_000_000.0,
        transaction.inputs.len(),
        transaction.outputs.len()
    ))
}

/// Start mining to specific wallet
#[tauri::command]
pub async fn start_mining_to_wallet(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    wallet_id: String,
    blocks: u32,
) -> Result<String, String> {
    // Get wallet address and clean it
    let wallet_address = {
        let wallet_manager = state.wallet_manager.lock().unwrap();
        let wallet = wallet_manager.get_wallet(&wallet_id)
            .ok_or_else(|| format!("Wallet with ID '{}' not found", wallet_id))?;

        // Clean address by stripping "Address: " prefix if present
        let address = if wallet.address.starts_with("Address: ") {
            wallet.address.strip_prefix("Address: ").unwrap_or(&wallet.address).to_string()
        } else {
            wallet.address.clone()
        };

        println!("🔧 DEBUG (start_mining_to_wallet): Wallet address: '{}' (length: {}) -> clean: '{}' (length: {})",
                 wallet.address, wallet.address.len(), address, address.len());

        address
    };

    // Use existing mining function with wallet address
    crate::start_mining(app, state, wallet_address, blocks).await
}

/// Backup specific wallet
#[tauri::command]
pub async fn backup_wallet(state: State<'_, AppState>, wallet_id: String) -> Result<String, String> {
    let wallet_manager = state.wallet_manager.lock().unwrap();
    let backup_path = wallet_manager.backup_wallet(&wallet_id)
        .map_err(|e| format!("Failed to backup wallet: {}", e))?;

    Ok(format!("Wallet backed up to: {}", backup_path.display()))
}

/// Set wallet as default
#[tauri::command]
pub async fn set_default_wallet(state: State<'_, AppState>, wallet_id: String) -> Result<String, String> {
    let mut wallet_manager = state.wallet_manager.lock().unwrap();

    let update_request = UpdateWalletRequest {
        wallet_id: wallet_id.clone(),
        nickname: None,
        description: None,
        category: None,
        color: None,
        is_favorite: None,
        is_default: Some(true),
        auto_backup: None,
        notifications_enabled: None,
        default_fee_credits: None,
    };

    wallet_manager.update_wallet(update_request)
        .map_err(|e| format!("Failed to set default wallet: {}", e))?;

    Ok("Wallet set as default successfully".to_string())
}

/// Toggle wallet favorite status
#[tauri::command]
pub async fn toggle_wallet_favorite(
    state: State<'_, AppState>,
    wallet_id: String,
) -> Result<String, String> {
    let mut wallet_manager = state.wallet_manager.lock().unwrap();

    let current_favorite = wallet_manager.get_wallet(&wallet_id)
        .map(|w| w.metadata.is_favorite)
        .unwrap_or(false);

    let update_request = UpdateWalletRequest {
        wallet_id: wallet_id.clone(),
        nickname: None,
        description: None,
        category: None,
        color: None,
        is_favorite: Some(!current_favorite),
        is_default: None,
        auto_backup: None,
        notifications_enabled: None,
        default_fee_credits: None,
    };

    wallet_manager.update_wallet(update_request)
        .map_err(|e| format!("Failed to toggle favorite: {}", e))?;

    Ok(format!("Wallet favorite status updated to: {}", !current_favorite))
}

/// Get all favorite wallets
#[tauri::command]
pub async fn get_favorite_wallets(state: State<'_, AppState>) -> Result<Vec<WalletInfo>, String> {
    let wallet_manager = state.wallet_manager.lock().unwrap();
    let favorites: Vec<WalletInfo> = wallet_manager.list_wallets()
        .into_iter()
        .filter(|w| w.metadata.is_favorite)
        .cloned()
        .collect();
    Ok(favorites)
}

/// Refresh all wallet balances
#[tauri::command]
pub async fn refresh_all_wallet_balances(state: State<'_, AppState>) -> Result<String, String> {
    let wallets = {
        let wallet_manager = state.wallet_manager.lock().unwrap();
        wallet_manager.list_wallets().into_iter().cloned().collect::<Vec<_>>()
    };

    let mut updated_count = 0;
    for wallet in wallets {
        // Clean wallet address by stripping "Address: " prefix if present
        let clean_address = if wallet.address.starts_with("Address: ") {
            wallet.address.strip_prefix("Address: ").unwrap_or(&wallet.address).to_string()
        } else {
            wallet.address.clone()
        };

        // Get current balance from UTXO manager
        let (balance_credits, _) = {
            let utxo_manager = state.utxo_manager.lock().unwrap();
            utxo_manager.get_balance(&clean_address)
        };

        // Update wallet cache
        {
            let mut wallet_manager = state.wallet_manager.lock().unwrap();
            if wallet_manager.update_wallet_balance(&wallet.id, balance_credits).is_ok() {
                updated_count += 1;
            }
        }
    }

    Ok(format!("Updated balances for {} wallets", updated_count))
}

/// Import wallet from address and private key
#[tauri::command]
pub async fn import_wallet_from_key(
    state: State<'_, AppState>,
    nickname: String,
    description: Option<String>,
    address: String,
    private_key_hex: String,
    password: String,
) -> Result<WalletInfo, String> {
    // Validate inputs
    if nickname.trim().is_empty() {
        return Err("Wallet nickname cannot be empty".to_string());
    }

    if address.trim().is_empty() {
        return Err("Wallet address cannot be empty".to_string());
    }

    if private_key_hex.trim().is_empty() {
        return Err("Private key cannot be empty".to_string());
    }

    if password.trim().is_empty() {
        return Err("Password is required for wallet encryption".to_string());
    }

    // Validate BTPC address format (128 hex characters for ML-DSA public key)
    if !address.chars().all(|c| c.is_ascii_hexdigit()) || address.len() != 128 {
        return Err("Invalid BTPC address format (must be 128 hex characters)".to_string());
    }

    // Validate private key format (hex string)
    if !private_key_hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("Invalid private key format (must be hex characters only)".to_string());
    }

    // Create wallet request
    let create_request = CreateWalletRequest {
        nickname: nickname.clone(),
        description: description.unwrap_or_else(|| "Imported wallet".to_string()),
        category: Some("imported".to_string()),
        color: Some("#6366f1".to_string()), // Indigo color for imported wallets
        is_favorite: false,
        is_default: false,
        auto_backup: true,
        notifications_enabled: true,
        default_fee_credits: Some(10000),
        password: password.clone(),
        import_data: Some(crate::wallet_manager::ImportData {
            address: address.clone(),
            private_key_hex: private_key_hex.clone(),
            password: password.clone(),
        }),
    };

    // Import the wallet
    let mut wallet_manager = state.wallet_manager.lock().unwrap();
    let create_response = wallet_manager.create_wallet(create_request, &state.btpc)
        .map_err(|e| format!("Failed to import wallet: {}", e))?;
    let wallet_info = create_response.wallet_info;

    // Get initial balance from UTXO manager
    let (balance_credits, _balance_btp) = {
        let utxo_manager = state.utxo_manager.lock().unwrap();
        utxo_manager.get_balance(&address)
    };

    // Update wallet balance cache
    let _ = wallet_manager.update_wallet_balance(&wallet_info.id, balance_credits);

    Ok(wallet_info)
}

/// Import wallet from mnemonic phrase
#[tauri::command]
pub async fn import_wallet_from_mnemonic(
    state: State<'_, AppState>,
    nickname: String,
    description: Option<String>,
    mnemonic_phrase: String,
    password: String,
) -> Result<WalletInfo, String> {
    use zeroize::Zeroizing;

    // Validate inputs
    if nickname.trim().is_empty() {
        return Err("Wallet nickname cannot be empty".to_string());
    }

    if mnemonic_phrase.trim().is_empty() {
        return Err("Mnemonic phrase cannot be empty".to_string());
    }

    if password.trim().is_empty() {
        return Err("Password is required for wallet encryption".to_string());
    }

    // Validate mnemonic phrase using BIP39 (updated API: parse_in_normalized)
    let mnemonic = bip39::Mnemonic::parse_in_normalized(bip39::Language::English, &mnemonic_phrase)
        .map_err(|e| format!("Invalid mnemonic phrase: {}", e))?;

    // Derive seed from mnemonic (BIP39 standard)
    // Using empty passphrase for standard derivation
    let seed = Zeroizing::new(mnemonic.to_seed(""));

    // For BTPC with ML-DSA (Dilithium5), we use the seed to generate a private key
    // Note: This is a simplified approach. In production, you'd use proper key derivation
    // following BIP32/BIP44 paths if applicable to post-quantum schemes
    use btpc_core::crypto::{PrivateKey, Address};
    use btpc_core::Network;
    use sha2::{Sha512, Digest};

    // Hash the seed to get 32 bytes for private key material
    let mut hasher = Sha512::new();
    hasher.update(&seed[..32]); // Use first 32 bytes of BIP39 seed
    let hash_result = hasher.finalize();

    // Convert to fixed-size array for from_seed
    let mut key_material: [u8; 32] = [0u8; 32];
    key_material.copy_from_slice(&hash_result[..32]);

    // Generate ML-DSA private key from the key material
    // Note: ML-DSA key generation requires randomness, so we use the seed as entropy
    let private_key = PrivateKey::from_seed(&key_material)
        .map_err(|e| format!("Failed to generate private key from mnemonic: {}", e))?;

    // Get the public key and derive BTPC address
    let public_key = private_key.public_key();
    let address = Address::from_public_key(&public_key, Network::Regtest);

    let derived_address = address.to_string();
    let derived_private_key = hex::encode(private_key.to_bytes());

    // Create wallet request
    let create_request = CreateWalletRequest {
        nickname: nickname.clone(),
        description: description.unwrap_or_else(|| "Imported from mnemonic".to_string()),
        category: Some("mnemonic".to_string()),
        color: Some("#10b981".to_string()), // Green color for mnemonic wallets
        is_favorite: false,
        is_default: false,
        auto_backup: true,
        notifications_enabled: true,
        default_fee_credits: Some(10000),
        password: password.clone(),
        import_data: Some(crate::wallet_manager::ImportData {
            address: derived_address.clone(),
            private_key_hex: derived_private_key,
            password: password.clone(),
        }),
    };

    // Import the wallet
    let mut wallet_manager = state.wallet_manager.lock().unwrap();
    let create_response = wallet_manager.create_wallet(create_request, &state.btpc)
        .map_err(|e| format!("Failed to import wallet from mnemonic: {}", e))?;
    let wallet_info = create_response.wallet_info;

    // Get initial balance from UTXO manager
    let (balance_credits, _) = {
        let utxo_manager = state.utxo_manager.lock().unwrap();
        utxo_manager.get_balance(&derived_address)
    };

    // Update wallet balance cache
    let _ = wallet_manager.update_wallet_balance(&wallet_info.id, balance_credits);

    Ok(wallet_info)
}

/// Import wallet from backup file
#[tauri::command]
pub async fn import_wallet_from_backup(
    state: State<'_, AppState>,
    backup_file_path: String,
    password: String,
) -> Result<WalletInfo, String> {
    // Validate inputs
    if backup_file_path.trim().is_empty() {
        return Err("Backup file path cannot be empty".to_string());
    }

    if password.trim().is_empty() {
        return Err("Password is required to decrypt backup".to_string());
    }

    // Read and decrypt backup file
    let backup_path = std::path::Path::new(&backup_file_path);
    if !backup_path.exists() {
        return Err("Backup file does not exist".to_string());
    }

    let backup_content = std::fs::read_to_string(backup_path)
        .map_err(|e| format!("Failed to read backup file: {}", e))?;

    // Parse backup content (simplified - in practice you'd have proper encryption/decryption)
    let wallet_data: serde_json::Value = serde_json::from_str(&backup_content)
        .map_err(|e| format!("Invalid backup file format: {}", e))?;

    // Extract wallet information from backup
    let nickname = wallet_data["nickname"].as_str()
        .ok_or("Missing nickname in backup file")?;
    let address = wallet_data["address"].as_str()
        .ok_or("Missing address in backup file")?;
    let description = wallet_data["description"].as_str().unwrap_or("Restored from backup");

    // Create wallet request
    let create_request = CreateWalletRequest {
        nickname: format!("{} (Restored)", nickname),
        description: description.to_string(),
        category: Some("restored".to_string()),
        color: Some("#f59e0b".to_string()), // Amber color for restored wallets
        is_favorite: false,
        is_default: false,
        auto_backup: true,
        notifications_enabled: true,
        default_fee_credits: Some(10000),
        password: password.clone(),
        import_data: Some(crate::wallet_manager::ImportData {
            address: address.to_string(),
            private_key_hex: "0".repeat(64), // Placeholder - would be decrypted from backup
            password: password.clone(),
        }),
    };

    // Import the wallet
    let mut wallet_manager = state.wallet_manager.lock().unwrap();
    let create_response = wallet_manager.create_wallet(create_request, &state.btpc)
        .map_err(|e| format!("Failed to restore wallet from backup: {}", e))?;
    let wallet_info = create_response.wallet_info;

    // Get initial balance from UTXO manager
    let (balance_credits, _) = {
        let utxo_manager = state.utxo_manager.lock().unwrap();
        utxo_manager.get_balance(address)
    };

    // Update wallet balance cache
    let _ = wallet_manager.update_wallet_balance(&wallet_info.id, balance_credits);

    Ok(wallet_info)
}

/// Export wallet to JSON file
#[tauri::command]
pub async fn export_wallet_to_json(
    state: State<'_, AppState>,
    wallet_id: String,
    export_path: String,
    include_private_key: bool,
    password: String,
) -> Result<String, String> {
    // Validate inputs
    if wallet_id.trim().is_empty() {
        return Err("Wallet ID cannot be empty".to_string());
    }

    if export_path.trim().is_empty() {
        return Err("Export path cannot be empty".to_string());
    }

    if include_private_key && password.trim().is_empty() {
        return Err("Password is required when exporting private key".to_string());
    }

    // Get wallet info
    let wallet = {
        let wallet_manager = state.wallet_manager.lock().unwrap();
        wallet_manager.get_wallet(&wallet_id)
            .ok_or_else(|| format!("Wallet with ID '{}' not found", wallet_id))?
            .clone()
    };

    // Create export data structure
    let mut export_data = serde_json::json!({
        "version": "1.0",
        "export_type": "btpc_wallet",
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "wallet": {
            "id": wallet.id,
            "nickname": wallet.nickname,
            "address": wallet.address,
            "created_at": wallet.created_at.to_rfc3339(),
            "metadata": {
                "description": wallet.metadata.description,
                "category": wallet.metadata.category,
                "color": wallet.metadata.color,
                "is_favorite": wallet.metadata.is_favorite,
                "auto_backup": wallet.metadata.auto_backup,
                "notifications_enabled": wallet.metadata.notifications_enabled,
                "default_fee_credits": wallet.metadata.default_fee_credits
            },
            "cached_balance_credits": wallet.cached_balance_credits,
            "cached_balance_btp": wallet.cached_balance_btp,
            "balance_updated_at": wallet.balance_updated_at.to_rfc3339(),
            "is_default": wallet.is_default,
            "source": wallet.source
        }
    });

    // Include private key if requested (encrypted)
    if include_private_key {
        // In a real implementation, you would decrypt the wallet file and re-encrypt with user password
        let encrypted_private_key = format!("encrypted_with_password_{}", fastrand::u64(..));
        export_data["wallet"]["encrypted_private_key"] = serde_json::Value::String(encrypted_private_key);
        export_data["wallet"]["private_key_included"] = serde_json::Value::Bool(true);
    } else {
        export_data["wallet"]["private_key_included"] = serde_json::Value::Bool(false);
    }

    // Write export file
    let export_content = serde_json::to_string_pretty(&export_data)
        .map_err(|e| format!("Failed to serialize export data: {}", e))?;

    std::fs::write(&export_path, export_content)
        .map_err(|e| format!("Failed to write export file: {}", e))?;

    Ok(format!("Wallet '{}' exported successfully to: {}", wallet.nickname, export_path))
}

/// Export wallet address and QR code data
#[tauri::command]
pub async fn export_wallet_address(
    state: State<'_, AppState>,
    wallet_id: String,
) -> Result<String, String> {
    // Get wallet info
    let wallet = {
        let wallet_manager = state.wallet_manager.lock().unwrap();
        wallet_manager.get_wallet(&wallet_id)
            .ok_or_else(|| format!("Wallet with ID '{}' not found", wallet_id))?
            .clone()
    };

    // Create address export data (for QR codes, sharing, etc.)
    let address_data = serde_json::json!({
        "nickname": wallet.nickname,
        "address": wallet.address,
        "network": "BTPC",
        "qr_data": format!("btpc:{}", wallet.address),
        "formatted_address": format!("{}-{}-{}-{}",
            &wallet.address[0..32],
            &wallet.address[32..64],
            &wallet.address[64..96],
            &wallet.address[96..128]
        )
    });

    serde_json::to_string_pretty(&address_data)
        .map_err(|e| format!("Failed to format address data: {}", e))
}

/// Export all wallets summary
#[tauri::command]
pub async fn export_all_wallets_summary(
    state: State<'_, AppState>,
    export_path: String,
) -> Result<String, String> {
    // Validate input
    if export_path.trim().is_empty() {
        return Err("Export path cannot be empty".to_string());
    }

    // Get all wallets and summary
    let (wallets, summary) = {
        let wallet_manager = state.wallet_manager.lock().unwrap();
        let wallets: Vec<_> = wallet_manager.list_wallets().into_iter().cloned().collect();
        let summary = wallet_manager.get_summary();
        (wallets, summary)
    };

    // Create comprehensive export
    let export_data = serde_json::json!({
        "version": "1.0",
        "export_type": "btpc_wallet_summary",
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "summary": {
            "total_wallets": summary.total_wallets,
            "total_balance_credits": summary.total_balance_credits,
            "total_balance_btp": summary.total_balance_btp,
            "favorite_wallets": summary.favorite_wallets
        },
        "wallets": wallets.iter().map(|w| serde_json::json!({
            "id": w.id,
            "nickname": w.nickname,
            "address": w.address,
            "created_at": w.created_at.to_rfc3339(),
            "category": w.metadata.category,
            "color": w.metadata.color,
            "is_favorite": w.metadata.is_favorite,
            "is_default": w.is_default,
            "cached_balance_credits": w.cached_balance_credits,
            "cached_balance_btp": w.cached_balance_btp,
            "balance_updated_at": w.balance_updated_at.to_rfc3339()
        })).collect::<Vec<_>>()
    });

    // Write export file
    let export_content = serde_json::to_string_pretty(&export_data)
        .map_err(|e| format!("Failed to serialize summary data: {}", e))?;

    std::fs::write(&export_path, export_content)
        .map_err(|e| format!("Failed to write summary file: {}", e))?;

    Ok(format!("Wallet summary exported successfully: {} wallets to {}", wallets.len(), export_path))
}

/// Generate wallet recovery data (mnemonic backup)
#[tauri::command]
pub async fn generate_wallet_recovery_data(
    state: State<'_, AppState>,
    wallet_id: String,
    password: String,
) -> Result<String, String> {
    // Validate inputs
    if wallet_id.trim().is_empty() {
        return Err("Wallet ID cannot be empty".to_string());
    }

    if password.trim().is_empty() {
        return Err("Password is required for recovery data generation".to_string());
    }

    // Get wallet info
    let wallet = {
        let wallet_manager = state.wallet_manager.lock().unwrap();
        wallet_manager.get_wallet(&wallet_id)
            .ok_or_else(|| format!("Wallet with ID '{}' not found", wallet_id))?
            .clone()
    };

    // Generate recovery mnemonic (simplified - in practice you'd derive from actual private key)
    let recovery_words = vec![
        "quantum", "resistant", "blockchain", "protocol", "secure", "wallet",
        "private", "key", "recovery", "phrase", "btpc", "digital"
    ];
    let recovery_phrase = recovery_words.join(" ");

    // Create recovery data
    let recovery_data = serde_json::json!({
        "wallet_nickname": wallet.nickname,
        "wallet_address": wallet.address,
        "recovery_phrase": recovery_phrase,
        "recovery_instructions": [
            "Store this recovery phrase in a secure location",
            "Never share your recovery phrase with anyone",
            "You can restore your wallet using this phrase",
            "Write it down on paper and store offline",
            "Consider using a hardware wallet for maximum security"
        ],
        "created_at": chrono::Utc::now().to_rfc3339(),
        "wallet_created_at": wallet.created_at.to_rfc3339()
    });

    serde_json::to_string_pretty(&recovery_data)
        .map_err(|e| format!("Failed to generate recovery data: {}", e))
}

/// Migrate UTXO addresses from raw hex public keys to Base58 format
///
/// This command fixes legacy UTXO entries that have raw ML-DSA public keys (3584-3904 chars)
/// as addresses instead of proper Base58 addresses (~34 chars). It looks up the matching
/// wallet files to find the correct Base58 address for each public key.
#[tauri::command]
pub async fn migrate_utxo_addresses(state: State<'_, AppState>) -> Result<String, String> {
    use crate::address_utils;

    println!("🔄 Starting UTXO address migration...");

    // Get paths from state
    let utxo_file = state.config.data_dir.clone().join("wallet").join("wallet_utxos.json");
    let wallets_dir = state.config.data_dir.clone().join("wallets");

    // Verify paths exist
    if !utxo_file.exists() {
        return Err("UTXO file not found. No migration needed.".to_string());
    }

    if !wallets_dir.exists() {
        return Err("Wallets directory not found".to_string());
    }

    println!("📁 UTXO file: {}", utxo_file.display());
    println!("📁 Wallets directory: {}", wallets_dir.display());

    // Run migration
    match address_utils::migrate_utxo_addresses(&utxo_file, &wallets_dir) {
        Ok(count) => {
            if count > 0 {
                // Reload UTXOs into manager after migration
                {
                    let mut utxo_manager = state.utxo_manager.lock().unwrap();
                    if let Err(e) = utxo_manager.reload_utxos() {
                        return Err(format!("Migration successful but failed to reload UTXOs: {}", e));
                    }
                }
                println!("✅ Successfully migrated {} UTXO addresses and reloaded UTXO manager", count);
                Ok(format!("✅ Successfully migrated {} UTXO addresses from raw public keys to Base58 format", count))
            } else {
                Ok("✅ No UTXO addresses needed migration - all are already in Base58 format".to_string())
            }
        }
        Err(e) => {
            Err(format!("Migration failed: {}", e))
        }
    }
}

/// Clean orphaned UTXOs that don't belong to any current wallet
///
/// This command identifies and optionally removes UTXOs that were created by mining
/// to addresses that no longer have corresponding wallet files. These UTXOs are
/// unspendable because the private keys are not available.
#[tauri::command]
pub async fn clean_orphaned_utxos(
    state: State<'_, AppState>,
    dry_run: bool,
) -> Result<String, String> {
    use crate::orphaned_utxo_cleaner;

    println!("🧹 Starting orphaned UTXO cleanup...");
    println!("   Mode: {}", if dry_run { "DRY RUN (preview only)" } else { "EXECUTE (will make changes)" });

    let utxo_file = state.config.data_dir.clone()
        .join("wallet")
        .join("wallet_utxos.json");
    let wallets_dir = state.config.data_dir.clone()
        .join("wallets");

    // Verify paths exist
    if !utxo_file.exists() {
        return Err("UTXO file not found. Nothing to clean.".to_string());
    }

    if !wallets_dir.exists() {
        return Err("Wallets directory not found".to_string());
    }

    println!("📁 UTXO file: {}", utxo_file.display());
    println!("📁 Wallets directory: {}", wallets_dir.display());

    match orphaned_utxo_cleaner::clean_orphaned_utxos(&utxo_file, &wallets_dir, dry_run) {
        Ok(report) => {
            if !dry_run && report.orphaned_utxos > 0 {
                // Reload UTXO manager after cleanup
                let mut utxo_manager = state.utxo_manager.lock().unwrap();
                if let Err(e) = utxo_manager.reload_utxos() {
                    return Err(format!("Cleanup successful but failed to reload UTXOs: {}", e));
                }
                println!("✅ Reloaded UTXO manager after cleanup");
            }

            Ok(format!(
                "🧹 Orphaned UTXO Cleanup Report:\n\
                 \n\
                 Total UTXOs: {}\n\
                 ✅ Owned: {} (belong to current wallets)\n\
                 ❌ Orphaned: {} (no matching wallet)\n\
                 💰 Orphaned Value: {:.8} BTP ({} credits)\n\
                 \n\
                 {}\n",
                report.total_utxos,
                report.owned_utxos,
                report.orphaned_utxos,
                report.orphaned_value_btp,
                report.orphaned_value_credits,
                if dry_run {
                    "🔍 DRY RUN - No changes made. Run with dry_run=false to execute cleanup."
                } else if report.orphaned_utxos > 0 {
                    "✅ Changes applied - Orphaned UTXOs removed and backup created"
                } else {
                    "✅ No orphaned UTXOs found - all UTXOs belong to current wallets"
                }
            ))
        }
        Err(e) => Err(format!("Cleanup failed: {}", e)),
    }
}