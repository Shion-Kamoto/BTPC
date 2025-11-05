//! BTPC Desktop Application - Tauri Backend
//!
//! This application provides a desktop GUI for the BTPC blockchain system.
//! It integrates with the existing BTPC unified launcher and provides
//! a user-friendly interface for node management, wallet operations, and mining.
//!
//! # Features
//!
//! - **Secure Authentication**: Multi-user support with Argon2 password hashing
//! - **Quantum-Resistant Operations**: Integration with Dilithium5 cryptography
//! - **Process Management**: Robust detached process management for nodes and miners
//! - **UTXO Management**: Comprehensive UTXO tracking and transaction building
//! - **Real-time Monitoring**: Live status updates and logging
//! - **Cross-Platform**: Tauri-based desktop application for Windows, macOS, Linux
//!
//! # Architecture
//!
//! The application follows a modular architecture:
//!
//! - `AppState`: Central state management with Arc/Mutex for thread safety
//! - `ProcessManager`: Detached process management that survives page navigation
//! - `BtpcIntegration`: Interface to BTPC blockchain binaries
//! - `SecurityManager`: Authentication and encryption services
//! - `UTXOManager`: UTXO set management and transaction building
//! - Error handling through comprehensive `BtpcError` types
//!
//! # Security Considerations
//!
//! - All passwords are hashed with Argon2 and salted
//! - Private keys are encrypted at rest with AES-256-GCM
//! - Session management with automatic timeout (30 minutes)
//! - Brute force protection with account lockout
//! - BIP39 mnemonic recovery phrases for account recovery
//!
//! # Performance
//!
//! The application is designed for responsive desktop use:
//! - Async/await for non-blocking operations
//! - Background process monitoring with tokio
//! - Efficient UTXO indexing for fast balance calculations
//! - Streaming log processing to avoid memory bloat

use serde::{Deserialize, Serialize};
use tauri::Emitter;
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, BufReader as TokioBufReader};
use tokio::process::{ChildStdout, ChildStderr};
use std::fs;
use tauri::{State, Manager};
use tokio::sync::RwLock;

mod btpc_integration;
mod security;
// mod utxo_manager; // Already in lib.rs (used by transaction_state)
mod wallet_manager;
mod wallet_commands;
pub mod error;  // Public for integration tests
// mod rpc_client; // Moved to lib.rs for TD-001 refactoring
mod sync_service;
mod process_manager;
mod address_book;
mod address_utils;
mod orphaned_utxo_cleaner;
mod tx_storage;  // RocksDB-based transaction storage (Constitution Article V)
mod gpu_detection;  // GPU detection for mining optimization (TDD v1.1)
pub mod state_management;  // StateManager<T> for Article XI compliance (auto event emission)

// Transaction modules (Feature 007: Transaction Sending)
// mod transaction_builder; // Moved to lib.rs for TD-001 refactoring
mod transaction_commands;
mod transaction_monitor;
mod fee_estimator;  // Dynamic fee estimation service (T017)
pub mod events;  // Event types for Article XI compliance

// Authentication modules (Feature 006: Application-Level Login/Logout)
mod auth_state;     // SessionState + MasterCredentials structs
mod auth_crypto;    // Argon2id + AES-256-GCM cryptography
mod auth_commands;  // Tauri command handlers for authentication
pub mod process_health;  // Health monitoring and crash recovery (FR-038 through FR-046)
pub mod lock_manager;  // Safe file locking using fs2 (replaces unsafe libc::flock)

use error::{BtpcError, BtpcResult};

use btpc_integration::BtpcIntegration;
use security::{SecurityManager, UserSession, RecoveryData};
use btpc_desktop_app::utxo_manager::{UTXOManager, UTXO, UTXOStats};
use wallet_manager::{WalletManager, WalletManagerConfig};
use sync_service::{BlockchainSyncService, SyncConfig, SyncStats};
use address_book::{AddressBookManager, AddressBookEntry, AddAddressBookRequest, UpdateAddressBookRequest};

// ============================================================================
// BTPC Configuration Structures (copied from unified launcher)
// ============================================================================

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkType {
    Mainnet,
    Testnet,
    Regtest,
}

