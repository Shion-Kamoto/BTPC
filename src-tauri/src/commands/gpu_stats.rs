//! GPU Statistics and Detection Commands
//!
//! Tauri commands for GPU detection and monitoring.
//! Exposes GPU information to the frontend UI.

use crate::gpu_detection::{detect_gpus, get_best_gpu, has_gpu, GpuInfo};
use serde::{Deserialize, Serialize};

/// GPU information for frontend consumption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfoResponse {
    pub name: String,
    pub vendor: String,
    pub is_available: bool,
}

impl From<GpuInfo> for GpuInfoResponse {
    fn from(info: GpuInfo) -> Self {
        Self {
            name: info.name,
            vendor: info.vendor,
            is_available: info.is_available,
        }
    }
}

/// Get list of all available GPUs on the system
///
/// # Frontend Usage
/// ```javascript
/// const gpus = await window.invoke('get_available_gpus');
/// console.log('Found GPUs:', gpus);
/// ```
///
/// # Returns
/// - `Ok(Vec<GpuInfoResponse>)` - List of detected GPUs (empty if none found)
/// - `Err(String)` - Should never error, returns empty vec on failure
#[tauri::command]
pub async fn get_available_gpus() -> Result<Vec<GpuInfoResponse>, String> {
    let gpus = detect_gpus();
    Ok(gpus.into_iter().map(|g| g.into()).collect())
}

/// Check if any GPU is available for mining
///
/// # Frontend Usage
/// ```javascript
/// const hasGpu = await window.invoke('check_gpu_available');
/// if (hasGpu) {
///     console.log('GPU mining available');
/// }
/// ```
///
/// # Returns
/// - `Ok(true)` - At least one GPU detected
/// - `Ok(false)` - No GPU detected
#[tauri::command]
pub async fn check_gpu_available() -> Result<bool, String> {
    Ok(has_gpu())
}

/// Get recommended GPU for mining (based on vendor priority: NVIDIA > AMD > Intel)
///
/// # Frontend Usage
/// ```javascript
/// const bestGpu = await window.invoke('get_recommended_gpu');
/// if (bestGpu) {
///     console.log('Best GPU:', bestGpu.name, '(' + bestGpu.vendor + ')');
/// } else {
///     console.log('No GPU available for mining');
/// }
/// ```
///
/// # Returns
/// - `Ok(Some(GpuInfoResponse))` - Best GPU found
/// - `Ok(None)` - No GPU available
#[tauri::command]
pub async fn get_recommended_gpu() -> Result<Option<GpuInfoResponse>, String> {
    Ok(get_best_gpu().map(|g| g.into()))
}