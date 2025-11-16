//! Contract Test: enumerate_gpus Command (T005)
//!
//! Feature: 012-create-an-new (GPU Mining Dashboard)
//! Test Requirement: Verify enumerate_gpus command contract
//!
//! EXPECTED RESULT: These tests MUST FAIL until T014 (GPU enumeration implementation)

use btpc_desktop_app::gpu_health_monitor::{enumerate_gpus, GpuDevice, GpuVendor};
use std::time::Instant;

/// Test: enumerate_gpus returns array of GPU devices with required fields
///
/// Contract verification from contracts/tauri-commands.yaml:
/// - Returns Vec<GpuDevice>
/// - Each device has: device_index, model_name, vendor, opencl_capable, compute_capability
#[test]
fn test_enumerate_gpus_returns_valid_devices() {
    // Call the GPU enumeration function
    let result = enumerate_gpus();

    // Assert: Function should succeed (when GPUs are available)
    // NOTE: This test MAY pass on systems without GPUs (returns empty vec)
    assert!(result.is_ok(), "enumerate_gpus should not return error on valid system");

    let devices = result.unwrap();

    // If GPUs are detected, validate structure
    if !devices.is_empty() {
        let first_device = &devices[0];

        // Assert: Required fields are present
        assert!(first_device.device_index >= 0, "device_index should be non-negative");
        assert!(!first_device.model_name.is_empty(), "model_name should not be empty");

        // Assert: Vendor is a valid enum value
        match first_device.vendor {
            GpuVendor::Nvidia | GpuVendor::Amd | GpuVendor::Intel | GpuVendor::Other => {
                // Valid vendor
            }
        }

        // Assert: opencl_capable is a boolean (no validation needed, type-safe)
        // Assert: compute_capability is optional
        if let Some(ref cap) = first_device.compute_capability {
            assert!(!cap.is_empty(), "compute_capability should not be empty string if present");
        }
    }
}

/// Test: enumerate_gpus handles "No GPUs detected" gracefully
///
/// Contract: Should return Ok(empty vec) when no GPUs available, NOT an error
#[test]
fn test_enumerate_gpus_no_gpus_graceful() {
    let result = enumerate_gpus();

    // Assert: Function should not panic or return error
    assert!(result.is_ok(), "enumerate_gpus should return Ok even with no GPUs");

    // NOTE: On systems with GPUs, this will be non-empty
    // This test primarily ensures the function doesn't crash
}

/// Test: enumerate_gpus completes in <500ms (NFR-001)
///
/// Performance requirement from plan.md: GPU enumeration <500ms
#[test]
fn test_enumerate_gpus_performance() {
    let start = Instant::now();
    let result = enumerate_gpus();
    let duration = start.elapsed();

    // Assert: Function must complete within performance budget
    assert!(
        duration.as_millis() < 500,
        "enumerate_gpus took {}ms, must be <500ms (NFR-001)",
        duration.as_millis()
    );

    // Assert: Function succeeded
    assert!(result.is_ok(), "Performance test requires successful execution");
}

/// Test: enumerate_gpus detects multiple GPUs correctly
///
/// Contract: Returns separate entries for each physical GPU
#[test]
fn test_enumerate_gpus_multiple_devices() {
    let result = enumerate_gpus();
    assert!(result.is_ok(), "enumerate_gpus should succeed");

    let devices = result.unwrap();

    // If multiple GPUs detected, ensure unique device indices
    if devices.len() > 1 {
        let mut indices = std::collections::HashSet::new();
        for device in &devices {
            assert!(
                indices.insert(device.device_index),
                "Device indices must be unique (found duplicate: {})",
                device.device_index
            );
        }
    }
}

/// Test: enumerate_gpus serialization compatibility
///
/// Ensures GpuDevice can be serialized to JSON for Tauri frontend
#[test]
fn test_enumerate_gpus_serialization() {
    // Create a mock device to test serialization
    let mock_device = GpuDevice {
        device_index: 0,
        model_name: "Test GPU".to_string(),
        vendor: GpuVendor::Nvidia,
        opencl_capable: true,
        compute_capability: Some("8.6".to_string()),
    };

    // Assert: Device can be serialized to JSON
    let json_result = serde_json::to_string(&mock_device);
    assert!(json_result.is_ok(), "GpuDevice must be serializable to JSON");

    let json = json_result.unwrap();

    // Assert: Required fields are in JSON
    assert!(json.contains("device_index"), "JSON must contain device_index");
    assert!(json.contains("model_name"), "JSON must contain model_name");
    assert!(json.contains("vendor"), "JSON must contain vendor");
    assert!(json.contains("opencl_capable"), "JSON must contain opencl_capable");

    // Assert: Can deserialize back
    let deserialized: Result<GpuDevice, _> = serde_json::from_str(&json);
    assert!(deserialized.is_ok(), "JSON must deserialize back to GpuDevice");
}
