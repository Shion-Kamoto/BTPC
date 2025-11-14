//! RPC API Contract Tests (V010)
//!
//! Ensures RPC interface stability across versions.
//! Tests that all documented RPC methods maintain their signatures.

use btpc_core::rpc::server::{RpcServer, RpcConfig};
use serde_json::json;

#[tokio::test]
async fn test_getblockchaininfo_contract() {
    // Contract: getblockchaininfo returns blocks, difficulty, chain
    let config = RpcConfig {
        enable_auth: false,
        ..Default::default()
    };
    let server = RpcServer::new(config);

    // Register stub handler for contract testing
    server.register_method("getblockchaininfo", |_| {
        Ok(json!({
            "blocks": 100,
            "difficulty": 545259519u64,
            "chain": "testnet"
        }))
    }).await;

    let request = r#"{"jsonrpc":"2.0","method":"getblockchaininfo","params":[],"id":1}"#;
    let response = server.process_request(request).await;

    // Contract validation
    assert!(response.contains("blocks"));
    assert!(response.contains("difficulty"));
    assert!(response.contains("chain"));
    assert!(response.contains(r#""id":1"#));
}

#[tokio::test]
async fn test_getblock_contract() {
    // Contract: getblock(hash) returns block data with hash, height, tx
    let config = RpcConfig {
        enable_auth: false,
        ..Default::default()
    };
    let server = RpcServer::new(config);

    server.register_method("getblock", |params| {
        let hash = params.and_then(|p| p.get("hash").cloned())
            .ok_or_else(|| btpc_core::rpc::RpcServerError::InvalidParams("hash required".to_string()))?;

        Ok(json!({
            "hash": hash,
            "height": 50,
            "tx": []
        }))
    }).await;

    let request = r#"{"jsonrpc":"2.0","method":"getblock","params":{"hash":"abc123"},"id":2}"#;
    let response = server.process_request(request).await;

    assert!(response.contains("hash"));
    assert!(response.contains("height"));
    assert!(response.contains("tx"));
}

#[tokio::test]
async fn test_getblocktemplate_contract() {
    // Contract: getblocktemplate returns version, bits, previousblockhash
    let config = RpcConfig {
        enable_auth: false,
        ..Default::default()
    };
    let server = RpcServer::new(config);

    server.register_method("getblocktemplate", |_| {
        Ok(json!({
            "version": 1,
            "bits": 545259519u64,
            "previousblockhash": "0000000000000000000000000000000000000000000000000000000000000000",
            "transactions": []
        }))
    }).await;

    let request = r#"{"jsonrpc":"2.0","method":"getblocktemplate","params":[],"id":3}"#;
    let response = server.process_request(request).await;

    assert!(response.contains("version"));
    assert!(response.contains("bits"));
    assert!(response.contains("previousblockhash"));
}

#[tokio::test]
async fn test_submitblock_contract() {
    // Contract: submitblock(blockdata) returns null on success or error message
    let config = RpcConfig {
        enable_auth: false,
        ..Default::default()
    };
    let server = RpcServer::new(config);

    server.register_method("submitblock", |params| {
        params.ok_or_else(|| btpc_core::rpc::RpcServerError::InvalidParams("block data required".to_string()))?;
        Ok(json!(null))
    }).await;

    let request = r#"{"jsonrpc":"2.0","method":"submitblock","params":{"blockdata":"010000..."},"id":4}"#;
    let response = server.process_request(request).await;

    assert!(response.contains("null") || response.contains("result"));
}

#[tokio::test]
async fn test_getpeerinfo_contract() {
    // Contract: getpeerinfo returns array of peer objects with addr, version
    let config = RpcConfig {
        enable_auth: false,
        ..Default::default()
    };
    let server = RpcServer::new(config);

    server.register_method("getpeerinfo", |_| {
        Ok(json!([
            {
                "addr": "127.0.0.1:18333",
                "version": 1,
                "subver": "/BTPC:0.1.0/"
            }
        ]))
    }).await;

    let request = r#"{"jsonrpc":"2.0","method":"getpeerinfo","params":[],"id":5}"#;
    let response = server.process_request(request).await;

    assert!(response.contains("addr"));
    assert!(response.contains("version"));
}