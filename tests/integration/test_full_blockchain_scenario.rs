// Full Blockchain Scenario Integration Tests
// These tests MUST FAIL initially to ensure TDD compliance

use btpc_core::blockchain::{Blockchain, Block, Transaction, TransactionOutput};
use btpc_core::consensus::Consensus;
use btpc_core::crypto::{Hash, PrivateKey};
use btpc_core::storage::BlockchainDatabase;
use btpc_core::wallet::Wallet;

#[cfg(test)]
mod full_blockchain_tests {
    use super::*;
    use std::time::Instant;
    use tempfile::tempdir;

    #[test]
    fn test_genesis_to_mature_blockchain() {
        // Integration: Complete blockchain lifecycle from genesis to mature state
        // Scenario: Genesis → Mining → Transactions → Chain validation

        let temp_dir = tempdir().unwrap();
        let mut blockchain = Blockchain::new(temp_dir.path()).unwrap();

        // Verify genesis block exists
        let genesis = blockchain.get_block_by_height(0).unwrap();
        assert_eq!(genesis.height(), 0, "Genesis block must be at height 0");
        assert!(genesis.is_genesis(), "First block must be genesis");

        // Mine 10 blocks with proper rewards
        for height in 1..=10 {
            let block = blockchain.mine_next_block().unwrap();
            assert_eq!(block.height(), height, "Block height must increment correctly");

            // Verify coinbase reward is correct for height
            let coinbase_tx = &block.transactions[0];
            let expected_reward = btpc_core::consensus::RewardCalculator::calculate_block_reward(height);
            assert_eq!(coinbase_tx.outputs[0].value, expected_reward, "Coinbase reward must match calculation");
        }

        // Verify chain integrity
        assert_eq!(blockchain.height(), 10, "Blockchain height must be 10");
        assert!(blockchain.validate_chain().is_ok(), "Complete chain must be valid");

        // Test chain reorganization scenario
        let fork_point = blockchain.get_block_by_height(8).unwrap();
        let alternate_chain = blockchain.create_fork_from(fork_point.hash()).unwrap();

        // Mine competing chain that's longer
        for _ in 0..3 {
            alternate_chain.mine_next_block().unwrap();
        }

        // Should reorganize to longer chain
        let reorg_result = blockchain.handle_reorganization(alternate_chain);
        assert!(reorg_result.is_ok(), "Blockchain reorganization must succeed");
        assert_eq!(blockchain.height(), 11, "Chain should reorganize to longer fork");
    }

