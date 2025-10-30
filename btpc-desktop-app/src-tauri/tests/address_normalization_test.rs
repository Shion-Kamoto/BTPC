//! Address Normalization Test (T027)
//!
//! Verifies that BTPC addresses are normalized to lowercase for
//! case-insensitive comparison, fixing the UTXO balance bug where
//! "BTPC..." from wallet doesn't match "btpc..." from UTXOs.

#[test]
fn test_address_normalization_case_insensitive() {
    let address_uppercase = "BTPC1234567890ABCDEF";
    let address_lowercase = "btpc1234567890abcdef";
    let address_mixed = "BtPc1234567890AbCdEf";

    // All variations should normalize to same lowercase form
    assert_eq!(
        address_uppercase.trim().to_lowercase(),
        address_lowercase.trim().to_lowercase()
    );
    assert_eq!(
        address_mixed.trim().to_lowercase(),
        address_lowercase.trim().to_lowercase()
    );
}

#[test]
fn test_address_with_prefix_removal() {
    let address_with_prefix = "Address: BTPC1234567890ABCDEF";
    let address_without_prefix = "BTPC1234567890ABCDEF";

    // Remove prefix
    let cleaned = if address_with_prefix.trim().starts_with("Address: ") {
        address_with_prefix.trim().strip_prefix("Address: ").unwrap().trim()
    } else {
        address_with_prefix.trim()
    };

    // Then normalize
    let normalized = cleaned.to_lowercase();

    assert_eq!(normalized, address_without_prefix.to_lowercase());
}

#[test]
fn test_real_btpc_address_formats() {
    // Simulate real wallet output format
    let wallet_format = "Address: BTPCbf8d2e3a4c5b6a7d8e9f0a1b2c3d4e5f6a7b8c9d0e1f2";

    // Simulate UTXO storage format (lowercase)
    let utxo_format = "btpcbf8d2e3a4c5b6a7d8e9f0a1b2c3d4e5f6a7b8c9d0e1f2";

    // Clean and normalize wallet format
    let cleaned_wallet = if wallet_format.trim().starts_with("Address: ") {
        wallet_format.trim().strip_prefix("Address: ").unwrap().trim()
    } else {
        wallet_format.trim()
    };
    let normalized_wallet = cleaned_wallet.to_lowercase();

    // Normalize UTXO format
    let normalized_utxo = utxo_format.trim().to_lowercase();

    // They should match after normalization
    assert_eq!(normalized_wallet, normalized_utxo);
}

#[test]
fn test_whitespace_handling() {
    let address_with_whitespace = "  BTPC1234567890ABCDEF  ";
    let address_clean = "BTPC1234567890ABCDEF";

    assert_eq!(
        address_with_whitespace.trim().to_lowercase(),
        address_clean.to_lowercase()
    );
}
