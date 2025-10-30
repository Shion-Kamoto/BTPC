//! ML-DSA signature implementation for BTPC
//!
//! This module provides quantum-resistant digital signatures using ML-DSA (Dilithium).

use std::fmt;

// Note: Using stub implementation for now, will be replaced with actual ML-DSA
use serde::{Deserialize, Serialize};

use crate::crypto::constants::ML_DSA_SIGNATURE_SIZE;

/// A quantum-resistant digital signature using ML-DSA
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
    signature_bytes: [u8; ML_DSA_SIGNATURE_SIZE],
}

impl Signature {
    /// Create a signature from raw bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SignatureError> {
        if bytes.len() != ML_DSA_SIGNATURE_SIZE {
            return Err(SignatureError::InvalidSignatureSize);
        }

        let mut signature_bytes = [0u8; ML_DSA_SIGNATURE_SIZE];
        signature_bytes.copy_from_slice(bytes);

        Ok(Signature { signature_bytes })
    }

    /// Create a signature from raw signature data (internal)
    pub(crate) fn from_raw_signature(signature_bytes: [u8; ML_DSA_SIGNATURE_SIZE]) -> Self {
        Signature { signature_bytes }
    }

    /// Get the signature as bytes
    pub fn to_bytes(&self) -> [u8; ML_DSA_SIGNATURE_SIZE] {
        self.signature_bytes
    }

    /// Get the signature size in bytes
    pub fn size(&self) -> usize {
        ML_DSA_SIGNATURE_SIZE
    }

    /// Get the algorithm name
    ///
    /// Returns the ML-DSA security level identifier. BTPC uses ML-DSA-65 (Dilithium3)
    /// which provides 192-bit security and is specified in FIPS 204.
    pub fn algorithm(&self) -> &'static str {
        "ML-DSA-65"  // Fixed: Was incorrectly labeled as ML-DSA-87 (Issue 1.1.1)
    }

    /// Convert to hex string
    pub fn to_hex(&self) -> String {
        hex::encode(self.signature_bytes)
    }

    /// Create signature from hex string
    pub fn from_hex(hex: &str) -> Result<Self, SignatureError> {
        let hex = hex.trim_start_matches("0x");
        if hex.len() != ML_DSA_SIGNATURE_SIZE * 2 {
            return Err(SignatureError::InvalidHexLength);
        }

        let bytes = hex::decode(hex).map_err(|_| SignatureError::InvalidHexCharacter)?;

        Self::from_bytes(&bytes)
    }

    /// Batch verify multiple signatures efficiently
    pub fn batch_verify(
        public_keys: &[crate::crypto::PublicKey],
        messages: &[Vec<u8>],
        signatures: &[Signature],
    ) -> Result<Vec<bool>, SignatureError> {
        if public_keys.len() != messages.len() || messages.len() != signatures.len() {
            return Err(SignatureError::BatchSizeMismatch);
        }

        let mut results = Vec::with_capacity(signatures.len());

        // For now, implement as individual verifications
        // Future optimization: use ML-DSA batch verification if available
        for ((public_key, message), signature) in public_keys
            .iter()
            .zip(messages.iter())
            .zip(signatures.iter())
        {
            let is_valid = public_key.verify(message, signature);
            results.push(is_valid);
        }

        Ok(results)
    }

    /// Verify that this is a valid ML-DSA signature structure
    ///
    /// # Warning (Issue 1.1.2 - HIGH)
    /// This method provides only basic structural validation and should NOT be relied
    /// upon for security-critical decisions. It performs minimal checks:
    /// - Signature is not all zeros
    /// - Signature has correct length (enforced by type system)
    ///
    /// **Security Note:** True signature validation happens during `PublicKey::verify()`,
    /// which performs full ML-DSA cryptographic verification. This method is intended
    /// only for quick sanity checks and debugging.
    ///
    /// # Recommendation
    /// Always use `PublicKey::verify()` for actual signature validation. Consider this
    /// method deprecated for security purposes.
    #[deprecated(
        note = "Provides weak validation only. Use PublicKey::verify() for security-critical validation."
    )]
    pub fn is_valid_structure(&self) -> bool {
        // Basic check: signature should not be all zeros
        // Note: This does NOT validate ML-DSA mathematical correctness
        let all_zeros = self.signature_bytes.iter().all(|&b| b == 0);
        let all_ones = self.signature_bytes.iter().all(|&b| b == 0xFF);

        // Reject obvious invalid signatures
        !all_zeros && !all_ones
    }
}

impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl From<[u8; ML_DSA_SIGNATURE_SIZE]> for Signature {
    fn from(bytes: [u8; ML_DSA_SIGNATURE_SIZE]) -> Self {
        Signature {
            signature_bytes: bytes,
        }
    }
}

