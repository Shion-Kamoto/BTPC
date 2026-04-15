//! Tauri command modules for the BTPC desktop application
//!
//! This module organizes all Tauri command handlers into logical groups
//! for better maintainability and code organization.

pub mod address_book;
pub mod blockchain;
pub mod database;
pub mod embedded_node;
pub mod gpu_stats;
pub mod mining_logs;
pub mod mining_status;
pub mod node;
pub mod node_commands;
pub mod system;
pub mod utxo;
pub mod wallet_basic;
pub mod wallet_encryption;
pub mod security;
pub mod tx_storage;