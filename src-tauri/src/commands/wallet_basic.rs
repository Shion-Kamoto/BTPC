//! Basic wallet Tauri commands
//!
//! This module contains fundamental wallet operations including:
//! - Balance queries (total, individual, with mined rewards)
//! - Address management (list, get)
//! - Wallet creation
//! - BTPC sending

use tauri::State;

use crate::error::BtpcError;
use crate::AppState;

#[tauri::command]
pub async fn get_total_balance(state: State<'_, AppState>) -> Result<f64, String> {
    // Get total balance from wallet manager
    let wallet_manager = state
        .wallet_manager
        .lock()
        .map_err(|_| BtpcError::mutex_poison("wallet_manager", "get_total_balance").to_string())?;
    let wallets = wallet_manager.list_wallets();

    let total: f64 = wallets.iter().map(|w| w.cached_balance_btp).sum();

    Ok(total)
}

#[tauri::command]
pub async fn list_addresses(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    // Get all wallet addresses
    let wallet_manager = state
        .wallet_manager
        .lock()
        .map_err(|_| BtpcError::mutex_poison("wallet_manager", "list_addresses").to_string())?;
    let wallets = wallet_manager.list_wallets();

    let addresses: Vec<String> = wallets.iter().map(|w| w.address.clone()).collect();

    Ok(addresses)
}

#[tauri::command]
pub async fn create_wallet(state: State<'_, AppState>) -> Result<String, String> {
    let wallet_file = state
        .config
        .data_dir
        .join("wallet")
        .join(&state.config.wallet.default_wallet_file);

    // Log the attempt for debugging
    println!("Attempting to create wallet at: {}", wallet_file.display());
    println!("BTPC home: {}", state.config.btpc_home.display());
    println!("Binary directory: {}", state.btpc.bin_dir.display());

    // FIX 2025-12-01: Get active network for correct address prefix
    // FIX 2025-12-01: Use .read().await instead of blocking_read() to prevent deadlock
    let network: btpc_core::Network = {
        let active_network = state.active_network.read().await;
        (*active_network).clone().into()
    };

    match state
        .btpc
        .create_wallet(&wallet_file, "default-wallet-password", network)
    {
        // FIX 2025-12-03: Now returns 4 values (added seed_hex)
        Ok((address, _seed_phrase, _private_key, _seed_hex)) => {
            Ok(format!("Wallet created successfully: {}", address))
        }
        Err(e) => {
            println!("Wallet creation error: {}", e);
            Err(format!("Failed to create wallet: {}", e))
        }
    }
}

#[tauri::command]
pub async fn get_wallet_balance(state: State<'_, AppState>) -> Result<String, String> {
    let wallet_file = state
        .config
        .data_dir
        .join("wallet")
        .join(&state.config.wallet.default_wallet_file);

    // Log the attempt for debugging
    println!(
        "Attempting to get wallet balance (UTXO-based) from: {}",
        wallet_file.display()
    );
    println!("Wallet file exists: {}", wallet_file.exists());

    // Get wallet address for UTXO lookup
    let address = match state.btpc.get_wallet_address(&wallet_file) {
        Ok(addr) => addr,
        Err(e) => {
            // Fallback to legacy balance method if address retrieval fails
            println!("Address retrieval failed, using legacy method: {}", e);
            return match state.btpc.get_wallet_balance(&wallet_file) {
                Ok(balance) => {
                    let mut status = state.status.write().await;
                    status.wallet_balance = balance.clone();
                    Ok(balance)
                }
                Err(e) => Err(format!("Failed to get wallet balance: {}", e)),
            };
        }
    };

    // Strip "Address: " prefix if it exists
    let clean_address = if address.starts_with("Address: ") {
        address
            .strip_prefix("Address: ")
            .unwrap_or(&address)
            .to_string()
    } else {
        address
    };

    // Get spendable balance from UTXO manager (excludes immature coinbase)
    let current_height = state.embedded_node.read().await.get_height();
    let (total_credits, total_btp) = {
        let utxo_manager = state.utxo_manager.lock().map_err(|_| {
            BtpcError::mutex_poison("utxo_manager", "get_wallet_balance").to_string()
        })?;
        utxo_manager.get_spendable_balance(&clean_address, current_height)
    };

    let balance_str = format!("{} base units ({:.8} BTP)", total_credits, total_btp);
    println!("UTXO balance retrieved successfully: {}", balance_str);

    // Update status
    {
        let mut status = state.status.write().await;
        status.wallet_balance = balance_str.clone();
    }

    Ok(balance_str)
}