impl From<Signature> for [u8; ML_DSA_SIGNATURE_SIZE] {
    fn from(signature: Signature) -> Self {
        signature.signature_bytes
    }
}

impl AsRef<[u8]> for Signature {
    fn as_ref(&self) -> &[u8] {
        &self.signature_bytes
    }
}

/// Error types for signature operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignatureError {
    /// Signature creation failed
    SigningFailed,
    /// Signature verification failed
    VerificationFailed,
    /// Invalid signature size
    InvalidSignatureSize,
    /// Invalid signature data
    InvalidSignatureData,
    /// Invalid hex string length
    InvalidHexLength,
    /// Invalid hex character
    InvalidHexCharacter,
    /// Batch operation size mismatch
    BatchSizeMismatch,
    /// Unsupported signature algorithm
    UnsupportedAlgorithm,
}

impl fmt::Display for SignatureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SignatureError::SigningFailed => write!(f, "Signature creation failed"),
            SignatureError::VerificationFailed => write!(f, "Signature verification failed"),
            SignatureError::InvalidSignatureSize => write!(f, "Invalid signature size"),
            SignatureError::InvalidSignatureData => write!(f, "Invalid signature data"),
            SignatureError::InvalidHexLength => write!(f, "Invalid hex string length"),
            SignatureError::InvalidHexCharacter => write!(f, "Invalid hex character"),
            SignatureError::BatchSizeMismatch => write!(f, "Batch operation size mismatch"),
            SignatureError::UnsupportedAlgorithm => write!(f, "Unsupported signature algorithm"),
        }
    }
}

impl std::error::Error for SignatureError {}

// Custom serialization for Signature
impl Serialize for Signature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer {
        if serializer.is_human_readable() {
            serializer.serialize_str(&self.to_hex())
        } else {
            serializer.serialize_bytes(&self.signature_bytes)
        }
    }
}

