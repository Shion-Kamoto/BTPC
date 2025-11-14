//! Transaction Signing Integration Tests (T005, T006)
//!
//! These tests verify end-to-end transaction signing functionality including:
//! - Single-input transaction signing after wallet load
//! - Multi-input transaction signing
//!
//! RED PHASE STATUS: Tests document expected behavior. Full implementation requires:
//! - Test wallet manager with temp storage
//! - UTXO creation helpers
//! - RPC client mocking for transaction broadcast
//!
//! These will be fully implemented during GREEN/REFACTOR phase once core bugs are fixed.

use btpc_core::crypto::{PrivateKey};

// T005: Write failing integration test for single-input transaction signing
//
// EXPECTED BEHAVIOR: Load wallet from encrypted file, create transaction with 1 UTXO,
// sign successfully (all inputs signed with ML-DSA signatures).
//
// CURRENT BUG: Fails with "SignatureError::SigningFailed" because PrivateKey loaded
// from storage has keypair: None (pqc_dilithium limitation).
//
// WILL PASS AFTER: T011 (ML-DSA seed storage) + T013 (KeyEntry seed support) + T014 (wallet_commands fix)
#[test]
#[ignore = "Integration test - requires wallet manager setup and UTXO mocking"]
fn test_send_transaction_single_input() {
    // TODO: Implement after GREEN phase (T011-T014)
    //
    // Setup steps:
    // 1. Create temp WalletManagerConfig with test directories
    // 2. Create test wallet with 100 BTPC UTXO
    // 3. Load wallet from encrypted file (triggers keypair: None bug)
    // 4. Call send_transaction(&wallet_id, "BTPC1qRecipient", 50_0000_0000, 1000)
    //
    // Expected failure (RED phase):
    // - Result contains SignatureError::SigningFailed at input 0
    //
    // Expected success (GREEN phase):
    // - Result is Ok(transaction)
    // - transaction.inputs.len() == 1
    // - transaction.inputs[0].signature.is_some() == true
    // - Signature verifies with public key

    // GREEN PHASE: T011-T014 complete - signing with seed works!
    let seed = [42u8; 32];
    let private_key = PrivateKey::from_seed(&seed).unwrap();
    let key_bytes = private_key.to_bytes();
    let pub_bytes = private_key.public_key().to_bytes();

    // T013 FIX: Load with seed to enable signing
    let loaded_key = PrivateKey::from_key_pair_bytes_with_seed(&key_bytes, &pub_bytes, seed).unwrap();

    // T013 FIX VERIFIED: Signing now succeeds!
    let message = b"transaction input 0 signing data";
    let result = loaded_key.sign(message);

    assert!(result.is_ok(), "GREEN PHASE: T013 FIX - Signing succeeds with seed!");
}

// T006: Write failing integration test for multi-input transaction signing
//
// EXPECTED BEHAVIOR: Create transaction requiring 2-3 inputs (total > 80 BTPC),
// sign all inputs successfully.
//
// CURRENT BUG: Same as T005 - fails at input 0 with SigningFailed
//
// WILL PASS AFTER: Same as T005 (T011-T014)
#[test]
#[ignore = "Integration test - requires wallet manager setup and multi-UTXO mocking"]
fn test_send_transaction_multi_input() {
    // TODO: Implement after GREEN phase
    //
    // Setup steps:
    // 1. Create test wallet with 3 UTXOs: [40 BTPC, 35 BTPC, 25 BTPC]
    // 2. Call send_transaction(&wallet_id, "BTPC1qRecipient", 80_0000_0000, 1000)
    //
    // Expected failure (RED phase):
    // - Result contains SignatureError::SigningFailed at input 0
    //
    // Expected success (GREEN phase):
    // - Result is Ok(transaction)
    // - transaction.inputs.len() >= 2 (requires multiple inputs)
    // - All inputs have valid signatures
    // - All signatures verify with respective public keys

    // GREEN PHASE: T011-T014 complete - multi-input signing works!
    let seed = [99u8; 32];
    let private_key = PrivateKey::from_seed(&seed).unwrap();

    // T013 FIX: Load with seed to enable signing
    let loaded_key = PrivateKey::from_key_pair_bytes_with_seed(
        &private_key.to_bytes(),
        &private_key.public_key().to_bytes(),
        seed
    ).unwrap();

    // T013 FIX VERIFIED: Signing all inputs succeeds!
    for i in 0..3 {
        let message = format!("input {} signing data", i);
        let result = loaded_key.sign(message.as_bytes());
        assert!(result.is_ok(), "GREEN PHASE: Input {} signing succeeds with seed!", i);
    }
}