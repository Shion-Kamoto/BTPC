//! Test stubs for wallet commands
//!
//! These stubs provide test-compatible versions of wallet commands
//! without requiring Tauri's AppState.

use serde::{Deserialize, Serialize};

/// Response structure for wallet creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWalletResponse {
    pub wallet_id: String,
    pub address: String,
    pub version: String,
    pub created_at: String,
}

/// Response structure for wallet recovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoverWalletResponse {
    pub wallet_id: String,
    pub address: String,
    pub version: String,
    pub recovered_at: String,
}

/// Create wallet from mnemonic (test stub)
pub async fn create_wallet_from_mnemonic(
    mnemonic: String,
    _password: String,
) -> Result<CreateWalletResponse, String> {
    let words: Vec<&str> = mnemonic.split_whitespace().collect();
    if words.len() != 24 {
        return Err("Invalid mnemonic: expected 24 words".to_string());
    }
    if mnemonic.contains("invalid") {
        return Err("Invalid mnemonic phrase".to_string());
    }

    Ok(CreateWalletResponse {
        wallet_id: uuid::Uuid::new_v4().to_string(),
        address: format!("btpc1q{}", hex::encode([0u8; 20])),
        version: "V2".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
    })
}

/// Recover wallet from mnemonic (test stub)
pub async fn recover_wallet_from_mnemonic(
    mnemonic: String,
    _password: String,
) -> Result<RecoverWalletResponse, String> {
    let words: Vec<&str> = mnemonic.split_whitespace().collect();
    if words.len() != 24 {
        return Err("Invalid mnemonic: expected 24 words".to_string());
    }
    if mnemonic.contains("invalid") {
        return Err("Invalid mnemonic phrase".to_string());
    }

    Ok(RecoverWalletResponse {
        wallet_id: uuid::Uuid::new_v4().to_string(),
        address: format!("btpc1q{}", hex::encode([0u8; 20])),
        version: "V2".to_string(),
        recovered_at: chrono::Utc::now().to_rfc3339(),
    })
}