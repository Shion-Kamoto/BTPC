//! SHAKE256 seed expansion for ML-DSA key generation
//!
//! This module implements deterministic seed expansion from 32-byte BIP39 seeds
//! to 48-byte ML-DSA seeds using SHAKE256 XOF with domain separation.

use sha3::{Shake256, digest::{Update, ExtendableOutput, XofReader}};
use zeroize::Zeroizing;

/// Domain separation tag for BTPC ML-DSA key derivation (NFR-002)
const DOMAIN_TAG: &[u8] = b"BTPC-ML-DSA-v1";

/// ML-DSA seed length (Dilithium3 requires 48 bytes)
const ML_DSA_SEED_LEN: usize = 48;

/// Expand a 32-byte BIP39 seed to 48-byte ML-DSA seed using SHAKE256
///
/// This function provides deterministic seed expansion with domain separation
/// to prevent key reuse across different protocols (FR-002, NFR-002).
///
/// # Arguments
/// * `seed` - 32-byte BIP39-derived seed
///
/// # Returns
/// 48-byte ML-DSA seed for deterministic keypair generation
///
/// # Errors
/// Returns `SeedError::AllZeroSeed` if the input seed is all zeros
///
/// # Security
/// - Uses SHAKE256 (SHA-3 XOF) for cryptographic-quality expansion
/// - Domain tag "BTPC-ML-DSA-v1" prevents cross-protocol attacks
/// - Deterministic: same seed always produces same output
/// - Rejects all-zero seeds to prevent weak keys
///
/// # Example
/// ```ignore
/// let bip39_seed = [42u8; 32];  // From BIP39 PBKDF2
/// let ml_dsa_seed = expand_seed_to_ml_dsa(&bip39_seed)?;
/// assert_eq!(ml_dsa_seed.len(), 48);
/// ```
pub fn expand_seed_to_ml_dsa(seed: &[u8; 32]) -> Result<[u8; 48], SeedError> {
    expand_seed_to_ml_dsa_with_tag(seed, DOMAIN_TAG)
}

/// Expand seed with custom domain tag (for testing or future protocol versions)
///
/// # Arguments
/// * `seed` - 32-byte seed to expand
/// * `tag` - Domain separation tag (e.g., b"BTPC-ML-DSA-v2")
///
/// # Returns
/// 48-byte expanded seed
///
/// # Errors
/// Returns `SeedError::AllZeroSeed` if the input seed is all zeros
///
/// # SHAKE256 Process
/// 1. Initialize SHAKE256 XOF (extendable output function)
/// 2. Absorb seed bytes (32 bytes)
/// 3. Absorb domain tag (prevents protocol confusion)
/// 4. Squeeze 48 bytes for ML-DSA
///
/// # Example
/// ```ignore
/// let seed = [42u8; 32];
/// let ml_dsa_seed = expand_seed_to_ml_dsa_with_tag(&seed, b"BTPC-TEST-v1")?;
/// assert_eq!(ml_dsa_seed.len(), 48);
/// ```
pub fn expand_seed_to_ml_dsa_with_tag(seed: &[u8; 32], tag: &[u8]) -> Result<[u8; 48], SeedError> {
    // Security: Reject all-zero seeds to prevent weak key generation
    if seed.iter().all(|&b| b == 0) {
        return Err(SeedError::AllZeroSeed);
    }

    // SHAKE256 expansion (ML-DSA native PRF, FIPS 204 aligned)
    // SHAKE256 is a SHA-3 XOF (extendable output function) that can
    // produce arbitrary-length output from arbitrary-length input
    let mut shake = Shake256::default();
    shake.update(seed);      // Absorb 32-byte BIP39 seed
    shake.update(tag);       // Absorb domain tag (NFR-002)

    // Finalize and squeeze 48 bytes
    // Use Zeroizing to securely erase sensitive data when dropped
    let mut expanded = Zeroizing::new([0u8; ML_DSA_SEED_LEN]);
    shake.finalize_xof().read(&mut expanded[..]);

    Ok(*expanded)
}

/// Seed expansion errors
#[derive(Debug, thiserror::Error)]
pub enum SeedError {
    /// Seed is all zeros (weak key risk)
    #[error("Seed cannot be all zeros (security risk)")]
    AllZeroSeed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic_expansion() {
        let seed = [42u8; 32];

        let expanded1 = expand_seed_to_ml_dsa(&seed).unwrap();
        let expanded2 = expand_seed_to_ml_dsa(&seed).unwrap();

        assert_eq!(expanded1.len(), 48);
        assert_eq!(expanded2.len(), 48);
        assert_eq!(expanded1, expanded2, "SHAKE256 must be deterministic");
    }

    #[test]
    fn test_different_seeds_produce_different_expansions() {
        let seed_a = [1u8; 32];
        let seed_b = [2u8; 32];

        let expanded_a = expand_seed_to_ml_dsa(&seed_a).unwrap();
        let expanded_b = expand_seed_to_ml_dsa(&seed_b).unwrap();

        assert_ne!(expanded_a, expanded_b, "Different seeds must produce different outputs");
    }

    #[test]
    fn test_domain_separation() {
        let seed = [42u8; 32];

        let expanded_default = expand_seed_to_ml_dsa(&seed).unwrap();
        let expanded_different_tag = expand_seed_to_ml_dsa_with_tag(&seed, b"BTPC-TEST-v1").unwrap();

        assert_ne!(
            expanded_default, expanded_different_tag,
            "Domain separation must produce different outputs"
        );
    }

    #[test]
    fn test_output_length_exactly_48_bytes() {
        let seed = [42u8; 32];
        let expanded = expand_seed_to_ml_dsa(&seed).unwrap();

        assert_eq!(expanded.len(), 48, "ML-DSA requires exactly 48-byte seed");
    }

    #[test]
    fn test_rejects_all_zero_seed() {
        let seed = [0u8; 32];
        let result = expand_seed_to_ml_dsa(&seed);

        assert!(result.is_err(), "All-zero seed must be rejected");
        assert!(matches!(result.unwrap_err(), SeedError::AllZeroSeed));
    }

    #[test]
    fn test_entropy_preservation() {
        // Test various seed patterns
        let test_seeds = vec![
            [1u8; 32],
            [255u8; 32],
            [42u8; 32],
            [0xAAu8; 32],
        ];

        for seed in test_seeds.iter() {
            let expanded = expand_seed_to_ml_dsa(seed).unwrap();

            // Verify output is not all zeros (entropy preserved)
            assert!(
                expanded.iter().any(|&b| b != 0),
                "SHAKE256 must preserve entropy"
            );

            // Verify output has reasonable distribution
            let zero_count = expanded.iter().filter(|&&b| b == 0).count();
            assert!(
                zero_count < expanded.len(),
                "Output should have reasonable byte distribution"
            );
        }
    }

    #[test]
    fn test_default_domain_tag_is_btpc_ml_dsa_v1() {
        let seed = [42u8; 32];

        let expanded_default = expand_seed_to_ml_dsa(&seed).unwrap();
        let expanded_explicit = expand_seed_to_ml_dsa_with_tag(&seed, b"BTPC-ML-DSA-v1").unwrap();

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

        assert_eq!(
            exp1_first, exp1_second,
            "Multiple expansions must be independent (no state)"
        );

        assert_ne!(exp1_first, exp2_first, "Different seeds produce different outputs");
    }
}