    #[test]
    fn test_multi_user_transaction_flow() {
        // Integration: Multiple users creating and validating transactions
        // Scenario: User A mines → User B receives → User B sends to User C

        let temp_dir = tempdir().unwrap();
        let mut blockchain = Blockchain::new(temp_dir.path()).unwrap();

        // Create three users with wallets
        let user_a = Wallet::create_new("user_a", temp_dir.path().join("wallet_a")).unwrap();
        let user_b = Wallet::create_new("user_b", temp_dir.path().join("wallet_b")).unwrap();
        let user_c = Wallet::create_new("user_c", temp_dir.path().join("wallet_c")).unwrap();

        // User A mines a block (gets coinbase reward)
        let mining_address = user_a.get_new_address().unwrap();
        let block1 = blockchain.mine_block_to_address(mining_address).unwrap();

        // Wait for coinbase maturity (mine 100 more blocks)
        for _ in 0..100 {
            blockchain.mine_next_block().unwrap();
        }

        // Verify User A has spendable balance
        let balance_a = user_a.get_balance(&blockchain).unwrap();
        assert!(balance_a > 0, "User A must have mining rewards");

        // User A sends 10 BTPC to User B
        let send_amount = 1000000000; // 10 BTPC in satoshis
        let user_b_address = user_b.get_new_address().unwrap();
        let tx_a_to_b = user_a.create_transaction(user_b_address, send_amount, &blockchain).unwrap();

        // Validate and include transaction in block
        assert!(blockchain.validate_transaction(&tx_a_to_b).is_ok(), "A→B transaction must be valid");
        let block_with_tx = blockchain.mine_block_with_transactions(vec![tx_a_to_b.clone()]).unwrap();

        // Verify balances updated correctly
        let new_balance_a = user_a.get_balance(&blockchain).unwrap();
        let balance_b = user_b.get_balance(&blockchain).unwrap();
        assert!(new_balance_a < balance_a, "User A balance must decrease");
        assert_eq!(balance_b, send_amount, "User B must receive exact amount");

        // User B sends 5 BTPC to User C
        let send_amount_2 = 500000000; // 5 BTPC
        let user_c_address = user_c.get_new_address().unwrap();
        let tx_b_to_c = user_b.create_transaction(user_c_address, send_amount_2, &blockchain).unwrap();

        // Include in blockchain and verify
        blockchain.mine_block_with_transactions(vec![tx_b_to_c]).unwrap();

        let final_balance_b = user_b.get_balance(&blockchain).unwrap();
        let balance_c = user_c.get_balance(&blockchain).unwrap();

        assert!(final_balance_b < balance_b, "User B balance must decrease after spending");
        assert_eq!(balance_c, send_amount_2, "User C must receive 5 BTPC");

        // Verify total supply conservation
        let total_supply = blockchain.calculate_total_supply().unwrap();
        let expected_supply = blockchain.calculate_expected_supply().unwrap();
        assert_eq!(total_supply, expected_supply, "Total supply must be conserved");
    }

    #[test]
    fn test_network_consensus_simulation() {
        // Integration: Simulate network consensus with multiple nodes
        // Scenario: 5 nodes reaching consensus on blockchain state

        let temp_dirs: Vec<_> = (0..5).map(|_| tempdir().unwrap()).collect();
        let mut nodes: Vec<Blockchain> = temp_dirs.iter()
            .map(|dir| Blockchain::new(dir.path()).unwrap())
            .collect();

        // Each node mines some blocks independently
        for (i, node) in nodes.iter_mut().enumerate() {
            for _ in 0..(i + 1) {
                node.mine_next_block().unwrap();
            }
        }

        // Simulate network synchronization
        let mut canonical_blocks = Vec::new();
        for height in 0..=5 {
            let mut candidates = Vec::new();

            // Collect all blocks at this height from all nodes
            for node in &nodes {
                if let Ok(block) = node.get_block_by_height(height) {
                    candidates.push(block);
                }
            }

            if !candidates.is_empty() {
                // Choose canonical block (first seen, or by hash in case of ties)
                let canonical = candidates.into_iter()
                    .min_by_key(|block| block.hash())
                    .unwrap();
                canonical_blocks.push(canonical);
            }
        }

        // Synchronize all nodes to canonical chain
        for node in &mut nodes {
            for block in &canonical_blocks {
                if node.get_block_by_height(block.height()).is_err() {
                    node.add_block(block.clone()).unwrap();
                }
            }
        }

        // Verify all nodes have identical state
        let reference_height = nodes[0].height();
        let reference_tip = nodes[0].get_tip_hash().unwrap();

        for (i, node) in nodes.iter().enumerate() {
            assert_eq!(node.height(), reference_height, "Node {} must have same height", i);
            assert_eq!(node.get_tip_hash().unwrap(), reference_tip, "Node {} must have same tip", i);
            assert!(node.validate_chain().is_ok(), "Node {} chain must be valid", i);
        }
    }

