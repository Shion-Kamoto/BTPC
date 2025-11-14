// Wallet Operations Integration Tests
// These tests MUST FAIL initially to ensure TDD compliance

use btpc_core::blockchain::Blockchain;
use btpc_core::crypto::{Hash, PrivateKey, Signature};
use btpc_core::wallet::{Wallet, WalletConfig, Address, WalletError};
use btpc_core::storage::WalletDatabase;

#[cfg(test)]
mod wallet_operations_tests {
    use super::*;
    use std::collections::HashMap;
    use std::time::Instant;
    use tempfile::tempdir;

    #[test]
    fn test_wallet_creation_and_recovery() {
        // Integration: Wallet creation, backup, and recovery scenarios
        // Security: Mnemonic backup and deterministic key generation

        let temp_dir = tempdir().unwrap();
        let wallet_path = temp_dir.path().join("test_wallet");

        // Create new wallet with mnemonic
        let original_wallet = Wallet::create_new("test_user", &wallet_path).unwrap();
        let mnemonic = original_wallet.export_mnemonic().unwrap();
        let master_key = original_wallet.get_master_key().unwrap();

        // Generate some addresses
        let addresses: Vec<_> = (0..10).map(|_| original_wallet.get_new_address().unwrap()).collect();

        // Close wallet and delete files
        drop(original_wallet);
        std::fs::remove_dir_all(&wallet_path).unwrap();

        // Recover wallet from mnemonic
        let recovered_wallet = Wallet::restore_from_mnemonic(&mnemonic, "test_user", &wallet_path).unwrap();
        let recovered_master_key = recovered_wallet.get_master_key().unwrap();

        // Verify master key matches
        assert_eq!(master_key.to_bytes(), recovered_master_key.to_bytes(), "Master key must match after recovery");

        // Verify deterministic address generation
        for (i, original_addr) in addresses.iter().enumerate() {
            let recovered_addr = recovered_wallet.derive_address_at_index(i as u32).unwrap();
            assert_eq!(*original_addr, recovered_addr, "Address {} must match after recovery", i);
        }

        // Test invalid mnemonic recovery
        let invalid_mnemonic = "invalid mnemonic words that should not work for recovery";
        let invalid_recovery = Wallet::restore_from_mnemonic(invalid_mnemonic, "test_user", &wallet_path);
        assert!(invalid_recovery.is_err(), "Invalid mnemonic must fail recovery");
    }

    #[test]
    fn test_multi_wallet_transaction_coordination() {
        // Integration: Multiple wallets coordinating complex transactions
        // Scenario: 2-of-3 multisig wallet operations

        let temp_dir = tempdir().unwrap();
        let blockchain = Blockchain::new(temp_dir.path().join("blockchain")).unwrap();

        // Create 3 wallets for multisig
        let wallet1 = Wallet::create_new("user1", temp_dir.path().join("wallet1")).unwrap();
        let wallet2 = Wallet::create_new("user2", temp_dir.path().join("wallet2")).unwrap();
        let wallet3 = Wallet::create_new("user3", temp_dir.path().join("wallet3")).unwrap();

        // Get public keys for multisig
        let pubkey1 = wallet1.get_master_public_key().unwrap();
        let pubkey2 = wallet2.get_master_public_key().unwrap();
        let pubkey3 = wallet3.get_master_public_key().unwrap();

        // Create 2-of-3 multisig address
        let multisig_script = btpc_core::crypto::Script::create_multisig(2, vec![pubkey1, pubkey2, pubkey3]).unwrap();
        let multisig_address = Address::from_script(&multisig_script);

        // Fund the multisig address (simulate receiving coins)
        let funding_amount = 1000000000; // 10 BTPC
        blockchain.add_test_utxo(multisig_address.clone(), funding_amount).unwrap();

        // Create transaction spending from multisig (requires 2 signatures)
        let spend_amount = 500000000; // 5 BTPC
        let recipient_address = wallet1.get_new_address().unwrap();

        // Wallet 1 creates and signs transaction
        let mut tx = wallet1.create_multisig_transaction(
            multisig_address,
            recipient_address,
            spend_amount,
            &blockchain
        ).unwrap();

        // Wallet 1 signs
        let signature1 = wallet1.sign_transaction(&tx).unwrap();
        tx.add_signature(0, signature1, pubkey1).unwrap();

        // Wallet 2 signs (completes 2-of-3)
        let signature2 = wallet2.sign_transaction(&tx).unwrap();
        tx.add_signature(0, signature2, pubkey2).unwrap();

        // Verify transaction is now valid
        assert!(tx.verify_signatures().is_ok(), "2-of-3 multisig transaction must be valid");
        assert!(blockchain.validate_transaction(&tx).is_ok(), "Multisig transaction must pass blockchain validation");

        // Test that single signature is insufficient
        let mut single_sig_tx = wallet1.create_multisig_transaction(
            multisig_address,
            recipient_address,
            spend_amount,
            &blockchain
        ).unwrap();

        let single_signature = wallet1.sign_transaction(&single_sig_tx).unwrap();
        single_sig_tx.add_signature(0, single_signature, pubkey1).unwrap();

        assert!(single_sig_tx.verify_signatures().is_err(), "Single signature must be insufficient for 2-of-3 multisig");
    }

