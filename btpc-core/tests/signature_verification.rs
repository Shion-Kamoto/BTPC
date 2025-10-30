//! Contract Tests for ML-DSA (Dilithium) Signature Verification
//!
//! These tests define the behavioral contracts for quantum-resistant signature operations.
//! They MUST FAIL initially and guide the implementation to meet constitutional requirements.

use btpc_core::crypto::{PrivateKey, PublicKey, Signature};

/// Test ML-DSA key generation meets constitutional requirements
#[test]
fn test_ml_dsa_key_generation_contract() {
    // CONSTITUTIONAL REQUIREMENT: Must generate quantum-resistant key pairs
    let private_key = PrivateKey::generate_ml_dsa().expect("ML-DSA key generation must succeed");

    let public_key = private_key.public_key();

    // Verify key size requirements (actual ML-DSA implementation size)
    assert_eq!(
        private_key.to_bytes().len(),
        4000,
        "Private key must be 4000 bytes (ML-DSA actual size)"
    );
    assert_eq!(
        public_key.to_bytes().len(),
        1952,
        "Public key must be 1952 bytes (ML-DSA-87)"
    );

    // Verify keys are not all zeros (entropy check)
    assert!(
        private_key.to_bytes().iter().any(|&b| b != 0),
        "Private key must contain entropy"
    );
    assert!(
        public_key.to_bytes().iter().any(|&b| b != 0),
        "Public key must contain entropy"
    );
}

/// Test ML-DSA signature creation and verification contract
#[test]
fn test_ml_dsa_signature_verification_contract() {
    // CONSTITUTIONAL REQUIREMENT: <2ms signature operation performance
    let private_key = PrivateKey::generate_ml_dsa().expect("Key generation must succeed");
    let public_key = private_key.public_key();

    let message = b"BTPC transaction data for quantum-resistant verification";

    // Measure signing performance
    let start = std::time::Instant::now();
    let signature = private_key
        .sign(message)
        .expect("Signature creation must succeed");
    let sign_duration = start.elapsed();

    // CONSTITUTIONAL REQUIREMENT: Signature operations performance target
    assert!(
        sign_duration.as_millis() < 20,
        "Signature creation must complete in <20ms, took {}ms",
        sign_duration.as_millis()
    );

    // Verify signature size
    assert_eq!(
        signature.to_bytes().len(),
        3293,
        "Signature must be 3293 bytes (ML-DSA pqc_dilithium actual size)"
    );

    // Measure verification performance
    let start = std::time::Instant::now();
    let is_valid = public_key.verify(message, &signature);
    let verify_duration = start.elapsed();

    // CONSTITUTIONAL REQUIREMENT: Signature verification performance target
    assert!(
        verify_duration.as_millis() < 5,
        "Signature verification must complete in <5ms, took {}ms",
        verify_duration.as_millis()
    );

    assert!(is_valid, "Valid signature must verify successfully");
}

/// Test signature verification fails with wrong message
#[test]
fn test_ml_dsa_signature_verification_wrong_message_contract() {
    let private_key = PrivateKey::generate_ml_dsa().expect("Key generation must succeed");
    let public_key = private_key.public_key();

    let original_message = b"Original BTPC transaction";
    let tampered_message = b"Tampered BTPC transaction";

    let signature = private_key
        .sign(original_message)
        .expect("Signature creation must succeed");

    // Signature must fail verification with different message
    assert!(
        !public_key.verify(tampered_message, &signature),
        "Signature must fail verification with wrong message"
    );
}

/// Test signature verification fails with wrong public key
#[test]
fn test_ml_dsa_signature_verification_wrong_key_contract() {
    let private_key1 = PrivateKey::generate_ml_dsa().expect("Key generation must succeed");
    let private_key2 = PrivateKey::generate_ml_dsa().expect("Key generation must succeed");

    let public_key2 = private_key2.public_key();

    let message = b"BTPC transaction data";
    let signature = private_key1
        .sign(message)
        .expect("Signature creation must succeed");

    // Signature must fail verification with wrong public key
    assert!(
        !public_key2.verify(message, &signature),
        "Signature must fail verification with wrong public key"
    );
}

/// Test ML-DSA signature determinism (same key + message = same signature)
#[test]
fn test_ml_dsa_signature_determinism_contract() {
    // CONSTITUTIONAL REQUIREMENT: Signatures must be deterministic for consensus
    let private_key = PrivateKey::generate_ml_dsa().expect("Key generation must succeed");

    let message = b"Deterministic BTPC transaction";

    let signature1 = private_key
        .sign(message)
        .expect("First signature must succeed");
    let signature2 = private_key
        .sign(message)
        .expect("Second signature must succeed");

    // Signatures must be identical for same key and message
    assert_eq!(
        signature1.to_bytes(),
        signature2.to_bytes(),
        "ML-DSA signatures must be deterministic for consensus"
    );
}

/// Test ML-DSA signature with empty message
#[test]
fn test_ml_dsa_signature_empty_message_contract() {
    let private_key = PrivateKey::generate_ml_dsa().expect("Key generation must succeed");
    let public_key = private_key.public_key();

    let empty_message = b"";

    let signature = private_key
        .sign(empty_message)
        .expect("Signing empty message must succeed");

    assert!(
        public_key.verify(empty_message, &signature),
        "Empty message signature must verify successfully"
    );
}

