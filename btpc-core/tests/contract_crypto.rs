//! Crypto API Contract Tests (V011)
//!
//! Ensures cryptographic API stability and correctness.

use btpc_core::crypto::{Hash, Address, PublicKey, Signature};
use btpc_core::Network;

// Crypto constants (not exported from btpc_core::crypto)
const ML_DSA_PUBLIC_KEY_SIZE: usize = 1952;  // pqc_dilithium PUBLICKEYBYTES
const ML_DSA_SIGNATURE_SIZE: usize = 3293;    // pqc_dilithium SIGNBYTES
const SHA512_HASH_SIZE: usize = 64;

#[test]
fn test_hash_contract() {
    // Contract: Hash is 64 bytes (SHA-512)
    let data = b"test data";
    let hash = Hash::hash(data);

    assert_eq!(hash.as_bytes().len(), SHA512_HASH_SIZE, "Hash must be 64 bytes (SHA-512)");

    // Contract: Hash is deterministic
    let hash2 = Hash::hash(data);
    assert_eq!(hash, hash2, "Hash must be deterministic");

    // Contract: Hash implements Copy
    let hash_copy = hash;
    assert_eq!(hash, hash_copy);
}

#[test]
fn test_address_contract() {
    // Contract: Address can be created from public key
    let pubkey_bytes = vec![0u8; ML_DSA_PUBLIC_KEY_SIZE];
    let pubkey = PublicKey::from_bytes(&pubkey_bytes).expect("valid pubkey");

    let address = Address::from_public_key(&pubkey, Network::Testnet);

    // Contract: Address has string representation
    let addr_str = address.to_string();
    assert!(!addr_str.is_empty(), "Address must have string representation");
}

#[test]
fn test_ml_dsa_signature_contract() {
    // Contract: ML-DSA signature size is fixed
    let sig_bytes = vec![0u8; ML_DSA_SIGNATURE_SIZE];
    let sig = Signature::from_bytes(&sig_bytes).expect("valid signature");

    assert_eq!(sig.to_bytes().len(), ML_DSA_SIGNATURE_SIZE);

    // Contract: Signature serialization is stable
    let serialized = sig.to_bytes();
    let deserialized = Signature::from_bytes(&serialized).expect("deserialize");
    assert_eq!(sig, deserialized);
}

#[test]
fn test_ml_dsa_public_key_contract() {
    // Contract: ML-DSA public key size is fixed
    let pubkey_bytes = vec![0u8; ML_DSA_PUBLIC_KEY_SIZE];
    let pubkey = PublicKey::from_bytes(&pubkey_bytes).expect("valid pubkey");

    assert_eq!(pubkey.to_bytes().len(), ML_DSA_PUBLIC_KEY_SIZE);
}

#[test]
fn test_hash_zero_contract() {
    // Contract: Hash::zero() returns all-zero hash
    let zero_hash = Hash::zero();
    assert_eq!(zero_hash.as_bytes(), &[0u8; SHA512_HASH_SIZE]);
}

#[test]
fn test_hash_from_hex_contract() {
    // Contract: Hash can be created from hex string
    let hex = "a".repeat(128); // 64 bytes = 128 hex chars
    let hash = Hash::from_hex(&hex).expect("valid hex");

    // Contract: to_hex() round-trips
    let hex2 = hash.to_hex();
    assert_eq!(hex.to_lowercase(), hex2.to_lowercase());
}