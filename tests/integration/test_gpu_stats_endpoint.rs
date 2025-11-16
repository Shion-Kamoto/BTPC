// Contract test for GPU stats HTTP endpoints
// Feature: 009-integrate-gpu-mining Phase 3
// Contract: /home/bob/BTPC/BTPC/specs/009-integrate-gpu-mining/contracts/gpu-stats-api.yaml

use reqwest;
use serde_json::Value;

/// Test /stats endpoint contract
#[tokio::test]
#[ignore] // Manual test - requires btpc_miner running with --gpu
async fn test_stats_endpoint_contract() {
    let client = reqwest::Client::new();
    let response = client
        .get("http://127.0.0.1:8333/stats")
        .send()
        .await
        .expect("Failed to connect to stats server");

    // Contract: HTTP 200 OK
    assert_eq!(response.status(), 200, "Expected HTTP 200 OK");

    // Contract: application/json content type
    let content_type = response
        .headers()
        .get("content-type")
        .expect("Missing content-type header");
    assert!(
        content_type.to_str().unwrap().contains("application/json"),
        "Expected application/json content type"
    );

    // Contract: Valid JSON response
    let json: Value = response.json().await.expect("Invalid JSON response");

    // Contract: GpuStats schema validation
    assert!(json.get("device_name").is_some(), "Missing device_name");
    assert!(json.get("vendor").is_some(), "Missing vendor");
    assert!(json.get("compute_units").is_some(), "Missing compute_units");
    assert!(json.get("max_work_group_size").is_some(), "Missing max_work_group_size");
    assert!(json.get("global_mem_size").is_some(), "Missing global_mem_size");
    assert!(json.get("local_mem_size").is_some(), "Missing local_mem_size");
    assert!(json.get("max_clock_frequency").is_some(), "Missing max_clock_frequency");
    assert!(json.get("hashrate").is_some(), "Missing hashrate");
    assert!(json.get("total_hashes").is_some(), "Missing total_hashes");
    assert!(json.get("uptime_seconds").is_some(), "Missing uptime_seconds");

    // Contract: Type validation
    assert!(json["device_name"].is_string(), "device_name must be string");
    assert!(json["vendor"].is_string(), "vendor must be string");
    assert!(json["compute_units"].is_number(), "compute_units must be number");
    assert!(json["hashrate"].is_number(), "hashrate must be number");
    assert!(json["total_hashes"].is_number(), "total_hashes must be number");
    assert!(json["uptime_seconds"].is_number(), "uptime_seconds must be number");

    // Contract: Value constraints
    let hashrate = json["hashrate"].as_f64().unwrap();
    assert!(hashrate >= 0.0, "hashrate must be >= 0");

    let compute_units = json["compute_units"].as_u64().unwrap();
    assert!(compute_units >= 1, "compute_units must be >= 1");

    let device_name = json["device_name"].as_str().unwrap();
    assert!(!device_name.is_empty(), "device_name must be non-empty");

    // Contract: Optional fields nullable
    assert!(
        json["temperature"].is_null() || json["temperature"].is_number(),
        "temperature must be null or number"
    );
    assert!(
        json["power_usage"].is_null() || json["power_usage"].is_number(),
        "power_usage must be null or number"
    );

    println!("✅ /stats endpoint contract validation passed");
    println!("   Device: {}", device_name);
    println!("   Hashrate: {} MH/s", hashrate);
    println!("   Compute Units: {}", compute_units);
}

/// Test /health endpoint contract
#[tokio::test]
#[ignore] // Manual test - requires btpc_miner running
async fn test_health_endpoint_contract() {
    let client = reqwest::Client::new();
    let response = client
        .get("http://127.0.0.1:8333/health")
        .send()
        .await
        .expect("Failed to connect to stats server");

    // Contract: HTTP 200 OK
    assert_eq!(response.status(), 200, "Expected HTTP 200 OK");

    // Contract: application/json content type
    let content_type = response
        .headers()
        .get("content-type")
        .expect("Missing content-type header");
    assert!(
        content_type.to_str().unwrap().contains("application/json"),
        "Expected application/json content type"
    );

    // Contract: Valid JSON response
    let json: Value = response.json().await.expect("Invalid JSON response");

    // Contract: HealthResponse schema validation
    assert!(json.get("status").is_some(), "Missing status field");
    assert!(json.get("service").is_some(), "Missing service field");

    // Contract: status must be "ok"
    assert_eq!(json["status"].as_str().unwrap(), "ok", "status must be 'ok'");

    // Contract: service must be "btpc_miner_stats"
    assert_eq!(
        json["service"].as_str().unwrap(),
        "btpc_miner_stats",
        "service must be 'btpc_miner_stats'"
    );

    println!("✅ /health endpoint contract validation passed");
}

/// Test /stats error handling (GPU not enabled)
#[tokio::test]
#[ignore] // Manual test - requires btpc_miner running WITHOUT --gpu
async fn test_stats_error_contract() {
    let client = reqwest::Client::new();
    let response = client
        .get("http://127.0.0.1:8333/stats")
        .send()
        .await
        .expect("Failed to connect to stats server");

    // Contract: HTTP 500 error when GPU not available
    assert_eq!(response.status(), 500, "Expected HTTP 500 error");

    // Contract: application/json content type
    let content_type = response
        .headers()
        .get("content-type")
        .expect("Missing content-type header");
    assert!(
        content_type.to_str().unwrap().contains("application/json"),
        "Expected application/json content type"
    );

    // Contract: Valid JSON error response
    let json: Value = response.json().await.expect("Invalid JSON response");

    // Contract: Error schema validation
    assert!(json.get("error").is_some(), "Missing error field");
    assert!(json["error"].is_string(), "error must be string");

    let error_msg = json["error"].as_str().unwrap();
    assert!(!error_msg.is_empty(), "error message must be non-empty");

    println!("✅ /stats error handling contract validation passed");
    println!("   Error: {}", error_msg);
}