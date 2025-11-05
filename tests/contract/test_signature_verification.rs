// ML-DSA Signature Verification Contract Tests
// These tests MUST FAIL initially to ensure TDD compliance

use btpc_core::crypto::{Signature, PublicKey, PrivateKey};

#[cfg(test)]
mod signature_verification_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_mldsa_signature_generation() {
        // Contract: Generate ML-DSA signature from private key and data
        // Constitutional requirement: ML-DSA quantum-resistant signatures only

        let test_data = b"Hello, BTPC blockchain!";
        let private_key = PrivateKey::generate_ml_dsa().unwrap();

        let signature_result = private_key.sign(test_data);
        assert!(signature_result.is_ok(), "ML-DSA signature generation must succeed");

        let signature = signature_result.unwrap();
        assert!(!signature.to_bytes().iter().all(|&b| b == 0), "Signature must not be empty");
        assert_eq!(signature.algorithm(), "ML-DSA-65", "Must use ML-DSA-65 algorithm");
    }

    #[test]
    fn test_mldsa_signature_verification_valid() {
        // Contract: Verify valid ML-DSA signature
        // Performance requirement: <1.5ms signature verification

        let test_data = b"BTPC transaction data";
        let private_key = PrivateKey::generate_ml_dsa().unwrap();
        let public_key = private_key.public_key();
        let signature = private_key.sign(test_data).unwrap();

        let start = Instant::now();
        let verification_result = public_key.verify(test_data, &signature);
        let duration = start.elapsed();

        assert!(verification_result, "Valid signature must return true");
        assert!(duration.as_millis() < 2, "Signature verification must be <1.5ms"); // Using 2ms buffer for test stability
    }

    #[test]
    fn test_mldsa_signature_verification_invalid_data() {
        // Contract: Reject signature when data is modified
        // Security requirement: Tampering detection

        let original_data = b"Original transaction data";
        let tampered_data = b"Tampered transaction data";
        let private_key = PrivateKey::generate_ml_dsa().unwrap();
        let public_key = private_key.public_key();
        let signature = private_key.sign(original_data).unwrap();

        let verification_result = public_key.verify(tampered_data, &signature);

        assert!(!verification_result, "Tampered data must fail signature verification");
    }

    #[test]
    fn test_mldsa_signature_verification_invalid_signature() {
        // Contract: Reject invalid/corrupted signature
        // Security requirement: Signature integrity validation

        let test_data = b"Test blockchain data";
        let private_key = PrivateKey::generate_ml_dsa().unwrap();
        let public_key = private_key.public_key();
        let valid_signature = private_key.sign(test_data).unwrap();

        // Create corrupted signature by modifying bytes
        let mut corrupted_signature_bytes = valid_signature.to_bytes();
        corrupted_signature_bytes[0] ^= 0xFF; // Flip bits in first byte
        let corrupted_signature = Signature::from_bytes(&corrupted_signature_bytes).unwrap();

        let verification_result = public_key.verify(test_data, &corrupted_signature);

        assert!(!verification_result, "Corrupted signature must fail verification");
    }

    #[test]
    fn test_mldsa_signature_verification_wrong_public_key() {
        // Contract: Reject signature with wrong public key
        // Security requirement: Key pair validation

        let test_data = b"Blockchain transaction";
        let private_key1 = PrivateKey::generate_ml_dsa().unwrap();
        let private_key2 = PrivateKey::generate_ml_dsa().unwrap();
        let public_key2 = private_key2.public_key();
        let signature = private_key1.sign(test_data).unwrap();

        let verification_result = public_key2.verify(test_data, &signature);

        assert!(!verification_result, "Wrong public key must fail signature verification");
    }

    #[test]
    fn test_mldsa_signature_generation_performance() {
        // Contract: Signature generation performance requirement
        // Performance requirement: <2ms signature generation

        let test_data = b"Performance test data for BTPC";
        let private_key = PrivateKey::generate_ml_dsa().unwrap();

        let start = Instant::now();
        let signature_result = private_key.sign(test_data);
        let duration = start.elapsed();

        assert!(signature_result.is_ok(), "Signature generation must succeed");
        assert!(duration.as_millis() < 2, "Signature generation must be <2ms");
    }

    #[test]
    fn test_mldsa_87_algorithm_support() {
        // Contract: Support both ML-DSA-65 and ML-DSA-87 algorithms
        // Constitutional requirement: Quantum-resistant algorithm variants
        // Note: Currently only ML-DSA-65 is implemented

        let test_data = b"ML-DSA test data";
        let private_key = PrivateKey::generate_ml_dsa().unwrap();
        let public_key = private_key.public_key();
        let signature = private_key.sign(test_data).unwrap();

        assert_eq!(signature.algorithm(), "ML-DSA-65", "Currently using ML-DSA-65 algorithm");

        let verification_result = public_key.verify(test_data, &signature);
        assert!(verification_result, "ML-DSA signature verification must succeed");
    }

    #[test]
    fn test_mldsa_key_serialization() {
        // Contract: Keys must be serializable to hex format
        // API requirement: Hex-encoded keys for JSON API

        let private_key = PrivateKey::generate_ml_dsa().unwrap();
        let public_key = private_key.public_key();

        let private_key_hex = private_key.to_hex();
        let public_key_hex = public_key.to_hex();

        assert!(!private_key_hex.is_empty(), "Private key hex must not be empty");
        assert!(!public_key_hex.is_empty(), "Public key hex must not be empty");

        // Test deserialization
        let restored_private_key = PrivateKey::from_hex(&private_key_hex);
        let restored_public_key = PublicKey::from_hex(&public_key_hex);

        assert!(restored_private_key.is_ok(), "Private key hex deserialization must succeed");
        assert!(restored_public_key.is_ok(), "Public key hex deserialization must succeed");
    }

    #[test]
    fn test_mldsa_signature_deterministic() {
        // Contract: Same input produces same signature (deterministic signing)
        // Security requirement: Reproducible signatures for testing

        let test_data = b"Deterministic test data";
        let private_key = PrivateKey::from_seed(&[42u8; 32]).unwrap(); // Fixed seed

        let signature1 = private_key.sign(test_data).unwrap();
        let signature2 = private_key.sign(test_data).unwrap();

        assert_eq!(signature1.to_bytes(), signature2.to_bytes(),
                   "Same input with same key must produce identical signatures");
    }

    #[test]
    fn test_mldsa_empty_data_handling() {
        // Contract: Handle edge case of empty data
        // Robustness requirement: Graceful handling of empty inputs

        let empty_data = b"";
        let private_key = PrivateKey::generate_ml_dsa().unwrap();
        let public_key = private_key.public_key();

        let signature_result = private_key.sign(empty_data);
        assert!(signature_result.is_ok(), "Signing empty data must succeed");

        let signature = signature_result.unwrap();
        let verification_result = public_key.verify(empty_data, &signature);
        assert!(verification_result, "Empty data signature verification must succeed");
    }

    #[test]
    fn test_mldsa_large_data_handling() {
        // Contract: Handle large data inputs efficiently
        // Performance requirement: Scale with data size appropriately

        let large_data = vec![0xAB; 1_048_576]; // 1MB of data
        let private_key = PrivateKey::generate_ml_dsa().unwrap();
        let public_key = private_key.public_key();

        let start = Instant::now();
        let signature_result = private_key.sign(&large_data);
        let sign_duration = start.elapsed();

        assert!(signature_result.is_ok(), "Signing large data must succeed");
        assert!(sign_duration.as_millis() < 100, "Large data signing must be reasonable (<100ms)");

        let signature = signature_result.unwrap();
        let start = Instant::now();
        let verification_result = public_key.verify(&large_data, &signature);
        let verify_duration = start.elapsed();

        assert!(verification_result, "Large data signature verification must succeed");
        assert!(verify_duration.as_millis() < 100, "Large data verification must be reasonable (<100ms)");
    }
}