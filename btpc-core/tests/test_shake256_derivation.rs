//! RED Phase Test: SHAKE256 seed expansion for ML-DSA
//!
//! These tests MUST FAIL initially - they define the required behavior
//! for deterministic seed expansion (FR-002, NFR-002).

use btpc_core::crypto::shake256_derivation::{expand_seed_to_ml_dsa, expand_seed_to_ml_dsa_with_tag};

#[test]
fn test_deterministic_shake256_expansion() {
    let seed = [42u8; 32];

    // Expand seed twice
    let expanded1 = expand_seed_to_ml_dsa(&seed).unwrap();
    let expanded2 = expand_seed_to_ml_dsa(&seed).unwrap();

    // Must be deterministic (same seed → same output)
    assert_eq!(expanded1.len(), 48, "ML-DSA requires 48-byte seed");
    assert_eq!(expanded2.len(), 48, "ML-DSA requires 48-byte seed");
    assert_eq!(
        expanded1, expanded2,
        "SHAKE256 expansion must be deterministic"
    );
}

#[test]
fn test_different_seeds_produce_different_expansions() {
    let seed_a = [1u8; 32];
    let seed_b = [2u8; 32];

    let expanded_a = expand_seed_to_ml_dsa(&seed_a).unwrap();
    let expanded_b = expand_seed_to_ml_dsa(&seed_b).unwrap();

    assert_ne!(
        expanded_a, expanded_b,
        "Different seeds must produce different expansions"
    );
}

#[test]
fn test_domain_separation() {
    let seed = [42u8; 32];

    // Expansion with default tag "BTPC-ML-DSA-v1"
    let expanded_default = expand_seed_to_ml_dsa(&seed).unwrap();

    // Expansion with different tag
    let expanded_different_tag = expand_seed_to_ml_dsa_with_tag(&seed, b"BTPC-TEST-v1").unwrap();

    // Domain separation means different tags produce different outputs (NFR-002)
    assert_ne!(
        expanded_default, expanded_different_tag,
        "Domain separation must produce different outputs for different tags"
    );
}

#[test]
fn test_shake256_preserves_entropy() {
    // Test with various seed patterns (excluding all-zeros which is rejected for security)
    let test_seeds = vec![
        [255u8; 32],  // All ones
        [42u8; 32],   // Constant value
        [0xAAu8; 32], // Alternating pattern
        [1u8; 32],    // Minimal non-zero
    ];

    for seed in test_seeds.iter() {
        let expanded = expand_seed_to_ml_dsa(seed).unwrap();

        // Verify output is not all zeros (entropy preserved)
        assert!(
            expanded.iter().any(|&b| b != 0),
            "SHAKE256 must preserve entropy - output should not be all zeros for seed: {:?}",
            &seed[0..4]
        );

        // Verify output has reasonable distribution (simple check)
        let zero_count = expanded.iter().filter(|&&b| b == 0).count();
        assert!(
            zero_count < expanded.len(),
            "Output should have reasonable byte distribution"
        );
    }
}

#[test]
fn test_output_length_exactly_48_bytes() {
    let seed = [42u8; 32];
    let expanded = expand_seed_to_ml_dsa(&seed).unwrap();

    assert_eq!(
        expanded.len(),
        48,
        "ML-DSA seed expansion must produce exactly 48 bytes"
    );
}

#[test]
fn test_shake256_is_not_sha256() {
    // SHAKE256 is XOF (extendable output function), not a fixed hash
    // This test verifies we're using SHAKE256, not SHA-256
    let seed = [42u8; 32];
    let expanded = expand_seed_to_ml_dsa(&seed).unwrap();

    // If this were SHA-256, output would be 32 bytes, not 48
    assert_eq!(expanded.len(), 48, "Must use SHAKE256 XOF, not SHA-256");
}

#[test]
fn test_default_domain_tag_is_btpc_ml_dsa_v1() {
    let seed = [42u8; 32];

    // Using default tag
    let expanded_default = expand_seed_to_ml_dsa(&seed).unwrap();

    // Using explicit "BTPC-ML-DSA-v1" tag
    let expanded_explicit = expand_seed_to_ml_dsa_with_tag(&seed, b"BTPC-ML-DSA-v1").unwrap();

    // Should be identical
    assert_eq!(
        expanded_default, expanded_explicit,
        "Default tag must be 'BTPC-ML-DSA-v1'"
    );
}

#[test]
fn test_multiple_expansions_are_independent() {
    // Expanding multiple times should not affect each other
    let seed1 = [1u8; 32];
    let seed2 = [2u8; 32];

    let exp1_first = expand_seed_to_ml_dsa(&seed1).unwrap();
    let exp2_first = expand_seed_to_ml_dsa(&seed2).unwrap();

    // Expand seed1 again after seed2
    let exp1_second = expand_seed_to_ml_dsa(&seed1).unwrap();

    // Should be identical to first expansion (no state pollution)
    assert_eq!(
        exp1_first, exp1_second,
        "Multiple expansions must be independent (no state)"
    );

    assert_ne!(exp1_first, exp2_first, "Different seeds produce different outputs");
}

#[test]
fn test_rejects_all_zero_seed() {
    let seed = [0u8; 32];

    // Should reject all-zero seed for security
    let result = expand_seed_to_ml_dsa(&seed);

    assert!(result.is_err(), "All-zero seed must be rejected");
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("all zeros") || err_msg.contains("security"), "Error should mention security risk");
}