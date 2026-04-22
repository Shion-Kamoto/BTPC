//! AppState module - Central application state management
//!
//! This module contains the AppState struct which manages all shared state
//! for the BTPC desktop application including:
//! - Configuration and network settings
//! - Process management for node and miners
//! - Security and wallet management
//! - UTXO and transaction storage
//! - Embedded blockchain node

use std::collections::HashMap;
use std::process::Child;
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;

use btpc_desktop_app::btpc_integration::BtpcIntegration;
use btpc_desktop_app::security::SecurityManager;
use btpc_desktop_app::utxo_manager::UTXOManager;
use btpc_desktop_app::wallet_manager::{WalletManager, WalletManagerConfig};

use crate::address_book::AddressBookManager;
use crate::config::{LauncherConfig, NetworkType};
use crate::error::{BtpcError, BtpcResult};
use crate::process_manager;
use crate::settings_storage;
use crate::state_management;
use crate::sync_service::BlockchainSyncService;
use crate::types::{MiningLogBuffer, MiningStats, MiningStatus, NodeStatus, SystemStatus};

// ============================================================================
// Application State Management
// ============================================================================

pub struct AppState {
    pub config: LauncherConfig,
    pub active_network: Arc<RwLock<NetworkType>>,
    pub active_rpc_port: Arc<RwLock<u16>>,
    pub active_p2p_port: Arc<RwLock<u16>>,
    pub process_manager: Arc<process_manager::ProcessManager>,
    pub mining_processes: Arc<Mutex<HashMap<String, Child>>>,
    pub status: Arc<RwLock<SystemStatus>>,

    // Article XI StateManager fields (auto-emit events on state changes)
    pub node_status: state_management::StateManager<NodeStatus>,
    pub mining_status: state_management::StateManager<MiningStatus>,

    pub btpc: BtpcIntegration,
    pub mining_logs: Arc<Mutex<MiningLogBuffer>>,
    pub mining_stats: Arc<Mutex<MiningStats>>,
    pub security: SecurityManager,
    pub current_session: Arc<Mutex<Option<String>>>,
    pub utxo_manager: Arc<Mutex<UTXOManager>>,
    pub wallet_manager: Arc<Mutex<WalletManager>>,
    pub sync_service: Arc<Mutex<Option<BlockchainSyncService>>>,
    pub address_book_manager: Arc<Mutex<AddressBookManager>>,
    /// Transaction history - SQLite-based storage for reliable transaction tracking
    /// FIX 2025-12-12: Replaced RocksDB tx_storage with SQLite tx_history for UPSERT support
    pub tx_storage: Arc<RwLock<btpc_desktop_app::tx_history::TransactionHistory>>,
    pub settings_storage: Arc<settings_storage::SettingsStorage>,
    pub wallet_password: Arc<RwLock<Option<btpc_core::crypto::SecurePassword>>>,
    pub wallets_locked: Arc<RwLock<bool>>,
    // Feature 007: Transaction state manager for transaction lifecycle tracking
    pub tx_state_manager: Arc<btpc_desktop_app::transaction_state::TransactionStateManager>,

    // Feature 012: GPU Mining Dashboard state
    pub mining_pool: Arc<RwLock<Option<btpc_desktop_app::mining_thread_pool::MiningThreadPool>>>,
    pub gpu_temperature_threshold: Arc<RwLock<f32>>,

    // Feature 013: Embedded Blockchain Node (self-contained, no external btpc_node process)
    pub embedded_node: Arc<RwLock<btpc_desktop_app::embedded_node::EmbeddedNode>>,

    // FR-058: Disk space monitoring for sync/mining protection
    pub disk_space_monitor: Arc<btpc_desktop_app::disk_space_monitor::DiskSpaceMonitor>,

    // Controls whether background polling loops (mining status, node status) should run
    // Set to false on logout/shutdown to stop background tasks from accessing the node
    pub node_active: Arc<std::sync::atomic::AtomicBool>,

    // Internet connectivity monitor: pauses mining when internet connection is lost
    pub network_monitor_active: Arc<std::sync::atomic::AtomicBool>,
    // User override: when true, monitor won't pause mining (user clicked "Resume Mining")
    pub network_pause_override: Arc<std::sync::atomic::AtomicBool>,
}

