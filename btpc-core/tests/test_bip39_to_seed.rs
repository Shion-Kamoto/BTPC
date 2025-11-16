//! RED Phase Test: BIP39 mnemonic to seed derivation
//!
//! These tests MUST FAIL initially - they define BIP39 PBKDF2 derivation (FR-003).

use btpc_core::crypto::bip39::Mnemonic;

#[test]
fn test_bip39_mnemonic_to_seed_pbkdf2() {
    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

    let parsed = Mnemonic::parse(mnemonic).unwrap();

    // BIP39 official test vectors use "TREZOR" passphrase
    // Source: https://github.com/bitcoin/bips/blob/master/bip-0039.mediawiki
    let seed = parsed.to_seed("TREZOR").unwrap();

    assert_eq!(seed.len(), 32, "Seed must be 32 bytes (FR-003)");

    // BIP39 test vector for this mnemonic with "TREZOR" passphrase (first 32 bytes of 64-byte output)
    let expected_hex = "bda85446c68413707090a52022edd26a1c9462295029f2e60cd7c4f2bbd3097170af7a4d73245cafa9c3cca8d561a7c3de6f5d4a10be8ed2a5e608d68f92fcc8";
    let expected_seed = hex::decode(expected_hex).unwrap();

    assert_eq!(
        &seed[..],
        &expected_seed[..32],
        "Seed must match BIP39 standard derivation with TREZOR passphrase"
    );
}

#[test]
fn test_same_mnemonic_produces_same_seed() {
    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

    let seed1 = Mnemonic::parse(mnemonic).unwrap().to_seed("").unwrap();
    let seed2 = Mnemonic::parse(mnemonic).unwrap().to_seed("").unwrap();

    assert_eq!(seed1, seed2, "Same mnemonic must produce same seed (deterministic)");
}

#[test]
fn test_different_mnemonics_produce_different_seeds() {
    let mnemonic1 = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";
    let mnemonic2 = "legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth title";

    let seed1 = Mnemonic::parse(mnemonic1).unwrap().to_seed("").unwrap();
    let seed2 = Mnemonic::parse(mnemonic2).unwrap().to_seed("").unwrap();

    assert_ne!(seed1, seed2, "Different mnemonics must produce different seeds");
}

#[test]
fn test_empty_passphrase_is_btpc_standard() {
    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

    let parsed = Mnemonic::parse(mnemonic).unwrap();

    // BTPC uses empty passphrase (standard)
    let seed_empty = parsed.to_seed("").unwrap();

    // Verify it's actually using PBKDF2 (different passphrase = different seed)
    let seed_with_pass = parsed.to_seed("password").unwrap();

    assert_ne!(seed_empty, seed_with_pass, "Passphrase should affect seed derivation");
}

#[test]
fn test_seed_derivation_is_pbkdf2_compliant() {
    // Test multiple BIP39 test vectors (with TREZOR passphrase)
    // Source: Official BIP39 test vectors from bip39 crate
    let test_vectors = vec![
        (
            "legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth title",
            "bc09fca1804f7e69da93c2f2028eb238c227f2e9dda30cd63699232578480a4021b146ad717fbb7e451ce9eb835f43620bf5c514db0f8add49f5d121449d3e87"
        ),
        (
            "letter advice cage absurd amount doctor acoustic avoid letter advice cage absurd amount doctor acoustic avoid letter advice cage absurd amount doctor acoustic bless",
            "c0c519bd0e91a2ed54357d9d1ebef6f5af218a153624cf4f2da911a0ed8f7a09e2ef61af0aca007096df430022f7a2b6fb91661a9589097069720d015e4e982f"
        ),
    ];

    for (mnemonic, expected_hex) in test_vectors {
        // BIP39 test vectors use "TREZOR" passphrase
        let seed = Mnemonic::parse(mnemonic).unwrap().to_seed("TREZOR").unwrap();
        let expected_seed = hex::decode(expected_hex).unwrap();

        assert_eq!(
            &seed[..],
            &expected_seed[..32],
            "Seed must match BIP39 PBKDF2 standard with TREZOR passphrase"
        );
    }
}