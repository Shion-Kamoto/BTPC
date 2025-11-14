//! GPU Health Monitoring Service
//!
//! Provides GPU enumeration and health metrics polling (temperature, fan speed, power, etc.)
//! Uses NVML for NVIDIA GPUs, fallback to sysinfo for AMD/Intel.
//!
//! Feature: 012-create-an-new

use serde::{Deserialize, Serialize};
use std::time::Instant;

/// GPU vendor enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GpuVendor {
    Nvidia,
    Amd,
    Intel,
    Other,
}

/// Represents a physical GPU device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuDevice {
    pub device_index: u32,
    pub model_name: String,
    pub vendor: GpuVendor,
    pub opencl_capable: bool,
    pub compute_capability: Option<String>,
}

/// Real-time GPU health metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuHealthMetrics {
    pub gpu_device_index: u32,
    pub temperature: Option<f32>,          // °C
    pub fan_speed: Option<u32>,            // RPM
    pub power_consumption: Option<f32>,    // Watts
    pub memory_used: Option<u64>,          // MB
    pub memory_total: Option<u64>,         // MB
    pub core_clock_speed: Option<u32>,     // MHz
    #[serde(skip, default = "Instant::now")]
    pub last_updated: Instant,
}

/// Enumerate all available GPUs on the system
///
/// Uses OpenCL device enumeration for cross-vendor compatibility.
/// Returns GPU device information for display in dashboard.
///
/// # Errors
/// - Returns error if OpenCL initialization fails
/// - Returns empty vec if no GPUs detected
pub fn enumerate_gpus() -> Result<Vec<GpuDevice>, String> {
    // Try OpenCL enumeration first (most reliable for mining GPUs)
    match enumerate_gpus_opencl() {
        Ok(devices) if !devices.is_empty() => return Ok(devices),
        _ => {
            // Fallback to sysinfo for basic detection
            enumerate_gpus_sysinfo()
        }
    }
}

/// Enumerate GPUs using OpenCL (preferred method)
fn enumerate_gpus_opencl() -> Result<Vec<GpuDevice>, String> {
    use opencl3::device::{Device, CL_DEVICE_TYPE_GPU};
    use opencl3::platform::get_platforms;

    // Get all OpenCL platforms
    let platforms = get_platforms()
        .map_err(|e| format!("OpenCL not available: {}", e))?;

    if platforms.is_empty() {
        return Ok(Vec::new());
    }

    let mut devices = Vec::new();
    let mut device_index = 0;

    // Prefer Rusticl platform (newer Mesa OpenCL) over Clover (older)
    // This avoids duplicates when same GPU is exposed through multiple platforms
    let preferred_platform = platforms.iter()
        .find(|p| p.name().unwrap_or_default().to_lowercase().contains("rusticl"))
        .or_else(|| platforms.first())
        .ok_or_else(|| "No OpenCL platforms available".to_string())?;

    println!("🔍 Using OpenCL platform: {}",
             preferred_platform.name().unwrap_or_else(|_| "Unknown".to_string()));

    // Get GPU devices from preferred platform only
    let device_ids = preferred_platform.get_devices(CL_DEVICE_TYPE_GPU)
        .map_err(|e| format!("No GPUs found: {}", e))?;

    if device_ids.is_empty() {
        return Ok(Vec::new());
    }

    // Iterate through GPU device IDs from the preferred platform
    for device_id in device_ids.iter() {
        // Wrap device ID in Device struct
        let device = Device::new(*device_id);

        // Extract real GPU device info using OpenCL APIs
        let model_name = device
            .name()
            .unwrap_or_else(|_| format!("OpenCL GPU {}", device_index));

        let vendor_str = device
            .vendor()
            .unwrap_or_else(|_| String::from("Unknown"));

        // Determine vendor from vendor string
        let vendor = if vendor_str.to_lowercase().contains("nvidia") {
            GpuVendor::Nvidia
        } else if vendor_str.to_lowercase().contains("amd")
               || vendor_str.to_lowercase().contains("advanced micro devices") {
            GpuVendor::Amd
        } else if vendor_str.to_lowercase().contains("intel") {
            GpuVendor::Intel
        } else {
            GpuVendor::Other
        };

        // Get OpenCL C version for compute capability
        let compute_capability = device
            .opencl_c_version()
            .ok();

        devices.push(GpuDevice {
            device_index: device_index as u32,
            model_name,
            vendor,
            opencl_capable: true,
            compute_capability,
        });
    }

    Ok(devices)
}

