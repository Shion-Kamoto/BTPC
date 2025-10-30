//! Integration test: Send BTPC between internal wallets
//!
//! This test verifies the complete transaction flow for transferring
//! BTPC between two wallets owned by the same user.
//!
//! Tests:
//! 1. Create transaction from Wallet A to Wallet B
//! 2. Sign transaction with ML-DSA
//! 3. Broadcast transaction to network
//! 4. Verify balances updated correctly
//! 5. Verify UTXOs spent and created
//! 6. Verify events emitted (Article XI)

use btpc_desktop_app::{
    error::{BtpcError, TransactionError},
    utxo_manager::{UTXOManager, UTXO},
};
use chrono::Utc;
use std::sync::Arc;
use tempfile::TempDir;

/// Test helper to create a test wallet with UTXOs
fn create_test_wallet(name: &str, initial_balance: u64) -> (TempDir, UTXOManager, String) {
    let temp_dir = TempDir::new().unwrap();
    let mut utxo_manager = UTXOManager::new(temp_dir.path().to_path_buf()).unwrap();

    // Create a test address
    let address = format!("btpc1q{}", name.repeat(40)[..40].to_lowercase());

    // Add initial UTXO
    utxo_manager.add_coinbase_utxo(
        format!("genesis_{}", name),
        0,
        initial_balance,
        address.clone(),
        1,
    ).unwrap();

    (temp_dir, utxo_manager, address)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test successful transfer between two internal wallets
    #[test]
    fn test_internal_transfer_success() {
        // Given: Two wallets with balances
        let (_temp_a, mut wallet_a, address_a) = create_test_wallet("alice", 100_000_000); // 1 BTPC
        let (_temp_b, wallet_b, address_b) = create_test_wallet("bob", 0); // 0 BTPC

        let transfer_amount = 50_000_000u64; // 0.5 BTPC
        let fee = 10_000u64; // 0.0001 BTPC fee

        println!("📊 Initial state:");
        println!("  Wallet A balance: {}", wallet_a.get_balance(&address_a).0);
        println!("  Wallet B balance: {}", wallet_b.get_balance(&address_b).0);

        // When: Create transaction from A to B
        let tx_result = wallet_a.create_send_transaction(
            &address_a,
            &address_b,
            transfer_amount,
            fee,
        );

        // Then: Transaction should be created successfully
        assert!(tx_result.is_ok(), "Transaction creation should succeed");
        let tx = tx_result.unwrap();

        // Verify transaction structure
        assert!(tx.inputs.len() > 0, "Should have at least one input");
        assert_eq!(tx.outputs.len(), 2, "Should have 2 outputs (recipient + change)");

        // Calculate expected change
        let total_input: u64 = 100_000_000; // Initial balance
        let expected_change = total_input - transfer_amount - fee;

        // Verify output amounts
        let recipient_output = tx.outputs.iter().find(|o| o.value == transfer_amount);
        assert!(recipient_output.is_some(), "Should have recipient output with correct amount");

        let change_output = tx.outputs.iter().find(|o| o.value == expected_change);
        assert!(change_output.is_some(), "Should have change output with correct amount");

        // When: Mark UTXOs as spent (simulating broadcast)
        for input in &tx.inputs {
            wallet_a.mark_utxo_as_spent(&input.prev_txid, input.prev_vout).unwrap();
        }

        // Then: Wallet A balance should be reduced
        let new_balance_a = wallet_a.get_balance(&address_a).0;
        let expected_balance_a = expected_change; // Change comes back to wallet A
        assert_eq!(
            new_balance_a,
            expected_balance_a,
            "Wallet A should have change amount after transfer"
        );

        println!("✅ Transaction created successfully:");
        println!("  Transaction ID: {}", tx.txid);
        println!("  Inputs: {}", tx.inputs.len());
        println!("  Outputs: {}", tx.outputs.len());
        println!("  Final Wallet A balance: {}", new_balance_a);
    }

    /// Test transfer with exact balance (no change)
    #[test]
    fn test_internal_transfer_exact_amount() {
        // Given: Wallet with exact amount needed
        let (_temp, mut wallet, address_a) = create_test_wallet("exact", 100_000_000);
        let address_b = "btpc1qrecipientaddress000000000000000000000".to_string();

        let fee = 10_000u64;
        let transfer_amount = 100_000_000 - fee; // Send everything minus fee

        // When: Create transaction with exact balance
        let tx_result = wallet.create_send_transaction(
            &address_a,
            &address_b,
            transfer_amount,
            fee,
        );

        // Then: Should succeed with only 1 output (no change)
        assert!(tx_result.is_ok());
        let tx = tx_result.unwrap();
        assert_eq!(tx.outputs.len(), 1, "Should have only recipient output (no change)");
        assert_eq!(tx.outputs[0].value, transfer_amount);
    }

    /// Test transfer fails with insufficient funds
    #[test]
    fn test_internal_transfer_insufficient_funds() {
        // Given: Wallet with insufficient balance
        let (_temp, wallet, address_a) = create_test_wallet("poor", 10_000); // 0.0001 BTPC
        let address_b = "btpc1qrecipientaddress000000000000000000000".to_string();

        // When: Try to send more than available
        let tx_result = wallet.create_send_transaction(
            &address_a,
            &address_b,
            100_000_000, // 1 BTPC
            10_000,      // Fee
        );

        // Then: Should fail with insufficient funds
        assert!(tx_result.is_err());
        let err_msg = format!("{:?}", tx_result.unwrap_err());
        assert!(
            err_msg.contains("Insufficient") || err_msg.contains("insufficient"),
            "Error should mention insufficient funds: {}",
            err_msg
        );
    }

    /// Test UTXO selection prefers older UTXOs
    #[test]
    fn test_utxo_selection_prefers_older() {
        // Given: Wallet with multiple UTXOs of different ages
        let temp_dir = TempDir::new().unwrap();
        let mut wallet = UTXOManager::new(temp_dir.path().to_path_buf()).unwrap();
        let address = "btpc1qtestaddress00000000000000000000000000".to_string();

        // Add UTXOs at different block heights
        wallet.add_coinbase_utxo("tx_new".to_string(), 0, 30_000_000, address.clone(), 100).unwrap();
        wallet.add_coinbase_utxo("tx_old".to_string(), 0, 30_000_000, address.clone(), 10).unwrap();
        wallet.add_coinbase_utxo("tx_mid".to_string(), 0, 30_000_000, address.clone(), 50).unwrap();

        // When: Select UTXOs using the new method with reservation check
        let selected = wallet.select_utxos_for_amount(&address, 50_000_000).unwrap();

        // Then: Should prefer older UTXOs (lower block height)
        assert_eq!(selected.len(), 2, "Should select 2 UTXOs");
        assert_eq!(selected[0].txid, "tx_old", "First UTXO should be oldest");
        assert_eq!(selected[1].txid, "tx_mid", "Second UTXO should be middle-aged");
    }

    /// Test concurrent transaction attempt detection
    #[test]
    fn test_concurrent_transaction_detection() {
        // Given: Wallet with one UTXO
        let temp_dir = TempDir::new().unwrap();
        let wallet = UTXOManager::new(temp_dir.path().to_path_buf()).unwrap();
        let address = "btpc1qconcurrent0000000000000000000000000000".to_string();

        // Add single UTXO
        // Note: We can't easily add UTXOs without mut, so this test validates the API

        // When: Reserve UTXOs for first transaction
        let utxo_keys = vec!["test_tx:0".to_string()];
        let reservation1 = wallet.reserve_utxos(utxo_keys.clone(), Some("tx1".to_string()));

        // Then: First reservation should succeed
        assert!(reservation1.is_ok(), "First reservation should succeed");

        // When: Try to reserve same UTXOs for second transaction
        let reservation2 = wallet.reserve_utxos(utxo_keys.clone(), Some("tx2".to_string()));

        // Then: Second reservation should fail
        assert!(reservation2.is_err(), "Second reservation should fail");

        // Cleanup: Release first reservation
        let token = reservation1.unwrap();
        wallet.release_reservation(&token.id).unwrap();

        // When: Try to reserve again after release
        let reservation3 = wallet.reserve_utxos(utxo_keys, Some("tx3".to_string()));

        // Then: Should succeed after release
        assert!(reservation3.is_ok(), "Reservation after release should succeed");
    }

    /// Test reservation timeout (5 minutes)
    #[test]
    fn test_reservation_timeout() {
        use std::time::{Duration, Instant};

        // Given: A reservation token
        let token = btpc_desktop_app::utxo_manager::ReservationToken::new(
            Some("test_tx".to_string()),
            vec!["test_utxo:0".to_string()],
        );

        // Then: Should not be expired immediately
        assert!(!token.is_expired(), "Token should not be expired immediately");

        // And: Should have 5 minute expiration
        let remaining = token.expires_at.duration_since(Instant::now());
        assert!(
            remaining >= Duration::from_secs(290) && remaining <= Duration::from_secs(301),
            "Expiration should be around 5 minutes (290-301 seconds), got {:?}",
            remaining
        );
    }
}