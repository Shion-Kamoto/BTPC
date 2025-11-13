//! BTPC Desktop Application Library
//!
//! This library exposes modules for integration testing.

// Declare modules as public
pub mod error;
pub mod events;
pub mod state_management;
pub mod process_health;
pub mod lock_manager;
pub mod utxo_manager;

// RPC client module (moved from main.rs for TD-001 refactoring)
pub mod rpc_client;

// Transaction modules (Feature 007: Fix Transaction Sending)
pub mod transaction_builder;
pub mod transaction_commands_core; // TD-001: Testable business logic (no Tauri deps)
pub mod transaction_state; // TD-001: Transaction state management (moved from transaction_commands.rs)
// Note: transaction_commands is in main.rs only (needs access to AppState)

// Authentication modules (Feature 006: Application-Level Login/Logout)
pub mod auth_commands;
pub mod auth_crypto;
pub mod auth_state;

// GPU Mining Dashboard modules (Feature 012)
pub mod gpu_health_monitor;       // GPU health monitoring and thermal management
pub mod gpu_stats_persistence;    // GPU stats persistence and historical tracking
pub mod mining_thread_pool;       // Unified CPU+GPU mining thread pool
pub mod gpu_miner;                // OpenCL GPU mining implementation

// Also re-export common items for convenience
pub use error::{BtpcError, BtpcResult, ProcessError};
pub use state_management::StateManager;
pub use process_health::{ProcessHealthMonitor, CrashInfo, HealthStatus};
pub use lock_manager::{LockManager, FileLockGuard, ensure_single_instance};