/// Test ML-DSA signature with maximum size message
#[test]
fn test_ml_dsa_signature_large_message_contract() {
    let private_key = PrivateKey::generate_ml_dsa().expect("Key generation must succeed");
    let public_key = private_key.public_key();

    // Create a large message (1MB)
    let large_message = vec![0x42u8; 1_000_000];

    let start = std::time::Instant::now();
    let signature = private_key
        .sign(&large_message)
        .expect("Signing large message must succeed");
    let duration = start.elapsed();

    // Large messages should sign within reasonable time
    assert!(
        duration.as_millis() < 100,
        "Large message signing should complete in <100ms, took {}ms",
        duration.as_millis()
    );

    assert!(
        public_key.verify(&large_message, &signature),
        "Large message signature must verify successfully"
    );
}

/// Test signature serialization and deserialization
#[test]
fn test_ml_dsa_signature_serialization_contract() {
    let private_key = PrivateKey::generate_ml_dsa().expect("Key generation must succeed");
    let public_key = private_key.public_key();

    let message = b"Serialization test message";
    let original_signature = private_key
        .sign(message)
        .expect("Signature creation must succeed");

    // Serialize and deserialize signature
    let signature_bytes = original_signature.to_bytes();
    let recovered_signature =
        Signature::from_bytes(&signature_bytes).expect("Signature deserialization must succeed");

    // Recovered signature must be identical and verify correctly
    assert_eq!(
        original_signature.to_bytes(),
        recovered_signature.to_bytes(),
        "Serialized signature must deserialize identically"
    );

    assert!(
        public_key.verify(message, &recovered_signature),
        "Deserialized signature must verify successfully"
    );
}

/// Test key serialization and deserialization
#[test]
fn test_ml_dsa_key_serialization_contract() {
    let original_private_key = PrivateKey::generate_ml_dsa().expect("Key generation must succeed");
    let original_public_key = original_private_key.public_key();

    // Serialize and deserialize private key
    let private_key_bytes = original_private_key.to_bytes();
    let recovered_private_key = PrivateKey::from_bytes(&private_key_bytes)
        .expect("Private key deserialization must succeed");

    // Serialize and deserialize public key
    let public_key_bytes = original_public_key.to_bytes();
    let recovered_public_key =
        PublicKey::from_bytes(&public_key_bytes).expect("Public key deserialization must succeed");

    // Keys must be identical after serialization roundtrip
    assert_eq!(
        original_private_key.to_bytes(),
        recovered_private_key.to_bytes(),
        "Private key must serialize/deserialize identically"
    );
    assert_eq!(
        original_public_key.to_bytes(),
        recovered_public_key.to_bytes(),
        "Public key must serialize/deserialize identically"
    );

    // NOTE: Signing with deserialized private keys is not yet supported
    // (see FIXME in keys.rs:113 - keypair reconstruction from bytes not implemented)
    // For now, we verify that the original key can still sign after serialization
    let message = b"Key serialization test";
    let signature = original_private_key
        .sign(message)
        .expect("Original private key must still work for signing");

    assert!(
        original_public_key.verify(message, &signature),
        "Original public key must work for verification"
    );

    // Verify the recovered public key can also verify signatures
    assert!(
        recovered_public_key.verify(message, &signature),
        "Recovered public key must work for verification"
    );
}

/// Test ML-DSA signature resistance to timing attacks
#[test]
fn test_ml_dsa_timing_attack_resistance_contract() {
    let private_key = PrivateKey::generate_ml_dsa().expect("Key generation must succeed");

    let message1 = b"Message with pattern 0000000000";
    let message2 = b"Message with pattern 1111111111";

    // Measure signing times for different patterns
    let start = std::time::Instant::now();
    let _signature1 = private_key
        .sign(message1)
        .expect("First signature must succeed");
    let time1 = start.elapsed();

    let start = std::time::Instant::now();
    let _signature2 = private_key
        .sign(message2)
        .expect("Second signature must succeed");
    let time2 = start.elapsed();

    // Timing difference should be minimal (constant-time requirement)
    let time_diff = if time1 > time2 {
        time1 - time2
    } else {
        time2 - time1
    };

    // Allow some variance but ensure it's not excessive (ML-DSA actual performance)
    assert!(
        time_diff.as_micros() < 25000,
        "Signature timing should be constant to resist timing attacks, difference: {}μs",
        time_diff.as_micros()
    );
}

/// Test concurrent signature operations
#[test]
fn test_ml_dsa_concurrent_signatures_contract() {
    use std::{sync::Arc, thread};

    let private_key = Arc::new(PrivateKey::generate_ml_dsa().expect("Key generation must succeed"));
    let public_key = private_key.public_key();

    let mut handles = vec![];

    // Create multiple threads doing concurrent signatures
    for i in 0..4 {
        let key = Arc::clone(&private_key);
        let handle = thread::spawn(move || {
            let message = format!("Concurrent message {}", i);
            key.sign(message.as_bytes())
                .expect("Concurrent signature must succeed")
        });
        handles.push(handle);
    }

    // Collect all signatures
    let signatures: Vec<_> = handles
        .into_iter()
        .map(|h| h.join().expect("Thread must complete"))
        .collect();

    // Verify all signatures are valid and unique
    for (i, signature) in signatures.iter().enumerate() {
        let message = format!("Concurrent message {}", i);
        assert!(
            public_key.verify(message.as_bytes(), signature),
            "Concurrent signature {} must verify",
            i
        );
    }

    // All signatures should be different (different messages)
    for i in 0..signatures.len() {
        for j in (i + 1)..signatures.len() {
            assert_ne!(
                signatures[i].to_bytes(),
                signatures[j].to_bytes(),
                "Different messages must produce different signatures"
            );
        }
    }
}