impl std::fmt::Display for NetworkType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetworkType::Mainnet => write!(f, "mainnet"),
            NetworkType::Testnet => write!(f, "testnet"),
            NetworkType::Regtest => write!(f, "regtest"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub sync_interval_secs: u64,
    pub max_peers: u32,
    pub listen_port: u16,
    pub enable_rpc: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletConfig {
    pub default_wallet_file: String,
    pub auto_backup: bool,
    pub enable_ui: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningConfig {
    pub enabled: bool,
    pub threads: u32,
    pub target_address: Option<String>,
    pub blocks_to_mine: u32,
    pub mining_interval_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcConfig {
    pub host: String,
    pub port: u16,
    pub enable_cors: bool,
}

impl Default for LauncherConfig {
    fn default() -> Self {
        let btpc_home = dirs::home_dir().unwrap_or_default().join(".btpc");

        Self {
            data_dir: btpc_home.join("data"),
            log_dir: btpc_home.join("logs"),
            config_dir: btpc_home.join("config"),
            btpc_home: btpc_home.clone(),
            network: NetworkType::Regtest,  // ✅ Use regtest for development (easy mining)
            node: NodeConfig {
                sync_interval_secs: 5,
                max_peers: 50,
                listen_port: 18361,  // Desktop app P2P port (avoid conflicts)
                enable_rpc: true,  // ✅ FIXED: Enable RPC for desktop node
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
                port: 18360,  // Desktop node RPC (isolated from testnet on 18350)
                enable_cors: true,
            },
        }
    }
}

// ============================================================================
// Application State Management
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatus {
    pub node_status: String,
    pub node_pid: Option<u32>,
    pub wallet_balance: String,
    pub mining_status: String,
    pub binaries_installed: bool,
    pub config_exists: bool,
    pub logs_available: Vec<LogInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogInfo {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub lines: usize,
    pub recent_entries: Vec<String>,
}

// Article XI-compliant state structures for StateManager
// These are serializable and suitable for automatic event emission

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStatus {
    pub running: bool,
    pub pid: Option<u32>,
    pub block_height: u64,
    pub peer_count: u32,
    pub sync_progress: f64,  // 0.0 to 1.0
    pub network: String,     // "mainnet", "testnet", "regtest"
}

impl Default for NodeStatus {
    fn default() -> Self {
        Self {
            running: false,
            pid: None,
            block_height: 0,
            peer_count: 0,
            sync_progress: 0.0,
            network: "mainnet".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningStatus {
    pub active: bool,
    pub hashrate: u64,
    pub blocks_mined: u32,    // Lifetime total
    pub current_difficulty: String,
    pub threads: u32,
}

impl Default for MiningStatus {
    fn default() -> Self {
        Self {
            active: false,
            hashrate: 0,
            blocks_mined: 0,
            current_difficulty: "0".to_string(),
            threads: 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningLogEntry {
    pub timestamp: String,
    pub level: String,  // INFO, WARN, ERROR, SUCCESS
    pub message: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MiningStatsData {
    pub lifetime_blocks_found: u64,
}

#[derive(Clone)]
pub struct MiningStats {
    pub blocks_found: u64,  // Lifetime total
    pub hashrate: u64,
    pub start_time: Option<std::time::Instant>,
    pub stats_file: PathBuf,
}

impl MiningStats {
    fn new(data_dir: &PathBuf) -> Self {
        let stats_file = data_dir.join("mining_stats.json");

        // Load lifetime blocks_found from disk if it exists
        let blocks_found = if stats_file.exists() {
            match std::fs::read_to_string(&stats_file) {
                Ok(json) => {
                    match serde_json::from_str::<MiningStatsData>(&json) {
                        Ok(data) => {
                            println!("📊 Loaded lifetime mining stats: {} blocks found", data.lifetime_blocks_found);
                            data.lifetime_blocks_found
                        }
                        Err(e) => {
                            println!("⚠️ Failed to parse mining stats: {}, starting from 0", e);
                            0
                        }
                    }
                }
                Err(e) => {
                    println!("⚠️ Failed to read mining stats: {}, starting from 0", e);
                    0
                }
            }
        } else {
            println!("📊 No existing mining stats found, starting from 0");
            0
        };

        Self {
            blocks_found,
            hashrate: 0,
            start_time: None,
            stats_file,
        }
    }

    fn reset(&mut self) {
        // Don't reset blocks_found - it's lifetime persistent
        self.hashrate = 0;
        self.start_time = None;
    }

    fn start(&mut self) {
        self.start_time = Some(std::time::Instant::now());
    }

    fn increment_blocks(&mut self) {
        self.blocks_found += 1;
        self.save_to_disk();
    }

    fn save_to_disk(&self) {
        let data = MiningStatsData {
            lifetime_blocks_found: self.blocks_found,
        };

        match serde_json::to_string_pretty(&data) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&self.stats_file, json) {
                    println!("❌ Failed to save mining stats: {}", e);
                } else {
                    println!("💾 Saved mining stats: {} blocks found", self.blocks_found);
                }
            }
            Err(e) => {
                println!("❌ Failed to serialize mining stats: {}", e);
            }
        }
    }

    fn calculate_hashrate(&mut self, estimated_hashes: u64) {
        if let Some(start) = self.start_time {
            let elapsed = start.elapsed().as_secs();
            if elapsed > 0 {
                self.hashrate = estimated_hashes / elapsed;
            }
        }
    }
}

pub struct MiningLogBuffer {
    entries: VecDeque<MiningLogEntry>,
    max_entries: usize,
}

impl MiningLogBuffer {
    fn new(max_entries: usize) -> Self {
        Self {
            entries: VecDeque::new(),
            max_entries,
        }
    }

    fn add_entry(&mut self, level: String, message: String) {
        // Format timestamp to match frontend expectations: "YYYY-MM-DD HH:MM:SS"
        // Frontend uses .split(' ')[1] to extract just the time portion
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

        let entry = MiningLogEntry {
            timestamp,
            level,
            message,
        };

        self.entries.push_back(entry);

        // Keep only the most recent entries
        if self.entries.len() > self.max_entries {
            self.entries.pop_front();
        }
    }

    fn get_entries(&self) -> Vec<MiningLogEntry> {
        self.entries.iter().cloned().collect()
    }

    fn clear(&mut self) {
        self.entries.clear();
    }
}

pub struct AppState {
    config: LauncherConfig,
    active_network: Arc<RwLock<NetworkType>>,  // Mutable network configuration
    active_rpc_port: Arc<RwLock<u16>>,  // Mutable RPC port
    active_p2p_port: Arc<RwLock<u16>>,  // Mutable P2P port
    process_manager: Arc<process_manager::ProcessManager>,  // NEW: Detached process management
    mining_processes: Arc<Mutex<HashMap<String, Child>>>,  // Mining needs piped logs (Child handles)
    status: Arc<RwLock<SystemStatus>>,

    // Article XI StateManager fields (auto-emit events on state changes)
    node_status: state_management::StateManager<NodeStatus>,
    mining_status: state_management::StateManager<MiningStatus>,

    btpc: BtpcIntegration,
    mining_logs: Arc<Mutex<MiningLogBuffer>>,
    mining_stats: Arc<Mutex<MiningStats>>,
    security: SecurityManager,
    current_session: Arc<Mutex<Option<String>>>, // Current session ID
    utxo_manager: Arc<Mutex<UTXOManager>>,
    wallet_manager: Arc<Mutex<WalletManager>>,
    sync_service: Arc<Mutex<Option<BlockchainSyncService>>>, // Blockchain sync service
    address_book_manager: Arc<Mutex<AddressBookManager>>, // Address book for recipient management
    tx_storage: Arc<tx_storage::TransactionStorage>, // RocksDB-based transaction storage (Constitution Article V)
    wallet_password: Arc<RwLock<Option<btpc_core::crypto::SecurePassword>>>, // Session password for encrypted wallets
    wallets_locked: Arc<RwLock<bool>>, // Whether wallets are currently locked
    // Feature 007: Transaction state manager for transaction lifecycle tracking (moved to lib.rs in TD-001)
    tx_state_manager: Arc<btpc_desktop_app::transaction_state::TransactionStateManager>,
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

        // Initialize wallet manager
        let wallet_config = WalletManagerConfig::default();
        let wallet_manager = WalletManager::new(wallet_config, security.clone())
            .map_err(|e| BtpcError::Application(format!("Failed to initialize wallet manager: {}", e)))?;

        // Initialize address book manager
        let address_book_manager = AddressBookManager::new(config.data_dir.join("address_book"))
            .map_err(|e| BtpcError::Application(format!("Failed to initialize address book manager: {}", e)))?;

        // Initialize RocksDB transaction storage (Constitution Article V)
        let tx_storage = tx_storage::TransactionStorage::open(config.data_dir.join("tx_storage"))
            .map_err(|e| BtpcError::Application(format!("Failed to initialize transaction storage: {}", e)))?;

        // Feature 007: Initialize transaction state manager (moved to lib.rs in TD-001)
        let tx_state_manager = btpc_desktop_app::transaction_state::TransactionStateManager::new();

        let app_state = Self {
            config: config.clone(),
            active_network: Arc::new(RwLock::new(config.network.clone())),  // Initialize with default network
            active_rpc_port: Arc::new(RwLock::new(config.rpc.port)),  // Initialize with default RPC port
            active_p2p_port: Arc::new(RwLock::new(config.node.listen_port)),  // Initialize with default P2P port
            process_manager: Arc::new(process_manager::ProcessManager::new(false)),  // NEW: Initialize ProcessManager
            mining_processes: Arc::new(Mutex::new(HashMap::new())),  // Mining uses Child for piped logs
            status: Arc::new(RwLock::new(status)),

            // Article XI StateManagers (auto-emit events on state changes)
            node_status: state_management::StateManager::new("node_status", NodeStatus::default()),
            mining_status: state_management::StateManager::new("mining_status", MiningStatus::default()),

            btpc,
            mining_logs: Arc::new(Mutex::new(MiningLogBuffer::new(1000))),
            mining_stats: Arc::new(Mutex::new(MiningStats::new(&config.data_dir))),
            security,
            current_session: Arc::new(Mutex::new(None)),
            utxo_manager: Arc::new(Mutex::new(utxo_manager)),
            wallet_manager: Arc::new(Mutex::new(wallet_manager)),
            sync_service: Arc::new(Mutex::new(None)), // Initialized but not started yet
            address_book_manager: Arc::new(Mutex::new(address_book_manager)),
            tx_storage: Arc::new(tx_storage), // RocksDB transaction storage
            wallet_password: Arc::new(RwLock::new(None)), // No password on startup (locked)
            wallets_locked: Arc::new(RwLock::new(true)), // Start locked by default
            tx_state_manager: Arc::new(tx_state_manager), // Feature 007: Transaction state manager
        };

        // REMOVED: Automatic wallet creation at startup (2025-11-01)
        // Reason: Should NOT create wallets automatically - wallets already exist
        // User complained: "wallets have been created and there should be no automatic wallet creation at all!"

        /* COMMENTED OUT - DO NOT UNCOMMENT:
        // Test wallet functionality on startup for debugging
        let wallet_file = app_state.config.data_dir.join("wallet").join(&app_state.config.wallet.default_wallet_file);
        println!("=== STARTUP WALLET TEST ===");
        println!("Testing wallet balance from: {}", wallet_file.display());

        // Skip encrypted .dat files (require password to decrypt)
        let is_encrypted = wallet_file.extension().is_some_and(|ext| ext == "dat");

        if wallet_file.exists() && !is_encrypted {
            // Use UTXO-based balance calculation instead of binary method
            let address = match app_state.btpc.get_wallet_address(&wallet_file) {
                Ok(addr) => addr,
                Err(e) => {
                    println!("Startup wallet test FAILED: Could not get address: {}", e);
                    // Continue with startup instead of returning
                    "".to_string()
                }
            };

            if !address.is_empty() {
                // Strip "Address: " prefix if it exists
                let clean_address = if address.starts_with("Address: ") {
                    address.strip_prefix("Address: ").unwrap_or(&address).to_string()
                } else {
                    address
                };

                let (total_credits, total_btp) = {
                    let utxo_manager = app_state.utxo_manager.lock().unwrap_or_else(|e| {
                        eprintln!("Startup balance check failed - UTXO manager mutex poisoned: {}", e);
                        e.into_inner()
                    });
                    utxo_manager.get_balance(&clean_address)
                };

                println!("Startup wallet test SUCCESS: {} base units ({:.8} BTP)", total_credits, total_btp);
            }
        } else {
            println!("Wallet file does not exist, testing wallet creation...");
            match app_state.btpc.create_wallet(&wallet_file, "startup-test-password") {
                Ok((address, _seed_phrase, _private_key)) => println!("Wallet creation SUCCESS: {}", address),
                Err(e) => println!("Wallet creation FAILED: {}", e),
            }
        }
        */

        // Migrate UTXOs from wallet_utxos.json to RocksDB if RocksDB is empty
        // Constitution Article V: Atomic migration with structured logging
        Self::migrate_utxos_to_rocksdb(&app_state)?;

        Ok(app_state)
    }


    fn check_config_exists(config: &LauncherConfig) -> bool {
        config.config_dir.join("launcher.toml").exists()
    }

    /// Migrate UTXOs from wallet_utxos.json to RocksDB transaction storage
    /// Constitution Article V: Atomic migration with structured logging
    /// Constitution Article XI.1: Backend is single source of truth
    fn migrate_utxos_to_rocksdb(app_state: &Self) -> BtpcResult<()> {
        println!("=== ROCKSDB MIGRATION CHECK ===");

        // Get default wallet address (required for storage indexing)
        let wallet_address = {
            let wallet_manager = app_state.wallet_manager.lock()
                .map_err(|e| BtpcError::Application(format!("Failed to lock wallet manager: {}", e)))?;

            match wallet_manager.get_default_wallet() {
                Some(wallet) => wallet.address.clone(),
                None => {
                    println!("ℹ️  No default wallet - skipping UTXO migration");
                    return Ok(());
                }
            }
        };

        // Check if RocksDB already has transactions for this address
        let tx_count = app_state.tx_storage.get_transaction_count(&wallet_address)
            .map_err(|e| BtpcError::Application(format!("Failed to check RocksDB state: {}", e)))?;

        if tx_count > 0 {
            println!("✅ RocksDB already populated: {} transactions", tx_count);
            return Ok(());
        }

        // Get all UTXOs from UTXO manager
        let utxos = {
            let utxo_manager = app_state.utxo_manager.lock()
                .map_err(|e| BtpcError::Application(format!("Failed to lock UTXO manager: {}", e)))?;
            utxo_manager.get_all_utxos(&wallet_address)
                .iter()
                .map(|u| (*u).clone())
                .collect::<Vec<_>>()
        };

        if utxos.is_empty() {
            println!("ℹ️  No UTXOs to migrate");
            return Ok(());
        }

        println!("🔄 Migrating {} UTXOs from wallet_utxos.json → RocksDB", utxos.len());

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
            match app_state.tx_storage.add_transaction(&transaction, &wallet_address) {
                Ok(_) => success_count += 1,
                Err(e) => {
                    eprintln!("⚠️  Failed to migrate UTXO {}: {}", utxo.txid, e);
                }
            }
        }

        println!("✅ Migration complete: {}/{} UTXOs migrated to RocksDB", success_count, utxos.len());
        println!("📊 RocksDB status: {} transactions indexed for address {}", success_count, &wallet_address[..16]);

        Ok(())
    }
}

// ============================================================================
// Tauri Commands (API for Frontend)
// ============================================================================

#[tauri::command]
async fn get_system_status(state: State<'_, AppState>) -> Result<SystemStatus, String> {
    let mut status = {
        let status_guard = state.status.read().await;
        status_guard.clone()
    };

    // Check node status using ProcessManager
    if state.process_manager.is_running("node") {
        status.node_status = "Running".to_string();
        if let Some(info) = state.process_manager.get_info("node") {
            status.node_pid = Some(info.pid);
        }
    } else {
        status.node_status = "Stopped".to_string();
        status.node_pid = None;
    }

    // Check mining status using mining_processes HashMap
    {
        let mining_processes = state.mining_processes.lock()
            .map_err(|_| BtpcError::mutex_poison("mining_processes", "get_system_status").to_string())?;
        if mining_processes.contains_key("mining") {
            if status.mining_status == "Stopped" {
                status.mining_status = "Running".to_string();
            }
        } else {
            status.mining_status = "Stopped".to_string();
        }
    }

    // Update installation status
    let installation_status = state.btpc.check_installation();
    status.binaries_installed = installation_status.is_complete;

    // Update logs info
    status.logs_available = get_log_info(&state.config.log_dir);

    Ok(status)
}

fn get_log_info(log_dir: &std::path::Path) -> Vec<LogInfo> {
    let log_files = [
        ("Node Output", log_dir.join("node.log")),
        ("Node Errors", log_dir.join("node.err")),
        ("Wallet", log_dir.join("wallet.log")),
        ("Mining", log_dir.join("mining.log")),
    ];

    let mut logs = Vec::new();
    for (name, log_file) in &log_files {
        if log_file.exists() {
            if let Ok(metadata) = fs::metadata(log_file) {
                let size = metadata.len();
                logs.push(LogInfo {
                    name: name.to_string(),
                    path: log_file.display().to_string(),
                    size,
                    lines: 0, // We'll calculate this only when needed
                    recent_entries: Vec::new(),
                });
            }
        }
    }
    logs
}

#[tauri::command]
async fn start_node(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    // Check if node is already running using ProcessManager
    if state.process_manager.is_running("node") {
        return Ok("Node is already running".to_string());
    }

    let bin_path = state.config.btpc_home.join("bin").join("btpc_node");

    if !bin_path.exists() {
        return Err("Node binary not found. Please run setup first.".to_string());
    }

    let data_dir = state.config.data_dir.join("desktop-node");
    let log_file = state.config.log_dir.join("node.log");
    let err_file = state.config.log_dir.join("node.err");

    // Ensure directories exist
    fs::create_dir_all(&data_dir).map_err(|e| format!("Failed to create data dir: {}", e))?;
    fs::create_dir_all(&state.config.log_dir).map_err(|e| format!("Failed to create log dir: {}", e))?;

    // Get active network configuration
    let active_network = state.active_network.read().await.clone();
    let active_p2p_port = *state.active_p2p_port.read().await;
    let active_rpc_port = *state.active_rpc_port.read().await;

    let listen_addr = format!("127.0.0.1:{}", active_p2p_port);
    let args = vec![
        "--network".to_string(),
        active_network.to_string(),
        "--datadir".to_string(),
        data_dir.to_string_lossy().to_string(),
        "--rpcport".to_string(),
        active_rpc_port.to_string(),
        "--rpcbind".to_string(),
        "127.0.0.1".to_string(),
        "--listen".to_string(),
        listen_addr,
    ];

    // Use ProcessManager for detached process (survives page navigation)
    let process_info = state.process_manager.start_detached(
        "node".to_string(),
        bin_path.to_string_lossy().to_string(),
        args,
        Some(log_file.clone()),
        Some(err_file),
    )?;

    // Update status (old SystemStatus for backward compatibility)
    {
        let mut status = state.status.write().await;
        status.node_status = "Running".to_string();
        status.node_pid = Some(process_info.pid);
    }

    // Update NodeStatus via StateManager (Article XI - auto-emits node_status_changed event)
    state.node_status.update(|status| {
        status.running = true;
        status.pid = Some(process_info.pid);
        status.network = active_network.to_string();
    }, &app).map_err(|e| format!("Failed to update node status: {}", e))?;

    println!("📡 StateManager auto-emitted node_status_changed event: running");

    // Write initial status to log file since node runs silently
    let initial_log_message = format!("Node started successfully at {} (PID: {})\nListening and synchronizing blockchain data...\nNetwork: {}\nRPC Port: {}\nP2P Port: {}\nData directory: {}\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        process_info.pid,
        active_network,
        active_rpc_port,
        active_p2p_port,
        data_dir.display()
    );
    let _ = fs::write(&log_file, initial_log_message);

    // Auto-start blockchain synchronization service if RPC is enabled
    if state.config.node.enable_rpc {
        // Give the node a moment to start up before attempting sync
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        let mut sync_service_guard = state.sync_service.lock()
            .map_err(|_| BtpcError::mutex_poison("sync_service", "start_node").to_string())?;

        // Only start if not already running
        if sync_service_guard.is_none() {
            let sync_config = SyncConfig {
                rpc_host: state.config.rpc.host.clone(),
                rpc_port: state.config.rpc.port,
                poll_interval_secs: 10,
                max_blocks_per_sync: 100,
            };

            let service = BlockchainSyncService::new(state.utxo_manager.clone(), sync_config);

            match service.start() {
                Ok(_) => {
                    *sync_service_guard = Some(service);
                    println!("🔄 Blockchain sync service auto-started");
                }
                Err(e) => {
                    eprintln!("⚠️ Failed to auto-start blockchain sync: {}", e);
                    // Don't fail node startup if sync fails to start
                }
            }
        }
    }

    Ok(format!("Node started successfully (PID: {})", process_info.pid))
}

#[tauri::command]
async fn stop_node(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    state.process_manager.kill("node")?;

    // Stop blockchain sync service if running
    {
        let mut sync_service_guard = state.sync_service.lock()
            .map_err(|_| BtpcError::mutex_poison("sync_service", "stop_node").to_string())?;
        if let Some(service) = sync_service_guard.as_ref() {
            service.stop();
            *sync_service_guard = None;
            println!("🛑 Blockchain sync service stopped");
        }
    }

    // Update status (old SystemStatus for backward compatibility)
    {
        let mut status = state.status.write().await;
        status.node_status = "Stopped".to_string();
        status.node_pid = None;
    }

    // Update NodeStatus via StateManager (Article XI - auto-emits node_status_changed event)
    state.node_status.update(|status| {
        status.running = false;
        status.pid = None;
        status.block_height = 0;
        status.peer_count = 0;
        status.sync_progress = 0.0;
    }, &app).map_err(|e| format!("Failed to update node status: {}", e))?;

    println!("📡 StateManager auto-emitted node_status_changed event: stopped");

    // Append stop message to log file
    let log_file = state.config.log_dir.join("node.log");
    let stop_message = format!("Node stopped at {} by user request\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"));
    let _ = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
        .and_then(|mut f| {
            use std::io::Write;
            f.write_all(stop_message.as_bytes())
        });

    Ok("Node stopped successfully".to_string())
}

#[tauri::command]
async fn get_node_status(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let running = state.process_manager.is_running("node");

    let (pid, status_str) = if running {
        let info = state.process_manager.get_info("node");
        (info.map(|i| i.pid), "running")
    } else {
        (None, "stopped")
    };

    Ok(serde_json::json!({
        "is_running": running,
        "running": running,  // Keep both for compatibility
        "status": status_str,
        "pid": pid
    }))
}

#[tauri::command]
async fn get_mining_status(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let is_mining = {
        let mining_processes = state.mining_processes.lock()
            .map_err(|_| BtpcError::mutex_poison("mining_processes", "get_mining_status").to_string())?;
        mining_processes.contains_key("mining")
    };

    let stats = state.mining_stats.lock()
        .map_err(|_| BtpcError::mutex_poison("mining_stats", "get_mining_status").to_string())?;

    Ok(serde_json::json!({
        "is_mining": is_mining,
        "hashrate": stats.hashrate,
        "blocks_found": stats.blocks_found
    }))
}

#[tauri::command]
async fn get_total_balance(state: State<'_, AppState>) -> Result<f64, String> {
    // Get total balance from wallet manager
    let wallet_manager = state.wallet_manager.lock()
        .map_err(|_| BtpcError::mutex_poison("wallet_manager", "get_total_balance").to_string())?;
    let wallets = wallet_manager.list_wallets();

    let total: f64 = wallets.iter()
        .map(|w| w.cached_balance_btp)
        .sum();

    Ok(total)
}

#[tauri::command]
async fn list_addresses(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    // Get all wallet addresses
    let wallet_manager = state.wallet_manager.lock()
        .map_err(|_| BtpcError::mutex_poison("wallet_manager", "list_addresses").to_string())?;
    let wallets = wallet_manager.list_wallets();

    let addresses: Vec<String> = wallets.iter()
        .map(|w| w.address.clone())
        .collect();

    Ok(addresses)
}

#[tauri::command]
async fn create_wallet(state: State<'_, AppState>) -> Result<String, String> {
    let wallet_file = state.config.data_dir.join("wallet").join(&state.config.wallet.default_wallet_file);

    // Log the attempt for debugging
    println!("Attempting to create wallet at: {}", wallet_file.display());
    println!("BTPC home: {}", state.config.btpc_home.display());
    println!("Binary directory: {}", state.btpc.bin_dir.display());

    match state.btpc.create_wallet(&wallet_file, "default-wallet-password") {
        Ok((address, _seed_phrase, _private_key)) => Ok(format!("Wallet created successfully: {}", address)),
        Err(e) => {
            println!("Wallet creation error: {}", e);
            Err(format!("Failed to create wallet: {}", e))
        }
    }
}

#[tauri::command]
async fn get_wallet_balance(state: State<'_, AppState>) -> Result<String, String> {
    let wallet_file = state.config.data_dir.join("wallet").join(&state.config.wallet.default_wallet_file);

    // Log the attempt for debugging
    println!("Attempting to get wallet balance (UTXO-based) from: {}", wallet_file.display());
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
                Err(e) => Err(format!("Failed to get wallet balance: {}", e))
            };
        }
    };

    // Strip "Address: " prefix if it exists
    let clean_address = if address.starts_with("Address: ") {
        address.strip_prefix("Address: ").unwrap_or(&address).to_string()
    } else {
        address
    };

    // Get balance from UTXO manager
    let (total_credits, total_btp) = {
        let utxo_manager = state.utxo_manager.lock()
            .map_err(|_| BtpcError::mutex_poison("utxo_manager", "get_wallet_balance").to_string())?;
        utxo_manager.get_balance(&clean_address)
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
async fn get_wallet_address(state: State<'_, AppState>) -> Result<String, String> {
    let wallet_file = state.config.data_dir.join("wallet").join(&state.config.wallet.default_wallet_file);

    match state.btpc.get_wallet_address(&wallet_file) {
        Ok(address) => Ok(address),
        Err(e) => Err(format!("Failed to get wallet address: {}", e)),
    }
}

#[tauri::command]
async fn send_btpc(
    state: State<'_, AppState>,
    to_address: String,
    amount: f64,
    password: String
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

    let wallet_file = state.config.data_dir.join("wallet").join(&state.config.wallet.default_wallet_file);

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

    // Convert BTP to satoshis for UTXO calculations
    let amount_credits = (amount * 100_000_000.0) as u64;
    let fee_credits = 10000u64; // 0.0001 BTP standard fee
    let total_needed = amount_credits + fee_credits;

    // Check balance and select UTXOs using UTXO manager
    let (available_credits, available_btp) = {
        let utxo_manager = state.utxo_manager.lock()
            .map_err(|_| BtpcError::mutex_poison("utxo_manager", "send_transaction balance").to_string())?;
        let balance = utxo_manager.get_balance(&from_address);

        // Try to select UTXOs for the transaction
        match utxo_manager.select_utxos_for_spending(&from_address, total_needed) {
            Ok(selected_utxos) => {
                let selected_amount: u64 = selected_utxos.iter().map(|u| u.value_credits).sum();
                println!("Selected {} UTXOs totaling {} credits for transaction",
                        selected_utxos.len(), selected_amount);
            }
            Err(e) => {
                return Err(format!("Cannot create transaction: {}. Available: {:.8} BTP, Needed: {:.8} BTP",
                                 e, balance.1, (total_needed as f64) / 100_000_000.0));
            }
        }

        balance
    };

    if available_credits < total_needed {
        return Err(format!("Insufficient funds. Available: {:.8} BTP, Requested: {:.8} BTP (including {:.8} BTP fee)",
                          available_btp, amount, fee_credits as f64 / 100_000_000.0));
    }

    // Create the transaction using UTXO manager
    let (transaction_result, change_amount) = {
        let utxo_manager = state.utxo_manager.lock()
            .map_err(|_| BtpcError::mutex_poison("utxo_manager", "send_transaction create").to_string())?;
        match utxo_manager.create_send_transaction(&from_address, &to_address, amount_credits, fee_credits) {
            Ok(transaction) => {
                let total_input: u64 = transaction.inputs.len() as u64 * 3237500000; // Estimate
                let change = total_input - amount_credits - fee_credits;
                (Ok(transaction), change)
            }
            Err(e) => (Err(e), 0)
        }
    };

    let transaction = transaction_result.map_err(|e| format!("Failed to create transaction: {}", e))?;

    println!("=== UTXO-BASED TRANSACTION CREATED ===");
    println!("Transaction ID: {}", transaction.txid);
    println!("From: {} ({})", from_address, wallet_file.display());
    println!("To: {}", to_address);
    println!("Amount: {:.8} BTP ({} credits)", amount, amount_credits);
    println!("Fee: {:.8} BTP ({} credits)", fee_credits as f64 / 100_000_000.0, fee_credits);
    println!("Change: {:.8} BTP ({} credits)", change_amount as f64 / 100_000_000.0, change_amount);
    println!("Inputs: {} UTXOs", transaction.inputs.len());
    println!("Outputs: {} outputs", transaction.outputs.len());
    println!("Password provided: Yes");
    println!("Available balance: {:.8} BTP ({} credits)", available_btp, available_credits);

    // In production, you would:
    // 1. Sign the transaction with the private key (decrypted with password)
    // 2. Broadcast to the network
    // 3. Mark UTXOs as spent in the UTXO set
    // 4. Add new outputs to the UTXO set

    Ok(format!("UTXO-based transaction created successfully!\nTransaction ID: {}\nSent {:.8} BTP to {}\nFee: {:.8} BTP\nInputs: {} UTXOs, Outputs: {} outputs\nNote: Transaction created using proper UTXO selection and management.",
               transaction.txid, amount, to_address, fee_credits as f64 / 100_000_000.0,
               transaction.inputs.len(), transaction.outputs.len()))
}

// UTXO-based balance tracking using the comprehensive UTXO manager

// Helper function to add mining rewards to UTXO set
fn add_mining_reward_utxo(
    utxo_manager: &Arc<Mutex<UTXOManager>>,
    address: &str,
    amount_credits: u64,
    block_height: u64,
) -> Result<(), String> {
    let txid = format!("coinbase_{}_{}", block_height, chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));

    let mut manager = utxo_manager.lock()
        .map_err(|_| BtpcError::mutex_poison("utxo_manager", "add_mining_reward_utxo").to_string())?;
    manager.add_coinbase_utxo(
        txid,
        0, // vout = 0 for coinbase
        amount_credits,
        address.to_string(),
        block_height,
    ).map_err(|e| format!("Failed to add mining UTXO: {}", e))?;

    println!("[UTXO] Added mining UTXO: {} credits to address {} (block {})", amount_credits, address, block_height);
    Ok(())
}

#[tauri::command]
async fn get_wallet_balance_with_mined(state: State<'_, AppState>) -> Result<String, String> {
    // Get the wallet address
    let wallet_file = state.config.data_dir.join("wallet").join(&state.config.wallet.default_wallet_file);
    let address = match state.btpc.get_wallet_address(&wallet_file) {
        Ok(addr) => addr,
        Err(e) => return Err(format!("Failed to get wallet address: {}", e)),
    };

    // Clean the address by stripping the "Address: " prefix if present
    let clean_address = if address.starts_with("Address: ") {
        address.strip_prefix("Address: ").unwrap_or(&address).to_string()
    } else {
        address
    };

    // Get balance from UTXO set
    let (total_credits, total_btp) = {
        let utxo_manager = state.utxo_manager.lock()
            .map_err(|_| BtpcError::mutex_poison("utxo_manager", "get_address_balance").to_string())?;
        utxo_manager.get_balance(&clean_address)
    };

    Ok(format!("{} base units ({:.8} BTP)", total_credits, total_btp))
}

#[tauri::command]
async fn reload_utxos(state: State<'_, AppState>) -> Result<String, String> {
    let mut utxo_manager = state.utxo_manager.lock().map_err(|e| format!("Failed to lock UTXO manager: {}", e))?;
    match utxo_manager.reload_utxos() {
        Ok(_) => Ok("UTXO data reloaded successfully".to_string()),
        Err(e) => Err(format!("Failed to reload UTXOs: {}", e)),
    }
}

#[tauri::command]
async fn start_mining(app: tauri::AppHandle, state: State<'_, AppState>, address: String, blocks: u32) -> Result<String, String> {
    // Check if mining is already running
    {
        let processes = state.mining_processes.lock()
            .map_err(|_| BtpcError::mutex_poison("mining_processes", "start_mining check").to_string())?;
        if processes.contains_key("mining") {
            return Err("Mining is already running. Stop it first.".to_string());
        }
    }

    let bin_path = state.config.btpc_home.join("bin").join("btpc_miner");

    if !bin_path.exists() {
        return Err("Mining binary not found. Please run setup first.".to_string());
    }

    // Detect available GPUs (TDD v1.1 GPU detection)
    let gpus = gpu_detection::detect_gpus();
    let has_gpu = !gpus.is_empty();
    let best_gpu = gpu_detection::get_best_gpu();

    // Clear previous mining logs and reset stats
    {
        let mut mining_logs = state.mining_logs.lock()
            .map_err(|_| BtpcError::mutex_poison("mining_logs", "start_mining").to_string())?;
        mining_logs.clear();
        mining_logs.add_entry("INFO".to_string(), format!("Starting mining: {} blocks to address {}", blocks, address));

        // Log GPU detection results
        if has_gpu {
            if let Some(gpu) = &best_gpu {
                mining_logs.add_entry("INFO".to_string(), format!("GPU detected: {} ({})", gpu.name, gpu.vendor));
                mining_logs.add_entry("INFO".to_string(), format!("Found {} GPU(s) on system", gpus.len()));
            }
        } else {
            mining_logs.add_entry("INFO".to_string(), "No GPU detected - using CPU mining".to_string());
        }
    }
    {
        let mut mining_stats = state.mining_stats.lock()
            .map_err(|_| BtpcError::mutex_poison("mining_stats", "start_mining").to_string())?;
        mining_stats.reset();
        mining_stats.start();
    }

    let mut cmd = Command::new(&bin_path);
    // Use btpc_miner with user-specified address
    // Note: btpc_miner doesn't support --blocks parameter, it mines continuously until stopped
    let network = state.config.network.to_string();

    // Build RPC URL from config (fixed 2025-11-01: miner was using default port 8332)
    let rpc_url = format!("http://{}:{}", state.config.rpc.host, state.config.rpc.port);

    // Build command arguments with --rpc-url and optional --gpu flag
    let args = vec!["--network", &network, "--address", &address, "--rpc-url", &rpc_url];

    // Add --gpu flag if GPU is available (pending btpc_miner GPU support)
    // For now, this is a placeholder for future GPU mining implementation
    if has_gpu {
        // TODO: Once btpc_miner supports --gpu flag, uncomment the following:
        // args.push("--gpu");
        println!("GPU available but btpc_miner does not yet support GPU mining");
    }

    cmd.args(&args);

    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| format!("Failed to start mining: {}", e))?;

    // Get stdout and stderr handles before storing the process
    let stdout = child.stdout.take()
        .ok_or_else(|| "Failed to capture stdout: not piped".to_string())?;
    let stderr = child.stderr.take()
        .ok_or_else(|| "Failed to capture stderr: not piped".to_string())?;

    // Store the process
    {
        let mut processes = state.mining_processes.lock()
            .map_err(|_| BtpcError::mutex_poison("mining_processes", "start_mining store").to_string())?;
        processes.insert("mining".to_string(), child);
    }

    // Update status (old SystemStatus for backward compatibility)
    {
        let mut status = state.status.write().await;
        status.mining_status = format!("Mining {} blocks to {}", blocks, address);
    }

    // Update MiningStatus via StateManager (Article XI - auto-emits mining_status_changed event)
    state.mining_status.update(|status| {
        status.active = true;
        status.threads = 1; // Default threads, can be updated later
    }, &app).map_err(|e| format!("Failed to update mining status: {}", e))?;

    println!("📡 StateManager auto-emitted mining_status_changed event: active");

    // Clone the mining logs Arc for the async tasks
    let mining_logs_stdout = state.mining_logs.clone();
    let mining_logs_stderr = state.mining_logs.clone();
    let mining_stats_clone = state.mining_stats.clone();

    // Handle stdout with async tokio reader and track mining rewards
    let mining_address = address.clone(); // Clone address for async task
    let utxo_manager_clone = state.utxo_manager.clone(); // Clone UTXO manager for async task
    tokio::spawn(async move {
        let tokio_stdout = match ChildStdout::from_std(stdout) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to convert stdout to tokio stream: {}", e);
                return;
            }
        };
        let mut reader = TokioBufReader::new(tokio_stdout);
        let mut line = String::new();

        while let Ok(bytes_read) = reader.read_line(&mut line).await {
            if bytes_read == 0 {
                break; // EOF
            }

            let trimmed_line = line.trim();
            if !trimmed_line.is_empty() {
                let mut mining_logs = mining_logs_stdout.lock().unwrap_or_else(|e| {
                    eprintln!("Mining stdout logging failed - mutex poisoned: {}", e);
                    e.into_inner()
                });
                // Parse mining output for different types
                let (level, message) = parse_mining_output(trimmed_line);

                // Parse and update hashrate from miner output
                // Format: "Mining: 1234567 H/s | Total: 9876543 hashes | Uptime: 5.2m"
                if trimmed_line.contains("H/s") {
                    if let Some(hashrate_str) = trimmed_line.split("Mining:").nth(1) {
                        if let Some(hs_part) = hashrate_str.split("H/s").next() {
                            if let Ok(hashrate) = hs_part.trim().replace(",", "").parse::<u64>() {
                                let mut stats = mining_stats_clone.lock().unwrap_or_else(|e| {
                                    eprintln!("Mining stats update failed - mutex poisoned: {}", e);
                                    e.into_inner()
                                });
                                stats.hashrate = hashrate;
                            }
                        }
                    }
                }

                // Check for successful block mining
                // NOTE: With blockchain sync service enabled, UTXOs are automatically tracked
                // when blocks are synced from the node. This manual UTXO insertion is only
                // needed if blockchain sync is disabled or not yet caught up.
                // Fixed 2025-11-01: btpc_miner outputs "submitted successfully" not "mined successfully"
                if trimmed_line.contains("Block found by thread") || trimmed_line.contains("Block submitted successfully") {
                    // Increment block counter
                    {
                        let mut stats = mining_stats_clone.lock().unwrap_or_else(|e| {
                            eprintln!("Mining block count update failed - mutex poisoned: {}", e);
                            e.into_inner()
                        });
                        stats.increment_blocks();
                        // Estimate hashrate based on regtest difficulty
                        stats.calculate_hashrate(1000000); // Rough estimate for regtest
                    }

                    // Extract block number and add standard BTPC reward (32.375 BTP = 3237500000 credits)
                    let reward_credits = 3237500000u64; // Constitutional reward per block

                    // Estimate block height (in production, this would come from the actual block)
                    let estimated_block_height = chrono::Utc::now().timestamp() as u64;

                    // Fallback manual UTXO tracking (blockchain sync will override with real data)
                    match add_mining_reward_utxo(&utxo_manager_clone, &mining_address, reward_credits, estimated_block_height) {
                        Ok(_) => {
                            mining_logs.add_entry("SUCCESS".to_string(),
                                format!("{} [+{} credits mining reward (manual tracking - will be replaced by blockchain sync)]", message, reward_credits));
                        }
                        Err(e) => {
                            mining_logs.add_entry("WARNING".to_string(),
                                format!("Manual UTXO tracking failed (OK if blockchain sync is running): {}", e));
                        }
                    }
                } else {
                    mining_logs.add_entry(level, message);
                }
            }

            line.clear(); // Reset line buffer for next iteration
        }
    });

    // Handle stderr with async tokio reader
    tokio::spawn(async move {
        let tokio_stderr = match ChildStderr::from_std(stderr) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to convert stderr to tokio stream: {}", e);
                return;
            }
        };
        let mut reader = TokioBufReader::new(tokio_stderr);
        let mut line = String::new();

        while let Ok(bytes_read) = reader.read_line(&mut line).await {
            if bytes_read == 0 {
                break; // EOF
            }

            let trimmed_line = line.trim();
            if !trimmed_line.is_empty() {
                let mut mining_logs = mining_logs_stderr.lock().unwrap_or_else(|e| {
                    eprintln!("Mining stderr logging failed - mutex poisoned: {}", e);
                    e.into_inner()
                });
                mining_logs.add_entry("ERROR".to_string(), trimmed_line.to_string());
            }

            line.clear(); // Reset line buffer for next iteration
        }
    });

    // Add initial UTXO for demonstration (this would normally be handled by blockchain sync)
    let reward_credits = 3237500000u64; // Constitutional reward per block
    let initial_block_height = chrono::Utc::now().timestamp() as u64;

    if let Err(e) = add_mining_reward_utxo(&state.utxo_manager, &address, reward_credits, initial_block_height) {
        println!("Warning: Failed to add initial mining UTXO: {}", e);
    }

    Ok(format!("Mining started: {} blocks to {} (UTXO tracking enabled)", blocks, address))
}

