//! RED Phase Test: Deterministic ML-DSA key generation from seeds
//!
//! These tests MUST FAIL initially - they define the required behavior
//! for deterministic key generation (FR-001).

use btpc_core::crypto::keys::PrivateKey;

#[test]
fn test_same_seed_produces_identical_keys() {
    let seed = [42u8; 32];

    // Generate keys twice from same seed
    let key1 = PrivateKey::from_seed_deterministic(&seed).unwrap();
    let key2 = PrivateKey::from_seed_deterministic(&seed).unwrap();

    // MUST be byte-identical (FR-001: same seed → same keys, always)
    assert_eq!(
        key1.to_bytes(),
        key2.to_bytes(),
        "Private keys must be identical for same seed"
    );

    assert_eq!(
        key1.public_key().to_bytes(),
        key2.public_key().to_bytes(),
        "Public keys must be identical for same seed"
    );
}

#[test]
fn test_different_seeds_produce_different_keys() {
    let seed_a = [1u8; 32];
    let seed_b = [2u8; 32];

    let key_a = PrivateKey::from_seed_deterministic(&seed_a).unwrap();
    let key_b = PrivateKey::from_seed_deterministic(&seed_b).unwrap();

    assert_ne!(
        key_a.to_bytes(),
        key_b.to_bytes(),
        "Different seeds must produce different keys"
    );

    assert_ne!(
        key_a.public_key().to_bytes(),
        key_b.public_key().to_bytes(),
        "Different seeds must produce different public keys"
    );
}

#[test]
fn test_deterministic_generation_is_truly_deterministic() {
    // Test with various seeds to ensure determinism across different inputs
    // (excluding all-zeros which is rejected for security)
    let test_seeds = [
        [1u8; 32],
        [255u8; 32],
        [42u8; 32],
        [170u8; 32], // 0xAA pattern
    ];

    for seed in test_seeds.iter() {
        let key1 = PrivateKey::from_seed_deterministic(seed).unwrap();
        let key2 = PrivateKey::from_seed_deterministic(seed).unwrap();

        assert_eq!(
            key1.to_bytes(),
            key2.to_bytes(),
            "Keys must be identical for seed pattern: {:?}",
            &seed[0..4]
        );
    }
}

#[test]
fn test_key_size_matches_ml_dsa_spec() {
    let seed = [42u8; 32];
    let key = PrivateKey::from_seed_deterministic(&seed).unwrap();

    // ML-DSA (Dilithium3/5) key sizes from research.md
    assert_eq!(
        key.to_bytes().len(),
        4000,
        "Private key must be 4000 bytes (Dilithium3)"
    );

    assert_eq!(
        key.public_key().to_bytes().len(),
        1952,
        "Public key must be 1952 bytes (Dilithium3)"
    );
}

#[test]
#[should_panic(expected = "all zeros")]
fn test_rejects_all_zero_seed() {
    let seed = [0u8; 32];

    // Should reject all-zero seed (security requirement)
    PrivateKey::from_seed_deterministic(&seed).unwrap();
}