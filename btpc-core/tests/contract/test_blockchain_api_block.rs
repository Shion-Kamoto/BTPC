//! Contract test for blockchain API - getblock endpoint
//! Per blockchain-api.yaml specification
//!
//! This test MUST FAIL initially (endpoint not implemented yet)

use serde::{Deserialize, Serialize};

/// Expected Block structure per blockchain-api.yaml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
    pub size: u32,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
    pub version: u32,
    pub prev_hash: String,
    pub merkle_root: String,
    pub timestamp: u32,
    pub bits: u32,
    pub nonce: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub version: u32,
    pub inputs: Vec<serde_json::Value>,
    pub outputs: Vec<serde_json::Value>,
    pub lock_time: u32,
    pub hash: String,
}

#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: Vec<serde_json::Value>,
    id: u64,
}

#[derive(Debug, Deserialize)]
struct JsonRpcResponse<T> {
    jsonrpc: String,
    result: Option<T>,
    error: Option<serde_json::Value>,
    id: u64,
}

#[tokio::test]
#[ignore] // Will fail until T029 is implemented
async fn test_getblock_by_hash() {
    // Arrange: Create request with block hash
    let block_hash = "0".repeat(128); // SHA-512 hash (128 hex chars)
    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "getblock".to_string(),
        params: vec![serde_json::json!(block_hash)],
        id: 1,
    };

    // Act: Send request
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:8332/api/v1/blockchain/block")
        .json(&request)
        .send()
        .await;

    // Assert
    assert!(response.is_ok(), "RPC server should be accessible");

    let response = response.unwrap();
    let body: JsonRpcResponse<Block> = response
        .json()
        .await
        .expect("Should return valid JSON-RPC response");

    assert_eq!(body.jsonrpc, "2.0");
    assert_eq!(body.id, 1);

    if let Some(block) = body.result {
        // Verify block structure per data-model.md
        assert_eq!(block.hash.len(), 128, "Block hash should be SHA-512 (128 hex)");
        assert!(block.size <= 1048576, "Block size must not exceed 1MB");

        // Verify header
        assert_eq!(
            block.header.prev_hash.len(),
            128,
            "Previous hash should be SHA-512"
        );
        assert_eq!(
            block.header.merkle_root.len(),
            128,
            "Merkle root should be SHA-512"
        );
        assert!(block.header.timestamp > 0, "Timestamp should be positive");

        // Verify transactions exist
        assert!(!block.transactions.is_empty(), "Block should have at least coinbase tx");
    }
}

#[tokio::test]
#[ignore] // Will fail until T029 is implemented
async fn test_getblock_by_height() {
    // Test retrieving block by height instead of hash
    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "getblock".to_string(),
        params: vec![serde_json::json!(0)], // Genesis block
        id: 2,
    };

    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:8332/api/v1/blockchain/block")
        .json(&request)
        .send()
        .await;

    assert!(response.is_ok());
}

#[tokio::test]
#[ignore] // Will fail until T029 is implemented
async fn test_getblock_not_found() {
    // Test error handling for non-existent block
    let invalid_hash = "f".repeat(128);
    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "getblock".to_string(),
        params: vec![serde_json::json!(invalid_hash)],
        id: 3,
    };

    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:8332/api/v1/blockchain/block")
        .json(&request)
        .send()
        .await;

    if let Ok(response) = response {
        let body: JsonRpcResponse<Block> = response.json().await.expect("Should parse");
        assert!(body.error.is_some(), "Should return error for non-existent block");
    }
}

#[test]
fn test_block_structure_serialization() {
    let block = Block {
        header: BlockHeader {
            version: 1,
            prev_hash: "0".repeat(128),
            merkle_root: "a".repeat(128),
            timestamp: 1609459200,
            bits: 0x1d00ffff,
            nonce: 12345,
        },
        transactions: vec![],
        size: 285,
        hash: "b".repeat(128),
    };

    let json = serde_json::to_string(&block).expect("Should serialize");
    let deserialized: Block = serde_json::from_str(&json).expect("Should deserialize");

    assert_eq!(deserialized.size, 285);
    assert_eq!(deserialized.header.nonce, 12345);
}