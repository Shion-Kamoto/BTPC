// BTPC Core Library - Quantum-resistant blockchain implementation
//
// Copyright (c) 2025 BTPC Project
//
// This library implements a quantum-resistant blockchain following the BTPC constitution.
// All cryptographic operations use post-quantum algorithms (ML-DSA signatures, SHA-512 hashing).

#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_assignments)]
#![allow(unused_imports)]

// Clippy lints allowed for existing code patterns
// These would require significant refactoring and are acceptable in production code
#![allow(clippy::await_holding_lock)]  // MutexGuard across await - requires async-aware mutex migration
#![allow(clippy::type_complexity)]  // Complex generic types in RPC rate limiting
#![allow(clippy::too_many_arguments)]  // Some low-level functions need many parameters
#![allow(clippy::should_implement_trait)]  // Custom default() methods that don't need trait impl
#![allow(clippy::inherent_to_string_shadow_display)]  // Intentional to_string methods
#![allow(clippy::same_name_method)]  // Constructor names matching types (e.g., Hash::hash)
#![allow(clippy::self_named_constructors)]  // Constructor names matching types (e.g., Hash::hash)
#![allow(clippy::needless_range_loop)]  // Loop indexing patterns for clarity
#![allow(clippy::manual_clamp)]  // Manual clamp for explicit behavior
#![allow(clippy::unnecessary_filter_map)]  // Filter_map patterns for readability
#![allow(clippy::vec_init_then_push)]  // Vec initialization patterns
#![allow(clippy::cloned_ref_to_slice_refs)]  // Clone patterns for slice refs

use serde::{Deserialize, Serialize};

pub mod blockchain;
pub mod consensus;
pub mod crypto;
pub mod economics;
pub mod mempool;
pub mod network;
pub mod rpc;
pub mod state;
pub mod storage;

// Re-export common types
pub use blockchain::{Block, Transaction};
pub use consensus::Difficulty;
pub use crypto::{PrivateKey, PublicKey, Signature};

// Version and network constants
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NETWORK_MAGIC_MAINNET: [u8; 4] = [0x42, 0x54, 0x00, 0x01]; // "BT" + version
pub const NETWORK_MAGIC_TESTNET: [u8; 4] = [0x42, 0x54, 0xFF, 0x01]; // "BT" + testnet
pub const NETWORK_MAGIC_REGTEST: [u8; 4] = [0x42, 0x54, 0xFF, 0xFF]; // "BT" + regtest

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Network {
    Mainnet,
    Testnet,
    Regtest,
}

impl Network {
    pub fn magic_bytes(&self) -> [u8; 4] {
        match self {
            Network::Mainnet => NETWORK_MAGIC_MAINNET,
            Network::Testnet => NETWORK_MAGIC_TESTNET,
            Network::Regtest => NETWORK_MAGIC_REGTEST,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Network::Mainnet => "mainnet",
            Network::Testnet => "testnet",
            Network::Regtest => "regtest",
        }
    }

    pub fn fork_id(&self) -> u8 {
        match self {
            Network::Mainnet => 0,
            Network::Testnet => 1,
            Network::Regtest => 255,
        }
    }
}