    #[test]
    fn test_wallet_balance_tracking_accuracy() {
        // Integration: Accurate balance tracking across complex transaction scenarios
        // Accuracy: Handle UTXOs, pending transactions, and confirmations

        let temp_dir = tempdir().unwrap();
        let mut blockchain = Blockchain::new(temp_dir.path().join("blockchain")).unwrap();
        let wallet = Wallet::create_new("balance_test", temp_dir.path().join("wallet")).unwrap();

        // Initial balance should be zero
        assert_eq!(wallet.get_balance(&blockchain).unwrap(), 0, "Initial balance must be zero");
        assert_eq!(wallet.get_confirmed_balance(&blockchain).unwrap(), 0, "Initial confirmed balance must be zero");
        assert_eq!(wallet.get_pending_balance(&blockchain).unwrap(), 0, "Initial pending balance must be zero");

        // Receive some coins (simulate mining rewards)
        let mining_address = wallet.get_new_address().unwrap();
        blockchain.mine_block_to_address(mining_address).unwrap();

        // Balance should reflect unconfirmed coinbase
        let coinbase_amount = btpc_core::consensus::RewardCalculator::calculate_block_reward(1);
        assert_eq!(wallet.get_balance(&blockchain).unwrap(), coinbase_amount, "Balance must include coinbase reward");
        assert_eq!(wallet.get_pending_balance(&blockchain).unwrap(), coinbase_amount, "Coinbase should be pending");
        assert_eq!(wallet.get_confirmed_balance(&blockchain).unwrap(), 0, "Coinbase should not be confirmed yet");

        // Mine 100 more blocks for coinbase maturity
        for _ in 0..100 {
            blockchain.mine_next_block().unwrap();
        }

        // Now coinbase should be confirmed and spendable
        assert_eq!(wallet.get_confirmed_balance(&blockchain).unwrap(), coinbase_amount, "Mature coinbase must be confirmed");
        assert_eq!(wallet.get_pending_balance(&blockchain).unwrap(), 0, "No pending balance after maturity");

        // Send some coins to another address
        let send_amount = 100000000; // 1 BTPC
        let recipient = PrivateKey::generate_ml_dsa().unwrap().public_key().to_address();
        let tx = wallet.create_transaction(recipient, send_amount, &blockchain).unwrap();

        // Before transaction is mined, balance should show pending spend
        let fee = wallet.calculate_transaction_fee(&tx).unwrap();
        let expected_balance_after_send = coinbase_amount - send_amount - fee;

        assert_eq!(wallet.get_balance(&blockchain).unwrap(), expected_balance_after_send, "Balance must reflect pending spend");
        assert_eq!(wallet.get_pending_balance(&blockchain).unwrap(), -(send_amount as i64 + fee as i64), "Negative pending for outgoing tx");

        // Mine transaction into a block
        blockchain.mine_block_with_transactions(vec![tx]).unwrap();

        // After confirmation, pending should be zero
        assert_eq!(wallet.get_confirmed_balance(&blockchain).unwrap(), expected_balance_after_send, "Confirmed balance after spend");
        assert_eq!(wallet.get_pending_balance(&blockchain).unwrap(), 0, "No pending balance after confirmation");

        // Test multiple unconfirmed transactions
        let mut pending_txs = Vec::new();
        let mut expected_pending = 0i64;

        for i in 0..5 {
            let amount = 10000000; // 0.1 BTPC each
            let recipient = PrivateKey::generate_ml_dsa().unwrap().public_key().to_address();
            let tx = wallet.create_transaction(recipient, amount, &blockchain).unwrap();

            let fee = wallet.calculate_transaction_fee(&tx).unwrap();
            expected_pending -= amount as i64 + fee as i64;
            pending_txs.push(tx);

            // Add to mempool but don't mine yet
            blockchain.add_to_mempool(pending_txs.last().unwrap().clone()).unwrap();
        }

        assert_eq!(wallet.get_pending_balance(&blockchain).unwrap(), expected_pending, "Pending balance must track multiple unconfirmed spends");

        // Mine all pending transactions
        blockchain.mine_block_with_transactions(pending_txs).unwrap();

        // All should now be confirmed
        assert_eq!(wallet.get_pending_balance(&blockchain).unwrap(), 0, "All transactions should be confirmed");

        // Verify total balance conservation
        let final_balance = wallet.get_confirmed_balance(&blockchain).unwrap();
        let total_spent = send_amount + (5 * 10000000); // Initial send + 5 * 0.1 BTPC
        let total_fees = wallet.get_total_fees_paid(&blockchain).unwrap();

        assert_eq!(final_balance + total_spent + total_fees, coinbase_amount, "Total balance must be conserved");
    }

