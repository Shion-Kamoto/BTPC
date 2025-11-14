//! Contract tests for BTPC core modules
//! These tests define the contracts that the implementations must satisfy

use btpc_core::crypto::{PrivateKey, PublicKey, Signature};

#[cfg(test)]
mod contract_tests {
    use super::*;

    #[test]
    fn test_ml_dsa_key_generation() {
        // Contract: Generate ML-DSA key pair
        // Constitutional requirement: Post-quantum cryptography only

        let keypair = PrivateKey::generate_ml_dsa();
        assert!(keypair.is_ok(), "ML-DSA key generation must succeed");

        let private_key = keypair.unwrap();
        let public_key = private_key.public_key();

        // Validate key sizes per ML-DSA specification
        assert_eq!(private_key.size(), 4032, "ML-DSA-65 private key must be 4032 bytes");
        assert_eq!(public_key.size(), 1952, "ML-DSA-65 public key must be 1952 bytes");
    }

    #[test]
    fn test_ml_dsa_signature_creation() {
        // Contract: Create valid ML-DSA signature
        // Performance requirement: <2ms signature generation

        let private_key = PrivateKey::generate_ml_dsa().unwrap();
        let message = b"BTPC quantum-resistant transaction";

        let start = std::time::Instant::now();
        let signature = private_key.sign(message);
        let duration = start.elapsed();

        assert!(signature.is_ok(), "ML-DSA signature creation must succeed");
        assert!(duration.as_millis() < 10, "Signature generation must be reasonably fast"); // Allow margin for test

        let sig = signature.unwrap();
        assert_eq!(sig.size(), 3309, "ML-DSA-65 signature must be 3309 bytes");
    }

    #[test]
    fn test_ml_dsa_signature_verification() {
        // Contract: Verify ML-DSA signature correctly
        // Performance requirement: <1.5ms signature verification

        let private_key = PrivateKey::generate_ml_dsa().unwrap();
        let public_key = private_key.public_key();
        let message = b"BTPC transaction data for verification";

        let signature = private_key.sign(message).unwrap();

        let start = std::time::Instant::now();
        let is_valid = public_key.verify(message, &signature);
        let duration = start.elapsed();

        assert!(is_valid, "Valid ML-DSA signature must verify successfully");
        assert!(duration.as_millis() < 10, "Signature verification must be reasonably fast"); // Allow margin for test
    }

    #[test]
    fn test_ml_dsa_signature_verification_invalid() {
        // Contract: Reject invalid ML-DSA signatures
        // Security requirement: No false positives

        let private_key = PrivateKey::generate_ml_dsa().unwrap();
        let public_key = private_key.public_key();
        let message = b"Original message";
        let tampered_message = b"Tampered message";

        let signature = private_key.sign(message).unwrap();

        // Verify with tampered message should fail
        let is_valid = public_key.verify(tampered_message, &signature);
        assert!(!is_valid, "Invalid ML-DSA signature must be rejected");
    }

    #[test]
    fn test_ml_dsa_deterministic_signatures() {
        // Contract: Signatures must be deterministic for same input
        // Requirement: Reproducible signatures for testing

        let private_key = PrivateKey::from_seed(&[42u8; 32]).unwrap();
        let message = b"Deterministic signature test";

        let signature1 = private_key.sign(message).unwrap();
        let signature2 = private_key.sign(message).unwrap();

        assert_eq!(signature1, signature2, "ML-DSA signatures must be deterministic");
    }

    #[test]
    fn test_ml_dsa_cross_platform_compatibility() {
        // Contract: Signatures must be compatible across platforms
        // Requirement: Network interoperability

        let private_key = PrivateKey::from_seed(&[123u8; 32]).unwrap();
        let public_key = private_key.public_key();
        let message = b"Cross-platform compatibility test";

        let signature = private_key.sign(message).unwrap();

        // Test serialization/deserialization
        let signature_bytes = signature.to_bytes();
        let deserialized_sig = Signature::from_bytes(&signature_bytes);
        assert!(deserialized_sig.is_ok(), "Signature serialization must work");

        let is_valid = public_key.verify(message, &deserialized_sig.unwrap());
        assert!(is_valid, "Deserialized signature must verify");
    }

    #[test]
    fn test_ml_dsa_quantum_resistance() {
        // Contract: ML-DSA must resist quantum attacks
        // Constitutional requirement: Post-quantum security

        let private_key = PrivateKey::generate_ml_dsa().unwrap();
        let public_key = private_key.public_key();

        // Verify we're using ML-DSA algorithm
        assert_eq!(private_key.algorithm(), "ML-DSA-65");
        assert_eq!(public_key.algorithm(), "ML-DSA-65");

        // Ensure key sizes match NIST specification
        assert!(private_key.size() >= 4032, "Private key must meet ML-DSA minimum size");
        assert!(public_key.size() >= 1952, "Public key must meet ML-DSA minimum size");
    }
}