impl AppState {
    pub fn new() -> BtpcResult<Self> {
        let config = LauncherConfig::default();
        let btpc = BtpcIntegration::new(config.btpc_home.clone());

        // Create directories if they don't exist
        btpc.setup_directories(&config.log_dir, &config.data_dir, &config.config_dir)?;

        let installation_status = btpc.check_installation();

        let status = SystemStatus {
            node_status: "Stopped".to_string(),
            node_pid: None,
            wallet_balance: "Unknown".to_string(),
            mining_status: "Stopped".to_string(),
            binaries_installed: installation_status.is_complete,
            config_exists: Self::check_config_exists(&config),
            logs_available: Vec::new(),
        };

        // Initialize security manager
        let security = SecurityManager::new(config.config_dir.clone());

        // FIX 2025-12-01: Network isolation - create network-specific data directory
        // Each network (mainnet/testnet/regtest) gets its own subdirectory for complete isolation
        // Directory structure: ~/.btpc/data/{network}/...
        let network_str = config.network.to_string();
        let network_data_dir = config.data_dir.join(&network_str);
        std::fs::create_dir_all(&network_data_dir).map_err(|e| {
            BtpcError::Application(format!("Failed to create network directory: {}", e))
        })?;
        eprintln!(
            "[BTPC::App] Network data directory: {:?} (network: '{}')",
            network_data_dir, network_str
        );

        // Initialize UTXO manager (network-isolated)
        // FIX 2025-12-01: Use network-specific path: ~/.btpc/data/{network}/wallet
        let utxo_manager = UTXOManager::new(network_data_dir.join("wallet"))
            .map_err(|e| anyhow::anyhow!("Failed to initialize UTXO manager: {}", e))?;

        // FIX 2025-11-16: Create Arc<TokioMutex<>> wrapper for UTXO manager BEFORE passing to components
        // Using std::sync::Mutex for simplicity and consistency
        // This allows both embedded_node and AppState to share the same UTXO manager instance
        let utxo_manager_arc = Arc::new(Mutex::new(utxo_manager));

        // Initialize wallet manager (network-isolated)
        // FIX 2025-12-03: Wallet metadata must be network-isolated to match UTXOs
        // Each network gets its own wallet directory: ~/.btpc/data/{network}/wallets/
        let wallet_config = WalletManagerConfig {
            wallets_dir: network_data_dir.join("wallets"),
            backups_dir: network_data_dir.join("wallet-backups"),
            ..WalletManagerConfig::default()
        };
        eprintln!(
            "[BTPC::App] Wallets directory: {:?}",
            wallet_config.wallets_dir
        );
        let wallet_manager = WalletManager::new(wallet_config, security.clone()).map_err(|e| {
            BtpcError::Application(format!("Failed to initialize wallet manager: {}", e))
        })?;

        // Initialize address book manager (network-isolated)
        // FIX 2025-12-01: Use network-specific path: ~/.btpc/data/{network}/address_book
        let address_book_manager = AddressBookManager::new(network_data_dir.join("address_book"))
            .map_err(|e| {
            BtpcError::Application(format!("Failed to initialize address book manager: {}", e))
        })?;

        // Initialize SQLite transaction history (Constitution Article V) (network-isolated)
        // FIX 2025-12-12: Replaced RocksDB with SQLite for UPSERT support (eliminates race conditions)
        // Path: ~/.btpc/data/{network}/tx_history.db
        let btpc_network: btpc_core::Network = (&config.network).into();
        let tx_storage = btpc_desktop_app::tx_history::TransactionHistory::open(
            network_data_dir.join("tx_history"),
            btpc_network,
        )
        .map_err(|e| {
            BtpcError::Application(format!("Failed to initialize transaction history: {}", e))
        })?;

        // Initialize RocksDB settings storage (GLOBAL - not network-isolated)
        // Settings like theme, language, etc. should be the same across all networks
        let settings_storage =
            settings_storage::SettingsStorage::open(config.data_dir.join("settings")).map_err(
                |e| BtpcError::Application(format!("Failed to initialize settings storage: {}", e)),
            )?;

        // Feature 007: Initialize transaction state manager (moved to lib.rs in TD-001)
        let tx_state_manager = btpc_desktop_app::transaction_state::TransactionStateManager::new();

        // Feature 013: Initialize embedded blockchain node (self-contained, no external btpc_node process)
        // FIX 2025-11-16: Pass the Arc<Mutex<UTXOManager>> to embedded node for UTXO creation on mining
        // Use tokio runtime to run async initialization synchronously
        let embedded_node = tokio::runtime::Runtime::new()
            .map_err(|e| BtpcError::Application(format!("Failed to create tokio runtime: {}", e)))?
            .block_on(async {
                btpc_desktop_app::embedded_node::EmbeddedNode::new(
                    config.data_dir.clone(),
                    &config.network.to_string(),
                    utxo_manager_arc.clone(), // Pass the shared UTXO manager
                )
                .await
            })
            .map_err(|e| {
                BtpcError::Application(format!("Failed to initialize embedded node: {}", e))
            })?;

        let app_state = Self {
            config: config.clone(),
            active_network: Arc::new(RwLock::new(config.network.clone())),
            active_rpc_port: Arc::new(RwLock::new(config.rpc.port)),
            active_p2p_port: Arc::new(RwLock::new(config.node.listen_port)),
            process_manager: Arc::new(process_manager::ProcessManager::new(false)),
            mining_processes: Arc::new(Mutex::new(HashMap::new())),
            status: Arc::new(RwLock::new(status)),

            // Article XI StateManagers (auto-emit events on state changes)
            node_status: state_management::StateManager::new("node_status", NodeStatus::default()),
            mining_status: state_management::StateManager::new(
                "mining_status",
                MiningStatus::default(),
            ),

            btpc,
            mining_logs: Arc::new(Mutex::new(MiningLogBuffer::new(1000))),
            // FIX 2025-01-25: Network-isolated mining stats
            // Each network (mainnet/testnet/regtest) gets its own mining_stats.json
            // Path: ~/.btpc/data/{network}/mining_stats.json
            mining_stats: Arc::new(Mutex::new(MiningStats::new(&network_data_dir))),
            security,
            current_session: Arc::new(Mutex::new(None)),
            utxo_manager: utxo_manager_arc.clone(),
            wallet_manager: Arc::new(Mutex::new(wallet_manager)),
            sync_service: Arc::new(Mutex::new(None)),
            address_book_manager: Arc::new(Mutex::new(address_book_manager)),
            tx_storage: Arc::new(RwLock::new(tx_storage)),
            settings_storage: Arc::new(settings_storage),
            wallet_password: Arc::new(RwLock::new(None)),
            wallets_locked: Arc::new(RwLock::new(true)),
            tx_state_manager: Arc::new(tx_state_manager),

            // Feature 012: GPU Mining Dashboard state
            mining_pool: Arc::new(RwLock::new(None)),
            gpu_temperature_threshold: Arc::new(RwLock::new(85.0)),

            // Feature 013: Embedded Blockchain Node (self-contained, no external btpc_node process)
            embedded_node,

            // FR-058: Disk space monitoring for sync/mining protection
            disk_space_monitor: Arc::new(
                btpc_desktop_app::disk_space_monitor::DiskSpaceMonitor::new(
                    config.data_dir.to_string_lossy().to_string(),
                ),
            ),

            // Node active flag - background polling loops check this.
            // Defaults to false: the node is OFF until either the backend
            // auto-start path (tauri_app.rs, gated by auto_connect_node) or
            // the user's Start Node click sets it to true. The status poller
            // reads the *real* sync state from embedded_node.is_sync_running(),
            // so this flag is purely a "is polling allowed" gate now.
            node_active: Arc::new(std::sync::atomic::AtomicBool::new(false)),

            // Internet connectivity monitor flag (set to true when mining starts, false when stopped)
            network_monitor_active: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            network_pause_override: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        };

        // Migrate UTXOs from wallet_utxos.json to RocksDB if RocksDB is empty
        // Constitution Article V: Atomic migration with structured logging
        Self::migrate_utxos_to_rocksdb(&app_state)?;

        Ok(app_state)
    }