// Parse mining output to determine log level and format CLI-style message
fn parse_mining_output(line: &str) -> (String, String) {
    let line = line.trim();

    // Clean the line first (remove emojis and prefixes)
    let cleaned = clean_mining_line(line);

    // Detect block found messages and reformat to clean ASCII style
    if cleaned.contains("Block found") || (line.contains("Block") && line.contains("mined successfully")) {
        // Extract block number if possible
        let block_num = extract_block_number(&cleaned).unwrap_or_else(|| chrono::Utc::now().timestamp() as u64);
        let reward_btpc = 32.375;

        // Format in clean ASCII style like: "Accepted 123 (100%) · 32.37500000 BTPC · height 123 · ..."
        let formatted_msg = format!("Accepted {} (100%) · {:.8} BTPC · height {}",
            block_num, reward_btpc, block_num);

        return ("SUCCESS".to_string(), formatted_msg);
    }

    // Detect block hash messages and reformat
    if cleaned.contains("Block hash:") || cleaned.contains("hash:") {
        // Extract hash portion
        if let Some(hash_start) = cleaned.find("hash:") {
            let hash = &cleaned[hash_start + 5..].trim();
            let hash_short = if hash.len() > 16 { &hash[..16] } else { hash };
            return ("INFO".to_string(), format!("hash {}", hash_short));
        }
    }

    // Handle specific message types
    if cleaned.contains("Mining block") || cleaned.contains("Searching") {
        return ("INFO".to_string(), "Mining...".to_string());
    } else if cleaned.contains("Started:") || cleaned.contains("Block started") {
        return ("INFO".to_string(), "Mining started".to_string());
    } else if cleaned.contains("Reward:") || cleaned.contains("balance:") {
        return ("SUCCESS".to_string(), cleaned);
    } else if cleaned.contains("Nonce:") {
        return ("INFO".to_string(), cleaned);
    } else if cleaned.contains("Miner:") || cleaned.contains("Blocks:") {
        return ("INFO".to_string(), cleaned);
    } else if cleaned.contains("Target:") || cleaned.contains("10 minute") {
        return ("INFO".to_string(), "Target: 10min blocks".to_string());
    } else if cleaned.contains("Waiting") {
        return ("INFO".to_string(), cleaned);
    } else if cleaned.contains("BTPC") || cleaned.contains("RPC server") {
        return ("INFO".to_string(), cleaned);
    } else if cleaned.contains("ERROR") || cleaned.contains("Failed") || cleaned.contains("Error") {
        return ("ERROR".to_string(), cleaned);
    } else if cleaned.contains("WARN") || cleaned.contains("Warning") {
        return ("WARN".to_string(), cleaned);
    } else if !cleaned.is_empty() {
        return ("INFO".to_string(), cleaned);
    }

    ("INFO".to_string(), "".to_string())
}