    #[test]
    fn test_wallet_performance_with_many_addresses() {
        // Integration: Wallet performance with large number of addresses and UTXOs
        // Performance: Handle thousands of addresses efficiently

        let temp_dir = tempdir().unwrap();
        let mut blockchain = Blockchain::new(temp_dir.path().join("blockchain")).unwrap();
        let wallet = Wallet::create_new("performance_test", temp_dir.path().join("wallet")).unwrap();

        // Generate 1000 addresses
        let start_time = Instant::now();
        let mut addresses = Vec::new();

        for _ in 0..1000 {
            addresses.push(wallet.get_new_address().unwrap());
        }

        let address_generation_time = start_time.elapsed();
        assert!(address_generation_time.as_secs() < 1, "Generating 1000 addresses must take <1 second");

        // Fund each address with a small amount
        let amount_per_address = 1000000; // 0.01 BTPC

        for address in &addresses {
            blockchain.add_test_utxo(address.clone(), amount_per_address).unwrap();
        }

        // Test balance calculation performance
        let balance_start = Instant::now();
        let total_balance = wallet.get_balance(&blockchain).unwrap();
        let balance_calculation_time = balance_start.elapsed();

        assert_eq!(total_balance, 1000 * amount_per_address, "Total balance must equal sum of all UTXOs");
        assert!(balance_calculation_time.as_millis() < 100, "Balance calculation must be fast (<100ms)");

        // Test transaction creation with many inputs
        let large_send_amount = 500 * amount_per_address; // Use 500 UTXOs
        let recipient = PrivateKey::generate_ml_dsa().unwrap().public_key().to_address();

        let tx_creation_start = Instant::now();
        let large_tx = wallet.create_transaction(recipient, large_send_amount, &blockchain).unwrap();
        let tx_creation_time = tx_creation_start.elapsed();

        assert!(tx_creation_time.as_secs() < 5, "Creating transaction with many inputs must be efficient");
        assert!(large_tx.inputs.len() >= 500, "Transaction must use approximately 500 inputs");

        // Test wallet scanning performance
        let scan_start = Instant::now();
        wallet.rescan_blockchain(&blockchain).unwrap();
        let scan_time = scan_start.elapsed();

        assert!(scan_time.as_secs() < 10, "Blockchain rescan must complete in reasonable time");

        // Test address lookup performance
        let lookup_start = Instant::now();
        for address in &addresses[0..100] {
            assert!(wallet.owns_address(address).unwrap(), "Wallet must own all generated addresses");
        }
        let lookup_time = lookup_start.elapsed();

        assert!(lookup_time.as_millis() < 50, "Address lookup must be fast");

        // Test UTXO management performance
        let utxo_start = Instant::now();
        let utxos = wallet.get_spendable_utxos(&blockchain).unwrap();
        let utxo_query_time = utxo_start.elapsed();

        assert_eq!(utxos.len(), 1000, "Must find all 1000 UTXOs");
        assert!(utxo_query_time.as_millis() < 200, "UTXO query must be efficient");
    }

