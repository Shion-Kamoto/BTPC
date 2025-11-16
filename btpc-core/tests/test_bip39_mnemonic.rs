//! RED Phase Test: BIP39 mnemonic parsing and validation
//!
//! These tests MUST FAIL initially - they define the required behavior
//! for BIP39 mnemonic handling (FR-003, FR-004).

use btpc_core::crypto::bip39::Mnemonic;

#[test]
fn test_parse_valid_24_word_mnemonic() {
    // Standard BIP39 test vector
    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

    let parsed = Mnemonic::parse(mnemonic).unwrap();

    assert_eq!(parsed.word_count(), 24, "Mnemonic must have 24 words");
    assert_eq!(parsed.entropy_bits(), 256, "Mnemonic must have 256 bits of entropy");
}

#[test]
fn test_reject_invalid_word_count() {
    let test_cases = vec![
        ("abandon abandon abandon", 3), // Too few
        ("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon", 12), // 12 words (valid for some, but BTPC requires 24)
        ("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon", 25), // Too many
    ];

    for (mnemonic, expected_count) in test_cases {
        let result = Mnemonic::parse(mnemonic);
        assert!(result.is_err(), "Should reject {} word mnemonic", expected_count);

        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("exactly 24 words") && err_msg.contains(&format!("found: {}", expected_count)),
            "Error should mention expected 24 words and found count. Got: {}",
            err_msg
        );
    }
}

#[test]
fn test_reject_invalid_word() {
    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon xyz";

    let result = Mnemonic::parse(mnemonic);
    assert!(result.is_err(), "Should reject invalid word 'xyz'");

    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Invalid word") || err_msg.contains("unknown") || err_msg.contains("BIP39"),
        "Error should identify invalid word. Got: {}",
        err_msg
    );
}

#[test]
fn test_reject_invalid_checksum() {
    // Valid words, but wrong checksum (last word should be 'art', not 'about')
    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    let result = Mnemonic::parse(mnemonic);
    assert!(result.is_err(), "Should reject invalid checksum");

    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("checksum"),
        "Error should mention checksum. Got: {}",
        err_msg
    );
}

#[test]
fn test_parse_normalizes_whitespace() {
    // Extra spaces, tabs, newlines should be normalized (but still 24 words total)
    let mnemonic_messy = "abandon  abandon\tabandon\nabandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";
    let mnemonic_clean = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

    let parsed_messy = Mnemonic::parse(mnemonic_messy).unwrap();
    let parsed_clean = Mnemonic::parse(mnemonic_clean).unwrap();

    // Should produce same results after normalization
    assert_eq!(parsed_messy.word_count(), parsed_clean.word_count());
}

#[test]
fn test_case_insensitive_parsing() {
    // BIP39 words should be case-insensitive
    let mnemonic_lower = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";
    let mnemonic_upper = "ABANDON ABANDON ABANDON ABANDON ABANDON ABANDON ABANDON ABANDON ABANDON ABANDON ABANDON ABANDON ABANDON ABANDON ABANDON ABANDON ABANDON ABANDON ABANDON ABANDON ABANDON ABANDON ABANDON ART";
    let mnemonic_mixed = "Abandon Abandon Abandon Abandon Abandon Abandon Abandon Abandon Abandon Abandon Abandon Abandon Abandon Abandon Abandon Abandon Abandon Abandon Abandon Abandon Abandon Abandon Abandon Art";

    let parsed_lower = Mnemonic::parse(mnemonic_lower).unwrap();
    let parsed_upper = Mnemonic::parse(mnemonic_upper).unwrap();
    let parsed_mixed = Mnemonic::parse(mnemonic_mixed).unwrap();

    // All should be valid
    assert_eq!(parsed_lower.word_count(), 24);
    assert_eq!(parsed_upper.word_count(), 24);
    assert_eq!(parsed_mixed.word_count(), 24);
}

#[test]
fn test_multiple_valid_mnemonics() {
    // Test various valid BIP39 test vectors
    let test_vectors = vec![
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art",
        "legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth title",
        "letter advice cage absurd amount doctor acoustic avoid letter advice cage absurd amount doctor acoustic avoid letter advice cage absurd amount doctor acoustic bless",
    ];

    for mnemonic in test_vectors {
        let parsed = Mnemonic::parse(mnemonic);
        assert!(
            parsed.is_ok(),
            "Valid mnemonic should parse successfully: {}",
            mnemonic
        );
        assert_eq!(parsed.unwrap().word_count(), 24);
    }
}

#[test]
fn test_empty_mnemonic_rejected() {
    let result = Mnemonic::parse("");
    assert!(result.is_err(), "Empty string should be rejected");

    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("exactly 24 words") && err_msg.contains("found: 0"),
        "Error should mention 0 words found. Got: {}",
        err_msg
    );
}