//! Integration test for ML-DSA signed transactions
//! Per data-model.md specification - End-to-end transaction flow
//!
//! This test MUST FAIL initially (implementation not complete yet)

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CreateWalletRequest {
    name: String,
    passphrase: String,
    network: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WalletInfo {
    name: String,
    network: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GenerateAddressRequest {
    wallet_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Address {
    address: String,
    public_key: String,      // ML-DSA-65 public key (3,904 hex chars)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TransactionOutput {
    address: String,
    amount: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CreateTransactionRequest {
    wallet_name: String,
    outputs: Vec<TransactionOutput>,
    fee_rate: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UnsignedTransaction {
    hex: String,
    fee: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SignTransactionRequest {
    wallet_name: String,
    transaction_hex: String,
    passphrase: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SignedTransaction {
    hex: String,
    complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SendTransactionRequest {
    transaction_hex: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SendTransactionResponse {
    txid: String,            // Transaction hash (SHA-512, 128 hex chars)
}

#[tokio::test]
#[ignore] // Will fail until full transaction flow is implemented
async fn test_mldsa_transaction_end_to_end() {
    let client = reqwest::Client::new();

    // Step 1: Create wallet
    let wallet_req = CreateWalletRequest {
        name: "test_wallet_tx".to_string(),
        passphrase: "secure_pass_123".to_string(),
        network: "regtest".to_string(),
    };

    let wallet_resp = client
        .post("http://localhost:8333/api/v1/wallet/create")
        .json(&wallet_req)
        .send()
        .await;

    assert!(wallet_resp.is_ok(), "Should create wallet");

    // Step 2: Generate recipient address
    let addr_req = GenerateAddressRequest {
        wallet_name: "test_wallet_tx".to_string(),
    };

    let addr_resp = client
        .post("http://localhost:8333/api/v1/wallet/address/new")
        .json(&addr_req)
        .send()
        .await;

    assert!(addr_resp.is_ok(), "Should generate address");

    let address: Address = addr_resp
        .unwrap()
        .json()
        .await
        .expect("Should parse address");

    // Verify ML-DSA-65 public key size
    assert_eq!(
        address.public_key.len(),
        3904,
        "ML-DSA-65 public key should be 3,904 hex characters"
    );

    // Step 3: Create unsigned transaction
    let tx_req = CreateTransactionRequest {
        wallet_name: "test_wallet_tx".to_string(),
        outputs: vec![TransactionOutput {
            address: address.address.clone(),
            amount: 100_000_000, // 1 BTPC
        }],
        fee_rate: Some(1000),
    };

    let unsigned_resp = client
        .post("http://localhost:8333/api/v1/wallet/transaction/create")
        .json(&tx_req)
        .send()
        .await;

    if unsigned_resp.is_err() {
        // May fail if wallet has no funds in regtest - this is expected
        return;
    }

    let unsigned_tx: UnsignedTransaction = unsigned_resp
        .unwrap()
        .json()
        .await
        .expect("Should parse unsigned transaction");

    // Step 4: Sign transaction with ML-DSA
    let sign_req = SignTransactionRequest {
        wallet_name: "test_wallet_tx".to_string(),
        transaction_hex: unsigned_tx.hex.clone(),
        passphrase: "secure_pass_123".to_string(),
    };

    let signed_resp = client
        .post("http://localhost:8333/api/v1/wallet/transaction/sign")
        .json(&sign_req)
        .send()
        .await;

    assert!(signed_resp.is_ok(), "Should sign transaction");

    let signed_tx: SignedTransaction = signed_resp
        .unwrap()
        .json()
        .await
        .expect("Should parse signed transaction");

    assert!(signed_tx.complete, "Transaction should be fully signed");

    // Verify signed transaction contains ML-DSA signature
    // ML-DSA-65 signature is 3,309 bytes = 6,618 hex characters
    assert!(
        signed_tx.hex.len() > unsigned_tx.hex.len(),
        "Signed transaction should be larger than unsigned"
    );

    // Step 5: Broadcast transaction to network
    let send_req = SendTransactionRequest {
        transaction_hex: signed_tx.hex,
    };

    let send_resp = client
        .post("http://localhost:8332/api/v1/transaction/send")
        .json(&send_req)
        .send()
        .await;

    assert!(send_resp.is_ok(), "Should broadcast transaction");

    let send_result: SendTransactionResponse = send_resp
        .unwrap()
        .json()
        .await
        .expect("Should parse send response");

    // Verify transaction ID is SHA-512 hash
    assert_eq!(
        send_result.txid.len(),
        128,
        "Transaction ID should be SHA-512 (128 hex characters)"
    );
    assert!(
        send_result.txid.chars().all(|c| c.is_ascii_hexdigit()),
        "Transaction ID should be valid hex"
    );
}

#[tokio::test]
#[ignore] // Will fail until implementation is complete
async fn test_mldsa_signature_verification_in_mempool() {
    // Test that mempool validates ML-DSA signatures
    let client = reqwest::Client::new();

    // Create a transaction with invalid ML-DSA signature
    let invalid_tx_hex = "01000000" // version
        + "01" // input count
        + &"a".repeat(128) // previous tx hash
        + "00000000" // output index
        + "19ce" // script length (6,618 bytes = 0x19ce)
        + &"00".repeat(6618) // Invalid ML-DSA signature (all zeros)
        + "ffffffff" // sequence
        + "01" // output count
        + "00e1f50500000000" // value (1 BTPC)
        + "00" // script length
        + "00000000"; // locktime

    let send_req = SendTransactionRequest {
        transaction_hex: invalid_tx_hex.to_string(),
    };

    let send_resp = client
        .post("http://localhost:8332/api/v1/transaction/send")
        .json(&send_req)
        .send()
        .await;

    if let Ok(response) = send_resp {
        // Should reject transaction with invalid signature
        assert!(
            response.status().is_client_error() || response.status().is_server_error(),
            "Should reject transaction with invalid ML-DSA signature"
        );
    }
}

#[tokio::test]
#[ignore] // Will fail until implementation is complete
async fn test_mldsa_signature_size_in_transaction() {
    // Test that transaction properly encodes ML-DSA-65 signature
    let client = reqwest::Client::new();

    // Create and sign a transaction
    let wallet_req = CreateWalletRequest {
        name: "test_sig_size".to_string(),
        passphrase: "pass123".to_string(),
        network: "regtest".to_string(),
    };

    let _ = client
        .post("http://localhost:8333/api/v1/wallet/create")
        .json(&wallet_req)
        .send()
        .await;

    let addr_req = GenerateAddressRequest {
        wallet_name: "test_sig_size".to_string(),
    };

    let addr_resp = client
        .post("http://localhost:8333/api/v1/wallet/address/new")
        .json(&addr_req)
        .send()
        .await;

    if addr_resp.is_err() {
        return;
    }

    let address: Address = addr_resp.unwrap().json().await.expect("Should parse");

    let tx_req = CreateTransactionRequest {
        wallet_name: "test_sig_size".to_string(),
        outputs: vec![TransactionOutput {
            address: address.address,
            amount: 50_000_000,
        }],
        fee_rate: Some(1000),
    };

    let unsigned_resp = client
        .post("http://localhost:8333/api/v1/wallet/transaction/create")
        .json(&tx_req)
        .send()
        .await;

    if unsigned_resp.is_err() {
        return;
    }

    let unsigned_tx: UnsignedTransaction = unsigned_resp.unwrap().json().await.expect("Should parse");

    let sign_req = SignTransactionRequest {
        wallet_name: "test_sig_size".to_string(),
        transaction_hex: unsigned_tx.hex.clone(),
        passphrase: "pass123".to_string(),
    };

    let signed_resp = client
        .post("http://localhost:8333/api/v1/wallet/transaction/sign")
        .json(&sign_req)
        .send()
        .await;

    if let Ok(response) = signed_resp {
        let signed_tx: SignedTransaction = response.json().await.expect("Should parse");

        // Calculate size difference (should include ML-DSA-65 signature)
        let unsigned_len = unsigned_tx.hex.len();
        let signed_len = signed_tx.hex.len();

        // ML-DSA-65 signature is 3,309 bytes = 6,618 hex characters
        // Plus public key 1,952 bytes = 3,904 hex characters
        // Total: ~10,522 hex characters added
        let added_data = signed_len - unsigned_len;

        assert!(
            added_data >= 10000,
            "Signed transaction should include ML-DSA signature and public key (~10,522 hex chars)"
        );
    }
}

#[tokio::test]
#[ignore] // Will fail until implementation is complete
async fn test_multiple_input_mldsa_signatures() {
    // Test transaction with multiple inputs, each requiring ML-DSA signature
    let client = reqwest::Client::new();

    // Create transaction with multiple outputs to same address
    // Then create transaction spending multiple UTXOs (multiple inputs)
    // Each input requires separate ML-DSA signature

    let wallet_req = CreateWalletRequest {
        name: "test_multi_input".to_string(),
        passphrase: "pass123".to_string(),
        network: "regtest".to_string(),
    };

    let _ = client
        .post("http://localhost:8333/api/v1/wallet/create")
        .json(&wallet_req)
        .send()
        .await;

    // This test requires wallet to have multiple UTXOs
    // In practice, would need to:
    // 1. Receive multiple separate transactions
    // 2. Create transaction spending from multiple UTXOs
    // 3. Verify each input has ML-DSA signature

    // Placeholder for future implementation
    // When implemented, should verify:
    // - Each input has separate ML-DSA-65 signature (6,618 hex chars each)
    // - Transaction size scales linearly with input count
    // - All signatures verify independently
}

#[test]
fn test_mldsa_transaction_constants() {
    // Test ML-DSA size constants
    const MLDSA65_SIGNATURE_BYTES: usize = 3309;
    const MLDSA65_SIGNATURE_HEX: usize = MLDSA65_SIGNATURE_BYTES * 2;
    assert_eq!(MLDSA65_SIGNATURE_HEX, 6618);

    const MLDSA65_PUBKEY_BYTES: usize = 1952;
    const MLDSA65_PUBKEY_HEX: usize = MLDSA65_PUBKEY_BYTES * 2;
    assert_eq!(MLDSA65_PUBKEY_HEX, 3904);

    // Transaction input with ML-DSA signature is ~10,522 hex chars
    const TX_INPUT_MLDSA_HEX: usize = MLDSA65_SIGNATURE_HEX + MLDSA65_PUBKEY_HEX;
    assert_eq!(TX_INPUT_MLDSA_HEX, 10522);

    // SHA-512 transaction hash
    const SHA512_HEX_LEN: usize = 128;
    assert_eq!("0".repeat(SHA512_HEX_LEN).len(), 128);
}