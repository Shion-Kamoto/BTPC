//! BTPC Desktop Application Library
//!
//! This library exposes modules for integration testing.

// Declare modules as public
pub mod error;
pub mod events;
pub mod lock_manager;
pub mod process_health;
pub mod state_management;
pub mod utxo_manager;

// RPC client module (moved from main.rs for TD-001 refactoring)
pub mod rpc_client;

// Transaction modules (Feature 007: Fix Transaction Sending)
pub mod transaction_builder;
pub mod transaction_commands_core; // TD-001: Testable business logic (no Tauri deps)
pub mod transaction_state; // TD-001: Transaction state management (moved from transaction_commands.rs)

// Authentication modules (Feature 006: Application-Level Login/Logout)
pub mod auth_crypto;
pub mod auth_state;

// GPU Mining Dashboard modules (Feature 012)
pub mod gpu_health_monitor; // GPU health monitoring and thermal management
pub mod gpu_miner; // OpenCL GPU mining implementation
pub mod gpu_stats_persistence; // GPU stats persistence and historical tracking
pub mod gpu_stats_types;
pub mod mining_thread_pool; // Unified CPU+GPU mining thread pool
pub mod thermal_throttle; // Thermal throttling for GPU mining
pub mod debug_logger; // Comprehensive debug event logger

// Embedded blockchain node modules
pub mod embedded_node;
pub mod unified_database; // Unified RocksDB database for blockchain and UTXO data
pub mod security; // Security manager for wallet encryption
pub mod btpc_integration; // BTPC node integration
pub mod wallet_manager; // Wallet management and balance caching (depends on security)

// Persistent settings storage
pub mod settings_storage; // RocksDB-based settings persistence

// Transaction history storage
pub mod tx_history; // SQLite-based transaction history storage (replaces RocksDB tx_storage)

// Disk space monitoring (FR-058)
pub mod disk_space_monitor; // System disk space monitoring and alerts

// Chain reorganization handling (FR-057)
pub mod reorg_handler; // Chain reorg detection, execution, and UTXO rollback

// Stratum V2-BTPC pool mining protocol
pub mod stratum;

// Note: fee_estimator is NOT exported here because it depends on EmbeddedNode
// which creates circular import issues. It's declared in main.rs instead.

// Note: commands, wallet_commands, and auth_commands are declared in main.rs
// because they depend on AppState and other binary-only modules.

// Also re-export common items for convenience
pub use embedded_node::{
    BlockchainState, EmbeddedNode, MempoolStats, SyncProgress, TransactionInfo,
};
pub use error::{BtpcError, BtpcResult, ProcessError};
pub use lock_manager::{ensure_single_instance, FileLockGuard, LockManager};
pub use mining_thread_pool::MiningThreadPool;
pub use process_health::{CrashInfo, HealthStatus, ProcessHealthMonitor};
pub use rpc_client::{RpcClient, RpcClientInterface};
pub use state_management::StateManager;
pub use unified_database::UnifiedDatabase;