    #[test]
    fn test_wallet_encryption_and_security() {
        // Integration: Wallet encryption, password protection, and security features
        // Security: Encrypted storage and secure key handling

        let temp_dir = tempdir().unwrap();
        let wallet_path = temp_dir.path().join("encrypted_wallet");

        // Create encrypted wallet
        let password = "secure_password_123";
        let encrypted_wallet = Wallet::create_encrypted("secure_user", &wallet_path, password).unwrap();

        // Verify wallet is encrypted on disk
        let wallet_file = std::fs::read(wallet_path.join("wallet.dat")).unwrap();
        assert!(!wallet_file.windows(b"test").any(|window| window == b"test"), "Wallet file must not contain plaintext");

        // Test wallet unlock/lock functionality
        assert!(encrypted_wallet.is_locked(), "New encrypted wallet must be locked");

        let unlock_result = encrypted_wallet.unlock(password);
        assert!(unlock_result.is_ok(), "Correct password must unlock wallet");
        assert!(!encrypted_wallet.is_locked(), "Unlocked wallet must not be locked");

        // Test operations with unlocked wallet
        let address = encrypted_wallet.get_new_address().unwrap();
        assert!(address.is_valid(), "Must be able to generate addresses when unlocked");

        // Lock wallet again
        encrypted_wallet.lock().unwrap();
        assert!(encrypted_wallet.is_locked(), "Wallet must be locked after lock() call");

        // Test operations fail when locked
        let locked_address_result = encrypted_wallet.get_new_address();
        assert!(locked_address_result.is_err(), "Address generation must fail when wallet is locked");

        // Test wrong password
        let wrong_unlock = encrypted_wallet.unlock("wrong_password");
        assert!(wrong_unlock.is_err(), "Wrong password must fail to unlock");

        // Test password change
        encrypted_wallet.unlock(password).unwrap();
        let new_password = "new_secure_password_456";
        let change_result = encrypted_wallet.change_password(password, new_password);
        assert!(change_result.is_ok(), "Password change must succeed with correct old password");

        // Verify old password no longer works
        encrypted_wallet.lock().unwrap();
        assert!(encrypted_wallet.unlock(password).is_err(), "Old password must not work after change");
        assert!(encrypted_wallet.unlock(new_password).is_ok(), "New password must work after change");

        // Test automatic lock after timeout
        encrypted_wallet.set_auto_lock_timeout(Duration::from_millis(100)).unwrap();
        std::thread::sleep(Duration::from_millis(150));
        assert!(encrypted_wallet.is_locked(), "Wallet must auto-lock after timeout");

        // Test key derivation security
        encrypted_wallet.unlock(new_password).unwrap();
        let private_key1 = encrypted_wallet.derive_private_key_at_index(0).unwrap();
        let private_key2 = encrypted_wallet.derive_private_key_at_index(0).unwrap();

        assert_eq!(private_key1.to_bytes(), private_key2.to_bytes(), "Same index must derive same key");

        // Test secure key clearing
        encrypted_wallet.lock().unwrap();
        // Private keys should be cleared from memory when locked
        // This is tested by attempting to access them (should fail)
        let locked_key_result = encrypted_wallet.derive_private_key_at_index(0);
        assert!(locked_key_result.is_err(), "Private key access must fail when locked");
    }

