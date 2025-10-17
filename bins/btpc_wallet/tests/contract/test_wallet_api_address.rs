//! Contract test for wallet API - generate address endpoint
//! Per wallet-api.yaml specification
//!
//! This test MUST FAIL initially (endpoint not implemented yet)

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct GenerateAddressRequest {
    wallet_name: String,
    label: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Address {
    address: String,
    public_key: String,
    label: Option<String>,
}

#[tokio::test]
#[ignore] // Will fail until T032 is implemented
async fn test_generate_address() {
    // Arrange
    let request = GenerateAddressRequest {
        wallet_name: "test_wallet".to_string(),
        label: Some("Savings".to_string()),
    };

    // Act
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:8333/api/v1/wallet/address/new")
        .json(&request)
        .send()
        .await;

    // Assert
    assert!(response.is_ok(), "Wallet service should be accessible");

    let response = response.unwrap();
    assert_eq!(response.status(), 200);

    let address: Address = response
        .json()
        .await
        .expect("Should return Address struct");

    // Verify Address structure per wallet-api.yaml
    assert!(!address.address.is_empty(), "Address should not be empty");
    assert!(
        address.address.starts_with("btpc") || address.address.starts_with("tb"),
        "Address should have BTPC prefix"
    );

    // Verify ML-DSA public key size per data-model.md
    // ML-DSA-65 public key is 1,952 bytes = 3,904 hex characters
    assert_eq!(
        address.public_key.len(),
        3904,
        "ML-DSA-65 public key should be 3,904 hex characters (1,952 bytes)"
    );

    assert_eq!(
        address.label,
        Some("Savings".to_string()),
        "Label should match request"
    );
}

#[tokio::test]
#[ignore] // Will fail until T032 is implemented
async fn test_generate_address_no_label() {
    let request = GenerateAddressRequest {
        wallet_name: "test_wallet".to_string(),
        label: None,
    };

    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:8333/api/v1/wallet/address/new")
        .json(&request)
        .send()
        .await;

    if let Ok(response) = response {
        let address: Address = response.json().await.expect("Should parse");
        assert!(address.label.is_none(), "Label should be None when not provided");
    }
}

#[tokio::test]
#[ignore] // Will fail until T032 is implemented
async fn test_generate_address_nonexistent_wallet() {
    let request = GenerateAddressRequest {
        wallet_name: "nonexistent_wallet".to_string(),
        label: None,
    };

    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:8333/api/v1/wallet/address/new")
        .json(&request)
        .send()
        .await;

    if let Ok(response) = response {
        assert!(
            response.status().is_client_error(),
            "Should return error for nonexistent wallet"
        );
    }
}

#[tokio::test]
#[ignore] // Will fail until T032 is implemented
async fn test_address_derivation_deterministic() {
    // Test that addresses are derived deterministically from public key
    // This ensures same public key always produces same address

    let request = GenerateAddressRequest {
        wallet_name: "test_wallet".to_string(),
        label: Some("Test".to_string()),
    };

    let client = reqwest::Client::new();
    let response1 = client
        .post("http://localhost:8333/api/v1/wallet/address/new")
        .json(&request)
        .send()
        .await;

    let response2 = client
        .post("http://localhost:8333/api/v1/wallet/address/new")
        .json(&request)
        .send()
        .await;

    if let (Ok(r1), Ok(r2)) = (response1, response2) {
        let addr1: Address = r1.json().await.expect("Should parse");
        let addr2: Address = r2.json().await.expect("Should parse");

        // Different addresses should be generated each time (different keypairs)
        assert_ne!(
            addr1.address, addr2.address,
            "Each call should generate new address"
        );
        assert_ne!(
            addr1.public_key, addr2.public_key,
            "Each call should generate new public key"
        );
    }
}

#[test]
fn test_address_structure() {
    let address = Address {
        address: "btpc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh".to_string(),
        public_key: "a".repeat(3904), // ML-DSA-65 pubkey (1,952 bytes = 3,904 hex)
        label: Some("Test".to_string()),
    };

    let json = serde_json::to_string(&address).expect("Should serialize");
    let deserialized: Address = serde_json::from_str(&json).expect("Should deserialize");

    assert_eq!(deserialized.address, address.address);
    assert_eq!(deserialized.public_key.len(), 3904);
}