/// Enumerate GPUs using sysinfo (fallback method)
fn enumerate_gpus_sysinfo() -> Result<Vec<GpuDevice>, String> {
    use sysinfo::Components;

    let mut devices = Vec::new();
    let mut device_index = 0u32;

    // Use sysinfo to detect GPUs via system components
    let components = Components::new_with_refreshed_list();

    // Look for GPU-related components
    for component in components.iter() {
        let label = component.label();

        // Check if this is a GPU component
        if label.to_lowercase().contains("gpu")
            || label.to_lowercase().contains("graphics")
            || label.to_lowercase().contains("video") {

            // Determine vendor from label
            let vendor = if label.to_lowercase().contains("nvidia") {
                GpuVendor::Nvidia
            } else if label.to_lowercase().contains("amd") || label.to_lowercase().contains("radeon") {
                GpuVendor::Amd
            } else if label.to_lowercase().contains("intel") {
                GpuVendor::Intel
            } else {
                GpuVendor::Other
            };

            devices.push(GpuDevice {
                device_index,
                model_name: label.to_string(),
                vendor,
                opencl_capable: false, // Unknown via sysinfo
                compute_capability: None,
            });

            device_index += 1;
        }
    }

    // If no GPUs found via components, return empty (graceful)
    Ok(devices)
}

/// Poll GPU health metrics for a specific device
///
/// Uses NVML for NVIDIA GPUs, fallback to sysinfo for AMD/Intel.
/// Returns None for unavailable sensors (graceful degradation).
///
/// # Arguments
/// * `device_index` - GPU device index to query
///
/// # Errors
/// - Returns error if GPU device not found
/// - Returns None fields for unavailable sensors (not an error)
///
/// # Implementation (T015)
/// PHASE 1: Basic sysinfo implementation with placeholder NVML support
/// PHASE 2: Full NVML integration (requires nvml-wrapper feature flag)
pub fn poll_gpu_health(device_index: u32) -> Result<GpuHealthMetrics, String> {
    // Try NVML for NVIDIA GPUs first (if feature enabled)
    #[cfg(feature = "nvml-wrapper")]
    {
        if let Ok(metrics) = poll_gpu_health_nvml(device_index) {
            return Ok(metrics);
        }
    }

    // Fallback to sysinfo for basic temperature monitoring
    poll_gpu_health_sysinfo(device_index)
}

/// Poll GPU health using NVML (NVIDIA Management Library)
///
/// Feature-gated: Only compiled when "nvml-wrapper" feature is enabled
#[cfg(feature = "nvml-wrapper")]
fn poll_gpu_health_nvml(device_index: u32) -> Result<GpuHealthMetrics, String> {
    use nvml_wrapper::Nvml;

    let nvml = Nvml::init().map_err(|e| format!("NVML initialization failed: {}", e))?;

    let device = nvml.device_by_index(device_index)
        .map_err(|e| format!("GPU device {} not found: {}", device_index, e))?;

    // Query temperature (always available on NVIDIA GPUs)
    let temperature = device.temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu)
        .ok();

    // Query fan speed (may not be available on all cards)
    let fan_speed = device.fan_speed(0).ok();

    // Query power consumption (requires power management support)
    let power_consumption = device.power_usage()
        .ok()
        .map(|milliwatts| milliwatts as f32 / 1000.0); // Convert mW to W

    // Query memory info
    let memory_info = device.memory_info().ok();
    let memory_used = memory_info.as_ref().map(|info| info.used / (1024 * 1024)); // Bytes to MB
    let memory_total = memory_info.as_ref().map(|info| info.total / (1024 * 1024));

    // Query core clock speed
    let core_clock_speed = device.clock_info(nvml_wrapper::enum_wrappers::device::Clock::Graphics)
        .ok();

    Ok(GpuHealthMetrics {
        gpu_device_index: device_index,
        temperature: temperature.map(|t| t as f32),
        fan_speed,
        power_consumption,
        memory_used,
        memory_total,
        core_clock_speed,
        last_updated: Instant::now(),
    })
}