    #[test]
    fn test_wallet_backup_and_synchronization() {
        // Integration: Wallet backup, export, and synchronization across devices
        // Reliability: Data consistency and recovery scenarios

        let temp_dir = tempdir().unwrap();
        let primary_wallet_path = temp_dir.path().join("primary_wallet");
        let backup_wallet_path = temp_dir.path().join("backup_wallet");
        let sync_wallet_path = temp_dir.path().join("sync_wallet");

        // Create primary wallet and generate some activity
        let primary_wallet = Wallet::create_new("primary_user", &primary_wallet_path).unwrap();
        let blockchain = Blockchain::new(temp_dir.path().join("blockchain")).unwrap();

        // Generate addresses and simulate transactions
        let mut addresses = Vec::new();
        for _ in 0..20 {
            addresses.push(primary_wallet.get_new_address().unwrap());
        }

        // Simulate receiving transactions to some addresses
        for i in 0..10 {
            blockchain.add_test_utxo(addresses[i].clone(), (i + 1) as u64 * 1000000).unwrap();
        }

        let original_balance = primary_wallet.get_balance(&blockchain).unwrap();

        // Create full backup
        let backup_data = primary_wallet.create_full_backup().unwrap();

        // Test wallet restoration from backup
        let restored_wallet = Wallet::restore_from_backup(&backup_data, "restored_user", &backup_wallet_path).unwrap();

        // Verify restored wallet matches original
        assert_eq!(restored_wallet.get_balance(&blockchain).unwrap(), original_balance, "Restored wallet balance must match");

        for (i, original_addr) in addresses.iter().enumerate() {
            let restored_addr = restored_wallet.derive_address_at_index(i as u32).unwrap();
            assert_eq!(*original_addr, restored_addr, "Restored address {} must match original", i);
        }

        // Test incremental backup/sync
        // Primary wallet generates more addresses after backup
        for _ in 0..10 {
            primary_wallet.get_new_address().unwrap();
        }

        // Create incremental backup
        let incremental_backup = primary_wallet.create_incremental_backup_since(backup_data.timestamp).unwrap();

        // Apply incremental backup to restored wallet
        restored_wallet.apply_incremental_backup(&incremental_backup).unwrap();

        // Verify both wallets are now synchronized
        let primary_addr_count = primary_wallet.get_address_count().unwrap();
        let restored_addr_count = restored_wallet.get_address_count().unwrap();
        assert_eq!(primary_addr_count, restored_addr_count, "Address counts must match after incremental sync");

        // Test cross-device synchronization scenario
        let sync_wallet = Wallet::create_new("sync_user", &sync_wallet_path).unwrap();

        // Set up bidirectional sync
        let sync_session = primary_wallet.create_sync_session(&sync_wallet).unwrap();

        // Primary wallet generates new addresses
        for _ in 0..5 {
            primary_wallet.get_new_address().unwrap();
        }

        // Sync wallet generates different addresses
        for _ in 0..3 {
            sync_wallet.get_new_address().unwrap();
        }

        // Perform synchronization
        sync_session.synchronize().unwrap();

        // Both wallets should now have consistent state
        let primary_final_count = primary_wallet.get_address_count().unwrap();
        let sync_final_count = sync_wallet.get_address_count().unwrap();

        // Note: Exact synchronization behavior depends on implementation
        // At minimum, no data should be lost
        assert!(primary_final_count >= 35, "Primary wallet must retain all addresses"); // 20 + 10 + 5
        assert!(sync_final_count >= 3, "Sync wallet must retain its addresses");

        // Test backup verification
        let verification_backup = primary_wallet.create_full_backup().unwrap();
        let verification_result = Wallet::verify_backup(&verification_backup);
        assert!(verification_result.is_ok(), "Backup verification must succeed");

        // Test corrupted backup detection
        let mut corrupted_backup = verification_backup.clone();
        corrupted_backup.data[50] ^= 0xFF; // Corrupt one byte
        let corruption_result = Wallet::verify_backup(&corrupted_backup);
        assert!(corruption_result.is_err(), "Corrupted backup must be detected");
    }
}

// Note: These tests WILL FAIL initially because the implementation doesn't exist yet.
// This is the correct TDD approach - write tests first, then implement to make them pass.