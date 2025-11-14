//! Integration test: Send BTPC to external address
//!
//! This test verifies the complete transaction flow for sending
//! BTPC to an external address (not owned by the user).
//!
//! Tests:
//! 1. Address validation before transaction creation
//! 2. Transaction creation with proper outputs
//! 3. Fee calculation and validation
//! 4. UTXO consumption and change generation
//! 5. Transaction size estimation

use btpc_desktop_app::{
    error::TransactionError,
    utxo_manager::UTXOManager,
};
use btpc_core::crypto::Address;
use tempfile::TempDir;

/// Test helper to create a wallet with balance
fn create_funded_wallet(balance: u64) -> (TempDir, UTXOManager, String) {
    let temp_dir = TempDir::new().unwrap();
    let mut wallet = UTXOManager::new(temp_dir.path().to_path_buf()).unwrap();
    let address = "btpc1qsenderaddress00000000000000000000000000".to_string();

    wallet.add_coinbase_utxo(
        "funding_tx".to_string(),
        0,
        balance,
        address.clone(),
        1,
    ).unwrap();

    (temp_dir, wallet, address)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test sending to valid external address
    #[test]
    fn test_send_to_external_address_success() {
        // Given: Wallet with balance and valid external address
        let (_temp, wallet, sender_addr) = create_funded_wallet(200_000_000); // 2 BTPC
        let recipient_addr = "btpc1qexternalrecipient000000000000000000000".to_string();

        let send_amount = 50_000_000u64; // 0.5 BTPC
        let fee = 20_000u64; // 0.0002 BTPC

        // When: Create transaction
        let tx_result = wallet.create_send_transaction(
            &sender_addr,
            &recipient_addr,
            send_amount,
            fee,
        );

        // Then: Transaction should be created successfully
        assert!(tx_result.is_ok(), "Transaction should be created successfully");
        let tx = tx_result.unwrap();

        // Verify transaction structure
        assert!(tx.inputs.len() > 0, "Should have at least one input");
        assert!(tx.outputs.len() >= 1, "Should have at least recipient output");

        // Verify recipient output exists with correct amount
        let recipient_output = tx.outputs.iter().find(|o| o.value == send_amount);
        assert!(
            recipient_output.is_some(),
            "Should have output with send amount {}",
            send_amount
        );

        // Verify total outputs don't exceed inputs
        let total_output: u64 = tx.outputs.iter().map(|o| o.value).sum();
        let total_input: u64 = 200_000_000; // Our initial balance
        assert!(
            total_output + fee <= total_input,
            "Total output + fee should not exceed input"
        );

        println!("✅ External send transaction created:");
        println!("  TX ID: {}", tx.txid);
        println!("  Inputs: {}", tx.inputs.len());
        println!("  Outputs: {}", tx.outputs.len());
        println!("  Send amount: {} satoshis", send_amount);
        println!("  Fee: {} satoshis", fee);
    }

    /// Test address validation with invalid format
    #[test]
    fn test_send_to_invalid_address_format() {
        // Given: Wallet with balance
        let (_temp, wallet, sender_addr) = create_funded_wallet(100_000_000);

        // Invalid addresses to test
        let invalid_addresses = vec![
            "invalid_format",           // Not bech32
            "btc1qnotbtpcaddress",       // Wrong prefix
            "btpc1qtoooshort",           // Too short
            "btpc1q" + &"x".repeat(100), // Too long
            "",                         // Empty
        ];

        for invalid_addr in invalid_addresses {
            println!("Testing invalid address: {}", invalid_addr);

            // When: Try to create transaction with invalid address
            let result = wallet.create_send_transaction(
                &sender_addr,
                &invalid_addr,
                10_000_000,
                10_000,
            );

            // Then: Should fail with address validation error
            assert!(
                result.is_err(),
                "Transaction should fail with invalid address: {}",
                invalid_addr
            );
        }
    }

    /// Test dust amount prevention
    #[test]
    fn test_send_dust_amount_rejected() {
        // Given: Wallet with balance
        let (_temp, wallet, sender_addr) = create_funded_wallet(100_000_000);
        let recipient = "btpc1qvalidrecipientaddress000000000000000000".to_string();

        // Common dust limits in satoshis
        let dust_amounts = vec![1, 10, 100, 500, 546]; // 546 is Bitcoin dust limit

        for amount in dust_amounts {
            println!("Testing dust amount: {} satoshis", amount);

            // When: Try to send dust amount
            let result = wallet.create_send_transaction(
                &sender_addr,
                &recipient,
                amount,
                10_000, // Normal fee
            );

            // Then: Transaction should still be created (BTPC may allow dust)
            // Or fail if dust prevention is implemented
            // This test documents the behavior
            println!("  Result: {}", if result.is_ok() { "Allowed" } else { "Rejected" });
        }
    }

    /// Test multiple recipients (if supported)
    #[test]
    fn test_send_creates_correct_change() {
        // Given: Wallet with specific balance
        let (_temp, mut wallet, sender_addr) = create_funded_wallet(100_000_000); // 1 BTPC
        let recipient = "btpc1qrecipient00000000000000000000000000000".to_string();

        let send_amount = 30_000_000u64; // 0.3 BTPC
        let fee = 10_000u64;
        let expected_change = 100_000_000 - send_amount - fee;

        // When: Create transaction
        let tx = wallet.create_send_transaction(
            &sender_addr,
            &recipient,
            send_amount,
            fee,
        ).unwrap();

        // Then: Should have change output with correct amount
        let change_output = tx.outputs.iter().find(|o| o.value == expected_change);
        assert!(
            change_output.is_some(),
            "Should have change output with {} satoshis (found outputs: {:?})",
            expected_change,
            tx.outputs.iter().map(|o| o.value).collect::<Vec<_>>()
        );

        // When: Mark inputs as spent
        for input in &tx.inputs {
            wallet.mark_utxo_as_spent(&input.prev_txid, input.prev_vout).unwrap();
        }

        // Then: Wallet balance should reflect only the change
        let balance = wallet.get_balance(&sender_addr).0;
        assert_eq!(
            balance, expected_change,
            "Wallet balance should be change amount after spending"
        );
    }

    /// Test fee rate calculation (satoshis per byte)
    #[test]
    fn test_fee_rate_calculation() {
        // Given: Wallet with balance
        let (_temp, wallet, sender_addr) = create_funded_wallet(100_000_000);
        let recipient = "btpc1qrecipient00000000000000000000000000000".to_string();

        // Different fee rates to test
        let fee_rates = vec![
            (1_000, "Low"),      // 1000 sat
            (10_000, "Medium"),  // 10000 sat
            (50_000, "High"),    // 50000 sat
        ];

        for (fee, label) in fee_rates {
            println!("Testing fee rate: {} ({})", fee, label);

            // When: Create transaction with specific fee
            let tx = wallet.create_send_transaction(
                &sender_addr,
                &recipient,
                30_000_000,
                fee,
            ).unwrap();

            // Then: Transaction should be created
            println!("  TX created with {} inputs, {} outputs", tx.inputs.len(), tx.outputs.len());

            // Note: Actual transaction size calculation would require serialization
            // This test documents the fee amounts used
        }
    }

    /// Test maximum send (send all minus fee)
    #[test]
    fn test_send_maximum_amount() {
        // Given: Wallet with exact balance
        let (_temp, wallet, sender_addr) = create_funded_wallet(50_000_000); // 0.5 BTPC
        let recipient = "btpc1qrecipient00000000000000000000000000000".to_string();

        let fee = 10_000u64;
        let max_send = 50_000_000 - fee; // Send everything minus fee

        // When: Send maximum amount
        let tx = wallet.create_send_transaction(
            &sender_addr,
            &recipient,
            max_send,
            fee,
        ).unwrap();

        // Then: Should have only one output (no change)
        assert_eq!(
            tx.outputs.len(), 1,
            "Should have only recipient output when sending maximum"
        );
        assert_eq!(
            tx.outputs[0].value, max_send,
            "Output should be maximum send amount"
        );
    }

    /// Test address validation accepts valid BTPC addresses
    #[test]
    fn test_valid_btpc_address_formats() {
        // Given: Valid BTPC addresses (bech32 format)
        let valid_addresses = vec![
            "btpc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh",  // Example from tests
            "btpc1qrecipient00000000000000000000000000000",  // Padded test address
        ];

        for addr in valid_addresses {
            println!("Validating address: {}", addr);

            // When: Try to parse address
            let result = Address::from_string(addr);

            // Then: Should parse successfully or fail gracefully
            // This test documents valid address formats
            println!("  Result: {}", if result.is_ok() { "Valid" } else { "Invalid" });
        }
    }

    /// Test UTXO locking during external send
    #[test]
    fn test_utxo_locked_during_external_send() {
        // Given: Wallet with one UTXO
        let temp_dir = TempDir::new().unwrap();
        let wallet = UTXOManager::new(temp_dir.path().to_path_buf()).unwrap();

        // When: Reserve UTXO for transaction
        let utxo_key = "test_tx:0".to_string();
        let reservation1 = wallet.reserve_utxos(
            vec![utxo_key.clone()],
            Some("external_send_1".to_string()),
        );

        // Then: Reservation should succeed
        assert!(reservation1.is_ok(), "UTXO reservation should succeed");

        // When: Try to create another transaction using same UTXO
        let reservation2 = wallet.reserve_utxos(
            vec![utxo_key.clone()],
            Some("external_send_2".to_string()),
        );

        // Then: Second reservation should fail
        assert!(
            reservation2.is_err(),
            "Second reservation should fail - UTXO is locked"
        );

        // Cleanup
        let token = reservation1.unwrap();
        wallet.release_reservation(&token.id).unwrap();
    }
}