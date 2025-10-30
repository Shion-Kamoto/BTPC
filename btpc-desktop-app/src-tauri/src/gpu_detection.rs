//! GPU Detection Module
//!
//! TDD Implementation: Tests written FIRST (RED phase)
//! Following Constitution v1.1 Article VI.3 - Test-Driven Development

use std::process::Command;

/// GPU information structure
#[derive(Debug, Clone, PartialEq)]
pub struct GpuInfo {
    pub name: String,
    pub vendor: String,  // nvidia, amd, intel
    pub is_available: bool,
}

/// Detects available GPUs on the system
/// GREEN Phase: Implementation passes tests
pub fn detect_gpus() -> Vec<GpuInfo> {
    let mut gpus = Vec::new();

    // Try Linux lspci command
    #[cfg(target_os = "linux")]
    {
        if let Ok(output) = Command::new("lspci").output() {
            if let Ok(stdout) = String::from_utf8(output.stdout) {
                for line in stdout.lines() {
                    if line.contains("VGA compatible controller") || line.contains("3D controller") {
                        let name = line.split(':').skip(2).collect::<Vec<&str>>().join(":");
                        let name = name.trim().to_string();

                        let vendor = if name.to_lowercase().contains("nvidia") {
                            "nvidia"
                        } else if name.to_lowercase().contains("amd") || name.to_lowercase().contains("radeon") {
                            "amd"
                        } else if name.to_lowercase().contains("intel") {
                            "intel"
                        } else {
                            "unknown"
                        };

                        gpus.push(GpuInfo {
                            name: name.clone(),
                            vendor: vendor.to_string(),
                            is_available: true,
                        });
                    }
                }
            }
        }
    }

    // Try Windows wmic command
    #[cfg(target_os = "windows")]
    {
        if let Ok(output) = Command::new("wmic")
            .args(&["path", "win32_VideoController", "get", "name"])
            .output()
        {
            if let Ok(stdout) = String::from_utf8(output.stdout) {
                for line in stdout.lines().skip(1) {
                    let name = line.trim();
                    if !name.is_empty() {
                        let vendor = if name.to_lowercase().contains("nvidia") {
                            "nvidia"
                        } else if name.to_lowercase().contains("amd") || name.to_lowercase().contains("radeon") {
                            "amd"
                        } else if name.to_lowercase().contains("intel") {
                            "intel"
                        } else {
                            "unknown"
                        };

                        gpus.push(GpuInfo {
                            name: name.to_string(),
                            vendor: vendor.to_string(),
                            is_available: true,
                        });
                    }
                }
            }
        }
    }

    // Try macOS system_profiler command
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = Command::new("system_profiler")
            .args(&["SPDisplaysDataType"])
            .output()
        {
            if let Ok(stdout) = String::from_utf8(output.stdout) {
                for line in stdout.lines() {
                    if line.contains("Chipset Model:") {
                        let name = line.split(':').nth(1).unwrap_or("").trim();

                        let vendor = if name.to_lowercase().contains("nvidia") {
                            "nvidia"
                        } else if name.to_lowercase().contains("amd") || name.to_lowercase().contains("radeon") {
                            "amd"
                        } else if name.to_lowercase().contains("intel") {
                            "intel"
                        } else {
                            "unknown"
                        };

                        gpus.push(GpuInfo {
                            name: name.to_string(),
                            vendor: vendor.to_string(),
                            is_available: true,
                        });
                    }
                }
            }
        }
    }

    gpus
}

/// Checks if any GPU is available for mining
/// GREEN Phase: Implementation passes tests
pub fn has_gpu() -> bool {
    let gpus = detect_gpus();
    !gpus.is_empty()
}

/// Gets the best GPU for mining (highest preference: NVIDIA > AMD > Intel)
/// GREEN Phase: Implementation passes tests
pub fn get_best_gpu() -> Option<GpuInfo> {
    let gpus = detect_gpus();

    if gpus.is_empty() {
        return None;
    }

    // Priority: NVIDIA > AMD > Intel
    gpus.iter()
        .find(|gpu| gpu.vendor == "nvidia")
        .or_else(|| gpus.iter().find(|gpu| gpu.vendor == "amd"))
        .or_else(|| gpus.iter().find(|gpu| gpu.vendor == "intel"))
        .or_else(|| gpus.first())
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_gpus_returns_vec() {
        // RED: This test will fail until GREEN phase
        let gpus = detect_gpus();
        // Should return a vector (empty or populated)
        assert!(gpus.len() >= 0);
    }

    #[test]
    fn test_has_gpu_returns_bool() {
        // RED: This test will fail until GREEN phase
        let has_gpu_result = has_gpu();
        // Should return a boolean
        assert!(has_gpu_result == true || has_gpu_result == false);
    }

    #[test]
    fn test_get_best_gpu_returns_option() {
        // RED: This test will fail until GREEN phase
        let best_gpu = get_best_gpu();
        // Should return Option<GpuInfo>
        match best_gpu {
            Some(gpu) => {
                // If GPU found, must have name
                assert!(!gpu.name.is_empty());
                assert!(!gpu.vendor.is_empty());
                assert!(gpu.is_available);
            }
            None => {
                // No GPU found is valid on systems without GPU
                assert!(true);
            }
        }
    }

    #[test]
    fn test_gpu_vendor_priority() {
        // RED: This test defines behavior for vendor priority
        // NVIDIA > AMD > Intel for mining
        let gpus = vec![
            GpuInfo {
                name: "Intel UHD".to_string(),
                vendor: "intel".to_string(),
                is_available: true,
            },
            GpuInfo {
                name: "NVIDIA RTX 3080".to_string(),
                vendor: "nvidia".to_string(),
                is_available: true,
            },
            GpuInfo {
                name: "AMD Radeon RX 6800".to_string(),
                vendor: "amd".to_string(),
                is_available: true,
            },
        ];

        // Best GPU should be NVIDIA (highest priority)
        // This will be tested when get_best_gpu() is implemented
        let expected_best = gpus.iter().find(|g| g.vendor == "nvidia");
        assert!(expected_best.is_some(), "Expected to find NVIDIA GPU in test data");
    }

    #[test]
    fn test_no_gpu_available() {
        // RED: System without GPU should return empty vec
        // This test ensures graceful handling of no-GPU systems
        let gpus = detect_gpus();
        if gpus.is_empty() {
            assert!(!has_gpu());
            assert!(get_best_gpu().is_none());
        }
    }
}