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
    pub tx_storage: Arc<btpc_desktop_app::tx_storage::TransactionStorage>,
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

        // Initialize UTXO manager
        let utxo_manager = UTXOManager::new(config.data_dir.join("wallet"))
            .map_err(|e| anyhow::anyhow!("Failed to initialize UTXO manager: {}", e))?;

        // FIX 2025-11-16: Create Arc<TokioMutex<>> wrapper for UTXO manager BEFORE passing to components
        // Using std::sync::Mutex for simplicity and consistency
        // This allows both embedded_node and AppState to share the same UTXO manager instance
        let utxo_manager_arc = Arc::new(Mutex::new(utxo_manager));

        // Initialize wallet manager
        let wallet_config = WalletManagerConfig::default();
        let wallet_manager = WalletManager::new(wallet_config, security.clone()).map_err(|e| {
            BtpcError::Application(format!("Failed to initialize wallet manager: {}", e))
        })?;

        // Initialize address book manager
        let address_book_manager = AddressBookManager::new(config.data_dir.join("address_book"))
            .map_err(|e| {
                BtpcError::Application(format!("Failed to initialize address book manager: {}", e))
            })?;

        // Initialize RocksDB transaction storage (Constitution Article V)
        let tx_storage = btpc_desktop_app::tx_storage::TransactionStorage::open(config.data_dir.join("tx_storage"))
            .map_err(|e| {
            BtpcError::Application(format!("Failed to initialize transaction storage: {}", e))
        })?;

        // Initialize RocksDB settings storage
        let settings_storage = settings_storage::SettingsStorage::open(config.data_dir.join("settings"))
            .map_err(|e| {
            BtpcError::Application(format!("Failed to initialize settings storage: {}", e))
        })?;

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
            mining_stats: Arc::new(Mutex::new(MiningStats::new(&config.data_dir))),
            security,
            current_session: Arc::new(Mutex::new(None)),
            utxo_manager: utxo_manager_arc.clone(),
            wallet_manager: Arc::new(Mutex::new(wallet_manager)),
            sync_service: Arc::new(Mutex::new(None)),
            address_book_manager: Arc::new(Mutex::new(address_book_manager)),
            tx_storage: Arc::new(tx_storage),
            settings_storage: Arc::new(settings_storage),
            wallet_password: Arc::new(RwLock::new(None)),
            wallets_locked: Arc::new(RwLock::new(true)),
            tx_state_manager: Arc::new(tx_state_manager),

            // Feature 012: GPU Mining Dashboard state
            mining_pool: Arc::new(RwLock::new(None)),
            gpu_temperature_threshold: Arc::new(RwLock::new(85.0)),

            // Feature 013: Embedded Blockchain Node (self-contained, no external btpc_node process)
            embedded_node,
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
        println!("=== ROCKSDB MIGRATION CHECK ===");

        // Get default wallet address (required for storage indexing)
        let wallet_address = {
            let wallet_manager = app_state.wallet_manager.lock().map_err(|e| {
                BtpcError::Application(format!("Failed to lock wallet manager: {}", e))
            })?;

            match wallet_manager.get_default_wallet() {
                Some(wallet) => wallet.address.clone(),
                None => {
                    println!("ℹ️  No default wallet - skipping UTXO migration");
                    return Ok(());
                }
            }
        };

        // Check if RocksDB already has transactions for this address
        let tx_count = app_state
            .tx_storage
            .get_transaction_count(&wallet_address)
            .map_err(|e| BtpcError::Application(format!("Failed to check RocksDB state: {}", e)))?;

        if tx_count > 0 {
            println!("✅ RocksDB already populated: {} transactions", tx_count);
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
            println!("ℹ️  No UTXOs to migrate");
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
                fork_id: 2, // Regtest network (migration code)
                block_height: Some(utxo.block_height),
                confirmed_at: Some(utxo.created_at),
                is_coinbase: true,
            };

            // Add to RocksDB (atomic operation per transaction)
            match app_state
                .tx_storage
                .add_transaction(&transaction, &wallet_address)
            {
                Ok(_) => success_count += 1,
                Err(e) => {
                    eprintln!("⚠️  Failed to migrate UTXO {}: {}", utxo.txid, e);
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
}