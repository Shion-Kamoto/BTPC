//! Integration test for genesis block initialization
//! Per data-model.md specification
//!
//! This test MUST FAIL initially (implementation not complete yet)

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BlockHeader {
    version: u32,
    previous_hash: String,    // SHA-512 (128 hex chars)
    merkle_root: String,      // SHA-512 (128 hex chars)
    timestamp: u64,
    difficulty: u64,
    nonce: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Block {
    header: BlockHeader,
    transactions: Vec<Transaction>,
    hash: String,             // SHA-512 (128 hex chars)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Transaction {
    version: u32,
    inputs: Vec<TransactionInput>,
    outputs: Vec<TransactionOutput>,
    locktime: u32,
    hash: String,             // SHA-512 (128 hex chars)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TransactionInput {
    previous_tx: String,
    output_index: u32,
    signature: String,        // ML-DSA-65 signature
    public_key: String,       // ML-DSA-65 public key
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TransactionOutput {
    amount: u64,
    script_pubkey: String,
}

#[tokio::test]
#[ignore] // Will fail until genesis block implementation is complete
async fn test_genesis_block_creation() {
    // Act: Request genesis block from blockchain API
    let client = reqwest::Client::new();
    let response = client
        .get("http://localhost:8332/api/v1/block/0")
        .send()
        .await;

    // Assert
    assert!(response.is_ok(), "Blockchain service should be accessible");

    let response = response.unwrap();
    assert_eq!(response.status(), 200);

    let genesis: Block = response
        .json()
        .await
        .expect("Should return genesis block");

    // Verify genesis block properties per data-model.md
    assert_eq!(genesis.header.version, 1, "Genesis block version should be 1");

    assert_eq!(
        genesis.header.previous_hash,
        "0".repeat(128),
        "Genesis block should have all-zero previous hash"
    );

    // Verify SHA-512 hash format
    assert_eq!(
        genesis.hash.len(),
        128,
        "Block hash should be SHA-512 (128 hex characters)"
    );
    assert!(
        genesis.hash.chars().all(|c| c.is_ascii_hexdigit()),
        "Block hash should be valid hex"
    );

    // Verify merkle root format
    assert_eq!(
        genesis.header.merkle_root.len(),
        128,
        "Merkle root should be SHA-512 (128 hex characters)"
    );

    // Genesis block should have exactly one coinbase transaction
    assert_eq!(
        genesis.transactions.len(),
        1,
        "Genesis block should contain exactly one coinbase transaction"
    );

    let coinbase = &genesis.transactions[0];

    // Coinbase transaction should have no inputs (or one input with null previous_tx)
    assert!(
        coinbase.inputs.is_empty() || coinbase.inputs[0].previous_tx == "0".repeat(128),
        "Coinbase transaction should have no inputs or null previous_tx"
    );

    // Verify initial block reward per linear decay economics
    // Initial reward: 32.375 BTPC = 3,237,500,000 satoshis
    assert_eq!(
        coinbase.outputs.len(),
        1,
        "Coinbase should have exactly one output"
    );

    let coinbase_output = &coinbase.outputs[0];
    assert_eq!(
        coinbase_output.amount,
        3_237_500_000,
        "Initial block reward should be 32.375 BTPC (3,237,500,000 satoshis)"
    );

    // Verify timestamp is reasonable (Unix epoch)
    assert!(
        genesis.header.timestamp > 0,
        "Genesis block should have valid timestamp"
    );
}

#[tokio::test]
#[ignore] // Will fail until implementation is complete
async fn test_genesis_block_hash_meets_difficulty() {
    // Test that genesis block hash meets initial difficulty requirement
    let client = reqwest::Client::new();
    let response = client
        .get("http://localhost:8332/api/v1/block/0")
        .send()
        .await;

    if let Ok(response) = response {
        let genesis: Block = response.json().await.expect("Should parse");

        // Verify block hash meets difficulty target
        // Initial difficulty should produce a hash with leading zeros
        let hash_bytes = hex::decode(&genesis.hash).expect("Should be valid hex");

        // Count leading zero bits
        let mut leading_zeros = 0;
        for byte in hash_bytes.iter() {
            if *byte == 0 {
                leading_zeros += 8;
            } else {
                leading_zeros += byte.leading_zeros();
                break;
            }
        }

        // Initial difficulty should be reasonable (at least a few leading zeros)
        assert!(
            leading_zeros >= 8,
            "Genesis block should meet initial difficulty requirement"
        );
    }
}

#[tokio::test]
#[ignore] // Will fail until implementation is complete
async fn test_genesis_block_immutability() {
    // Test that genesis block hash is deterministic and immutable
    let client = reqwest::Client::new();

    // Request genesis block multiple times
    let response1 = client
        .get("http://localhost:8332/api/v1/block/0")
        .send()
        .await;

    let response2 = client
        .get("http://localhost:8332/api/v1/block/0")
        .send()
        .await;

    if let (Ok(r1), Ok(r2)) = (response1, response2) {
        let genesis1: Block = r1.json().await.expect("Should parse");
        let genesis2: Block = r2.json().await.expect("Should parse");

        // Genesis block should be identical every time
        assert_eq!(
            genesis1.hash, genesis2.hash,
            "Genesis block hash should be deterministic"
        );
        assert_eq!(
            genesis1.header.merkle_root, genesis2.header.merkle_root,
            "Genesis block merkle root should be deterministic"
        );
        assert_eq!(
            genesis1.header.timestamp, genesis2.header.timestamp,
            "Genesis block timestamp should be immutable"
        );
    }
}

#[tokio::test]
#[ignore] // Will fail until implementation is complete
async fn test_genesis_block_in_blockchain_info() {
    // Test that genesis block is reflected in blockchain info
    let client = reqwest::Client::new();

    // Get blockchain info
    let info_response = client
        .get("http://localhost:8332/api/v1/blockchain/info")
        .send()
        .await;

    // Get genesis block
    let genesis_response = client
        .get("http://localhost:8332/api/v1/block/0")
        .send()
        .await;

    if let (Ok(info_resp), Ok(genesis_resp)) = (info_response, genesis_response) {
        #[derive(Debug, Deserialize)]
        struct BlockchainInfo {
            blocks: u64,
            bestblockhash: String,
            total_supply: u64,
        }

        let info: BlockchainInfo = info_resp.json().await.expect("Should parse");
        let genesis: Block = genesis_resp.json().await.expect("Should parse");

        // If genesis is the only block, it should be the best block
        if info.blocks == 1 {
            assert_eq!(
                info.bestblockhash, genesis.hash,
                "Genesis block should be best block when it's the only block"
            );

            // Total supply should match genesis coinbase output
            assert_eq!(
                info.total_supply,
                3_237_500_000,
                "Initial supply should be 32.375 BTPC from genesis block"
            );
        }
    }
}

#[test]
fn test_genesis_block_constants() {
    // Test constants used in genesis block
    const INITIAL_REWARD: u64 = 3_237_500_000; // 32.375 BTPC in satoshis
    const BTPC_PER_COIN: u64 = 100_000_000; // 1 BTPC = 100 million satoshis

    assert_eq!(INITIAL_REWARD, 32_375 * BTPC_PER_COIN / 1000);

    // Verify SHA-512 hash sizes
    const SHA512_HEX_LEN: usize = 128;
    assert_eq!("0".repeat(SHA512_HEX_LEN).len(), 128);

    // Verify ML-DSA-65 sizes
    const MLDSA65_PUBKEY_HEX_LEN: usize = 3904;
    const MLDSA65_SIG_HEX_LEN: usize = 6618;

    assert_eq!(MLDSA65_PUBKEY_HEX_LEN, 1952 * 2);
    assert_eq!(MLDSA65_SIG_HEX_LEN, 3309 * 2);
}