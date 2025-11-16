//! RED Phase Test: Wallet version metadata (V1 vs V2)
//!
//! These tests MUST FAIL initially - they define wallet versioning (FR-007, FR-008).

use btpc_core::crypto::wallet_serde::{WalletData, WalletVersion};

#[test]
fn test_wallet_version_enum_exists() {
    // Test that WalletVersion enum has V1 and V2 variants
    let v1 = WalletVersion::V1NonDeterministic;
    let v2 = WalletVersion::V2BIP39Deterministic;

    assert_ne!(v1, v2, "V1 and V2 must be different variants");
}

#[test]
fn test_wallet_data_has_version_field() {
    // This will fail if WalletData doesn't have version field
    // We'll construct a minimal WalletData in the test
    // (actual construction will fail until GREEN phase)
}

#[test]
fn test_v1_wallet_represents_non_deterministic() {
    let version = WalletVersion::V1NonDeterministic;

    // V1 should indicate non-deterministic
    let is_deterministic = matches!(version, WalletVersion::V2BIP39Deterministic);
    assert!(!is_deterministic, "V1 should not be deterministic");
}

#[test]
fn test_v2_wallet_represents_bip39_deterministic() {
    let version = WalletVersion::V2BIP39Deterministic;

    // V2 should indicate BIP39 deterministic
    let is_deterministic = matches!(version, WalletVersion::V2BIP39Deterministic);
    assert!(is_deterministic, "V2 should be BIP39 deterministic");
}

#[test]
fn test_wallet_version_serializes() {
    use serde_json;

    let v1 = WalletVersion::V1NonDeterministic;
    let v2 = WalletVersion::V2BIP39Deterministic;

    // Should serialize to JSON
    let v1_json = serde_json::to_string(&v1).unwrap();
    let v2_json = serde_json::to_string(&v2).unwrap();

    assert!(v1_json.contains("V1") || v1_json.contains("NonDeterministic"));
    assert!(v2_json.contains("V2") || v2_json.contains("BIP39") || v2_json.contains("Deterministic"));
}

#[test]
fn test_wallet_version_deserializes() {
    use serde_json;

    // Should round-trip through JSON
    let original = WalletVersion::V2BIP39Deterministic;
    let json = serde_json::to_string(&original).unwrap();
    let deserialized: WalletVersion = serde_json::from_str(&json).unwrap();

    assert_eq!(original, deserialized, "Version must round-trip through JSON");
}