    pub fn check_config_exists(config: &LauncherConfig) -> bool {
        config.config_dir.join("launcher.toml").exists()
    }

    /// Migrate UTXOs from wallet_utxos.json to RocksDB transaction storage
    /// Constitution Article V: Atomic migration with structured logging
    /// Constitution Article XI.1: Backend is single source of truth
    pub fn migrate_utxos_to_rocksdb(app_state: &Self) -> BtpcResult<()> {
        eprintln!("[BTPC::DB] Checking UTXO migration status...");

        // Get default wallet address (required for storage indexing)
        let wallet_address = {
            let wallet_manager = app_state.wallet_manager.lock().map_err(|e| {
                BtpcError::Application(format!("Failed to lock wallet manager: {}", e))
            })?;

            match wallet_manager.get_default_wallet() {
                Some(wallet) => wallet.address.clone(),
                None => {
                    eprintln!("[BTPC::DB] No default wallet found - skipping UTXO migration");
                    return Ok(());
                }
            }
        };

        // Check if RocksDB already has transactions for this address
        // FIX 2025-12-05: Use blocking_read() for sync context
        let tx_count = app_state
            .tx_storage
            .blocking_read()
            .get_transaction_count(&wallet_address)
            .map_err(|e| BtpcError::Application(format!("Failed to check RocksDB state: {}", e)))?;

        if tx_count > 0 {
            eprintln!(
                "[BTPC::DB] Transaction store already populated: {} transactions",
                tx_count
            );
            return Ok(());
        }

        // Get all UTXOs from UTXO manager
        let utxos = {
            let utxo_manager = app_state.utxo_manager.lock().map_err(|e| {
                BtpcError::Application(format!("Failed to lock UTXO manager: {}", e))
            })?;
            utxo_manager
                .get_all_utxos(&wallet_address)
                .iter()
                .map(|u| (*u).clone())
                .collect::<Vec<_>>()
        };

        if utxos.is_empty() {
            eprintln!("[BTPC::DB] No UTXOs to migrate");
            return Ok(());
        }

        println!(
            "🔄 Migrating {} UTXOs from wallet_utxos.json → RocksDB",
            utxos.len()
        );

        // Migrate each UTXO as a transaction
        let mut success_count = 0;
        for utxo in &utxos {
            // Create a Transaction from UTXO (these are all coinbase transactions)
            let transaction = btpc_desktop_app::utxo_manager::Transaction {
                txid: utxo.txid.clone(),
                version: 1,
                inputs: vec![], // Coinbase transactions have no inputs
                outputs: vec![btpc_desktop_app::utxo_manager::TxOutput {
                    value: utxo.value_credits,
                    script_pubkey: utxo.script_pubkey.clone(),
                }],
                lock_time: 0,
                fork_id: app_state.active_network.blocking_read().fork_id(),
                block_height: Some(utxo.block_height),
                confirmed_at: Some(utxo.created_at),
                is_coinbase: true,
                sender_address: None, // Coinbase transactions have no sender
            };

            // Add to RocksDB (atomic operation per transaction)
            // FIX 2025-12-05: Use blocking_read() for sync context
            match app_state
                .tx_storage
                .blocking_read()
                .add_transaction(&transaction, &wallet_address)
            {
                Ok(_) => success_count += 1,
                Err(e) => {
                    eprintln!(
                        "[BTPC::DB] WARN: Failed to migrate UTXO {}: {}",
                        utxo.txid, e
                    );
                }
            }
        }

        println!(
            "✅ Migration complete: {}/{} UTXOs migrated to RocksDB",
            success_count,
            utxos.len()
        );
        println!(
            "📊 RocksDB status: {} transactions indexed for address {}",
            success_count,
            &wallet_address[..16]
        );

        Ok(())
    }

