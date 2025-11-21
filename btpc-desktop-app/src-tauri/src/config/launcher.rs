use std::path::PathBuf;
use serde::{Deserialize, Serialize};

use super::{NetworkType, NodeConfig, WalletConfig, MiningConfig, RpcConfig};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherConfig {
    pub btpc_home: PathBuf,
    pub network: NetworkType,
    pub data_dir: PathBuf,
    pub log_dir: PathBuf,
    pub config_dir: PathBuf,
    pub node: NodeConfig,
    pub wallet: WalletConfig,
    pub mining: MiningConfig,
    pub rpc: RpcConfig,
}

impl Default for LauncherConfig {
    fn default() -> Self {
        let btpc_home = dirs::home_dir().unwrap_or_default().join(".btpc");

        Self {
            data_dir: btpc_home.join("data"),
            log_dir: btpc_home.join("logs"),
            config_dir: btpc_home.join("config"),
            btpc_home: btpc_home.clone(),
            network: NetworkType::Regtest, // Use regtest for development (easy mining)
            node: NodeConfig {
                sync_interval_secs: 5,
                max_peers: 50,
                listen_port: 18361, // Desktop app P2P port (avoid conflicts)
                enable_rpc: true,   // FIXED: Enable RPC for desktop node
            },
            wallet: WalletConfig {
                default_wallet_file: "wallet.dat".to_string(),
                auto_backup: true,
                enable_ui: false,
            },
            mining: MiningConfig {
                enabled: false,
                threads: num_cpus::get() as u32,
                target_address: None,
                blocks_to_mine: 5,
                mining_interval_ms: 1000,
            },
            rpc: RpcConfig {
                host: "127.0.0.1".to_string(),
                port: 18360, // Desktop node RPC (isolated from testnet on 18350)
                enable_cors: true,
            },
        }
    }
}