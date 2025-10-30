//! Economic constants and parameters for BTPC
//!
//! This module defines all economic parameters including reward schedules,
//! block timing, and monetary policy constants.

/// Initial block reward: 32.375 BTPC in CREDITS
pub const INITIAL_REWARD: u64 = 3_237_500_000;

/// Tail emission reward: 0.5 BTPC in CREDITS
pub const TAIL_EMISSION: u64 = 50_000_000;

/// Duration of linear decay in years
pub const DECAY_DURATION_YEARS: u32 = 24;

/// Target block time in seconds (10 minutes)
pub const TARGET_BLOCK_TIME: u64 = 600;

/// Blocks per year calculation
/// 365.25 days * 24 hours * 60 minutes / 10 minutes per block = 52,596
pub const BLOCKS_PER_YEAR: u32 = 52_596;

/// Total blocks over which decay occurs (24 years)
pub const DECAY_END_HEIGHT: u32 = DECAY_DURATION_YEARS * BLOCKS_PER_YEAR;

/// Difficulty adjustment period (every 2016 blocks, ~2 weeks)
pub const DIFFICULTY_ADJUSTMENT_PERIOD: u32 = 2016;

/// Target time for difficulty adjustment period (2 weeks in seconds)
pub const DIFFICULTY_TARGET_TIMESPAN: u64 = 14 * 24 * 60 * 60; // 14 days

/// Maximum difficulty adjustment factor (4x increase)
pub const MAX_DIFFICULTY_ADJUSTMENT_FACTOR: u32 = 4;

/// Minimum difficulty adjustment factor (1/4x decrease)
pub const MIN_DIFFICULTY_ADJUSTMENT_FACTOR: u32 = 4;

/// Coinbase maturity period (100 blocks)
pub const COINBASE_MATURITY: u32 = 100;

/// Maximum block size (1MB, Bitcoin-compatible)
pub const MAX_BLOCK_SIZE: usize = 1_000_000;

/// Maximum transaction size
pub const MAX_TRANSACTION_SIZE: usize = 100_000;

/// Maximum number of inputs in a transaction
pub const MAX_TRANSACTION_INPUTS: usize = 1000;

/// Maximum number of outputs in a transaction
pub const MAX_TRANSACTION_OUTPUTS: usize = 1000;

/// Minimum transaction fee in CREDITS per byte
pub const MIN_FEE_PER_BYTE: u64 = 1;

/// Standard transaction fee (0.0001 BTPC)
pub const STANDARD_FEE: u64 = 10_000;

/// Dust threshold (546 CREDITS, Bitcoin-compatible)
pub const DUST_THRESHOLD: u64 = 546;

/// Maximum money supply (theoretical maximum)
/// This is calculated as the sum of all rewards over infinite time
pub const MAX_MONEY: u64 = {
    // Sum of decay period (arithmetic sequence)
    let decay_supply = (DECAY_END_HEIGHT as u64) * (INITIAL_REWARD + TAIL_EMISSION) / 2;

    // Tail emission continues forever, but for practical purposes we calculate
    // the supply after a very long time (1000 years of tail emission)
    let tail_years = 1000;
    let tail_blocks = tail_years * BLOCKS_PER_YEAR as u64;
    let tail_supply = tail_blocks * TAIL_EMISSION;

    decay_supply + tail_supply
};

/// Network magic bytes for mainnet
pub const MAINNET_MAGIC: [u8; 4] = [0x42, 0x54, 0x50, 0x43]; // "BTPC"

/// Network magic bytes for testnet
pub const TESTNET_MAGIC: [u8; 4] = [0x74, 0x42, 0x54, 0x50]; // "tBTP"

/// Network magic bytes for regtest
pub const REGTEST_MAGIC: [u8; 4] = [0x72, 0x42, 0x54, 0x50]; // "rBTP"

/// Default port for mainnet P2P
pub const MAINNET_PORT: u16 = 8333;

/// Default port for testnet P2P
pub const TESTNET_PORT: u16 = 18333;

/// Default port for regtest P2P
pub const REGTEST_PORT: u16 = 18444;

/// Default RPC port for mainnet
pub const MAINNET_RPC_PORT: u16 = 8332;

/// Default RPC port for testnet
pub const TESTNET_RPC_PORT: u16 = 18332;

/// Default RPC port for regtest
pub const REGTEST_RPC_PORT: u16 = 18443;

/// Maximum number of peer connections
pub const MAX_PEER_CONNECTIONS: usize = 125;

/// Maximum number of outbound connections
pub const MAX_OUTBOUND_CONNECTIONS: usize = 8;

/// Peer connection timeout in seconds
pub const PEER_TIMEOUT: u64 = 60;

/// Maximum message size for P2P protocol
pub const MAX_MESSAGE_SIZE: usize = 32 * 1024 * 1024; // 32MB

/// Block download timeout in seconds
pub const BLOCK_DOWNLOAD_TIMEOUT: u64 = 300; // 5 minutes

/// Maximum blocks to download in one request
pub const MAX_BLOCKS_IN_FLIGHT: usize = 16;

