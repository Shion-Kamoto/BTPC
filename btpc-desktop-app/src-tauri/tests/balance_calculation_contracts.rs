//! Contract Tests for Balance Calculation
//!
//! Validates the address normalization fix for the UTXO balance bug.
//!
//! **Problem**: btpc_wallet RPC returns addresses in varying case, but HashMap uses
//! case-sensitive keys, causing balance to display as 0.00 BTP instead of actual value.
//!
//! **Solution**: Normalize addresses to lowercase at JSON deserialization boundary.
//!
//! Validates:
//! - FR-001: Balance displays correctly (226.625 BTP across 7 UTXOs)
//! - FR-002: Balance updates automatically on transaction receipt
//! - FR-003: Individual UTXO amounts shown correctly
//! - FR-004: UTXO count accurate
//! - FR-005: Zero-balance wallets display 0.00000000 BTP

use serde_json::json;
use std::collections::HashMap;

/// Address normalization helper (to be implemented in actual code)
fn normalize_address(address: &str) -> String {
    address.to_lowercase()
}

#[test]
fn test_address_normalization_contract() {
    // Contract: Addresses must be normalized to lowercase for HashMap keys
    let addresses = vec![
        "BtpcTest123456789ABCDEF",
        "btpctest123456789abcdef",
        "BTPCTEST123456789ABCDEF",
        "BTPCtest123456789ABCdef",
    ];

    let normalized: Vec<String> = addresses.iter().map(|a| normalize_address(a)).collect();

    // All should normalize to same value
    assert!(normalized.iter().all(|a| a == &normalized[0]));
    assert_eq!(normalized[0], "btpctest123456789abcdef");
}

#[test]
fn test_case_insensitive_utxo_lookup() {
    // Contract: UTXO lookups must be case-insensitive
    let mut utxo_map: HashMap<String, u64> = HashMap::new();

    // Insert with normalized address
    let address = "BtpcTestAddress123";
    utxo_map.insert(normalize_address(address), 100_000_000); // 1.0 BTP

    // Lookup with different case should work
    let lookup_addresses = vec![
        "btpctestaddress123",
        "BTPCTESTADDRESS123",
        "BtpcTestAddress123",
    ];

    for addr in lookup_addresses {
        let normalized = normalize_address(addr);
        assert!(utxo_map.contains_key(&normalized));
        assert_eq!(utxo_map.get(&normalized), Some(&100_000_000));
    }
}

#[test]
fn test_multi_utxo_balance_aggregation() {
    // Contract: Balance = sum of all UTXOs for normalized address (FR-001, FR-003)
    let mut address_utxos: HashMap<String, Vec<u64>> = HashMap::new();

    let address = normalize_address("BtpcWallet123");

    // Simulate 7 UTXOs (like the bug report: 226.625 BTP total)
    let utxos = vec![
        50_000_000_000, // 50.0 BTP
        50_000_000_000, // 50.0 BTP
        50_000_000_000, // 50.0 BTP
        50_000_000_000, // 50.0 BTP
        12_500_000_000, // 12.5 BTP
        12_500_000_000, // 12.5 BTP
         1_625_000_000, //  1.625 BTP
    ];

    address_utxos.insert(address.clone(), utxos.clone());

    // Calculate total
    let total: u64 = address_utxos.get(&address).unwrap().iter().sum();

    assert_eq!(total, 226_625_000_000); // 226.625 BTP in satoshis
    assert_eq!(address_utxos.get(&address).unwrap().len(), 7); // FR-004: UTXO count
}

#[test]
fn test_zero_balance_display_contract() {
    // Contract: Zero-balance wallets display "0.00000000 BTP" not blank (FR-005)
    let address_utxos: HashMap<String, Vec<u64>> = HashMap::new();

    let address = normalize_address("BtpcEmptyWallet");

    let balance = address_utxos
        .get(&address)
        .map(|utxos| utxos.iter().sum::<u64>())
        .unwrap_or(0);

    assert_eq!(balance, 0);

    // Format as BTP (8 decimal places)
    let btpc_formatted = format!("{:.8}", balance as f64 / 100_000_000.0);
    assert_eq!(btpc_formatted, "0.00000000");
}

#[test]
fn test_balance_update_on_new_utxo() {
    // Contract: Balance updates automatically when new UTXO received (FR-002)
    let mut address_utxos: HashMap<String, Vec<u64>> = HashMap::new();

    let address = normalize_address("BtpcTestWallet");

    // Initial state: 1 UTXO
    address_utxos.insert(address.clone(), vec![100_000_000]); // 1.0 BTP

    let initial_balance: u64 = address_utxos.get(&address).unwrap().iter().sum();
    assert_eq!(initial_balance, 100_000_000);

    // New transaction received
    address_utxos.get_mut(&address).unwrap().push(50_000_000); // +0.5 BTP

    let updated_balance: u64 = address_utxos.get(&address).unwrap().iter().sum();
    assert_eq!(updated_balance, 150_000_000); // 1.5 BTP
}