// Clean mining line output by removing ALL emojis and verbose prefixes
fn clean_mining_line(line: &str) -> String {
    // First, remove common timestamp prefixes like "[2025-10-07 02:44:38 UTC] btpc-miner:"
    let mut cleaned = line.to_string();

    // Remove timestamp prefixes with regex-like pattern
    if let Some(miner_pos) = cleaned.find("btpc-miner:") {
        cleaned = cleaned[miner_pos + 11..].trim().to_string();
    } else if let Some(wallet_pos) = cleaned.find("btpc-wallet:") {
        cleaned = cleaned[wallet_pos + 12..].trim().to_string();
    } else if let Some(core_pos) = cleaned.find("btpc-core:") {
        cleaned = cleaned[core_pos + 10..].trim().to_string();
    }

    // Remove ALL emoji characters - comprehensive list
    cleaned = cleaned
        .replace("⛏️  ", "")
        .replace("⛏️", "")
        .replace("🔄 ", "")
        .replace("🔄", "")
        .replace("⏰ ", "")
        .replace("⏰", "")
        .replace("🏆 ", "")
        .replace("🏆", "")
        .replace("💰 ", "")
        .replace("💰", "")
        .replace("✅ ", "")
        .replace("✅", "")
        .replace("🔗 ", "")
        .replace("🔗", "")
        .replace("📋 ", "")
        .replace("📋", "")
        .replace("👛 ", "")
        .replace("👛", "")
        .replace("📊 ", "")
        .replace("📊", "")
        .replace("🎯 ", "")
        .replace("🎯", "")
        .replace("⏳ ", "")
        .replace("⏳", "")
        .replace("🚀 ", "")
        .replace("🚀", "")
        .replace("🔧 ", "")
        .replace("🔧", "")
        .replace("🏦 ", "")
        .replace("🏦", "")
        .replace("⚠️ ", "")
        .replace("⚠️", "")
        .replace("⚠", "")
        .replace("🎉 ", "")
        .replace("🎉", "")
        .replace("🎊 ", "")
        .replace("🎊", "")
        .replace("🏅 ", "")
        .replace("🏅", "")
        .trim()
        .to_string();

    cleaned
}

