// ! Test Infrastructure for Feature 007 Integration Tests
//!
//! Provides TestEnvironment with:
//! - Mock RPC client (no real btpc_node needed)
//! - Wallet creation with synthetic UTXOs
//! - Event tracking for verification
//! - Transaction command wrappers

pub mod mock_rpc;
pub mod test_env;
pub mod wallet_fixtures;

pub use mock_rpc::MockRpcClient;
pub use test_env::TestEnvironment;
pub use wallet_fixtures::TestWallet;