//! UTXO Deserialization Test (T028)
//!
//! Verifies that the custom deserializer for UTXO addresses
//! normalizes mixed-case addresses to lowercase when loading from disk.

use btpc_desktop_app::utxo_manager::UTXO;
use serde_json;

#[test]
fn test_utxo_address_normalized_on_deserialize() {
    // Simulate JSON data loaded from disk with mixed-case address
    let json_data = r#"{
        "txid": "abc123def456",
        "vout": 0,
        "value_credits": 5000000000,
        "value_btp": 50.0,
        "address": "BTPCtest123MixedCaseAddress",
        "block_height": 100,
        "is_coinbase": true,
        "created_at": "2025-01-01T00:00:00Z",
        "spent": false,
        "spent_in_tx": null,
        "spent_at_height": null,
        "script_pubkey": []
    }"#;

    // Deserialize the UTXO
    let utxo: UTXO = serde_json::from_str(json_data).expect("Failed to deserialize UTXO");

    // Verify the address was normalized to lowercase
    assert_eq!(utxo.address, "btpctest123mixedcaseaddress");
}

#[test]
fn test_utxo_uppercase_address_normalized() {
    let json_data = r#"{
        "txid": "xyz789",
        "vout": 1,
        "value_credits": 10000000000,
        "value_btp": 100.0,
        "address": "BTPC1234567890UPPERCASE",
        "block_height": 200,
        "is_coinbase": false,
        "created_at": "2025-01-15T12:00:00Z",
        "spent": true,
        "spent_in_tx": "spent_tx_123",
        "spent_at_height": 250,
        "script_pubkey": []
    }"#;

    let utxo: UTXO = serde_json::from_str(json_data).expect("Failed to deserialize UTXO");

    // All uppercase should become lowercase
    assert_eq!(utxo.address, "btpc1234567890uppercase");
}

#[test]
fn test_utxo_lowercase_address_unchanged() {
    let json_data = r#"{
        "txid": "already_lowercase",
        "vout": 2,
        "value_credits": 2500000000,
        "value_btp": 25.0,
        "address": "btpc_already_lowercase_123",
        "block_height": 300,
        "is_coinbase": true,
        "created_at": "2025-02-01T00:00:00Z",
        "spent": false,
        "spent_in_tx": null,
        "spent_at_height": null,
        "script_pubkey": []
    }"#;

    let utxo: UTXO = serde_json::from_str(json_data).expect("Failed to deserialize UTXO");

    // Already lowercase should remain unchanged
    assert_eq!(utxo.address, "btpc_already_lowercase_123");
}

#[test]
fn test_utxo_address_with_prefix_normalized() {
    // Test with "Address: " prefix that might be in wallet output
    let json_data = r#"{
        "txid": "prefix_test",
        "vout": 0,
        "value_credits": 1000000000,
        "value_btp": 10.0,
        "address": "Address: BTPCmixedCase",
        "block_height": 400,
        "is_coinbase": false,
        "created_at": "2025-03-01T00:00:00Z",
        "spent": false,
        "spent_in_tx": null,
        "spent_at_height": null,
        "script_pubkey": []
    }"#;

    let utxo: UTXO = serde_json::from_str(json_data).expect("Failed to deserialize UTXO");

    // Should normalize including removing whitespace and lowercasing
    // Note: The deserializer only lowercases, prefix stripping happens in clean_address
    assert_eq!(utxo.address, "address: btpcmixedcase");
}

#[test]
fn test_multiple_utxos_batch_deserialization() {
    let json_array = r#"[
        {
            "txid": "tx1",
            "vout": 0,
            "value_credits": 1000000000,
            "value_btp": 10.0,
            "address": "BTPCaddr1",
            "block_height": 100,
            "is_coinbase": true,
            "created_at": "2025-01-01T00:00:00Z",
            "spent": false,
            "spent_in_tx": null,
            "spent_at_height": null,
            "script_pubkey": []
        },
        {
            "txid": "tx2",
            "vout": 0,
            "value_credits": 2000000000,
            "value_btp": 20.0,
            "address": "btpcaddr2",
            "block_height": 101,
            "is_coinbase": true,
            "created_at": "2025-01-02T00:00:00Z",
            "spent": false,
            "spent_in_tx": null,
            "spent_at_height": null,
            "script_pubkey": []
        },
        {
            "txid": "tx3",
            "vout": 0,
            "value_credits": 3000000000,
            "value_btp": 30.0,
            "address": "BtPcAdDr3",
            "block_height": 102,
            "is_coinbase": true,
            "created_at": "2025-01-03T00:00:00Z",
            "spent": false,
            "spent_in_tx": null,
            "spent_at_height": null,
            "script_pubkey": []
        }
    ]"#;

    let utxos: Vec<UTXO> = serde_json::from_str(json_array).expect("Failed to deserialize UTXO array");

    // Verify all addresses were normalized
    assert_eq!(utxos[0].address, "btpcaddr1");
    assert_eq!(utxos[1].address, "btpcaddr2");
    assert_eq!(utxos[2].address, "btpcaddr3");
}