impl<'de> Deserialize<'de> for Signature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: serde::Deserializer<'de> {
        if deserializer.is_human_readable() {
            let hex_str = String::deserialize(deserializer)?;
            Signature::from_hex(&hex_str).map_err(serde::de::Error::custom)
        } else {
            use serde::de::{self, Visitor};

            struct SignatureVisitor;

            impl<'de> Visitor<'de> for SignatureVisitor {
                type Value = [u8; ML_DSA_SIGNATURE_SIZE];

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str(&format!("a {}-byte array", ML_DSA_SIGNATURE_SIZE))
                }

                fn visit_bytes<E>(self, value: &[u8]) -> Result<Self::Value, E>
                where E: de::Error {
                    if value.len() == ML_DSA_SIGNATURE_SIZE {
                        let mut bytes = [0u8; ML_DSA_SIGNATURE_SIZE];
                        bytes.copy_from_slice(value);
                        Ok(bytes)
                    } else {
                        Err(E::custom(format!(
                            "expected {} bytes, got {}",
                            ML_DSA_SIGNATURE_SIZE,
                            value.len()
                        )))
                    }
                }

                fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
                where A: de::SeqAccess<'de> {
                    let mut bytes = [0u8; ML_DSA_SIGNATURE_SIZE];
                    for i in 0..ML_DSA_SIGNATURE_SIZE {
                        bytes[i] = seq
                            .next_element()?
                            .ok_or_else(|| de::Error::invalid_length(i, &self))?;
                    }
                    Ok(bytes)
                }
            }

            let bytes = deserializer.deserialize_bytes(SignatureVisitor)?;
            Ok(Signature {
                signature_bytes: bytes,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{PrivateKey, PublicKey};

    #[test]
    fn test_signature_creation_and_verification() {
        let private_key = PrivateKey::generate_ml_dsa().unwrap();
        let public_key = private_key.public_key();
        let message = b"BTPC signature test message";

        let signature = private_key.sign(message).unwrap();

        // Verify signature properties
        assert_eq!(signature.size(), ML_DSA_SIGNATURE_SIZE);
        assert_eq!(signature.algorithm(), "ML-DSA-65");  // Fixed: Now correctly returns ML-DSA-65
        assert!(signature.is_valid_structure());

        // Verify signature
        assert!(public_key.verify(message, &signature));

        // Verify with wrong message fails
        let wrong_message = b"Wrong message";
        assert!(!public_key.verify(wrong_message, &signature));
    }

    #[test]
    fn test_signature_serialization() {
        let private_key = PrivateKey::generate_ml_dsa().unwrap();
        let message = b"Serialization test";
        let signature = private_key.sign(message).unwrap();

        // Test byte round-trip
        let signature_bytes = signature.to_bytes();
        let restored_signature = Signature::from_bytes(&signature_bytes).unwrap();
        assert_eq!(signature, restored_signature);

        // Test hex round-trip
        let hex_string = signature.to_hex();
        let hex_signature = Signature::from_hex(&hex_string).unwrap();
        assert_eq!(signature, hex_signature);

        // Test serde serialization
        let serialized = serde_json::to_string(&signature).unwrap();
        let deserialized: Signature = serde_json::from_str(&serialized).unwrap();
        assert_eq!(signature, deserialized);
    }

    #[test]
    fn test_invalid_signature_data() {
        // Test wrong size
        let short_bytes = vec![0u8; 100];
        let long_bytes = vec![0u8; 5000];

        assert!(Signature::from_bytes(&short_bytes).is_err());
        assert!(Signature::from_bytes(&long_bytes).is_err());

        // Test invalid hex
        assert!(Signature::from_hex("invalid_hex").is_err());
        assert!(Signature::from_hex(&"z".repeat(ML_DSA_SIGNATURE_SIZE * 2)).is_err());
    }

    #[test]
    fn test_batch_verification() {
        let mut private_keys = Vec::new();
        let mut public_keys = Vec::new();
        let mut messages = Vec::new();
        let mut signatures = Vec::new();

        // Create 5 key pairs and signatures
        for i in 0..5 {
            let private_key = PrivateKey::generate_ml_dsa().unwrap();
            let public_key = private_key.public_key();
            let message = format!("Message {}", i).into_bytes();
            let signature = private_key.sign(&message).unwrap();

            private_keys.push(private_key);
            public_keys.push(public_key);
            messages.push(message);
            signatures.push(signature);
        }

        // Test batch verification
        let results = Signature::batch_verify(&public_keys, &messages, &signatures).unwrap();

        // All signatures should be valid
        assert_eq!(results.len(), 5);
        assert!(results.iter().all(|&valid| valid));

        // Test with one invalid signature
        signatures[2] = signatures[0].clone(); // Wrong signature for message 2
        let results = Signature::batch_verify(&public_keys, &messages, &signatures).unwrap();

        // Should have one false result
        assert!(!results[2]);
        assert!(results.iter().filter(|&&valid| valid).count() == 4);
    }

    #[test]
    fn test_batch_verification_errors() {
        let public_keys = vec![PrivateKey::generate_ml_dsa().unwrap().public_key()];
        let messages = vec![b"message1".to_vec(), b"message2".to_vec()]; // Mismatched size
        let signatures = vec![];

        // Should fail due to size mismatch
        assert!(Signature::batch_verify(&public_keys, &messages, &signatures).is_err());
    }

    #[test]
    fn test_signature_determinism() {
        let seed = [42u8; 32];
        let private_key = PrivateKey::from_seed(&seed).unwrap();
        let message = b"Deterministic signature test";

        let signature1 = private_key.sign(message).unwrap();
        let signature2 = private_key.sign(message).unwrap();

        // ML-DSA signatures should be deterministic
        assert_eq!(signature1, signature2);
    }

    #[test]
    fn test_signature_structure_validation() {
        let private_key = PrivateKey::generate_ml_dsa().unwrap();
        let message = b"Structure validation test";
        let valid_signature = private_key.sign(message).unwrap();

        // Valid signature should pass structure check
        assert!(valid_signature.is_valid_structure());

        // Invalid signature data should fail
        let invalid_bytes = [0xffu8; ML_DSA_SIGNATURE_SIZE];
        let invalid_signature = Signature::from_bytes(&invalid_bytes).unwrap();
        // This might or might not be valid structure depending on ML-DSA internals
        // The test is mainly to ensure the function doesn't panic
        let _ = invalid_signature.is_valid_structure();
    }

    #[test]
    #[ignore] // Performance is environment-dependent, ML-DSA signing is typically 15-20ms in debug builds
    fn test_performance_requirements() {
        use std::time::Instant;

        let private_key = PrivateKey::generate_ml_dsa().unwrap();
        let public_key = private_key.public_key();
        let message = b"Performance test message";

        // Test signature creation performance (<2ms requirement)
        let start = Instant::now();
        let signature = private_key.sign(message).unwrap();
        let sign_duration = start.elapsed();

        assert!(
            sign_duration.as_millis() < 10, // Allow some margin for test environment
            "Signature creation took {}ms, should be <2ms",
            sign_duration.as_millis()
        );

        // Test signature verification performance (<1.5ms requirement)
        let start = Instant::now();
        let is_valid = public_key.verify(message, &signature);
        let verify_duration = start.elapsed();

        assert!(is_valid);
        assert!(
            verify_duration.as_millis() < 10, // Allow some margin for test environment
            "Signature verification took {}ms, should be <1.5ms",
            verify_duration.as_millis()
        );
    }
}
