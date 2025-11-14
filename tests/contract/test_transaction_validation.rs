// Transaction Validation Contract Tests
// These tests MUST FAIL initially to ensure TDD compliance

use btpc_core::blockchain::{Transaction, TransactionInput, TransactionOutput, OutPoint};
use btpc_core::consensus::TransactionValidator;
use btpc_core::crypto::{Hash, PrivateKey, Script};
use btpc_core::storage::UTXOSet;

#[cfg(test)]
mod transaction_validation_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_transaction_structure_validation() {
        // Contract: Validate Bitcoin-compatible transaction structure
        // Constitutional requirement: Bitcoin-compatible transaction format

        let valid_tx = Transaction {
            version: 1,
            inputs: vec![TransactionInput {
                previous_output: OutPoint {
                    txid: Hash::random(),
                    vout: 0,
                },
                script_sig: Script::ml_dsa_signature(b"valid_signature"),
                sequence: 0xffffffff,
            }],
            outputs: vec![TransactionOutput {
                value: 1000000, // 0.01 BTPC
                script_pubkey: Script::pay_to_pubkey_hash(Hash::random()),
            }],
            lock_time: 0,
        };

        let validation_result = TransactionValidator::validate_structure(&valid_tx);
        assert!(validation_result.is_ok(), "Valid transaction structure must pass validation");
    }

    #[test]
    fn test_ml_dsa_signature_validation() {
        // Contract: All transaction signatures must use ML-DSA
        // Constitutional requirement: Quantum-resistant signatures only

        let private_key = PrivateKey::generate_ml_dsa().unwrap();
        let public_key = private_key.public_key();

        let tx_data = b"BTPC transaction data to sign";
        let signature = private_key.sign(tx_data).unwrap();

        let ml_dsa_input = TransactionInput {
            previous_output: OutPoint { txid: Hash::random(), vout: 0 },
            script_sig: Script::ml_dsa_signature(signature.to_bytes()),
            sequence: 0xffffffff,
        };

        // Valid ML-DSA signature
        assert!(TransactionValidator::validate_ml_dsa_signature(&ml_dsa_input, &public_key, tx_data),
                "Valid ML-DSA signature must pass validation");

        // Invalid signature (wrong data)
        let wrong_data = b"Different transaction data";
        assert!(!TransactionValidator::validate_ml_dsa_signature(&ml_dsa_input, &public_key, wrong_data),
                "Invalid ML-DSA signature must fail validation");
    }

    #[test]
    fn test_transaction_validation_performance() {
        // Contract: Transaction validation must be fast
        // Performance requirement: <5ms per transaction

        let test_tx = Transaction::create_complex_test_transaction(); // Multiple inputs/outputs
        let utxo_set = UTXOSet::create_test_set();

        let start = Instant::now();
        let validation_result = TransactionValidator::validate(&test_tx, &utxo_set);
        let duration = start.elapsed();

        assert!(validation_result.is_ok(), "Complex transaction validation must succeed");
        assert!(duration.as_millis() < 5, "Transaction validation must be <5ms");
    }

    #[test]
    fn test_utxo_reference_validation() {
        // Contract: All inputs must reference valid UTXOs
        // Security requirement: Prevent spending non-existent coins

        let existing_utxo = OutPoint { txid: Hash::from_hex("abc123").unwrap(), vout: 0 };
        let non_existent_utxo = OutPoint { txid: Hash::from_hex("def456").unwrap(), vout: 0 };

        let mut utxo_set = UTXOSet::new();
        utxo_set.add_utxo(existing_utxo.clone(), TransactionOutput {
            value: 2000000,
            script_pubkey: Script::pay_to_pubkey_hash(Hash::random()),
        });

        let valid_tx = Transaction {
            version: 1,
            inputs: vec![TransactionInput {
                previous_output: existing_utxo,
                script_sig: Script::ml_dsa_signature(b"valid_signature"),
                sequence: 0xffffffff,
            }],
            outputs: vec![TransactionOutput {
                value: 1500000, // Less than input (leaves fee)
                script_pubkey: Script::pay_to_pubkey_hash(Hash::random()),
            }],
            lock_time: 0,
        };

        let invalid_tx = Transaction {
            version: 1,
            inputs: vec![TransactionInput {
                previous_output: non_existent_utxo,
                script_sig: Script::ml_dsa_signature(b"signature"),
                sequence: 0xffffffff,
            }],
            outputs: vec![TransactionOutput {
                value: 1000000,
                script_pubkey: Script::pay_to_pubkey_hash(Hash::random()),
            }],
            lock_time: 0,
        };

        assert!(TransactionValidator::validate_utxo_references(&valid_tx, &utxo_set),
                "Valid UTXO references must pass");
        assert!(!TransactionValidator::validate_utxo_references(&invalid_tx, &utxo_set),
                "Invalid UTXO references must fail");
    }

    #[test]
    fn test_double_spending_prevention() {
        // Contract: Prevent spending the same UTXO twice
        // Security requirement: No double-spending attacks

        let utxo_ref = OutPoint { txid: Hash::from_hex("abc123").unwrap(), vout: 0 };
        let mut utxo_set = UTXOSet::new();
        utxo_set.add_utxo(utxo_ref.clone(), TransactionOutput {
            value: 3000000,
            script_pubkey: Script::pay_to_pubkey_hash(Hash::random()),
        });

        let tx1 = Transaction {
            version: 1,
            inputs: vec![TransactionInput {
                previous_output: utxo_ref.clone(),
                script_sig: Script::ml_dsa_signature(b"signature1"),
                sequence: 0xffffffff,
            }],
            outputs: vec![TransactionOutput {
                value: 2500000,
                script_pubkey: Script::pay_to_pubkey_hash(Hash::random()),
            }],
            lock_time: 0,
        };

        let tx2 = Transaction {
            version: 1,
            inputs: vec![TransactionInput {
                previous_output: utxo_ref, // Same UTXO!
                script_sig: Script::ml_dsa_signature(b"signature2"),
                sequence: 0xffffffff,
            }],
            outputs: vec![TransactionOutput {
                value: 2000000,
                script_pubkey: Script::pay_to_pubkey_hash(Hash::random()),
            }],
            lock_time: 0,
        };

        // First transaction should succeed
        assert!(TransactionValidator::validate(&tx1, &utxo_set), "First spend must succeed");

        // Simulate spending the UTXO
        utxo_set.remove_utxo(&tx1.inputs[0].previous_output);

        // Second transaction should fail (double spend)
        assert!(!TransactionValidator::validate(&tx2, &utxo_set), "Double spend must be rejected");
    }

    #[test]
    fn test_value_balance_validation() {
        // Contract: Input values must equal output values plus fees
        // Economic requirement: Conservation of value

        let input_value = 5000000; // 0.05 BTPC
        let output_value = 4800000; // 0.048 BTPC
        let expected_fee = 200000;  // 0.002 BTPC

        let utxo_ref = OutPoint { txid: Hash::random(), vout: 0 };
        let mut utxo_set = UTXOSet::new();
        utxo_set.add_utxo(utxo_ref.clone(), TransactionOutput {
            value: input_value,
            script_pubkey: Script::pay_to_pubkey_hash(Hash::random()),
        });

        let balanced_tx = Transaction {
            version: 1,
            inputs: vec![TransactionInput {
                previous_output: utxo_ref.clone(),
                script_sig: Script::ml_dsa_signature(b"signature"),
                sequence: 0xffffffff,
            }],
            outputs: vec![TransactionOutput {
                value: output_value,
                script_pubkey: Script::pay_to_pubkey_hash(Hash::random()),
            }],
            lock_time: 0,
        };

        let unbalanced_tx = Transaction {
            version: 1,
            inputs: vec![TransactionInput {
                previous_output: utxo_ref,
                script_sig: Script::ml_dsa_signature(b"signature"),
                sequence: 0xffffffff,
            }],
            outputs: vec![TransactionOutput {
                value: input_value + 1000000, // More output than input!
                script_pubkey: Script::pay_to_pubkey_hash(Hash::random()),
            }],
            lock_time: 0,
        };

        let balance_result = TransactionValidator::validate_value_balance(&balanced_tx, &utxo_set);
        assert!(balance_result.is_ok(), "Balanced transaction must pass");
        assert_eq!(balance_result.unwrap(), expected_fee, "Fee calculation must be correct");

        assert!(TransactionValidator::validate_value_balance(&unbalanced_tx, &utxo_set).is_err(),
                "Unbalanced transaction must fail");
    }

    #[test]
    fn test_coinbase_transaction_validation() {
        // Contract: Coinbase transactions have special validation rules
        // Requirement: Proper mining reward handling

        let coinbase_tx = Transaction::coinbase(3237500000, Hash::random()); // 32.375 BTPC
        let regular_tx = Transaction::create_test_transfer(1000000, Hash::random());

        assert!(TransactionValidator::is_coinbase(&coinbase_tx), "Coinbase transaction must be identified");
        assert!(!TransactionValidator::is_coinbase(&regular_tx), "Regular transaction must not be coinbase");

        // Coinbase validation (no UTXO checking)
        let empty_utxo_set = UTXOSet::new();
        assert!(TransactionValidator::validate_coinbase(&coinbase_tx, 0), "Valid coinbase must pass");
        assert!(TransactionValidator::validate(&coinbase_tx, &empty_utxo_set), "Coinbase doesn't need UTXOs");
    }

    #[test]
    fn test_script_execution_validation() {
        // Contract: Transaction scripts must execute successfully
        // Security requirement: Proper spending authorization

        let private_key = PrivateKey::generate_ml_dsa().unwrap();
        let public_key = private_key.public_key();
        let pub_key_hash = Hash::hash(&public_key.to_bytes());

        // Create output script (Pay-to-PubkeyHash)
        let output_script = Script::pay_to_pubkey_hash(pub_key_hash);

        // Create input script with valid signature
        let tx_data = b"transaction data to sign";
        let signature = private_key.sign(tx_data).unwrap();
        let input_script = Script::unlock_p2pkh(signature.to_bytes(), public_key.to_bytes());

        assert!(TransactionValidator::execute_script(&input_script, &output_script, tx_data),
                "Valid script execution must succeed");

        // Test with invalid signature
        let wrong_signature = PrivateKey::generate_ml_dsa().unwrap().sign(tx_data).unwrap();
        let invalid_input_script = Script::unlock_p2pkh(wrong_signature.to_bytes(), public_key.to_bytes());

        assert!(!TransactionValidator::execute_script(&invalid_input_script, &output_script, tx_data),
                "Invalid script execution must fail");
    }

    #[test]
    fn test_transaction_size_limits() {
        // Contract: Transactions must not exceed size limits
        // Requirement: Network resource protection

        let normal_tx = Transaction::create_test_transfer(1000000, Hash::random());
        assert!(TransactionValidator::validate_size(&normal_tx), "Normal transaction must pass size check");

        let oversized_tx = Transaction::create_oversized_test_transaction(); // Too many inputs/outputs
        assert!(!TransactionValidator::validate_size(&oversized_tx), "Oversized transaction must be rejected");
    }

    #[test]
    fn test_lock_time_validation() {
        // Contract: Transactions with lock_time must be handled correctly
        // Requirement: Time-locked transaction support

        let current_height = 100000;
        let current_time = 1735344000;

        // Height-based lock time (valid)
        let height_locked_tx = Transaction {
            version: 1,
            inputs: vec![TransactionInput {
                previous_output: OutPoint { txid: Hash::random(), vout: 0 },
                script_sig: Script::ml_dsa_signature(b"signature"),
                sequence: 0xfffffffe, // Enables lock time
            }],
            outputs: vec![TransactionOutput {
                value: 1000000,
                script_pubkey: Script::pay_to_pubkey_hash(Hash::random()),
            }],
            lock_time: current_height - 10, // 10 blocks ago
        };

        // Time-based lock time (valid)
        let time_locked_tx = Transaction {
            version: 1,
            inputs: vec![TransactionInput {
                previous_output: OutPoint { txid: Hash::random(), vout: 0 },
                script_sig: Script::ml_dsa_signature(b"signature"),
                sequence: 0xfffffffe,
            }],
            outputs: vec![TransactionOutput {
                value: 1000000,
                script_pubkey: Script::pay_to_pubkey_hash(Hash::random()),
            }],
            lock_time: current_time - 3600, // 1 hour ago
        };

        // Future lock time (invalid)
        let future_locked_tx = Transaction {
            version: 1,
            inputs: vec![TransactionInput {
                previous_output: OutPoint { txid: Hash::random(), vout: 0 },
                script_sig: Script::ml_dsa_signature(b"signature"),
                sequence: 0xfffffffe,
            }],
            outputs: vec![TransactionOutput {
                value: 1000000,
                script_pubkey: Script::pay_to_pubkey_hash(Hash::random()),
            }],
            lock_time: current_height + 100, // Future height
        };

        assert!(TransactionValidator::validate_lock_time(&height_locked_tx, current_height, current_time),
                "Past height lock time must be valid");
        assert!(TransactionValidator::validate_lock_time(&time_locked_tx, current_height, current_time),
                "Past time lock time must be valid");
        assert!(!TransactionValidator::validate_lock_time(&future_locked_tx, current_height, current_time),
                "Future lock time must be invalid");
    }

    #[test]
    fn test_memory_pool_validation() {
        // Contract: Transactions in mempool must be valid for inclusion
        // Requirement: Prevent invalid transactions from propagating

        let utxo_set = UTXOSet::create_test_set();
        let valid_tx = Transaction::create_valid_test_transaction(&utxo_set);
        let invalid_tx = Transaction::create_invalid_test_transaction();

        assert!(TransactionValidator::validate_for_mempool(&valid_tx, &utxo_set),
                "Valid transaction must be accepted to mempool");
        assert!(!TransactionValidator::validate_for_mempool(&invalid_tx, &utxo_set),
                "Invalid transaction must be rejected from mempool");

        // Test fee requirements for mempool
        let low_fee_tx = Transaction::create_low_fee_test_transaction(&utxo_set);
        assert!(!TransactionValidator::validate_for_mempool(&low_fee_tx, &utxo_set),
                "Low fee transaction must be rejected from mempool");
    }
}

// Note: These tests WILL FAIL initially because the implementation doesn't exist yet.
// This is the correct TDD approach - write tests first, then implement to make them pass.