/// Memory pool maximum size
pub const MEMPOOL_MAX_SIZE: usize = 300 * 1024 * 1024; // 300MB

/// Maximum transaction age in mempool (hours)
pub const MEMPOOL_MAX_AGE: u64 = 72; // 3 days

/// Minimum relay fee (0.00001 BTPC per kB)
pub const MIN_RELAY_FEE: u64 = 1000;

/// Default ancestor limit for mempool
pub const MEMPOOL_ANCESTOR_LIMIT: usize = 25;

/// Default descendant limit for mempool
pub const MEMPOOL_DESCENDANT_LIMIT: usize = 25;

/// RPC timeout in seconds
pub const RPC_TIMEOUT: u64 = 30;

/// Maximum RPC batch size
pub const MAX_RPC_BATCH_SIZE: usize = 100;

/// Wallet key derivation path (BIP 44)
/// m/44'/0'/0'/0/x for mainnet
/// m/44'/1'/0'/0/x for testnet
pub const WALLET_DERIVATION_PATH_MAINNET: &str = "m/44'/0'/0'/0";
pub const WALLET_DERIVATION_PATH_TESTNET: &str = "m/44'/1'/0'/0";

/// Address version bytes
pub const ADDRESS_VERSION_MAINNET: u8 = 0x00;
pub const ADDRESS_VERSION_TESTNET: u8 = 0x6f;
pub const ADDRESS_VERSION_REGTEST: u8 = 0x6f;

/// Script hash version bytes
pub const SCRIPT_VERSION_MAINNET: u8 = 0x05;
pub const SCRIPT_VERSION_TESTNET: u8 = 0xc4;
pub const SCRIPT_VERSION_REGTEST: u8 = 0xc4;

/// Bech32 human-readable parts
pub const BECH32_HRP_MAINNET: &str = "btpc";
pub const BECH32_HRP_TESTNET: &str = "tbtpc";
pub const BECH32_HRP_REGTEST: &str = "rbtpc";

/// Genesis block timestamp (January 1, 2025 00:00:00 UTC)
pub const GENESIS_TIMESTAMP: u64 = 1735689600;

/// Genesis block nonce (will be determined during mining)
pub const GENESIS_NONCE: u32 = 0;

/// Genesis block difficulty bits (minimum difficulty)
pub const GENESIS_BITS: u32 = 0x207fffff;

/// CREDITS per BTPC
pub const CREDITS_PER_BTPC: u64 = 100_000_000;

/// Economic model validation constants
/// These are used to validate the economic model consistency
/// Total reward decrease over decay period
pub const TOTAL_DECAY_AMOUNT: u64 = INITIAL_REWARD - TAIL_EMISSION;

/// Decrease per block during decay period
pub const DECREASE_PER_BLOCK: u64 = TOTAL_DECAY_AMOUNT / (DECAY_END_HEIGHT as u64);

/// Validate economic constants at compile time
const _: () = {
    // Ensure decay calculation is consistent
    assert!(DECAY_END_HEIGHT == DECAY_DURATION_YEARS * BLOCKS_PER_YEAR);

    // Ensure tail emission is less than initial reward
    assert!(TAIL_EMISSION < INITIAL_REWARD);

    // Ensure reasonable block time
    assert!(TARGET_BLOCK_TIME >= 60); // At least 1 minute
    assert!(TARGET_BLOCK_TIME <= 3600); // At most 1 hour

    // Ensure reasonable coinbase maturity
    assert!(COINBASE_MATURITY >= 6); // At least 6 blocks
    assert!(COINBASE_MATURITY <= 1000); // At most 1000 blocks

    // Ensure reasonable block size limits
    assert!(MAX_BLOCK_SIZE >= 100_000); // At least 100KB
    assert!(MAX_BLOCK_SIZE <= 10_000_000); // At most 10MB

    // Ensure fee constants are reasonable
    assert!(STANDARD_FEE > 0);
    assert!(DUST_THRESHOLD > 0);
    assert!(MIN_FEE_PER_BYTE > 0);
};

/// Get network-specific constants
pub struct NetworkConstants {
    pub magic: [u8; 4],
    pub port: u16,
    pub rpc_port: u16,
    pub address_version: u8,
    pub script_version: u8,
    pub bech32_hrp: &'static str,
    pub min_difficulty_bits: u32,
}

impl NetworkConstants {
    pub fn mainnet() -> Self {
        NetworkConstants {
            magic: MAINNET_MAGIC,
            port: MAINNET_PORT,
            rpc_port: MAINNET_RPC_PORT,
            address_version: ADDRESS_VERSION_MAINNET,
            script_version: SCRIPT_VERSION_MAINNET,
            bech32_hrp: BECH32_HRP_MAINNET,
            min_difficulty_bits: 0x1d00ffff, // Mainnet minimum difficulty
        }
    }

    pub fn testnet() -> Self {
        NetworkConstants {
            magic: TESTNET_MAGIC,
            port: TESTNET_PORT,
            rpc_port: TESTNET_RPC_PORT,
            address_version: ADDRESS_VERSION_TESTNET,
            script_version: SCRIPT_VERSION_TESTNET,
            bech32_hrp: BECH32_HRP_TESTNET,
            min_difficulty_bits: 0x207fffff, // Testnet minimum difficulty (easier)
        }
    }

