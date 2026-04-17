//! Stratum V2-BTPC Pool Mining Protocol
//!
//! Implements a BTPC-adapted Stratum V2 protocol for pool mining.
//! Key differences from standard Stratum V2:
//! - SHA-512 targets (64-byte) instead of SHA-256 (32-byte)
//! - ML-DSA (Dilithium5) post-quantum signatures
//! - Noise_NX encrypted transport
//!
//! ## Architecture
//! - `messages`: Protocol message types and serialization
//! - `transport`: Noise-encrypted TCP transport
//! - `codec`: Binary frame codec for length-prefixed messages
//! - `pool_client`: High-level pool client with reconnection
//! - `vardiff`: Variable share difficulty controller

pub mod codec;
pub mod messages;
pub mod pool_client;
pub mod transport;
pub mod vardiff;

pub use messages::*;
pub use pool_client::StratumPoolClient;
pub use vardiff::VardiffController;