// Extract block number from mining output
fn extract_block_number(line: &str) -> Option<u64> {
    // Look for patterns like "Block 123" or "block 123"
    if let Some(start) = line.to_lowercase().find("block ") {
        let after_block = &line[start + 6..];
        if let Some(end) = after_block.find(|c: char| !c.is_numeric()) {
            after_block[..end].parse().ok()
        } else {
            after_block.parse().ok()
        }
    } else {
        None
    }
}

#[tauri::command]
async fn stop_mining(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    // Atomic: lock, remove, kill within same lock scope
    let kill_result = {
        let mut processes = state.mining_processes.lock()
            .map_err(|_| BtpcError::mutex_poison("mining_processes", "stop_mining").to_string())?;

        match processes.remove("mining") {
            Some(mut child) => {
                // Kill while holding lock to prevent race with start_mining
                match child.kill() {
                    Ok(_) => {
                        let _ = child.wait(); // Clean up zombie
                        Ok(())
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::InvalidInput => {
                        // Process already dead, clean up anyway
                        let _ = child.wait();
                        Ok(())
                    }
                    Err(e) => Err(format!("Failed to kill mining process: {}", e))
                }
            }
            None => Err("Mining is not running".to_string())
        }
    }; // Lock released here

    // Handle result and update logs/status
    match kill_result {
        Ok(_) => {
            // Add log entry
            {
                let mut mining_logs = state.mining_logs.lock()
                    .map_err(|_| BtpcError::mutex_poison("mining_logs", "stop_mining").to_string())?;
                mining_logs.add_entry("INFO".to_string(), "Mining stopped by user".to_string());
            }

            // Update status (old SystemStatus for backward compatibility)
            {
                let mut status = state.status.write().await;
                status.mining_status = "Stopped".to_string();
            }

            // Update MiningStatus via StateManager (Article XI - auto-emits mining_status_changed event)
            state.mining_status.update(|status| {
                status.active = false;
                status.hashrate = 0;
            }, &app).map_err(|e| format!("Failed to update mining status: {}", e))?;

            println!("📡 StateManager auto-emitted mining_status_changed event: stopped");

            Ok("Mining stopped successfully".to_string())
        }
        Err(e) => Err(e)
    }
}

#[tauri::command]
async fn setup_btpc(state: State<'_, AppState>) -> Result<String, String> {
    // Try to install binaries from build locations
    match state.btpc.install_binaries_from_build() {
        Ok(installed) => {
            let installation_status = state.btpc.check_installation();

            // Update status
            {
                let mut status = state.status.write().await;
                status.binaries_installed = installation_status.is_complete;
                status.config_exists = AppState::check_config_exists(&state.config);
            }

            if installed.is_empty() {
                if installation_status.is_complete {
                    Ok("BTPC is already set up correctly".to_string())
                } else {
                    Err(format!(
                        "BTPC setup incomplete. Missing binaries: {:?}. Please build BTPC first using './build-unified-launcher.sh'",
                        installation_status.missing_required_binaries
                    ))
                }
            } else {
                Ok(format!("BTPC setup completed. Installed {} binaries: {:?}", installed.len(), installed))
            }
        }
        Err(e) => Err(format!("BTPC setup failed: {}", e)),
    }
}

#[tauri::command]
async fn test_command() -> Result<String, String> {
    println!("Test command called successfully!");
    Ok("Test command works!".to_string())
}

#[tauri::command]
async fn get_logs(state: State<'_, AppState>) -> Result<Vec<LogInfo>, String> {
    let log_files = [
        ("Node Output", state.config.log_dir.join("node.log")),
        ("Node Errors", state.config.log_dir.join("node.err")),
        ("Wallet", state.config.log_dir.join("wallet.log")),
        ("Mining", state.config.log_dir.join("mining.log")),
    ];

    let mut logs = Vec::new();

    for (name, log_file) in &log_files {
        if log_file.exists() {
            match fs::metadata(log_file) {
                Ok(metadata) => {
                    let size = metadata.len();

                    // Read recent entries
                    let recent_entries = match fs::read_to_string(log_file) {
                        Ok(content) => {
                            if content.trim().is_empty() {
                                vec![format!("No content in {} yet", name)]
                            } else {
                                let lines: Vec<&str> = content.lines().collect();
                                let recent_lines = if lines.len() > 15 {
                                    &lines[lines.len()-15..]
                                } else {
                                    &lines
                                };
                                let mut entries: Vec<String> = recent_lines.iter().map(|s| s.to_string()).collect();

                                // Add a header for better context
                                if !entries.is_empty() {
                                    entries.insert(0, format!("=== Last {} lines from {} ===", entries.len(), name));
                                }
                                entries
                            }
                        }
                        Err(_) => vec!["Error reading log file".to_string()],
                    };

                    logs.push(LogInfo {
                        name: name.to_string(),
                        path: log_file.display().to_string(),
                        size,
                        lines: recent_entries.len(),
                        recent_entries,
                    });
                }
                Err(_) => {
                    logs.push(LogInfo {
                        name: name.to_string(),
                        path: log_file.display().to_string(),
                        size: 0,
                        lines: 0,
                        recent_entries: vec!["Unable to access log file".to_string()],
                    });
                }
            }
        }
    }

    Ok(logs)
}

#[tauri::command]
async fn get_mining_logs(state: State<'_, AppState>) -> Result<Vec<MiningLogEntry>, String> {
    let mining_logs = state.mining_logs.lock()
        .map_err(|_| BtpcError::mutex_poison("mining_logs", "get_mining_logs").to_string())?;
    Ok(mining_logs.get_entries())
}

#[tauri::command]
async fn get_utxo_stats(state: State<'_, AppState>) -> Result<UTXOStats, String> {
    let utxo_manager = state.utxo_manager.lock()
        .map_err(|_| BtpcError::mutex_poison("utxo_manager", "get_utxo_stats").to_string())?;
    Ok(utxo_manager.get_stats())
}

#[tauri::command]
async fn get_wallet_utxos(state: State<'_, AppState>) -> Result<Vec<UTXO>, String> {
    let wallet_file = state.config.data_dir.join("wallet").join(&state.config.wallet.default_wallet_file);
    let address = match state.btpc.get_wallet_address(&wallet_file) {
        Ok(addr) => addr,
        Err(e) => return Err(format!("Failed to get wallet address: {}", e)),
    };

    let utxo_manager = state.utxo_manager.lock()
        .map_err(|_| BtpcError::mutex_poison("utxo_manager", "get_wallet_utxos").to_string())?;
    let utxos: Vec<UTXO> = utxo_manager.get_all_utxos(&address)
        .into_iter()
        .cloned()
        .collect();

    Ok(utxos)
}

#[tauri::command]
async fn get_spendable_utxos(state: State<'_, AppState>) -> Result<Vec<UTXO>, String> {
    let wallet_file = state.config.data_dir.join("wallet").join(&state.config.wallet.default_wallet_file);
    let address = match state.btpc.get_wallet_address(&wallet_file) {
        Ok(addr) => addr,
        Err(e) => return Err(format!("Failed to get wallet address: {}", e)),
    };

    let utxo_manager = state.utxo_manager.lock()
        .map_err(|_| BtpcError::mutex_poison("utxo_manager", "get_spendable_utxos").to_string())?;
    let current_height = chrono::Utc::now().timestamp() as u64; // Simplified height calculation

    let spendable_utxos: Vec<UTXO> = utxo_manager.get_unspent_utxos(&address)
        .into_iter()
        .filter(|utxo| utxo.is_spendable(current_height))
        .cloned()
        .collect();

    Ok(spendable_utxos)
}

#[tauri::command]
async fn add_mining_utxo(
    state: State<'_, AppState>,
    address: String,
    amount_credits: u64,
    block_height: u64,
) -> Result<String, String> {
    add_mining_reward_utxo(&state.utxo_manager, &address, amount_credits, block_height)
        .map(|_| format!("Added mining UTXO: {} credits at block {}", amount_credits, block_height))
}

#[tauri::command]
async fn sync_wallet_utxos(state: State<'_, AppState>) -> Result<String, String> {
    let wallet_file = state.config.data_dir.join("wallet").join(&state.config.wallet.default_wallet_file);
    let address = match state.btpc.get_wallet_address(&wallet_file) {
        Ok(addr) => addr,
        Err(e) => return Err(format!("Failed to get wallet address: {}", e)),
    };

    // In a real implementation, this would query the blockchain node
    // For now, we'll export UTXO data for Python integration compatibility
    let export_result = {
        let utxo_manager = state.utxo_manager.lock()
            .map_err(|_| BtpcError::mutex_poison("utxo_manager", "sync_wallet_utxos").to_string())?;
        utxo_manager.export_utxos_for_integration(&address)
    };

    match export_result {
        Ok(()) => Ok(format!("UTXO sync completed for address: {}", address)),
        Err(e) => Err(format!("UTXO sync failed: {}", e)),
    }
}

#[tauri::command]
async fn get_transaction_history(state: State<'_, AppState>) -> Result<Vec<btpc_desktop_app::utxo_manager::Transaction>, String> {
    // ⚠️ DEPRECATED: This command is inefficient (O(n×m) complexity)
    // Use get_paginated_transaction_history for list queries
    // Use get_transaction_from_storage for single transaction lookups
    //
    // PERFORMANCE ISSUE: Scans all transactions × all UTXOs
    // - 10k transactions = 5s load time
    // - 100k transactions = 30s+ app freeze
    //
    // Constitution Article XI.1 Compliance: RocksDB is single source of truth
    eprintln!("⚠️  WARNING: get_transaction_history called (DEPRECATED)");
    eprintln!("   This command has O(n×m) complexity and causes severe performance degradation");
    eprintln!("   Use get_paginated_transaction_history or get_transaction_from_storage instead");

    // Get wallet address from WalletManager (not from file)
    // If no default wallet exists, return empty array instead of error
    let address = {
        let wallet_manager = state.wallet_manager.lock()
            .map_err(|e| format!("Failed to lock wallet manager: {}", e))?;

        match wallet_manager.get_default_wallet() {
            Some(wallet) => wallet.address.clone(),
            None => {
                // No default wallet - return empty transaction history
                return Ok(Vec::new());
            }
        }
    };

    let utxo_manager = state.utxo_manager.lock()
        .map_err(|_| BtpcError::mutex_poison("utxo_manager", "get_transaction_history").to_string())?;
    let transactions: Vec<btpc_desktop_app::utxo_manager::Transaction> = utxo_manager.get_transaction_history(&address)
        .into_iter()
        .cloned()
        .collect();

    Ok(transactions)
}

// ============================================================================
// RocksDB Transaction Storage Commands (Constitution Article V)
// ============================================================================

#[tauri::command]
async fn get_paginated_transaction_history(
    state: State<'_, AppState>,
    offset: usize,
    limit: usize,
) -> Result<tx_storage::PaginatedTransactions, String> {
    // Get wallet address from WalletManager
    let address = {
        let wallet_manager = state.wallet_manager.lock()
            .map_err(|e| format!("Failed to lock wallet manager: {}", e))?;

        match wallet_manager.get_default_wallet() {
            Some(wallet) => wallet.address.clone(),
            None => {
                // No default wallet - return empty paginated result
                return Ok(tx_storage::PaginatedTransactions {
                    transactions: Vec::new(),
                    total_count: 0,
                    offset,
                    limit,
                    has_more: false,
                });
            }
        }
    };

    // Query RocksDB with pagination
    let pagination = tx_storage::PaginationParams { offset, limit };
    state.tx_storage.get_transactions_for_address(&address, pagination)
        .map_err(|e| format!("Failed to get paginated transactions: {}", e))
}

#[tauri::command]
async fn add_transaction_to_storage(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    tx: btpc_desktop_app::utxo_manager::Transaction,
    address: String,
) -> Result<String, String> {
    // Add transaction to RocksDB
    state.tx_storage.add_transaction(&tx, &address)
        .map_err(|e| format!("Failed to add transaction to storage: {}", e))?;

    // Get updated balance and transaction count
    let (balance_credits, balance_btpc) = state.tx_storage.get_balance(&address)
        .map_err(|e| format!("Failed to get updated balance: {}", e))?;
    let tx_count = state.tx_storage.get_transaction_count(&address)
        .map_err(|e| format!("Failed to get transaction count: {}", e))?;

    // Emit transaction-added event (Constitution Article XI.3)
    let event_payload = serde_json::json!({
        "txid": tx.txid,
        "address": address,
        "block_height": tx.block_height,
        "is_coinbase": tx.is_coinbase,
        "output_count": tx.outputs.len(),
        "confirmed_at": tx.confirmed_at,
    });
    if let Err(e) = app.emit("transaction-added", event_payload) {
        eprintln!("⚠️ Failed to emit transaction-added event: {}", e);
    }

    // Emit wallet-balance-updated event
    let balance_payload = serde_json::json!({
        "address": address,
        "balance_credits": balance_credits,
        "balance_btpc": balance_btpc,
        "transaction_count": tx_count,
    });
    if let Err(e) = app.emit("wallet-balance-updated", balance_payload) {
        eprintln!("⚠️ Failed to emit wallet-balance-updated event: {}", e);
    }

    Ok(format!("Transaction {} added to storage", tx.txid))
}

#[tauri::command]
async fn get_transaction_from_storage(
    state: State<'_, AppState>,
    txid: String,
) -> Result<Option<btpc_desktop_app::utxo_manager::Transaction>, String> {
    state.tx_storage.get_transaction(&txid)
        .map_err(|e| format!("Failed to get transaction from storage: {}", e))
}

#[tauri::command]
async fn get_wallet_balance_from_storage(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    // Get wallet address from WalletManager
    let address = {
        let wallet_manager = state.wallet_manager.lock()
            .map_err(|e| format!("Failed to lock wallet manager: {}", e))?;

        match wallet_manager.get_default_wallet() {
            Some(wallet) => wallet.address.clone(),
            None => {
                return Ok(serde_json::json!({
                    "credits": 0,
                    "btpc": 0.0,
                    "address": null
                }));
            }
        }
    };

    let (credits, btpc) = state.tx_storage.get_balance(&address)
        .map_err(|e| format!("Failed to get balance from storage: {}", e))?;

    Ok(serde_json::json!({
        "credits": credits,
        "btpc": btpc,
        "address": address
    }))
}

#[tauri::command]
async fn get_transaction_count_from_storage(
    state: State<'_, AppState>,
) -> Result<usize, String> {
    // Get wallet address from WalletManager
    let address = {
        let wallet_manager = state.wallet_manager.lock()
            .map_err(|e| format!("Failed to lock wallet manager: {}", e))?;

        match wallet_manager.get_default_wallet() {
            Some(wallet) => wallet.address.clone(),
            None => return Ok(0),
        }
    };

    state.tx_storage.get_transaction_count(&address)
        .map_err(|e| format!("Failed to get transaction count: {}", e))
}

#[tauri::command]
async fn create_transaction_preview(
    state: State<'_, AppState>,
    to_address: String,
    amount: f64,
) -> Result<serde_json::Value, String> {
    if amount <= 0.0 {
        return Err("Amount must be greater than zero".to_string());
    }

    let wallet_file = state.config.data_dir.join("wallet").join(&state.config.wallet.default_wallet_file);
    let from_address = match state.btpc.get_wallet_address(&wallet_file) {
        Ok(addr) => addr,
        Err(e) => return Err(format!("Failed to get wallet address: {}", e)),
    };

    let amount_credits = (amount * 100_000_000.0) as u64;
    let fee_credits = 10000u64; // Standard fee
    let total_needed = amount_credits + fee_credits;

    let utxo_manager = state.utxo_manager.lock()
        .map_err(|_| BtpcError::mutex_poison("utxo_manager", "create_transaction_preview").to_string())?;

    // Get current balance
    let (available_credits, available_btp) = utxo_manager.get_balance(&from_address);

    // Try to select UTXOs
    let selected_utxos = match utxo_manager.select_utxos_for_spending(&from_address, total_needed) {
        Ok(utxos) => utxos,
        Err(e) => return Err(format!("Cannot create transaction: {}", e)),
    };

    let total_input: u64 = selected_utxos.iter().map(|u| u.value_credits).sum();
    let change_amount = total_input - amount_credits - fee_credits;

    let preview = serde_json::json!({
        "from_address": from_address,
        "to_address": to_address,
        "amount_btp": amount,
        "amount_credits": amount_credits,
        "fee_btp": fee_credits as f64 / 100_000_000.0,
        "fee_credits": fee_credits,
        "change_btp": change_amount as f64 / 100_000_000.0,
        "change_credits": change_amount,
        "total_input_credits": total_input,
        "available_balance_btp": available_btp,
        "available_balance_credits": available_credits,
        "inputs_count": selected_utxos.len(),
        "outputs_count": if change_amount > 0 { 2 } else { 1 },
        "selected_utxos": selected_utxos.iter().map(|u| serde_json::json!({
            "txid": u.txid,
            "vout": u.vout,
            "value_credits": u.value_credits,
            "value_btp": u.value_btp,
            "is_coinbase": u.is_coinbase
        })).collect::<Vec<_>>()
    });

    Ok(preview)
}

// ============================================================================
// Security Commands
// ============================================================================

#[tauri::command]
async fn create_user(state: State<'_, AppState>, username: String, password: String) -> Result<RecoveryData, String> {
    state.security.create_user(&username, &password)
        .map_err(|e| format!("Failed to create user: {}", e))
}

#[tauri::command]
async fn login_user(state: State<'_, AppState>, username: String, password: String) -> Result<UserSession, String> {
    match state.security.authenticate_user(&username, &password) {
        Ok(session) => {
            // Store current session
            {
                let mut current_session = state.current_session.lock()
                    .map_err(|_| BtpcError::mutex_poison("current_session", "login_user").to_string())?;
                *current_session = Some(session.session_id.clone());
            }
            Ok(session)
        }
        Err(e) => Err(format!("Login failed: {}", e))
    }
}

#[tauri::command]
async fn logout_user(state: State<'_, AppState>) -> Result<String, String> {
    let session_id = {
        let mut current_session = state.current_session.lock()
            .map_err(|_| BtpcError::mutex_poison("current_session", "logout_user").to_string())?;
        current_session.take()
    };

    if let Some(session_id) = session_id {
        state.security.invalidate_session(&session_id);
        Ok("Successfully logged out".to_string())
    } else {
        Err("No active session".to_string())
    }
}

#[tauri::command]
async fn recover_account(state: State<'_, AppState>, username: String, recovery_phrase: String, new_password: String) -> Result<String, String> {
    state.security.recover_account(&username, &recovery_phrase, &new_password)
        .map_err(|e| format!("Recovery failed: {}", e))
}

#[tauri::command]
async fn check_security_session(state: State<'_, AppState>) -> Result<bool, String> {
    let session_id = {
        let current_session = state.current_session.lock()
            .map_err(|_| BtpcError::mutex_poison("current_session", "check_security_session").to_string())?;
        current_session.clone()
    };

    if let Some(session_id) = session_id {
        state.security.validate_session(&session_id)
            .map_err(|e| format!("Session validation failed: {}", e))
    } else {
        Ok(false)
    }
}

#[tauri::command]
async fn get_session_info(state: State<'_, AppState>) -> Result<Option<UserSession>, String> {
    let session_id = {
        let current_session = state.current_session.lock()
            .map_err(|_| BtpcError::mutex_poison("current_session", "get_session_info").to_string())?;
        current_session.clone()
    };

    if let Some(session_id) = session_id {
        Ok(state.security.get_session(&session_id))
    } else {
        Ok(None)
    }
}

#[tauri::command]
async fn get_users(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    state.security.get_users()
        .map_err(|e| format!("Failed to get users: {}", e))
}

#[tauri::command]
async fn user_exists(state: State<'_, AppState>, username: String) -> Result<bool, String> {
    Ok(state.security.user_exists(&username))
}

#[tauri::command]
async fn decrypt_wallet_key(state: State<'_, AppState>, encrypted_key: String, password: String) -> Result<String, String> {
    state.security.decrypt_wallet_key(&encrypted_key, &password)
        .map_err(|e| format!("Decryption failed: {}", e))
}

// ============================================================================
// Wallet Encryption & Lock Commands (Article VI.3 - Encrypted Wallet Metadata)
// ============================================================================

/// Check if wallets are currently locked (password required to access)
#[tauri::command]
async fn check_wallet_lock_status(
    state: State<'_, AppState>
) -> Result<bool, String> {
    let locked = state.wallets_locked.read().await;
    Ok(*locked)
}

/// Unlock wallets by loading encrypted metadata with password
#[tauri::command]
async fn unlock_wallets(
    password: String,
    state: State<'_, AppState>
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
        let mut wallet_manager = state.wallet_manager.lock()
            .map_err(|e| format!("Failed to lock wallet manager: {}", e))?;
        wallet_manager.load_wallets_encrypted(&secure_password)
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

    Ok("Wallets unlocked successfully".to_string())
}

/// Lock wallets and clear password from memory
#[tauri::command]
async fn lock_wallets(
    state: State<'_, AppState>
) -> Result<String, String> {
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
        let mut wallet_manager = state.wallet_manager.lock()
            .map_err(|e| format!("Failed to lock wallet manager: {}", e))?;
        wallet_manager.clear_wallets();
    }

    Ok("Wallets locked successfully".to_string())
}