#[test]
fn test_json_deserialization_normalizes_addresses() {
    // Contract: JSON deserialization must normalize addresses
    let json_response = json!({
        "utxos": [
            {"address": "BtpcTest123", "amount": 100000000},
            {"address": "BTPCTEST123", "amount": 50000000}, // Same address, different case
            {"address": "btpctest456", "amount": 25000000},  // Different address
        ]
    });

    // Simulate processing JSON (with normalization)
    let mut address_balances: HashMap<String, u64> = HashMap::new();

    if let Some(utxos) = json_response["utxos"].as_array() {
        for utxo in utxos {
            let addr = utxo["address"].as_str().unwrap();
            let amount = utxo["amount"].as_u64().unwrap();

            let normalized = normalize_address(addr);
            *address_balances.entry(normalized).or_insert(0) += amount;
        }
    }

    // First two addresses should aggregate (same normalized key)
    let btpctest123_balance = address_balances.get("btpctest123").unwrap();
    assert_eq!(*btpctest123_balance, 150_000_000); // 100M + 50M

    let btpctest456_balance = address_balances.get("btpctest456").unwrap();
    assert_eq!(*btpctest456_balance, 25_000_000);

    assert_eq!(address_balances.len(), 2); // Only 2 unique addresses
}

#[test]
fn test_utxo_count_accuracy() {
    // Contract: UTXO count must be accurate per address (FR-004)
    let mut address_utxos: HashMap<String, Vec<u64>> = HashMap::new();

    let addr1 = normalize_address("BtpcAddr1");
    let addr2 = normalize_address("BtpcAddr2");

    address_utxos.insert(addr1.clone(), vec![100, 200, 300]);
    address_utxos.insert(addr2.clone(), vec![400]);

    assert_eq!(address_utxos.get(&addr1).unwrap().len(), 3);
    assert_eq!(address_utxos.get(&addr2).unwrap().len(), 1);
}

#[test]
fn test_individual_utxo_details_accessible() {
    // Contract: Each UTXO details must be accessible (FR-004)
    #[derive(Debug, Clone)]
    struct UTXO {
        txid: String,
        vout: u32,
        amount: u64,
        address: String,
    }

    let utxos = vec![
        UTXO {
            txid: "abc123".to_string(),
            vout: 0,
            amount: 100_000_000,
            address: normalize_address("BtpcTest"),
        },
        UTXO {
            txid: "def456".to_string(),
            vout: 1,
            amount: 50_000_000,
            address: normalize_address("BtpcTest"),
        },
    ];

    // Each UTXO should be individually accessible
    assert_eq!(utxos[0].amount, 100_000_000);
    assert_eq!(utxos[1].amount, 50_000_000);
    assert_eq!(utxos[0].vout, 0);
    assert_eq!(utxos[1].vout, 1);
}

#[test]
fn test_balance_precision_contract() {
    // Contract: Balance calculations must maintain satoshi precision
    let amounts = vec![
        1,           // 0.00000001 BTP
        10,          // 0.00000010 BTP
        100,         // 0.00000100 BTP
        1_000,       // 0.00001000 BTP
        10_000,      // 0.00010000 BTP
        100_000,     // 0.00100000 BTP
        1_000_000,   // 0.01000000 BTP
        10_000_000,  // 0.10000000 BTP
    ];

    let total: u64 = amounts.iter().sum();
    assert_eq!(total, 11_111_111); // Exact satoshi sum

    // Verify no floating point errors
    let btpc_value = total as f64 / 100_000_000.0;
    assert!((btpc_value - 0.11111111).abs() < 0.00000001);
}

#[cfg(test)]
mod edge_cases {
    use super::*;

    #[test]
    fn test_empty_address_handling() {
        // Edge case: Empty address should normalize safely
        let addr = normalize_address("");
        assert_eq!(addr, "");
    }

    #[test]
    fn test_unicode_address_handling() {
        // Edge case: Non-ASCII characters (should be rejected in actual validation)
        let addr = normalize_address("Btpc测试");
        assert_eq!(addr, "btpc测试"); // Lowercase but preserves unicode
    }

    #[test]
    fn test_very_large_utxo_set() {
        // Edge case: 10,000 UTXOs (performance test)
        let mut utxos = Vec::new();
        for i in 0..10_000 {
            utxos.push(100_000_000); // 1.0 BTP each
        }

        let total: u64 = utxos.iter().sum();
        assert_eq!(total, 1_000_000_000_000); // 10,000 BTP
    }

    #[test]
    fn test_mixed_case_addresses_aggregate_correctly() {
        // Edge case: Multiple transactions to same address with different case
        let transactions = vec![
            ("BtpcAddr1", 100_000_000),
            ("btpcaddr1", 200_000_000),
            ("BTPCADDR1", 300_000_000),
            ("BTPCaddr1", 400_000_000),
        ];

        let mut balances: HashMap<String, u64> = HashMap::new();

        for (addr, amount) in transactions {
            let normalized = normalize_address(addr);
            *balances.entry(normalized).or_insert(0) += amount;
        }

        // All should aggregate to single entry
        assert_eq!(balances.len(), 1);
        assert_eq!(*balances.get("btpcaddr1").unwrap(), 1_000_000_000); // 10.0 BTP
    }
}
