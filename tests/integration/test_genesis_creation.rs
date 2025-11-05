// Genesis Block Creation Integration Tests
// These tests MUST FAIL initially to ensure TDD compliance

use btpc_core::blockchain::{Block, BlockHeader, Transaction};
use btpc_core::consensus::{GenesisBuilder, NetworkParams};
use btpc_core::crypto::Hash;
use btpc_core::Network;

#[cfg(test)]
mod genesis_creation_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_mainnet_genesis_creation() {
        // Integration: Create valid mainnet genesis block
        // Constitutional requirement: Deterministic genesis across all implementations

        let start = Instant::now();
        let genesis = GenesisBuilder::create_for_network(Network::Mainnet);
        let duration = start.elapsed();

        assert!(genesis.is_ok(), "Mainnet genesis creation must succeed");
        assert!(duration.as_millis() < 100, "Genesis creation must be fast (<100ms)");

        let genesis_block = genesis.unwrap();

        // Verify genesis block structure
        assert_eq!(genesis_block.header.version, 1, "Genesis version must be 1");
        assert_eq!(genesis_block.header.prev_hash, Hash::zero(), "Genesis prev_hash must be zero");
        assert_eq!(genesis_block.transactions.len(), 1, "Genesis must have exactly one transaction");
        assert!(genesis_block.transactions[0].is_coinbase(), "Genesis transaction must be coinbase");

        // Verify coinbase reward
        let coinbase_value = genesis_block.transactions[0].outputs[0].value;
        assert_eq!(coinbase_value, 3237500000, "Genesis coinbase must be 32.375 BTPC");