    #[test]
    fn test_performance_under_load() {
        // Integration: Test blockchain performance under transaction load
        // Performance: Process 1000 transactions efficiently

        let temp_dir = tempdir().unwrap();
        let mut blockchain = Blockchain::new(temp_dir.path()).unwrap();
        let wallet = Wallet::create_new("test_user", temp_dir.path().join("wallet")).unwrap();

        // Mine initial blocks to get spendable coins
        let mining_address = wallet.get_new_address().unwrap();
        for _ in 0..101 { // Genesis + 100 blocks for coinbase maturity
            blockchain.mine_block_to_address(mining_address).unwrap();
        }

        // Create 1000 transactions
        let start = Instant::now();
        let mut transactions = Vec::new();

        for i in 0..1000 {
            let amount = 100000; // 0.001 BTPC each
            let recipient = PrivateKey::generate_ml_dsa().unwrap().public_key().to_address();

            match wallet.create_transaction(recipient, amount, &blockchain) {
                Ok(tx) => transactions.push(tx),
                Err(_) => break, // Stop when wallet runs out of funds
            }
        }

        let tx_creation_time = start.elapsed();
        assert!(tx_creation_time.as_secs() < 30, "Creating 1000 transactions must take <30 seconds");
        assert!(transactions.len() >= 100, "Must create at least 100 transactions");

        // Process transactions in batches (blocks)
        let validation_start = Instant::now();
        let batch_size = 50;

        for batch in transactions.chunks(batch_size) {
            // Validate all transactions in batch
            for tx in batch {
                assert!(blockchain.validate_transaction(tx).is_ok(), "All transactions must be valid");
            }

            // Mine block with batch
            blockchain.mine_block_with_transactions(batch.to_vec()).unwrap();
        }

        let validation_time = validation_start.elapsed();
        let blocks_mined = (transactions.len() + batch_size - 1) / batch_size;

        assert!(validation_time.as_secs() < 60, "Processing transactions must be efficient");
        assert_eq!(blockchain.height(), 101 + blocks_mined as u32, "All transaction blocks must be mined");

        // Verify blockchain integrity after load
        assert!(blockchain.validate_chain().is_ok(), "Blockchain must remain valid under load");

        // Performance metrics
        let avg_tx_time = validation_time.as_millis() / transactions.len() as u128;
        assert!(avg_tx_time < 10, "Average transaction processing must be <10ms");
    }

    #[test]
    fn test_blockchain_recovery_from_corruption() {
        // Integration: Test blockchain recovery from data corruption
        // Resilience: Detect and recover from database corruption

        let temp_dir = tempdir().unwrap();
        let mut blockchain = Blockchain::new(temp_dir.path()).unwrap();

        // Create initial blockchain state
        for _ in 0..20 {
            blockchain.mine_next_block().unwrap();
        }

        let original_height = blockchain.height();
        let original_tip = blockchain.get_tip_hash().unwrap();

        // Close blockchain cleanly
        drop(blockchain);

        // Simulate corruption by modifying database files
        let db_path = temp_dir.path().join("blockchain.db");
        if db_path.exists() {
            std::fs::write(db_path.join("CURRENT"), b"corrupted_data").unwrap();
        }

        // Attempt to reopen blockchain
        let recovery_result = Blockchain::open_with_recovery(temp_dir.path());

        match recovery_result {
            Ok(recovered_blockchain) => {
                // If recovery succeeds, verify integrity
                assert!(recovered_blockchain.validate_chain().is_ok(), "Recovered chain must be valid");
                assert!(recovered_blockchain.height() <= original_height, "Recovered height must not exceed original");
            },
            Err(_) => {
                // If recovery fails, should be able to rebuild from genesis
                let rebuilt = Blockchain::rebuild_from_genesis(temp_dir.path());
                assert!(rebuilt.is_ok(), "Must be able to rebuild from genesis");

                let new_blockchain = rebuilt.unwrap();
                assert_eq!(new_blockchain.height(), 0, "Rebuilt blockchain starts from genesis");
                assert!(new_blockchain.validate_chain().is_ok(), "Rebuilt chain must be valid");
            }
        }
    }
}

// Note: These tests WILL FAIL initially because the implementation doesn't exist yet.
// This is the correct TDD approach - write tests first, then implement to make them pass.