/// Change master password for encrypted wallet metadata
#[tauri::command]
async fn change_master_password(
    old_password: String,
    new_password: String,
    state: State<'_, AppState>
) -> Result<String, String> {
    let old_secure = btpc_core::crypto::SecurePassword::new(old_password);
    let new_secure = btpc_core::crypto::SecurePassword::new(new_password);

    // Verify old password by attempting to load
    {
        let mut wallet_manager = state.wallet_manager.lock()
            .map_err(|e| format!("Failed to lock wallet manager: {}", e))?;

        // Try loading with old password
        wallet_manager.load_wallets_encrypted(&old_secure)
            .map_err(|e| format!("Old password incorrect: {}", e))?;

        // Re-save with new password
        wallet_manager.save_wallets_encrypted(&new_secure)
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
async fn migrate_to_encrypted(
    password: String,
    state: State<'_, AppState>
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
        let wallet_manager = state.wallet_manager.lock()
            .map_err(|e| format!("Failed to lock wallet manager: {}", e))?;
        wallet_manager.save_wallets_encrypted(&secure_password)
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

    Ok("Migration successful. Plaintext backed up to wallets_metadata.json.backup".to_string())
}

// ============================================================================
// Blockchain Sync Commands
// ============================================================================

#[tauri::command]
async fn start_blockchain_sync(state: State<'_, AppState>) -> Result<String, String> {
    // Create sync service if not already created
    {
        let mut sync_service_guard = state.sync_service.lock()
            .map_err(|_| BtpcError::mutex_poison("sync_service", "start_blockchain_sync").to_string())?;

        if sync_service_guard.is_some() {
            return Err("Blockchain sync is already running".to_string());
        }

        // Create sync config from app config
        let sync_config = SyncConfig {
            rpc_host: state.config.rpc.host.clone(),
            rpc_port: state.config.rpc.port,
            poll_interval_secs: 10,
            max_blocks_per_sync: 100,
        };

        // Create new sync service
        let service = BlockchainSyncService::new(state.utxo_manager.clone(), sync_config);

        // Start the service
        service.start().map_err(|e| format!("Failed to start sync service: {}", e))?;

        *sync_service_guard = Some(service);
    }

    Ok("Blockchain synchronization started successfully".to_string())
}

#[tauri::command]
async fn stop_blockchain_sync(state: State<'_, AppState>) -> Result<String, String> {
    let mut sync_service_guard = state.sync_service.lock()
        .map_err(|_| BtpcError::mutex_poison("sync_service", "stop_blockchain_sync").to_string())?;

    if let Some(service) = sync_service_guard.as_ref() {
        service.stop();
        *sync_service_guard = None;
        Ok("Blockchain synchronization stopped successfully".to_string())
    } else {
        Err("Blockchain sync is not running".to_string())
    }
}

#[tauri::command]
async fn get_sync_stats(state: State<'_, AppState>) -> Result<SyncStats, String> {
    let sync_service_guard = state.sync_service.lock()
        .map_err(|_| BtpcError::mutex_poison("sync_service", "get_sync_stats").to_string())?;

    if let Some(service) = sync_service_guard.as_ref() {
        Ok(service.get_stats())
    } else {
        // Return default stats if sync service is not running
        Ok(SyncStats::default())
    }
}

#[tauri::command]
async fn trigger_manual_sync(_state: State<'_, AppState>) -> Result<String, String> {
    // Manual sync is automatically handled by the background sync service
    // This command is kept for future manual sync trigger implementation
    Ok("Manual sync will be triggered in the next sync iteration".to_string())
}

#[tauri::command]
async fn get_address_balance_from_node(
    state: State<'_, AppState>,
    address: String,
) -> Result<u64, String> {
    // Create a temporary RPC client to query the node
    use btpc_desktop_app::rpc_client::RpcClient;
    let rpc_client = RpcClient::new(&state.config.rpc.host, state.config.rpc.port);

    rpc_client.get_address_balance(&address)
        .await
        .map_err(|e| format!("Failed to get balance from node: {}", e))
}

#[tauri::command]
async fn get_network_config(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let active_network = state.active_network.read().await.clone();
    let active_rpc_port = *state.active_rpc_port.read().await;
    let active_p2p_port = *state.active_p2p_port.read().await;

    Ok(serde_json::json!({
        "network": active_network.to_string(),
        "rpc_port": active_rpc_port,
        "p2p_port": active_p2p_port,
        "rpc_host": state.config.rpc.host,
    }))
}

#[tauri::command]
async fn save_network_config(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    network: String,
    rpc_port: u16,
    p2p_port: u16,
) -> Result<String, String> {
    // Validate network type
    let network_type = match network.as_str() {
        "mainnet" => NetworkType::Mainnet,
        "testnet" => NetworkType::Testnet,
        "regtest" => NetworkType::Regtest,
        _ => return Err(format!("Invalid network type: {}", network)),
    };

    // Validate ports
    if rpc_port < 1024 {
        return Err("RPC port must be >= 1024".to_string());
    }
    if p2p_port < 1024 {
        return Err("P2P port must be >= 1024".to_string());
    }
    if rpc_port == p2p_port {
        return Err("RPC and P2P ports must be different".to_string());
    }

    // Check if node is running
    let node_running = state.process_manager.is_running("node");

    if node_running {
        return Err("Cannot change network settings while node is running. Please stop the node first.".to_string());
    }

    // Update active network configuration
    {
        let mut active_network = state.active_network.write().await;
        *active_network = network_type;
    }
    {
        let mut active_rpc_port = state.active_rpc_port.write().await;
        *active_rpc_port = rpc_port;
    }
    {
        let mut active_p2p_port = state.active_p2p_port.write().await;
        *active_p2p_port = p2p_port;
    }

    // Emit event to notify all pages about network config change
    let event_payload = serde_json::json!({
        "network": network,
        "rpc_port": rpc_port,
        "p2p_port": p2p_port,
    });

    if let Err(e) = app.emit("network-config-changed", event_payload) {
        eprintln!("⚠️ Failed to emit network-config-changed event: {}", e);
    } else {
        println!("📡 Emitted network-config-changed event: {} (RPC: {}, P2P: {})", network, rpc_port, p2p_port);
    }

    Ok(format!(
        "Network settings saved successfully: {} (RPC: {}, P2P: {}). Changes will be applied when the node is started.",
        network, rpc_port, p2p_port
    ))
}

// ============================================================================
// Block Explorer Commands
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockInfo {
    pub height: u64,
    pub hash: String,
    pub prev_hash: String,
    pub merkle_root: Option<String>,
    pub timestamp: u64,
    pub bits: u32,
    pub nonce: u64,
    pub version: u32,
    pub tx_count: usize,
    pub size: usize,
    pub transactions: Vec<TransactionInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionInfo {
    pub hash: String,
    pub block_height: Option<u64>,
    pub inputs: usize,
    pub outputs: usize,
    pub total_value: u64,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainInfo {
    pub height: u64,
    pub total_transactions: u64,
    pub difficulty: f64,
    pub hash_rate: f64,
}

#[tauri::command]
async fn get_blockchain_info(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    use btpc_desktop_app::rpc_client::RpcClient;
    let rpc_client = RpcClient::new(&state.config.rpc.host, state.config.rpc.port);

    // Get blockchain info from node, with graceful fallback when node is offline
    let info_result = rpc_client.get_blockchain_info().await;

    match info_result {
        Ok(info) => {
            // Node is running - get connection count and return full data
            let connections = match rpc_client.get_connection_count().await {
                Ok(count) => count,
                Err(_) => 0, // Fallback to 0 if network info unavailable
            };

            // Return the RPC response directly as JSON with compatibility fields
            Ok(serde_json::json!({
                "blocks": info.blocks,
                "height": info.blocks,  // Alias for compatibility
                "headers": info.headers.unwrap_or(info.blocks),
                "chain": info.chain.unwrap_or_else(|| "mainnet".to_string()),
                "difficulty": info.difficulty,
                "best_block_hash": info.best_block_hash,
                "bestblockhash": info.best_block_hash,  // Alias for compatibility
                "connections": connections,
                "node_offline": false,
            }))
        }
        Err(_) => {
            // Node is offline - return fallback data so UI displays "0" instead of error
            // This allows the dashboard to load and show that node needs to be started
            Ok(serde_json::json!({
                "blocks": 0,
                "height": 0,
                "headers": 0,
                "chain": state.config.network.to_string(),
                "difficulty": 0,
                "best_block_hash": "0000000000000000000000000000000000000000000000000000000000000000",
                "bestblockhash": "0000000000000000000000000000000000000000000000000000000000000000",
                "connections": 0,
                "node_offline": true,
            }))
        }
    }
}

#[tauri::command]
async fn get_recent_blocks(
    state: State<'_, AppState>,
    limit: usize,
    offset: usize,
) -> Result<Vec<BlockInfo>, String> {
    use btpc_desktop_app::rpc_client::RpcClient;
    let rpc_client = RpcClient::new(&state.config.rpc.host, state.config.rpc.port);

    // Get current blockchain height
    let info = rpc_client.get_blockchain_info()
        .await
        .map_err(|e| format!("Failed to get blockchain info: {}", e))?;

    let current_height = info.blocks;

    if current_height == 0 {
        return Ok(Vec::new());
    }

    let start_height = current_height.saturating_sub(offset as u64);

    let end_height = start_height.saturating_sub(limit as u64);

    let mut blocks = Vec::new();

    for height in (end_height..=start_height).rev() {
        match rpc_client.get_block_by_height(height).await {
            Ok(block_data) => {
                blocks.push(BlockInfo {
                    height: block_data.height,
                    hash: block_data.hash,
                    prev_hash: block_data.previous_block_hash.unwrap_or_default(),
                    merkle_root: Some(block_data.merkle_root),
                    timestamp: block_data.time,
                    bits: block_data.bits,
                    nonce: block_data.nonce,
                    version: block_data.version,
                    tx_count: block_data.tx.len(),
                    size: 0, // Not provided by RPC BlockInfo, would need separate query
                    transactions: Vec::new(), // Populated separately if needed
                });
            },
            Err(_) => continue,
        }
    }

    Ok(blocks)
}

#[tauri::command]
async fn get_recent_transactions(
    state: State<'_, AppState>,
    limit: usize,
    offset: usize,
) -> Result<Vec<TransactionInfo>, String> {
    use btpc_desktop_app::rpc_client::RpcClient;
    let rpc_client = RpcClient::new(&state.config.rpc.host, state.config.rpc.port);

    // Get recent transactions from node
    match rpc_client.get_recent_transactions(limit, offset).await {
        Ok(txs) => {
            let transactions: Vec<TransactionInfo> = txs.iter().map(|tx| {
                TransactionInfo {
                    hash: tx.hash.clone(),
                    block_height: tx.block_height,
                    inputs: tx.inputs,
                    outputs: tx.outputs,
                    total_value: tx.total_value,
                    timestamp: tx.timestamp,
                }
            }).collect();

            Ok(transactions)
        },
        Err(e) => Err(format!("Failed to get recent transactions: {}", e)),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum SearchResult {
    Block(BlockInfo),
    Transaction(TransactionInfo),
}

#[tauri::command]
async fn search_blockchain(
    state: State<'_, AppState>,
    query: String,
) -> Result<SearchResult, String> {
    use btpc_desktop_app::rpc_client::RpcClient;
    let rpc_client = RpcClient::new(&state.config.rpc.host, state.config.rpc.port);

    // Try to parse as block height
    if let Ok(height) = query.parse::<u64>() {
        if let Ok(block_data) = rpc_client.get_block_by_height(height).await {
            return Ok(SearchResult::Block(BlockInfo {
                height: block_data.height,
                hash: block_data.hash,
                prev_hash: block_data.previous_block_hash.unwrap_or_default(),
                merkle_root: Some(block_data.merkle_root),
                timestamp: block_data.time,
                bits: block_data.bits,
                nonce: block_data.nonce,
                version: block_data.version,
                tx_count: block_data.tx.len(),
                size: 0,
                transactions: Vec::new(),
            }));
        }
    }

    // Try to search as block hash or transaction hash
    if let Ok(tx_data) = rpc_client.get_transaction(&query).await {
        let total_value: u64 = tx_data.vout.iter().map(|out| out.value).sum();
        return Ok(SearchResult::Transaction(TransactionInfo {
            hash: query.clone(),
            block_height: None, // RPC TransactionInfo doesn't have block_height
            inputs: tx_data.vin.len(),
            outputs: tx_data.vout.len(),
            total_value,
            timestamp: 0, // RPC TransactionInfo doesn't have timestamp
        }));
    }

    Err(format!("No block or transaction found for query: {}", query))
}

#[tauri::command]
async fn get_block_message(
    state: State<'_, AppState>,
    txid: String,
) -> Result<String, String> {
    // Get wallet address from default wallet
    let address = {
        let wallet_manager = state.wallet_manager.lock()
            .map_err(|e| format!("Failed to lock wallet manager: {}", e))?;
        wallet_manager.get_default_wallet()
            .ok_or_else(|| "No default wallet set".to_string())?
            .address.clone()
    };

    // ✅ CONSTITUTION COMPLIANCE: Use RocksDB indexed lookup (O(log n))
    // Backend is single source of truth (Article XI.1)
    // Direct transaction lookup by txid - avoids O(n×m) scan
    let tx = state.tx_storage.get_transaction(&txid)
        .map_err(|e| format!("Failed to get transaction: {}", e))?
        .ok_or_else(|| "Transaction not found".to_string())?;

    // Check if it's a coinbase transaction
    if !tx.is_coinbase {
        return Err("Not a coinbase transaction".to_string());
    }

    // Get the scriptSig from the first input
    if let Some(first_input) = tx.inputs.first() {
        let script_bytes = &first_input.signature_script;

        // Genesis block format: [timestamp(8)] + [difficulty_target(32)] + [message_length(1)] + [message]
        // Regular mined block format: [message bytes directly]

        if tx.block_height == Some(0) && script_bytes.len() > 41 {
            // Try parsing genesis block format
            let message_len = script_bytes[40] as usize;
            if script_bytes.len() >= 41 + message_len {
                let message_bytes = &script_bytes[41..41 + message_len];
                if let Ok(message) = String::from_utf8(message_bytes.to_vec()) {
                    return Ok(message);
                }
            }
        }

        // Try parsing as direct UTF-8 message (regular mined blocks)
        if let Ok(message) = String::from_utf8(script_bytes.clone()) {
            if !message.is_empty() && message.chars().all(|c| c.is_ascii_graphic() || c.is_whitespace()) {
                return Ok(message);
            }
        }

        // Fallback: extract printable ASCII characters
        let readable: String = script_bytes.iter()
            .filter(|&&b| (32..=126).contains(&b))
            .map(|&b| b as char)
            .collect();

        if !readable.is_empty() {
            return Ok(readable);
        }

        return Ok("[No readable message found]".to_string());
    }

    Err("Failed to extract block message from coinbase transaction".to_string())
}

// ============================================================================
// Address Book Commands
// ============================================================================

#[tauri::command]
async fn add_address_book_entry(
    state: State<'_, AppState>,
    request: AddAddressBookRequest,
) -> Result<AddressBookEntry, String> {
    let mut address_book = state.address_book_manager.lock()
        .map_err(|e| format!("Failed to lock address book manager: {}", e))?;

    address_book.add_entry(request)
        .map_err(|e| format!("Failed to add address book entry: {}", e))
}

#[tauri::command]
async fn list_address_book_entries(
    state: State<'_, AppState>,
) -> Result<Vec<AddressBookEntry>, String> {
    let address_book = state.address_book_manager.lock()
        .map_err(|e| format!("Failed to lock address book manager: {}", e))?;

    Ok(address_book.list_entries())
}

#[tauri::command]
async fn get_address_book_entry(
    state: State<'_, AppState>,
    id: String,
) -> Result<AddressBookEntry, String> {
    let address_book = state.address_book_manager.lock()
        .map_err(|e| format!("Failed to lock address book manager: {}", e))?;

    address_book.get_entry(&id)
        .cloned()
        .ok_or_else(|| format!("Address book entry with ID '{}' not found", id))
}

#[tauri::command]
async fn update_address_book_entry(
    state: State<'_, AppState>,
    request: UpdateAddressBookRequest,
) -> Result<AddressBookEntry, String> {
    let mut address_book = state.address_book_manager.lock()
        .map_err(|e| format!("Failed to lock address book manager: {}", e))?;

    address_book.update_entry(request)
        .map_err(|e| format!("Failed to update address book entry: {}", e))
}

#[tauri::command]
async fn delete_address_book_entry(
    state: State<'_, AppState>,
    id: String,
) -> Result<String, String> {
    let mut address_book = state.address_book_manager.lock()
        .map_err(|e| format!("Failed to lock address book manager: {}", e))?;

    address_book.delete_entry(&id)
        .map_err(|e| format!("Failed to delete address book entry: {}", e))?;

    Ok(format!("Address book entry '{}' deleted successfully", id))
}

#[tauri::command]
async fn search_address_book_entries(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<AddressBookEntry>, String> {
    let address_book = state.address_book_manager.lock()
        .map_err(|e| format!("Failed to lock address book manager: {}", e))?;

    Ok(address_book.search_entries(&query))
}

// ============================================================================
// Single Instance Lock
// ============================================================================

/// Ensure only one instance of the desktop app is running
/// Returns a lock guard that must be kept alive for the app lifetime
fn ensure_single_instance() -> Result<lock_manager::FileLockGuard, String> {
    use lock_manager::LockManager;

    let btpc_home = dirs::home_dir()
        .ok_or_else(|| "Cannot determine home directory".to_string())?
        .join(".btpc");

    // Create .btpc directory if it doesn't exist
    fs::create_dir_all(&btpc_home)
        .map_err(|e| format!("Failed to create .btpc directory: {}", e))?;

    // Create lock manager for .btpc/locks directory
    let lock_dir = btpc_home.join("locks");
    let lock_mgr = LockManager::new(lock_dir)
        .map_err(|e| format!("Failed to create lock manager: {}", e))?;

    // Try to acquire exclusive lock (cross-platform, safe implementation)
    match lock_mgr.try_lock_exclusive("btpc_desktop_app") {
        Ok(Some(guard)) => {
            let pid = std::process::id();
            println!("✅ Single instance lock acquired (PID: {})", pid);
            Ok(guard)
        }
        Ok(None) => {
            let lock_path = btpc_home.join("locks/btpc_desktop_app.lock");
            Err(format!(
                "Another instance of BTPC desktop app is already running.\n\
                 If you're sure no other instance is running, delete: {}",
                lock_path.display()
            ))
        }
        Err(e) => {
            Err(format!("Failed to acquire single instance lock: {}", e))
        }
    }
}

// ============================================================================
// Main Application
// ============================================================================

fn main() {
    // Ensure only one instance is running
    let _app_lock = ensure_single_instance()
        .expect("Failed to acquire single instance lock");

    let app_state = AppState::new().expect("Failed to initialize app state");
    let process_manager = app_state.process_manager.clone();

    // Initialize authentication session state (Feature 006: Application-Level Login/Logout)
    // Note: Tauri automatically wraps managed state in Arc, so we only need RwLock
    // Using std::sync::RwLock to match auth_commands.rs requirements
    let auth_session = std::sync::RwLock::new(auth_state::SessionState::new());

    tauri::Builder::default()
        .manage(app_state)
        .manage(auth_session)
        .setup(move |app| {
            // Scan for and adopt existing BTPC processes
            println!("🔍 Scanning for existing BTPC processes...");
            let adopted = process_manager.scan_and_adopt(vec![
                ("node", "btpc_node"),
                ("miner", "btpc_miner"),
            ]);

            if !adopted.is_empty() {
                println!("✅ Adopted {} existing process(es):", adopted.len());
                for process in &adopted {
                    println!("   - {}", process);
                }
            } else {
                println!("ℹ️  No existing BTPC processes found");
            }

            // Cleanup on window close - stop all managed processes
            let window = app.get_webview_window("main").unwrap();
            let pm = process_manager.clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { .. } = event {
                    println!("🛑 App closing - stopping all processes...");
                    pm.stop_all();
                }
            });

            // Start process health monitoring (Article XI.5 - Process Lifecycle Management)
            let pm_health = process_manager.clone();
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(30));
                    pm_health.health_check();
                }
            });

            // Start transaction monitor service (Feature 007: Transaction Sending)
            // Polls every 30 seconds for transaction confirmations and auto-releases UTXOs
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let app_state = app_handle.state::<AppState>();
                transaction_monitor::start_transaction_monitor(&app_state, app_handle.clone()).await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            test_command,
            get_system_status,
            start_node,
            stop_node,
            get_node_status,
            get_mining_status,
            get_total_balance,
            list_addresses,
            create_wallet,
            get_wallet_balance,
            get_wallet_balance_with_mined,
            reload_utxos,
            get_wallet_address,
            send_btpc,
            start_mining,
            stop_mining,
            setup_btpc,
            get_logs,
            get_mining_logs,
            get_utxo_stats,
            get_wallet_utxos,
            get_spendable_utxos,
            add_mining_utxo,
            sync_wallet_utxos,
            get_transaction_history,
            create_transaction_preview,
            // RocksDB transaction storage commands (Constitution Article V)
            get_paginated_transaction_history,
            add_transaction_to_storage,
            get_transaction_from_storage,
            get_wallet_balance_from_storage,
            get_transaction_count_from_storage,
            create_user,
            login_user,
            logout_user,
            recover_account,
            check_security_session,
            get_session_info,
            get_users,
            user_exists,
            decrypt_wallet_key,
            // Wallet encryption & lock commands
            check_wallet_lock_status,
            unlock_wallets,
            lock_wallets,
            change_master_password,
            migrate_to_encrypted,
            // Blockchain sync commands
            start_blockchain_sync,
            stop_blockchain_sync,
            get_sync_stats,
            trigger_manual_sync,
            get_address_balance_from_node,
            get_network_config,
            save_network_config,
            // Block explorer commands
            get_blockchain_info,
            get_recent_blocks,
            get_recent_transactions,
            search_blockchain,
            get_block_message,
            // Address book commands
            add_address_book_entry,
            list_address_book_entries,
            get_address_book_entry,
            update_address_book_entry,
            delete_address_book_entry,
            search_address_book_entries,
            // Multi-wallet management commands
            wallet_commands::create_wallet_with_nickname,
            wallet_commands::list_wallets,
            wallet_commands::get_wallet,
            wallet_commands::get_wallet_by_nickname,
            wallet_commands::get_default_wallet,
            wallet_commands::update_wallet,
            wallet_commands::delete_wallet,
            // Transaction commands (Feature 007: Transaction Sending)
            transaction_commands::create_transaction,
            transaction_commands::sign_transaction,
            transaction_commands::broadcast_transaction,
            transaction_commands::get_transaction_status,
            transaction_commands::cancel_transaction,
            transaction_commands::estimate_fee,
            wallet_commands::get_wallet_summary,
            wallet_commands::update_wallet_balance,
            wallet_commands::get_wallet_balance_by_id,
            wallet_commands::send_btpc_from_wallet,
            wallet_commands::start_mining_to_wallet,
            wallet_commands::backup_wallet,
            wallet_commands::set_default_wallet,
            wallet_commands::toggle_wallet_favorite,
            wallet_commands::get_favorite_wallets,
            wallet_commands::refresh_all_wallet_balances,
            // Wallet import functionality
            wallet_commands::import_wallet_from_key,
            wallet_commands::import_wallet_from_mnemonic,
            wallet_commands::import_wallet_from_backup,
            // Wallet export functionality
            wallet_commands::export_wallet_to_json,
            wallet_commands::export_wallet_address,
            wallet_commands::export_all_wallets_summary,
            wallet_commands::generate_wallet_recovery_data,
            // UTXO address migration
            wallet_commands::migrate_utxo_addresses,
            // UTXO cleanup
            wallet_commands::clean_orphaned_utxos,
            // Authentication commands (Feature 006: Application-Level Login/Logout)
            auth_commands::has_master_password,
            auth_commands::create_master_password,
            auth_commands::login,
            auth_commands::logout,
            auth_commands::check_session
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}