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

// Re-export lib crate modules for backward compatibility
pub use btpc_desktop_app::mining_thread_pool;
pub use btpc_desktop_app::utxo_manager;
pub use btpc_desktop_app::wallet_manager;

// Core modules
mod address_book;
mod address_utils;
pub mod error;
mod gpu_detection;
mod orphaned_utxo_cleaner;
mod process_manager;
mod settings_storage;
pub mod state_management;
mod sync_service;
mod wallet_commands;

// GPU Mining Dashboard modules (Feature 012)
mod gpu_health_monitor;
mod gpu_miner;
mod gpu_stats_commands;
mod gpu_stats_persistence;
mod mining_commands;

// Transaction modules (Feature 007)
pub mod events;
mod fee_estimator;
mod transaction_commands;
mod transaction_monitor;

// Authentication modules (Feature 006)
mod auth_commands;
mod auth_crypto;
mod auth_state;
pub mod lock_manager;
pub mod process_health;

// Command modules (Feature 013)
mod commands;

// Refactored modules
mod app_state;
mod config;
use btpc_desktop_app::types;

// Tauri application builder
mod tauri_app;

pub use app_state::AppState;

fn main() {
    // Note: input method env vars (GTK_IM_MODULE, XMODIFIERS, IBUS_DISABLE_SNOOPER)
    // are intentionally NOT set here — they can break WebKitGTK keyboard input.
    // IBUS snooper issues are handled at the package.json launch script level only
    // if they manifest on a specific system.

    tauri_app::run();
}
