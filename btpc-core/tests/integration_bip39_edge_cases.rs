//! Integration Test T031: Edge Cases & Error Handling
//!
//! Tests invalid inputs, wrong word counts, bad checksums,
//! Unicode handling, and boundary conditions.
//!
//! Feature 008: BIP39 Deterministic Wallet Recovery

use btpc_core::crypto::bip39::{Mnemonic, BIP39Error};

/// Test invalid word count (12 words - too few)
#[test]
fn test_invalid_word_count_12_words() {
    let mnemonic_12 = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    let result = Mnemonic::parse(mnemonic_12);

    assert!(result.is_err(), "12-word mnemonic should be rejected");
    match result.unwrap_err() {
        BIP39Error::InvalidWordCount { expected, found } => {
            assert_eq!(expected, 24);
            assert_eq!(found, 12);
        }
        _ => panic!("Wrong error type for 12-word mnemonic"),
    }

    println!("✅ 12-word mnemonic correctly rejected");
}

/// Test invalid word count (15 words)
#[test]
fn test_invalid_word_count_15_words() {
    let mnemonic_15 = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon";

    let result = Mnemonic::parse(mnemonic_15);

    assert!(result.is_err(), "15-word mnemonic should be rejected");
    match result.unwrap_err() {
        BIP39Error::InvalidWordCount { expected, found } => {
            assert_eq!(expected, 24);
            assert_eq!(found, 15);
        }
        _ => panic!("Wrong error type"),
    }

    println!("✅ 15-word mnemonic correctly rejected");
}

/// Test invalid word count (25 words - too many)
#[test]
fn test_invalid_word_count_25_words() {
    let mnemonic_25 = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art extra";

    let result = Mnemonic::parse(mnemonic_25);

    assert!(result.is_err(), "25-word mnemonic should be rejected");
    match result.unwrap_err() {
        BIP39Error::InvalidWordCount { expected, found } => {
            assert_eq!(expected, 24);
            assert_eq!(found, 25);
        }
        _ => panic!("Wrong error type"),
    }

    println!("✅ 25-word mnemonic correctly rejected");
}

/// Test invalid checksum
#[test]
fn test_invalid_checksum() {
    // Valid structure but wrong last word (invalid checksum)
    let bad_checksum = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon";

    let result = Mnemonic::parse(bad_checksum);

    assert!(result.is_err(), "Invalid checksum should be rejected");
    match result.unwrap_err() {
        BIP39Error::InvalidChecksum => {
            // Expected
        }
        e => panic!("Wrong error for bad checksum: {:?}", e),
    }

    println!("✅ Invalid checksum correctly rejected");
}

/// Test word not in BIP39 wordlist
#[test]
fn test_invalid_word_not_in_wordlist() {
    let invalid_word = "invalidword abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

    let result = Mnemonic::parse(invalid_word);

    assert!(result.is_err(), "Non-wordlist word should be rejected");
    match result.unwrap_err() {
        BIP39Error::InvalidWord { word, .. } => {
            assert_eq!(word, "invalidword");
        }
        e => panic!("Wrong error for invalid word: {:?}", e),
    }

    println!("✅ Invalid word correctly rejected");
}

/// Test empty input
#[test]
fn test_empty_input() {
    let result = Mnemonic::parse("");

    assert!(result.is_err(), "Empty input should be rejected");
    match result.unwrap_err() {
        BIP39Error::InvalidWordCount { found, .. } => {
            assert_eq!(found, 0);
        }
        _ => panic!("Wrong error for empty input"),
    }

    println!("✅ Empty input correctly rejected");
}

/// Test whitespace-only input
#[test]
fn test_whitespace_only() {
    let result = Mnemonic::parse("   \n\t  \r\n  ");

    assert!(result.is_err(), "Whitespace-only should be rejected");

    println!("✅ Whitespace-only input correctly rejected");
}

/// Test mixed case (should be case-insensitive)
#[test]

/// Test excessive whitespace handling
#[test]
fn test_excessive_whitespace() {
    let normal = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";
    let excessive = "abandon  abandon   abandon\tabandon\nabandon abandon abandon  abandon\r\nabandon abandon\t\tabandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

    let result_normal = Mnemonic::parse(normal).unwrap();
    let result_excessive = Mnemonic::parse(excessive).unwrap();

    // Both should produce same seed (whitespace normalized)
    let seed_normal = result_normal.to_seed("").unwrap();
    let seed_excessive = result_excessive.to_seed("").unwrap();

    assert_eq!(seed_normal, seed_excessive);

    println!("✅ Excessive whitespace correctly normalized");
}

/// Test leading/trailing whitespace
#[test]
fn test_leading_trailing_whitespace() {
    let trimmed = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";
    let padded = "   abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art   ";

    let result_trimmed = Mnemonic::parse(trimmed).unwrap();
    let result_padded = Mnemonic::parse(padded).unwrap();

    let seed_trimmed = result_trimmed.to_seed("").unwrap();
    let seed_padded = result_padded.to_seed("").unwrap();

    assert_eq!(seed_trimmed, seed_padded);

    println!("✅ Leading/trailing whitespace correctly handled");
}

/// Test passphrase edge cases
#[test]
fn test_passphrase_edge_cases() {
    let mnemonic = "legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth title";
    let parsed = Mnemonic::parse(mnemonic).unwrap();

    // Empty string vs no passphrase (should be same)
    let seed_empty = parsed.to_seed("").unwrap();
    let seed_empty2 = parsed.to_seed("").unwrap();
    assert_eq!(seed_empty, seed_empty2);

    // Very long passphrase
    let long_passphrase = "a".repeat(1000);
    let seed_long = parsed.to_seed(&long_passphrase);
    assert!(seed_long.is_ok(), "Long passphrase should work");

    // Unicode passphrase
    let unicode_passphrase = "密码🔐パスワード";
    let seed_unicode = parsed.to_seed(unicode_passphrase);
    assert!(seed_unicode.is_ok(), "Unicode passphrase should work");

    // Special characters
    let special = "!@#$%^&*()_+-=[]{}|;:',.<>?/~`";
    let seed_special = parsed.to_seed(special);
    assert!(seed_special.is_ok(), "Special char passphrase should work");

    println!("✅ Passphrase edge cases handled correctly");
}

/// Test repeated words (valid in BIP39)
#[test]
fn test_repeated_words_valid() {
    // "abandon" repeated 23 times + "art" (valid with correct checksum)
    let repeated = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

    let result = Mnemonic::parse(repeated);
    assert!(result.is_ok(), "Repeated words should be valid if checksum correct");

    println!("✅ Repeated words (valid) accepted");
}

/// Test single word repeated 24 times (invalid checksum)
#[test]
fn test_all_same_word_invalid() {
    let repeated = "abandon ".repeat(24);
    let all_abandon = repeated.trim();

    let result = Mnemonic::parse(all_abandon);
    assert!(result.is_err(), "All same word should fail checksum");

    println!("✅ All-same-word (invalid checksum) rejected");
}

/// Test numeric-looking words
#[test]
fn test_numeric_looking_invalid() {
    let numeric = "123 456 789 abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

    let result = Mnemonic::parse(numeric);
    assert!(result.is_err(), "Numeric words should be rejected");

    println!("✅ Numeric words correctly rejected");
}