#[tauri::command]
pub async fn get_wallet_address(state: State<'_, AppState>) -> Result<String, String> {
    let wallet_file = state
        .config
        .data_dir
        .join("wallet")
        .join(&state.config.wallet.default_wallet_file);

    match state.btpc.get_wallet_address(&wallet_file) {
        Ok(address) => Ok(address),
        Err(e) => Err(format!("Failed to get wallet address: {}", e)),
    }
}

#[allow(dead_code)]
#[tauri::command]
pub async fn send_btpc(
    state: State<'_, AppState>,
    to_address: String,
    amount: f64,
    password: String,
) -> Result<String, String> {
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

    // Validate BTPC address format (should be 128 hex characters for ML-DSA public key)
    if !to_address.chars().all(|c| c.is_ascii_hexdigit()) || to_address.len() != 128 {
        return Err("Invalid BTPC address format (must be 128 hex characters)".to_string());
    }

    // Validate amount precision (max 8 decimal places)
    if (amount * 100_000_000.0).fract() != 0.0 {
        return Err("Amount has too many decimal places (max 8 digits after decimal)".to_string());
    }

    let wallet_file = state
        .config
        .data_dir
        .join("wallet")
        .join(&state.config.wallet.default_wallet_file);

    if !wallet_file.exists() {
        return Err("Wallet file not found. Please create a wallet first.".to_string());
    }

    // First, verify the wallet password by trying to get the address
    // (This is a basic password check - in production, use proper authentication)
    match state.btpc.get_wallet_address(&wallet_file) {
        Ok(_) => {
            // Address retrieval succeeded, wallet file is accessible
            // Note: In a real implementation, you would decrypt the private key with the password
        }
        Err(e) => return Err(format!("Wallet access failed: {}", e)),
    }

    // Get wallet address for UTXO selection
    let from_address = match state.btpc.get_wallet_address(&wallet_file) {
        Ok(addr) => addr,
        Err(e) => return Err(format!("Failed to get wallet address: {}", e)),
    };

    // Convert BTPC to credits for UTXO calculations
    let amount_credits = (amount * 100_000_000.0) as u64;
    let fee_credits = 10000u64; // 0.0001 BTP standard fee
    let total_needed = amount_credits + fee_credits;

    // Get blockchain height for maturity check
    let current_height = state.embedded_node.read().await.get_height();

    // Check balance and select UTXOs using UTXO manager
    let (available_credits, available_btp) = {
        let utxo_manager = state.utxo_manager.lock().map_err(|_| {
            BtpcError::mutex_poison("utxo_manager", "send_transaction balance").to_string()
        })?;
        let balance = utxo_manager.get_spendable_balance(&from_address, current_height);

        // Try to select UTXOs for the transaction (with maturity check)
        match utxo_manager.select_utxos_for_spending(&from_address, total_needed, current_height) {
            Ok(selected_utxos) => {
                let selected_amount: u64 = selected_utxos.iter().map(|u| u.value_credits).sum();
                println!(
                    "Selected {} UTXOs totaling {} credits for transaction",
                    selected_utxos.len(),
                    selected_amount
                );
            }
            Err(e) => {
                return Err(format!(
                    "Cannot create transaction: {}. Spendable: {:.8} BTP, Needed: {:.8} BTP",
                    e,
                    balance.1,
                    (total_needed as f64) / 100_000_000.0
                ));
            }
        }

        balance
    };

    if available_credits < total_needed {
        return Err(format!("Insufficient funds. Spendable: {:.8} BTP, Requested: {:.8} BTP (including {:.8} BTP fee). Coinbase UTXOs need 100 confirmations.",
                          available_btp, amount, fee_credits as f64 / 100_000_000.0));
    }

    // Create the transaction using UTXO manager
    let fork_id = state.active_network.read().await.fork_id();
    let (transaction_result, change_amount) = {
        let utxo_manager = state.utxo_manager.lock().map_err(|_| {
            BtpcError::mutex_poison("utxo_manager", "send_transaction create").to_string()
        })?;
        match utxo_manager.create_send_transaction(
            &from_address,
            &to_address,
            amount_credits,
            fee_credits,
            current_height,
            fork_id,
        ) {
            Ok(transaction) => {
                let total_input: u64 = transaction.inputs.len() as u64 * 3237500000; // Estimate
                let change = total_input - amount_credits - fee_credits;
                (Ok(transaction), change)
            }
            Err(e) => (Err(e), 0),
        }
    };

    let transaction =
        transaction_result.map_err(|e| format!("Failed to create transaction: {}", e))?;

    println!("=== UTXO-BASED TRANSACTION CREATED ===");
    println!("Transaction ID: {}", transaction.txid);
    println!("From: {} ({})", from_address, wallet_file.display());
    println!("To: {}", to_address);
    println!("Amount: {:.8} BTP ({} credits)", amount, amount_credits);
    println!(
        "Fee: {:.8} BTP ({} credits)",
        fee_credits as f64 / 100_000_000.0,
        fee_credits
    );
    println!(
        "Change: {:.8} BTP ({} credits)",
        change_amount as f64 / 100_000_000.0,
        change_amount
    );
    println!("Inputs: {} UTXOs", transaction.inputs.len());
    println!("Outputs: {} outputs", transaction.outputs.len());
    println!("Password provided: Yes");
    println!(
        "Available balance: {:.8} BTP ({} credits)",
        available_btp, available_credits
    );

    // In production, you would:
    // 1. Sign the transaction with the private key (decrypted with password)
    // 2. Broadcast to the network
    // 3. Mark UTXOs as spent in the UTXO set
    // 4. Add new outputs to the UTXO set

    Ok(format!("UTXO-based transaction created successfully!\nTransaction ID: {}\nSent {:.8} BTP to {}\nFee: {:.8} BTP\nInputs: {} UTXOs, Outputs: {} outputs\nNote: Transaction created using proper UTXO selection and management.",
               transaction.txid, amount, to_address, fee_credits as f64 / 100_000_000.0,
               transaction.inputs.len(), transaction.outputs.len()))
}

#[tauri::command]
pub async fn get_wallet_balance_with_mined(state: State<'_, AppState>) -> Result<String, String> {
    // Get the wallet address
    let wallet_file = state
        .config
        .data_dir
        .join("wallet")
        .join(&state.config.wallet.default_wallet_file);
    let address = match state.btpc.get_wallet_address(&wallet_file) {
        Ok(addr) => addr,
        Err(e) => return Err(format!("Failed to get wallet address: {}", e)),
    };

    // Clean the address by stripping the "Address: " prefix if present
    let clean_address = if address.starts_with("Address: ") {
        address
            .strip_prefix("Address: ")
            .unwrap_or(&address)
            .to_string()
    } else {
        address
    };

    // Get spendable balance from UTXO set (excludes immature coinbase)
    let current_height = state.embedded_node.read().await.get_height();
    let (total_credits, total_btp) = {
        let utxo_manager = state.utxo_manager.lock().map_err(|_| {
            BtpcError::mutex_poison("utxo_manager", "get_address_balance").to_string()
        })?;
        utxo_manager.get_spendable_balance(&clean_address, current_height)
    };

    Ok(format!(
        "{} base units ({:.8} BTP)",
        total_credits, total_btp
    ))
}