        // Verify proof-of-work
        let target = btpc_core::consensus::DifficultyTarget::from_bits(genesis_block.header.bits);
        let block_hash = genesis_block.header.hash();
        assert!(target.validates_hash(&block_hash), "Genesis block must have valid proof-of-work");
    }

    #[test]
    fn test_testnet_genesis_creation() {
        // Integration: Create valid testnet genesis block
        // Requirement: Different genesis for testnet to prevent confusion

        let genesis = GenesisBuilder::create_for_network(Network::Testnet).unwrap();

        // Testnet should have different parameters
        assert_ne!(genesis.header.bits, 0x1d00ffff, "Testnet difficulty must differ from mainnet");
        assert!(genesis.header.timestamp > 0, "Testnet genesis must have valid timestamp");

        // But same economic model
        let coinbase_value = genesis.transactions[0].outputs[0].value;
        assert_eq!(coinbase_value, 3237500000, "Testnet genesis reward must match mainnet");
    }

    #[test]
    fn test_regtest_genesis_creation() {
        // Integration: Create regtest genesis for development
        // Requirement: Easy mining for development and testing

        let genesis = GenesisBuilder::create_for_network(Network::Regtest).unwrap();

        // Regtest should have very easy difficulty
        assert_eq!(genesis.header.bits, 0x207fffff, "Regtest must have easy difficulty");

        // Same reward structure for consistency
        let coinbase_value = genesis.transactions[0].outputs[0].value;
        assert_eq!(coinbase_value, 3237500000, "Regtest genesis reward must be consistent");

        // Should mine instantly
        let target = btpc_core::consensus::DifficultyTarget::from_bits(genesis.header.bits);
        let block_hash = genesis.header.hash();
        assert!(target.validates_hash(&block_hash), "Regtest genesis must have valid PoW");
    }

    #[test]
    fn test_genesis_determinism() {
        // Integration: Genesis blocks must be deterministic
        // Security requirement: Same genesis across all nodes

        let genesis1 = GenesisBuilder::create_for_network(Network::Mainnet).unwrap();
        let genesis2 = GenesisBuilder::create_for_network(Network::Mainnet).unwrap();

        // Headers must be identical
        assert_eq!(genesis1.header.version, genesis2.header.version);
        assert_eq!(genesis1.header.prev_hash, genesis2.header.prev_hash);
        assert_eq!(genesis1.header.merkle_root, genesis2.header.merkle_root);
        assert_eq!(genesis1.header.timestamp, genesis2.header.timestamp);
        assert_eq!(genesis1.header.bits, genesis2.header.bits);
        assert_eq!(genesis1.header.nonce, genesis2.header.nonce);

        // Block hashes must be identical
        assert_eq!(genesis1.header.hash(), genesis2.header.hash());

        // Transactions must be identical
        assert_eq!(genesis1.transactions.len(), genesis2.transactions.len());
        assert_eq!(genesis1.transactions[0].hash(), genesis2.transactions[0].hash());
    }

    #[test]
    fn test_genesis_merkle_root_calculation() {
        // Integration: Verify merkle root calculation for genesis
        // Security requirement: Prevent genesis transaction tampering

        let genesis = GenesisBuilder::create_for_network(Network::Mainnet).unwrap();

        // Calculate merkle root from transactions
        let calculated_merkle = btpc_core::blockchain::calculate_merkle_root(&genesis.transactions);
        assert_eq!(genesis.header.merkle_root, calculated_merkle,
                   "Genesis merkle root must match calculated value");

        // Verify single transaction merkle root (special case)
        assert_eq!(genesis.transactions.len(), 1, "Genesis must have exactly one transaction");
        let tx_hash = genesis.transactions[0].hash();
        let double_hash = btpc_core::crypto::Hash::double_sha512(&tx_hash);
        assert_eq!(calculated_merkle, double_hash, "Single transaction merkle root must be double hash");
    }

    #[test]
    fn test_genesis_coinbase_structure() {
        // Integration: Verify genesis coinbase transaction structure
        // Requirement: Proper coinbase for block validation

        let genesis = GenesisBuilder::create_for_network(Network::Mainnet).unwrap();
        let coinbase = &genesis.transactions[0];

        // Coinbase input validation
        assert_eq!(coinbase.inputs.len(), 1, "Coinbase must have exactly one input");
        let coinbase_input = &coinbase.inputs[0];
        assert_eq!(coinbase_input.previous_output.txid, Hash::zero(),
                   "Coinbase input txid must be zero");
        assert_eq!(coinbase_input.previous_output.vout, 0xffffffff,
                   "Coinbase input vout must be 0xffffffff");

        // Coinbase output validation
        assert_eq!(coinbase.outputs.len(), 1, "Genesis coinbase must have exactly one output");
        let coinbase_output = &coinbase.outputs[0];
        assert_eq!(coinbase_output.value, 3237500000, "Genesis coinbase value must be 32.375 BTPC");

        // Verify script_sig contains genesis message
        let script_sig = &coinbase_input.script_sig;
        assert!(script_sig.contains_genesis_message(), "Genesis coinbase must contain genesis message");
    }

    #[test]
    fn test_genesis_block_size() {
        // Integration: Verify genesis block meets size requirements
        // Requirement: Genesis block must be under size limits

        let genesis = GenesisBuilder::create_for_network(Network::Mainnet).unwrap();
        let serialized = genesis.serialize().unwrap();

        assert!(serialized.len() < 1_000_000, "Genesis block must be under 1MB");
        assert!(serialized.len() > 200, "Genesis block must be reasonably sized");
    }

    #[test]
    fn test_genesis_timestamp_validation() {
        // Integration: Verify genesis timestamp is reasonable
        // Requirement: Genesis timestamp must be after BTPC project start

        let genesis = GenesisBuilder::create_for_network(Network::Mainnet).unwrap();
        let btpc_project_start = 1735344000; // 2024-12-28 00:00:00 UTC (BTPC project start)

        assert!(genesis.header.timestamp >= btpc_project_start,
                "Genesis timestamp must be after BTPC project start");

        // Should not be too far in the future
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert!(genesis.header.timestamp <= current_time + 3600,
                "Genesis timestamp must not be more than 1 hour in future");
    }

    #[test]
    fn test_genesis_network_magic() {
        // Integration: Verify genesis includes network-specific data
        // Requirement: Prevent cross-network genesis confusion

        let mainnet_genesis = GenesisBuilder::create_for_network(Network::Mainnet).unwrap();
        let testnet_genesis = GenesisBuilder::create_for_network(Network::Testnet).unwrap();
        let regtest_genesis = GenesisBuilder::create_for_network(Network::Regtest).unwrap();

        // Different networks should produce different genesis blocks
        assert_ne!(mainnet_genesis.header.hash(), testnet_genesis.header.hash(),
                   "Mainnet and testnet genesis must differ");
        assert_ne!(mainnet_genesis.header.hash(), regtest_genesis.header.hash(),
                   "Mainnet and regtest genesis must differ");
        assert_ne!(testnet_genesis.header.hash(), regtest_genesis.header.hash(),
                   "Testnet and regtest genesis must differ");
    }

    #[test]
    fn test_genesis_difficulty_targets() {
        // Integration: Verify network-specific difficulty targets
        // Constitutional requirement: Bitcoin-compatible difficulty encoding

        let mainnet_genesis = GenesisBuilder::create_for_network(Network::Mainnet).unwrap();
        let testnet_genesis = GenesisBuilder::create_for_network(Network::Testnet).unwrap();
        let regtest_genesis = GenesisBuilder::create_for_network(Network::Regtest).unwrap();

        // Verify difficulty progression: regtest < testnet < mainnet
        let mainnet_target = btpc_core::consensus::DifficultyTarget::from_bits(mainnet_genesis.header.bits);
        let testnet_target = btpc_core::consensus::DifficultyTarget::from_bits(testnet_genesis.header.bits);
        let regtest_target = btpc_core::consensus::DifficultyTarget::from_bits(regtest_genesis.header.bits);

        assert!(regtest_target.is_easier_than(&testnet_target), "Regtest must be easier than testnet");
        assert!(testnet_target.is_easier_than(&mainnet_target), "Testnet must be easier than mainnet");

        // All targets must be valid
        assert!(mainnet_target.is_valid(), "Mainnet target must be valid");
        assert!(testnet_target.is_valid(), "Testnet target must be valid");
        assert!(regtest_target.is_valid(), "Regtest target must be valid");
    }

    #[test]
    fn test_genesis_serialization_roundtrip() {
        // Integration: Verify genesis can be serialized and deserialized
        // Requirement: Network transmission and storage compatibility

        let original_genesis = GenesisBuilder::create_for_network(Network::Mainnet).unwrap();

        // Serialize genesis block
        let serialized = original_genesis.serialize().unwrap();
        assert!(serialized.len() > 0, "Genesis serialization must produce data");

        // Deserialize genesis block
        let deserialized_genesis = Block::deserialize(&serialized).unwrap();

        // Verify roundtrip preservation
        assert_eq!(original_genesis.header.hash(), deserialized_genesis.header.hash(),
                   "Genesis hash must be preserved through serialization");
        assert_eq!(original_genesis.transactions.len(), deserialized_genesis.transactions.len(),
                   "Genesis transaction count must be preserved");
        assert_eq!(original_genesis.transactions[0].hash(), deserialized_genesis.transactions[0].hash(),
                   "Genesis coinbase hash must be preserved");
    }

    #[test]
    fn test_genesis_validation_integration() {
        // Integration: Verify genesis blocks pass full validation
        // Requirement: Genesis must pass all block validation rules

        let networks = vec![Network::Mainnet, Network::Testnet, Network::Regtest];

        for network in networks {
            let genesis = GenesisBuilder::create_for_network(network).unwrap();

            // Full block validation
            let validation_result = btpc_core::consensus::BlockValidator::validate(&genesis);
            assert!(validation_result.is_ok(),
                    "Genesis block for {:?} must pass full validation", network);

            // Specific genesis validation
            let genesis_validation = btpc_core::consensus::BlockValidator::validate_genesis(&genesis, network);
            assert!(genesis_validation.is_ok(),
                    "Genesis block for {:?} must pass genesis-specific validation", network);
        }
    }

    #[test]
    fn test_genesis_chainstate_initialization() {
        // Integration: Verify genesis properly initializes blockchain state
        // Requirement: Genesis must create valid initial UTXO set

        let genesis = GenesisBuilder::create_for_network(Network::Mainnet).unwrap();

        // Initialize blockchain state with genesis
        let mut utxo_set = btpc_core::storage::UTXOSet::new();
        let initialization_result = utxo_set.apply_genesis_block(&genesis);
        assert!(initialization_result.is_ok(), "Genesis UTXO initialization must succeed");

        // Verify genesis UTXO exists
        let genesis_outpoint = btpc_core::blockchain::OutPoint {
            txid: genesis.transactions[0].hash(),
            vout: 0,
        };
        let genesis_utxo = utxo_set.get_utxo(&genesis_outpoint);
        assert!(genesis_utxo.is_some(), "Genesis UTXO must exist in set");

        let utxo = genesis_utxo.unwrap();
        assert_eq!(utxo.output.value, 3237500000, "Genesis UTXO value must be 32.375 BTPC");
        assert_eq!(utxo.height, 0, "Genesis UTXO height must be 0");
        assert!(utxo.is_coinbase, "Genesis UTXO must be marked as coinbase");
    }
}

// Note: These tests WILL FAIL initially because the implementation doesn't exist yet.
// This is the correct TDD approach - write tests first, then implement to make them pass.