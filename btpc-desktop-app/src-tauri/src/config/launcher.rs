use std::path::{Path, PathBuf};
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

        // FIX 2025-12-01: Network isolation - load saved network from settings storage
        // This ensures the app uses the last selected network on restart
        let (network, rpc_port, p2p_port) = Self::load_saved_network_settings(&btpc_home);

        Self {
            data_dir: btpc_home.join("data"),
            log_dir: btpc_home.join("logs"),
            config_dir: btpc_home.join("config"),
            btpc_home: btpc_home.clone(),
            network, // FIX 2025-12-01: Use saved network instead of hardcoded Mainnet
            node: NodeConfig {
                sync_interval_secs: 5,
                max_peers: 50,
                listen_port: p2p_port, // FIX 2025-12-01: Use saved P2P port
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
                port: rpc_port, // FIX 2025-12-01: Use saved RPC port
                enable_cors: true,
            },
        }
    }
}

impl LauncherConfig {
    /// FIX 2025-12-01: Load saved network settings from RocksDB settings storage
    ///
    /// This is called during config initialization to restore the last selected network.
    /// Returns (network, rpc_port, p2p_port) with defaults if settings not found.
    fn load_saved_network_settings(btpc_home: &Path) -> (NetworkType, u16, u16) {
        let settings_path = btpc_home.join("data").join("settings");

        // Default values
        let default_network = NetworkType::Mainnet;
        let default_rpc_port = 18340u16;
        let default_p2p_port = 18341u16;

        // Try to open settings storage and read saved values
        match crate::settings_storage::SettingsStorage::open(&settings_path) {
            Ok(settings_storage) => {
                // Load network
                let network = match settings_storage.load_setting("network") {
                    Ok(Some(network_str)) => match network_str.as_str() {
                        "mainnet" => NetworkType::Mainnet,
                        "testnet" => NetworkType::Testnet,
                        "regtest" => NetworkType::Regtest,
                        _ => {
                            eprintln!("⚠️ Invalid saved network '{}', using mainnet", network_str);
                            default_network
                        }
                    },
                    Ok(None) => default_network,
                    Err(e) => {
                        eprintln!("⚠️ Failed to load saved network: {}", e);
                        default_network
                    }
                };

                // Load RPC port
                let rpc_port = match settings_storage.load_setting("rpc_port") {
                    Ok(Some(port_str)) => port_str.parse().unwrap_or(default_rpc_port),
                    _ => default_rpc_port,
                };

                // Load P2P port
                let p2p_port = match settings_storage.load_setting("p2p_port") {
                    Ok(Some(port_str)) => port_str.parse().unwrap_or(default_p2p_port),
                    _ => default_p2p_port,
                };

                eprintln!("📁 Loaded saved network settings: {} (RPC: {}, P2P: {})",
                    match network {
                        NetworkType::Mainnet => "mainnet",
                        NetworkType::Testnet => "testnet",
                        NetworkType::Regtest => "regtest",
                    },
                    rpc_port, p2p_port);

                (network, rpc_port, p2p_port)
            }
            Err(e) => {
                eprintln!("⚠️ Settings storage not available (first run?): {}", e);
                (default_network, default_rpc_port, default_p2p_port)
            }
        }
    }
}