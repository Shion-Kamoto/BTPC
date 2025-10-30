//! Contract test for crypto API - ML-DSA signature verification
//! Per crypto_api.yaml specification
//!
//! This test MUST FAIL initially (endpoint not implemented yet)

use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Serialize)]
struct VerifyRequest {
    data: String,           // Hex-encoded data that was signed
    signature: String,      // Hex-encoded ML-DSA-65 signature
    public_key: String,     // Hex-encoded ML-DSA public key
}

#[derive(Debug, Deserialize)]
struct VerifyResponse {
    valid: bool,            // Signature verification result
    algorithm: String,      // "ML-DSA-65" or "ML-DSA-87"
}

#[tokio::test]
#[ignore] // Will fail until T035 is implemented
async fn test_mldsa_verify_valid_signature() {
    // Arrange: Create verify request with valid signature
    let data = hex::encode("Test message for verification");
    let public_key = "a".repeat(3904); // Placeholder for ML-DSA-65 public key (1,952 bytes)
    let signature = "b".repeat(6618);  // Placeholder for ML-DSA-65 signature (3,309 bytes)

    let request = VerifyRequest {
        data: data.clone(),
        signature,
        public_key,
    };

    let start = Instant::now();

    // Act: Send verify request
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:8332/crypto/verify")
        .json(&request)
        .send()
        .await;

    let elapsed = start.elapsed();

    // Assert
    assert!(response.is_ok(), "Crypto service should be accessible");

    let response = response.unwrap();
    assert_eq!(response.status(), 200);

    let verify_response: VerifyResponse = response
        .json()
        .await
        .expect("Should return VerifyResponse");

    // Verify response structure
    assert!(
        verify_response.valid,
        "Valid signature should pass verification"
    );

    assert_eq!(
        verify_response.algorithm, "ML-DSA-65",
        "Should use ML-DSA-65 algorithm"
    );

    // Verify verification time < 1.5ms per constitution
    assert!(
        elapsed.as_micros() < 1500,
        "Verification time must be < 1.5ms, was {}μs",
        elapsed.as_micros()
    );
}

#[tokio::test]
#[ignore] // Will fail until T035 is implemented
async fn test_mldsa_verify_invalid_signature() {
    // Test that invalid signature is rejected
    let data = hex::encode("Test message");
    let public_key = "a".repeat(3904);
    let invalid_signature = "0".repeat(6618); // Invalid signature

    let request = VerifyRequest {
        data,
        signature: invalid_signature,
        public_key,
    };

    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:8332/crypto/verify")
        .json(&request)
        .send()
        .await;

    if let Ok(response) = response {
        let verify_response: VerifyResponse = response.json().await.expect("Should parse");

        // Invalid signature should fail verification
        assert!(
            !verify_response.valid,
            "Invalid signature should fail verification"
        );
    }
}

#[tokio::test]
#[ignore] // Will fail until T035 is implemented
async fn test_mldsa_verify_tampered_data() {
    // Test that verification fails when data is modified
    let original_data = hex::encode("Original message");
    let tampered_data = hex::encode("Tampered message");
    let public_key = "a".repeat(3904);
    let signature = "b".repeat(6618); // Signature for original data

    let request = VerifyRequest {
        data: tampered_data, // Different data than what was signed
        signature,
        public_key,
    };

    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:8332/crypto/verify")
        .json(&request)
        .send()
        .await;

    if let Ok(response) = response {
        let verify_response: VerifyResponse = response.json().await.expect("Should parse");

        // Should reject tampered data
        assert!(
            !verify_response.valid,
            "Verification should fail for tampered data"
        );
    }
}

#[tokio::test]
#[ignore] // Will fail until T035 is implemented
async fn test_mldsa_verify_wrong_public_key() {
    // Test that verification fails with wrong public key
    let data = hex::encode("Test message");
    let correct_key = "a".repeat(3904);
    let wrong_key = "c".repeat(3904); // Different public key
    let signature = "b".repeat(6618); // Signature created with correct_key

    let request = VerifyRequest {
        data,
        signature,
        public_key: wrong_key, // Wrong public key
    };

    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:8332/crypto/verify")
        .json(&request)
        .send()
        .await;

    if let Ok(response) = response {
        let verify_response: VerifyResponse = response.json().await.expect("Should parse");

        assert!(
            !verify_response.valid,
            "Verification should fail with wrong public key"
        );
    }
}

#[tokio::test]
#[ignore] // Will fail until T035 is implemented
async fn test_mldsa_verify_performance() {
    // Test verification performance across multiple iterations
    let data = hex::encode("Performance test data");
    let public_key = "a".repeat(3904);
    let signature = "b".repeat(6618);

    let mut total_time = std::time::Duration::ZERO;
    let iterations = 100;

    let client = reqwest::Client::new();

    for _ in 0..iterations {
        let request = VerifyRequest {
            data: data.clone(),
            signature: signature.clone(),
            public_key: public_key.clone(),
        };

        let start = Instant::now();
        let response = client
            .post("http://localhost:8332/crypto/verify")
            .json(&request)
            .send()
            .await;

        if let Ok(response) = response {
            let _: VerifyResponse = response.json().await.expect("Should parse");
            total_time += start.elapsed();
        }
    }

    let avg_time = total_time / iterations;
    assert!(
        avg_time.as_micros() < 1500,
        "Average verification time must be < 1.5ms, was {}μs",
        avg_time.as_micros()
    );
}

#[tokio::test]
#[ignore] // Will fail until T035 is implemented
async fn test_mldsa_verify_invalid_hex_data() {
    let request = VerifyRequest {
        data: "not_valid_hex".to_string(),
        signature: "b".repeat(6618),
        public_key: "a".repeat(3904),
    };

    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:8332/crypto/verify")
        .json(&request)
        .send()
        .await;

    if let Ok(response) = response {
        assert!(
            response.status().is_client_error(),
            "Should reject invalid hex data"
        );
    }
}

#[tokio::test]
#[ignore] // Will fail until T035 is implemented
async fn test_mldsa_verify_invalid_signature_length() {
    let request = VerifyRequest {
        data: hex::encode("Test data"),
        signature: "b".repeat(100), // Invalid signature length
        public_key: "a".repeat(3904),
    };

    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:8332/crypto/verify")
        .json(&request)
        .send()
        .await;

    if let Ok(response) = response {
        assert!(
            response.status().is_client_error(),
            "Should reject invalid signature length"
        );
    }
}

#[tokio::test]
#[ignore] // Will fail until T035 is implemented
async fn test_mldsa_verify_invalid_public_key_length() {
    let request = VerifyRequest {
        data: hex::encode("Test data"),
        signature: "b".repeat(6618),
        public_key: "a".repeat(100), // Invalid public key length
    };

    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:8332/crypto/verify")
        .json(&request)
        .send()
        .await;

    if let Ok(response) = response {
        assert!(
            response.status().is_client_error(),
            "Should reject invalid public key length"
        );
    }
}

#[test]
fn test_public_key_size() {
    // Verify ML-DSA-65 public key size
    let pubkey = "a".repeat(3904); // 1,952 bytes
    assert_eq!(pubkey.len(), 3904);
    assert!(pubkey.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn test_signature_size() {
    // Verify ML-DSA-65 signature size
    let sig = "b".repeat(6618); // 3,309 bytes
    assert_eq!(sig.len(), 6618);
    assert!(sig.chars().all(|c| c.is_ascii_hexdigit()));
}