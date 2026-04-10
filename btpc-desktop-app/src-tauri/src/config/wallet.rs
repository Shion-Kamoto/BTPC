use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletConfig {
    pub default_wallet_file: String,
    pub auto_backup: bool,
    pub enable_ui: bool,
}