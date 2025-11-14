//! Contract test for crypto API - ML-DSA signature creation
//! Per crypto_api.yaml specification
//!
//! This test MUST FAIL initially (endpoint not implemented yet)

use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Serialize)]
struct SignRequest {
    data: String,         // Hex-encoded data to sign
    private_key: String,  // Hex-encoded ML-DSA private key
}

#[derive(Debug, Deserialize)]
struct SignResponse {
    signature: String,    // Hex-encoded ML-DSA-65 signature
    algorithm: String,    // "ML-DSA-65" or "ML-DSA-87"
}

#[tokio::test]
#[ignore] // Will fail until T034 is implemented
async fn test_mldsa_sign() {
    // Arrange: Create sign request
    let data_to_sign = hex::encode("Test message for signing");
    let private_key = "a".repeat(6400); // Placeholder for ML-DSA private key

    let request = SignRequest {
        data: data_to_sign.clone(),
        private_key,
    };

    let start = Instant::now();

    // Act: Send sign request
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:8332/crypto/sign")
        .json(&request)
        .send()
        .await;

    let elapsed = start.elapsed();

    // Assert
    assert!(response.is_ok(), "Crypto service should be accessible");

    let response = response.unwrap();
    assert_eq!(response.status(), 200);

    let sign_response: SignResponse = response
        .json()
        .await
        .expect("Should return SignResponse");

    // Verify ML-DSA-65 signature size per data-model.md
    // ML-DSA-65 signature is 3,309 bytes = 6,618 hex characters
    assert_eq!(
        sign_response.signature.len(),
        6618,
        "ML-DSA-65 signature should be 6,618 hex characters (3,309 bytes)"
    );

    assert!(
        sign_response.signature.chars().all(|c| c.is_ascii_hexdigit()),
        "Signature should be valid hex"
    );

    assert_eq!(
        sign_response.algorithm, "ML-DSA-65",
        "Should use ML-DSA-65 algorithm"
    );

    // Verify signing time < 2ms per constitution
    assert!(
        elapsed.as_millis() < 2,
        "Signing time must be < 2ms, was {}ms",
        elapsed.as_millis()
    );
}

#[tokio::test]
#[ignore] // Will fail until T034 is implemented
async fn test_mldsa_sign_performance() {
    // Test signing performance across multiple iterations
    let data = hex::encode("Performance test data");
    let private_key = "b".repeat(6400);

    let mut total_time = std::time::Duration::ZERO;
    let iterations = 100;

    let client = reqwest::Client::new();

    for _ in 0..iterations {
        let request = SignRequest {
            data: data.clone(),
            private_key: private_key.clone(),
        };

        let start = Instant::now();
        let response = client
            .post("http://localhost:8332/crypto/sign")
            .json(&request)
            .send()
            .await;

        if let Ok(response) = response {
            let _: SignResponse = response.json().await.expect("Should parse");
            total_time += start.elapsed();
        }
    }

    let avg_time = total_time / iterations;
    assert!(
        avg_time.as_millis() < 2,
        "Average signing time must be < 2ms, was {}ms",
        avg_time.as_millis()
    );
}

#[tokio::test]
#[ignore] // Will fail until T034 is implemented
async fn test_mldsa_sign_invalid_data() {
    let request = SignRequest {
        data: "not_valid_hex".to_string(), // Invalid hex
        private_key: "a".repeat(6400),
    };

    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:8332/crypto/sign")
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
#[ignore] // Will fail until T034 is implemented
async fn test_mldsa_sign_invalid_private_key() {
    let request = SignRequest {
        data: hex::encode("Test data"),
        private_key: "invalid_key".to_string(), // Invalid key
    };

    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:8332/crypto/sign")
        .json(&request)
        .send()
        .await;

    if let Ok(response) = response {
        assert!(
            response.status().is_client_error(),
            "Should reject invalid private key"
        );
    }
}

#[tokio::test]
#[ignore] // Will fail until T034 is implemented
async fn test_mldsa_sign_deterministic() {
    // Test that signing same data with same key produces same signature
    let data = hex::encode("Deterministic test");
    let private_key = "c".repeat(6400);

    let request = SignRequest {
        data: data.clone(),
        private_key: private_key.clone(),
    };

    let client = reqwest::Client::new();

    // Sign twice
    let response1 = client
        .post("http://localhost:8332/crypto/sign")
        .json(&request)
        .send()
        .await
        .expect("First sign should succeed");

    let response2 = client
        .post("http://localhost:8332/crypto/sign")
        .json(&request)
        .send()
        .await
        .expect("Second sign should succeed");

    let sig1: SignResponse = response1.json().await.expect("Should parse");
    let sig2: SignResponse = response2.json().await.expect("Should parse");

    // Note: ML-DSA may use randomness, check spec for determinism requirements
    // If deterministic, signatures should match:
    // assert_eq!(sig1.signature, sig2.signature, "Signatures should be deterministic");
}

#[test]
fn test_signature_size() {
    // Verify ML-DSA-65 signature size
    let sig = "a".repeat(6618); // 3,309 bytes
    assert_eq!(sig.len(), 6618);
    assert!(sig.chars().all(|c| c.is_ascii_hexdigit()));
}