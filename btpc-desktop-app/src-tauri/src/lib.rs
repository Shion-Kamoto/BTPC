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

// Transaction modules (Feature 007: Fix Transaction Sending)
pub mod transaction_builder;
// Note: transaction_commands is in main.rs only (needs access to AppState)

// Authentication modules (Feature 006: Application-Level Login/Logout)
pub mod auth_commands;
pub mod auth_crypto;
pub mod auth_state;

// Also re-export common items for convenience
pub use error::{BtpcError, BtpcResult, ProcessError};
pub use state_management::StateManager;
pub use process_health::{ProcessHealthMonitor, CrashInfo, HealthStatus};
pub use lock_manager::{LockManager, FileLockGuard, ensure_single_instance};