/// Poll GPU health using sysinfo (cross-platform fallback)
///
/// Limited sensor data compared to NVML/ADL, but works across all vendors
fn poll_gpu_health_sysinfo(device_index: u32) -> Result<GpuHealthMetrics, String> {
    use sysinfo::Components;

    let components = Components::new_with_refreshed_list();

    // Find GPU component by index
    let mut gpu_component_index = 0u32;
    let mut temperature: Option<f32> = None;

    for component in components.iter() {
        let label = component.label().to_lowercase();

        // Check if this is a GPU component
        if label.contains("gpu") || label.contains("graphics") || label.contains("video") {
            if gpu_component_index == device_index {
                // Found the target GPU - read temperature
                temperature = Some(component.temperature());
                break;
            }
            gpu_component_index += 1;
        }
    }

    if temperature.is_none() {
        return Err(format!("GPU device {} not found or no temperature sensor available", device_index));
    }

    // sysinfo doesn't provide fan speed, power, memory, or clock data
    // Return metrics with only temperature available
    Ok(GpuHealthMetrics {
        gpu_device_index: device_index,
        temperature,
        fan_speed: None,           // Not available via sysinfo
        power_consumption: None,   // Not available via sysinfo
        memory_used: None,         // Not available via sysinfo
        memory_total: None,        // Not available via sysinfo
        core_clock_speed: None,    // Not available via sysinfo
        last_updated: Instant::now(),
    })
}

/// Poll health metrics for all GPUs
///
/// Convenience function to poll all enumerated GPUs.
/// Filters out errors and returns successful results only.
///
/// # Implementation (T015)
/// Enumerates all GPUs and polls health for each device
pub fn poll_all_gpu_health() -> Vec<GpuHealthMetrics> {
    // Enumerate all available GPUs
    let devices = match enumerate_gpus() {
        Ok(devs) => devs,
        Err(_) => return vec![], // No GPUs or enumeration failed
    };

    // Poll health for each device, filtering out errors
    devices
        .iter()
        .filter_map(|device| poll_gpu_health(device.device_index).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_device_serialization() {
        let device = GpuDevice {
            device_index: 0,
            model_name: "Test GPU".to_string(),
            vendor: GpuVendor::Nvidia,
            opencl_capable: true,
            compute_capability: Some("8.6".to_string()),
        };

        let json = serde_json::to_string(&device).unwrap();
        let deserialized: GpuDevice = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.device_index, 0);
        assert_eq!(deserialized.vendor, GpuVendor::Nvidia);
    }

    #[test]
    fn test_gpu_health_metrics_graceful_none() {
        let metrics = GpuHealthMetrics {
            gpu_device_index: 0,
            temperature: Some(65.0),
            fan_speed: None,  // Unavailable sensor
            power_consumption: Some(150.0),
            memory_used: None,  // Unavailable sensor
            memory_total: Some(8192),
            core_clock_speed: Some(1800),
            last_updated: Instant::now(),
        };

        // Verify None fields serialize correctly
        let json = serde_json::to_string(&metrics).unwrap();
        assert!(json.contains("null"));
    }
}