//! Simplified Contract Tests (V010-V013)
//!
//! Basic API stability tests without deep implementation coupling.

use btpc_core::Network;

#[test]
fn test_network_enum_contract() {
    // Contract: Network enum has 3 variants
    let mainnet = Network::Mainnet;
    let testnet = Network::Testnet;
    let regtest = Network::Regtest;

    // Contract: Magic bytes are distinct
    assert_ne!(mainnet.magic_bytes(), testnet.magic_bytes());
    assert_ne!(testnet.magic_bytes(), regtest.magic_bytes());
}

#[test]
fn test_block_basics_contract() {
    // Contract: Block and Transaction types exist
    use btpc_core::{Block, Transaction};

    // Type existence check (will fail to compile if removed)
    let _ = std::marker::PhantomData::<Block>;
    let _ = std::marker::PhantomData::<Transaction>;
}

#[test]
fn test_crypto_types_contract() {
    // Contract: Core crypto types exist
    use btpc_core::crypto::{Hash, Address, PublicKey, PrivateKey, Signature};

    // Type existence checks
    let _ = std::marker::PhantomData::<Hash>;
    let _ = std::marker::PhantomData::<Address>;
    let _ = std::marker::PhantomData::<PublicKey>;
    let _ = std::marker::PhantomData::<PrivateKey>;
    let _ = std::marker::PhantomData::<Signature>;
}

#[test]
fn test_hash_determinism_contract() {
    use btpc_core::crypto::Hash;

    // Contract: Hash is deterministic
    let data = b"test data for contract validation";
    let hash1 = Hash::hash(data);
    let hash2 = Hash::hash(data);

    assert_eq!(hash1, hash2, "Hash must be deterministic");
}

#[tokio::test]
async fn test_rpc_server_contract() {
    use btpc_core::rpc::server::{RpcServer, RpcConfig};

    // Contract: RPC server can be created and methods registered
    let config = RpcConfig {
        enable_auth: false,
        ..Default::default()
    };
    let server = RpcServer::new(config);

    server.register_method("test", |_| Ok(serde_json::json!({"status": "ok"}))).await;

    let methods = server.get_methods().await;
    assert!(methods.contains(&"test".to_string()));
}