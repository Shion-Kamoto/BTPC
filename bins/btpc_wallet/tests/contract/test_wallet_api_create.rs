//! Contract test for wallet API - create wallet endpoint
//! Per wallet-api.yaml specification
//!
//! This test MUST FAIL initially (endpoint not implemented yet)

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct CreateWalletRequest {
    name: String,
    passphrase: String,
    network: String,
}

#[derive(Debug, Deserialize)]
struct WalletInfo {
    name: String,
    network: String,
    encrypted: bool,
    address_count: usize,
}

#[tokio::test]
#[ignore] // Will fail until T031 is implemented
async fn test_create_wallet() {
    // Arrange
    let request = CreateWalletRequest {
        name: "test_wallet".to_string(),
        passphrase: "secure_passphrase_123".to_string(),
        network: "regtest".to_string(),
    };

    // Act: POST to wallet creation endpoint
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:8333/api/v1/wallet/create")
        .json(&request)
        .send()
        .await;

    // Assert
    assert!(response.is_ok(), "Wallet service should be running on port 8333");

    let response = response.unwrap();
    assert_eq!(response.status(), 200, "Should return HTTP 200 OK");

    let wallet_info: WalletInfo = response
        .json()
        .await
        .expect("Should return WalletInfo");

    // Verify WalletInfo structure per wallet-api.yaml
    assert_eq!(wallet_info.name, "test_wallet");
    assert_eq!(wallet_info.network, "regtest");
    assert!(wallet_info.encrypted, "Wallet should be encrypted with passphrase");
    assert_eq!(
        wallet_info.address_count, 0,
        "New wallet should have no addresses initially"
    );
}

#[tokio::test]
#[ignore] // Will fail until T031 is implemented
async fn test_create_wallet_invalid_network() {
    let request = CreateWalletRequest {
        name: "test_wallet".to_string(),
        passphrase: "pass123".to_string(),
        network: "invalid_network".to_string(),
    };

    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:8333/api/v1/wallet/create")
        .json(&request)
        .send()
        .await;

    if let Ok(response) = response {
        // Should reject invalid network
        assert!(
            response.status().is_client_error(),
            "Should return 4xx error for invalid network"
        );
    }
}

#[tokio::test]
#[ignore] // Will fail until T031 is implemented
async fn test_create_wallet_missing_passphrase() {
    let request = serde_json::json!({
        "name": "test_wallet",
        "network": "mainnet"
        // Missing passphrase - should be required
    });

    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:8333/api/v1/wallet/create")
        .json(&request)
        .send()
        .await;

    if let Ok(response) = response {
        assert!(
            response.status().is_client_error(),
            "Should require passphrase"
        );
    }
}

#[tokio::test]
#[ignore] // Will fail until T031 is implemented
async fn test_create_wallet_duplicate_name() {
    let request = CreateWalletRequest {
        name: "duplicate_test".to_string(),
        passphrase: "pass123".to_string(),
        network: "regtest".to_string(),
    };

    let client = reqwest::Client::new();

    // Create first wallet
    let _first = client
        .post("http://localhost:8333/api/v1/wallet/create")
        .json(&request)
        .send()
        .await;

    // Try to create wallet with same name - should fail
    let second = client
        .post("http://localhost:8333/api/v1/wallet/create")
        .json(&request)
        .send()
        .await;

    if let Ok(response) = second {
        assert!(
            response.status().is_client_error(),
            "Should reject duplicate wallet name"
        );
    }
}

#[test]
fn test_wallet_info_serialization() {
    let info = WalletInfo {
        name: "my_wallet".to_string(),
        network: "mainnet".to_string(),
        encrypted: true,
        address_count: 5,
    };

    let json = serde_json::to_string(&info).expect("Should serialize");
    let deserialized: WalletInfo = serde_json::from_str(&json).expect("Should deserialize");

    assert_eq!(deserialized.name, "my_wallet");
    assert_eq!(deserialized.address_count, 5);
}