    /// Reinitialize all network-specific data stores for a new network
    /// FIX 2025-12-05: This allows network switching without requiring app restart
    ///
    /// Called when user changes network in Settings. Reinitializes:
    /// - tx_storage (RocksDB transaction history)
    /// - utxo_manager (JSON UTXO set)
    /// - wallet_manager (wallet metadata)
    /// - address_book_manager (address book)
    /// - embedded_node's tx_storage reference
    ///
    /// # Safety
    /// This must NOT be called while mining or node is running.
    /// The caller (set_network_config) must verify node is stopped first.
    pub async fn reinitialize_for_network(&self, new_network: &NetworkType) -> BtpcResult<()> {
        let network_str = new_network.to_string();
        let network_data_dir = self.config.data_dir.join(&network_str);

        // Create network directory if it doesn't exist
        std::fs::create_dir_all(&network_data_dir).map_err(|e| {
            BtpcError::Application(format!("Failed to create network directory: {}", e))
        })?;

        eprintln!(
            "[BTPC::App] Reinitializing data stores for network '{}'...",
            network_str
        );
        eprintln!("[BTPC::App] Network data directory: {:?}", network_data_dir);

        // 1. Reinitialize tx_storage (SQLite)
        // FIX 2025-12-12: Changed from RocksDB to SQLite for UPSERT support
        let btpc_network: btpc_core::Network = new_network.into();
        let new_tx_storage = btpc_desktop_app::tx_history::TransactionHistory::open(
            network_data_dir.join("tx_history"),
            btpc_network,
        )
        .map_err(|e| BtpcError::Application(format!("Failed to reinitialize tx_history: {}", e)))?;

        {
            let mut tx_storage_guard = self.tx_storage.write().await;
            *tx_storage_guard = new_tx_storage;
            eprintln!(
                "[BTPC::App] tx_history reinitialized for network '{}'",
                network_str
            );
        }

        // 2. Reinitialize utxo_manager (JSON)
        let mut new_utxo_manager =
            UTXOManager::new(network_data_dir.join("wallet")).map_err(|e| {
                BtpcError::Application(format!("Failed to reinitialize utxo_manager: {}", e))
            })?;

        // FIX 2025-12-12: Sync UTXOManager from tx_history (SQLite is authoritative)
        // This ensures both stores are in sync after network switch or crash recovery
        {
            let tx_storage_guard = self.tx_storage.read().await;
            match tx_storage_guard.get_all_unspent_utxos() {
                Ok(utxos) => {
                    if !utxos.is_empty() {
                        match new_utxo_manager.sync_from_tx_storage_utxos(utxos) {
                            Ok(count) => {
                                eprintln!(
                                    "[BTPC::App] Synced {} UTXOs from tx_history to UTXO manager",
                                    count
                                );
                            }
                            Err(e) => {
                                eprintln!("[BTPC::App] WARN: Failed to sync UTXOs: {} (continuing with JSON data)", e);
                            }
                        }
                    } else {
                        eprintln!("[BTPC::App] No UTXOs in tx_history to sync (new network or empty database)");
                    }
                }
                Err(e) => {
                    eprintln!("[BTPC::App] WARN: Failed to get UTXOs from tx_history: {} (continuing with JSON data)", e);
                }
            }
        }

        {
            let mut utxo_manager_guard = self.utxo_manager.lock().map_err(|e| {
                BtpcError::Application(format!("Failed to lock utxo_manager: {}", e))
            })?;
            *utxo_manager_guard = new_utxo_manager;
            eprintln!(
                "[BTPC::App] UTXO manager reinitialized for network '{}'",
                network_str
            );
        }

        // 3. Reinitialize wallet_manager
        let wallet_config = WalletManagerConfig {
            wallets_dir: network_data_dir.join("wallets"),
            backups_dir: network_data_dir.join("wallet-backups"),
            ..WalletManagerConfig::default()
        };
        let mut new_wallet_manager = WalletManager::new(wallet_config, self.security.clone())
            .map_err(|e| {
                BtpcError::Application(format!("Failed to reinitialize wallet_manager: {}", e))
            })?;

        // FIX 2025-12-09: Load encrypted wallets if password is available
        // Without this, switching networks would lose all wallet data because:
        // - WalletManager::new() only loads unencrypted .json files
        // - But wallets are stored encrypted in .dat files after user logs in
        // - The stored password in wallet_password must be used to decrypt
        // FIX 2025-12-12: Changed blocking_read() to .read().await to prevent deadlock
        {
            let password_guard = self.wallet_password.read().await;
            if let Some(ref password) = *password_guard {
                eprintln!(
                    "[BTPC::App] Loading encrypted wallets for network '{}'...",
                    network_str
                );
                if let Err(e) = new_wallet_manager.load_wallets_encrypted(password) {
                    // Log warning but don't fail - might be a fresh network with no wallets yet
                    eprintln!(
                        "[BTPC::App] WARN: Could not load encrypted wallets for '{}':{}",
                        network_str, e
                    );
                } else {
                    eprintln!(
                        "[BTPC::App] Encrypted wallets loaded for network '{}'",
                        network_str
                    );
                }
            } else {
                eprintln!(
                    "[BTPC::App] WARN: No wallet password - encrypted wallets not loaded for '{}'",
                    network_str
                );
            }
        }

        {
            let mut wallet_manager_guard = self.wallet_manager.lock().map_err(|e| {
                BtpcError::Application(format!("Failed to lock wallet_manager: {}", e))
            })?;
            *wallet_manager_guard = new_wallet_manager;
            eprintln!(
                "[BTPC::App] Wallet manager reinitialized for network '{}'",
                network_str
            );
        }

        // 4. Reinitialize address_book_manager
        let new_address_book = AddressBookManager::new(network_data_dir.join("address_book"))
            .map_err(|e| {
                BtpcError::Application(format!(
                    "Failed to reinitialize address_book_manager: {}",
                    e
                ))
            })?;

        {
            let mut address_book_guard = self.address_book_manager.lock().map_err(|e| {
                BtpcError::Application(format!("Failed to lock address_book_manager: {}", e))
            })?;
            *address_book_guard = new_address_book;
            eprintln!(
                "[BTPC::App] Address book reinitialized for network '{}'",
                network_str
            );
        }

        // 5. Reinitialize mining_stats (network-isolated)
        // FIX 2025-01-25: Mining stats must be network-isolated like wallets/UTXOs
        // Each network gets its own mining_stats.json: ~/.btpc/data/{network}/mining_stats.json
        let new_mining_stats = MiningStats::new(&network_data_dir);

        {
            let mut mining_stats_guard = self.mining_stats.lock().map_err(|e| {
                BtpcError::Application(format!("Failed to lock mining_stats: {}", e))
            })?;
            *mining_stats_guard = new_mining_stats;
            eprintln!(
                "[BTPC::App] Mining stats reinitialized for network '{}' (blocks_found: {})",
                network_str, mining_stats_guard.blocks_found
            );
        }

        // 6. Reset mining_pool to force re-initialization with new network's block count
        // FIX 2025-01-25: The mining_pool caches lifetime_blocks_found on first start
        // When network changes, we must reset it so it picks up the new network's count
        {
            let mut mining_pool_guard = self.mining_pool.write().await;
            if mining_pool_guard.is_some() {
                eprintln!("[BTPC::App] Resetting mining_pool for network switch (will re-init on next start_mining)");
            }
            *mining_pool_guard = None;
        }

        // 7. Update embedded_node's tx_storage reference
        // The embedded_node also holds a reference to tx_storage for storing mined transactions
        // FIX 2025-12-11: Share the SAME tx_storage instance instead of opening a new one
        // Opening a separate RocksDB handle causes data inconsistency (flickering data)
        {
            let mut embedded_node_guard = self.embedded_node.write().await;

            // Pass the same Arc<RwLock<TransactionStorage>> that AppState uses
            // This ensures both AppState commands and EmbeddedNode mining use the same DB handle
            embedded_node_guard.set_tx_storage(self.tx_storage.clone());
            eprintln!(
                "[BTPC::App] Embedded node tx_storage updated for network '{}' (shared instance)",
                network_str
            );
        }

        eprintln!(
            "[BTPC::App] All data stores reinitialized for network '{}' - no restart required",
            network_str
        );

        Ok(())
    }
}
