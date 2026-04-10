//! Test-compatible command stubs for integration testing
//!
//! This module provides test stubs for commands that normally require
//! Tauri's AppState. These stubs allow integration tests to compile
//! and run without a full Tauri application context.
//!
//! Note: The actual production commands are in main.rs with AppState access.

pub mod embedded_node;
pub mod mining;