    pub fn regtest() -> Self {
        NetworkConstants {
            magic: REGTEST_MAGIC,
            port: REGTEST_PORT,
            rpc_port: REGTEST_RPC_PORT,
            address_version: ADDRESS_VERSION_REGTEST,
            script_version: SCRIPT_VERSION_REGTEST,
            bech32_hrp: BECH32_HRP_REGTEST,
            min_difficulty_bits: 0x207fffff, // Regtest minimum difficulty (very easy)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_economic_constants_consistency() {
        // Test that all economic constants are internally consistent
        assert_eq!(DECAY_END_HEIGHT, DECAY_DURATION_YEARS * BLOCKS_PER_YEAR);
        assert_eq!(TOTAL_DECAY_AMOUNT, INITIAL_REWARD - TAIL_EMISSION);
        assert_eq!(
            DECREASE_PER_BLOCK,
            TOTAL_DECAY_AMOUNT / (DECAY_END_HEIGHT as u64)
        );
    }

    #[test]
    fn test_network_constants() {
        let mainnet = NetworkConstants::mainnet();
        let testnet = NetworkConstants::testnet();
        let regtest = NetworkConstants::regtest();

        // Ensure each network has unique magic bytes
        assert_ne!(mainnet.magic, testnet.magic);
        assert_ne!(mainnet.magic, regtest.magic);
        assert_ne!(testnet.magic, regtest.magic);

        // Ensure ports are different
        assert_ne!(mainnet.port, testnet.port);
        assert_ne!(mainnet.port, regtest.port);

        // Ensure RPC ports are different
        assert_ne!(mainnet.rpc_port, testnet.rpc_port);
        assert_ne!(mainnet.rpc_port, regtest.rpc_port);
    }

    #[test]
    fn test_satoshi_conversion() {
        assert_eq!(CREDITS_PER_BTPC, 100_000_000);

        // Test some known conversions
        let one_btpc = CREDITS_PER_BTPC;
        let half_btpc = CREDITS_PER_BTPC / 2;
        let quarter_btpc = CREDITS_PER_BTPC / 4;

        assert_eq!(one_btpc, 100_000_000);
        assert_eq!(half_btpc, 50_000_000);
        assert_eq!(quarter_btpc, 25_000_000);

        // Test initial reward and tail emission
        assert_eq!(INITIAL_REWARD, 32375 * 100_000); // 32.375 BTPC
        assert_eq!(TAIL_EMISSION, 5 * 10_000_000); // 0.5 BTPC
    }

    #[test]
    fn test_timing_constants() {
        // Test block timing calculations
        assert_eq!(TARGET_BLOCK_TIME, 10 * 60); // 10 minutes

        // Test blocks per year calculation
        let expected_blocks_per_year = (365.25 * 24.0 * 60.0 / 10.0) as u32;
        assert_eq!(BLOCKS_PER_YEAR, expected_blocks_per_year);

        // Test difficulty adjustment timing
        let expected_adjustment_time = DIFFICULTY_ADJUSTMENT_PERIOD as u64 * TARGET_BLOCK_TIME;
        assert_eq!(expected_adjustment_time, DIFFICULTY_TARGET_TIMESPAN);
    }

    #[test]
    fn test_size_limits() {
        // Test that size limits are reasonable
        assert!(MAX_BLOCK_SIZE >= 100_000); // At least 100KB
        assert!(MAX_TRANSACTION_SIZE <= MAX_BLOCK_SIZE);
        assert!(MAX_TRANSACTION_INPUTS > 0);
        assert!(MAX_TRANSACTION_OUTPUTS > 0);
    }

    #[test]
    fn test_fee_constants() {
        // Test fee-related constants
        assert!(MIN_FEE_PER_BYTE > 0);
        assert!(STANDARD_FEE > 0);
        assert!(DUST_THRESHOLD > 0);
        assert!(MIN_RELAY_FEE > 0);

        // Test relationships
        assert!(STANDARD_FEE > DUST_THRESHOLD);
    }

    #[test]
    fn test_mempool_constants() {
        // Test mempool-related constants
        assert!(MEMPOOL_MAX_SIZE > MAX_BLOCK_SIZE);
        assert!(MEMPOOL_MAX_AGE > 0);
        assert!(MEMPOOL_ANCESTOR_LIMIT > 0);
        assert!(MEMPOOL_DESCENDANT_LIMIT > 0);
    }

    #[test]
    fn test_network_limits() {
        // Test networking constants
        assert!(MAX_PEER_CONNECTIONS > MAX_OUTBOUND_CONNECTIONS);
        assert!(PEER_TIMEOUT > 0);
        assert!(MAX_MESSAGE_SIZE > MAX_BLOCK_SIZE);
        assert!(BLOCK_DOWNLOAD_TIMEOUT > 0);
        assert!(MAX_BLOCKS_IN_FLIGHT > 0);
    }
}
