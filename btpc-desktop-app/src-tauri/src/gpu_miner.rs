

//! GPU Mining Module
//!
//! Implements OpenCL-based GPU mining for BTPC using SHA-512 proof-of-work.
//!
//! Architecture:
//! - One OpenCL context per GPU device
//! - Parallel nonce search across work-items
//! - Atomic result reporting (first valid nonce wins)

use anyhow::{anyhow, Result};
use btpc_core::blockchain::BlockHeader;
use btpc_core::consensus::pow::MiningTarget;
use btpc_core::crypto::Hash;
use opencl3::command_queue::{CommandQueue, CL_QUEUE_PROFILING_ENABLE};
use opencl3::context::Context;
use opencl3::device::{get_all_devices, Device, CL_DEVICE_TYPE_GPU, CL_DEVICE_TYPE_ALL};
use opencl3::kernel::{Kernel, ExecuteKernel};
use opencl3::memory::{Buffer, CL_MEM_READ_ONLY, CL_MEM_WRITE_ONLY};
use opencl3::platform::get_platforms;
use opencl3::program::Program;
use opencl3::types::{cl_uint, CL_BLOCKING, CL_NON_BLOCKING};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// OpenCL kernel source (embedded at compile time)
const KERNEL_SOURCE: &str = include_str!("sha512_kernel.cl");

/// Work group size (tuned for most GPUs)
const WORK_GROUP_SIZE: usize = 256;

/// Nonces to try per kernel execution
pub const NONCES_PER_BATCH: u32 = 1_000_000;

/// GPU device information
#[derive(Debug, Clone)]
pub struct GpuDevice {
    pub device_index: u32,
    pub model_name: String,
    pub vendor: String,
    pub compute_units: u32,
    pub max_clock_frequency: u32,
    pub global_mem_size: u64,
}

/// GPU Miner for a single device
pub struct GpuMiner {
    device: Device,
    device_index: u32,
    context: Context,
    queue: CommandQueue,
    kernel: Kernel,

    // Buffers (mutable for enqueue operations)
    prev_hash_buffer: Buffer<u8>,
    merkle_root_buffer: Buffer<u8>,
    target_buffer: Buffer<u8>,
    result_buffer: Buffer<cl_uint>,

    // Stats
    hashes_computed: Arc<AtomicU64>,
    blocks_found: Arc<AtomicU64>,
    is_mining: Arc<AtomicBool>,
}

impl GpuMiner {
    /// Create a new GPU miner for the specified device
    pub fn new(device_index: u32) -> Result<Self> {
        // Get all GPU devices from ALL platforms (Clover has the GPU, not rusticl)
        let platforms = get_platforms().map_err(|e| anyhow!("Failed to get OpenCL platforms: {}", e))?;

        // Collect devices from ALL platforms
        let mut all_devices = Vec::new();
        for platform in platforms.iter() {
            let device_ids = match platform.get_devices(CL_DEVICE_TYPE_GPU) {
                Ok(ids) if !ids.is_empty() => ids,
                _ => match platform.get_devices(CL_DEVICE_TYPE_ALL) {
                    Ok(ids) if !ids.is_empty() => ids,
                    _ => continue,
                }
            };
            for device_id in device_ids {
                all_devices.push(device_id);
            }
        }

        if all_devices.is_empty() {
            return Err(anyhow!("No GPU devices found"));
        }

        if device_index as usize >= all_devices.len() {
            return Err(anyhow!("Device index {} out of range (found {} devices)",
                device_index, all_devices.len()));
        }

        let device = Device::new(all_devices[device_index as usize]);

        // Create OpenCL context
        let context = Context::from_device(&device)
            .map_err(|e| anyhow!("Failed to create OpenCL context: {}", e))?;

        // Create command queue with profiling enabled
        let queue = CommandQueue::create_default_with_properties(
            &context,
            CL_QUEUE_PROFILING_ENABLE,
            0
        ).map_err(|e| anyhow!("Failed to create command queue: {}", e))?;

        // Build program from kernel source
        // NOTE: Mesa libclc headers fixed with stub clctypes.h (2025-11-13)
        // Default build options work, or use -cl-fast-relaxed-math for optimization
        const BUILD_OPTIONS: &str = "-w"; // Suppress warnings

        let program = match Program::create_and_build_from_source(&context, KERNEL_SOURCE, BUILD_OPTIONS) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("❌ OpenCL kernel compilation failed: {}", e);

                // Try to get build log from the program (even though it failed)
                // This requires creating the program first without building
                match Program::create_from_source(&context, KERNEL_SOURCE) {
                    Ok(mut prog) => {
                        // Try to build it to get build log
                        match prog.build(&[device.id()], BUILD_OPTIONS) {
                            Ok(_) => {
                                eprintln!("✅ Second build attempt succeeded unexpectedly");
                                prog
                            }
                            Err(build_err) => {
                                // Get build log
                                match prog.get_build_log(device.id()) {
                                    Ok(log) => {
                                        eprintln!("📋 OpenCL Build Log:\n{}", log);
                                        return Err(anyhow!("OpenCL kernel build failed:\n{}", log));
                                    }
                                    Err(log_err) => {
                                        eprintln!("⚠️ Could not get build log: {}", log_err);
                                        return Err(anyhow!("OpenCL kernel build failed: {}, log error: {}", build_err, log_err));
                                    }
                                }
                            }
                        }
                    }
                    Err(create_err) => {
                        eprintln!("❌ Could not even create program: {}", create_err);
                        return Err(anyhow!("Failed to create OpenCL program: {}", create_err));
                    }
                }
            }
        };

        // Create kernel
        let kernel = Kernel::create(&program, "mine_block")
            .map_err(|e| anyhow!("Failed to create kernel: {}", e))?;

        // Create buffers (will be populated per mining session)
        let prev_hash_buffer = unsafe {
            Buffer::<u8>::create(&context, CL_MEM_READ_ONLY, 64, std::ptr::null_mut())
                .map_err(|e| anyhow!("Failed to create prev_hash buffer: {}", e))?
        };

        let merkle_root_buffer = unsafe {
            Buffer::<u8>::create(&context, CL_MEM_READ_ONLY, 64, std::ptr::null_mut())
                .map_err(|e| anyhow!("Failed to create merkle_root buffer: {}", e))?
        };

        let target_buffer = unsafe {
            Buffer::<u8>::create(&context, CL_MEM_READ_ONLY, 64, std::ptr::null_mut())
                .map_err(|e| anyhow!("Failed to create target buffer: {}", e))?
        };

        let result_buffer = unsafe {
            Buffer::<cl_uint>::create(&context, CL_MEM_WRITE_ONLY, 1, std::ptr::null_mut())
                .map_err(|e| anyhow!("Failed to create result buffer: {}", e))?
        };

        Ok(GpuMiner {
            device,
            device_index,
            context,
            queue,
            kernel,
            prev_hash_buffer,
            merkle_root_buffer,
            target_buffer,
            result_buffer,
            hashes_computed: Arc::new(AtomicU64::new(0)),
            blocks_found: Arc::new(AtomicU64::new(0)),
            is_mining: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Get GPU device information
    pub fn get_device_info(&self) -> Result<GpuDevice> {
        let model_name = self.device.name()
            .map_err(|e| anyhow!("Failed to get device name: {}", e))?;

        let vendor = self.device.vendor()
            .map_err(|e| anyhow!("Failed to get vendor: {}", e))?;

        let compute_units = self.device.max_compute_units()
            .map_err(|e| anyhow!("Failed to get compute units: {}", e))?;

        let max_clock_frequency = self.device.max_clock_frequency()
            .map_err(|e| anyhow!("Failed to get clock frequency: {}", e))?;

        let global_mem_size = self.device.global_mem_size()
            .map_err(|e| anyhow!("Failed to get memory size: {}", e))?;

        Ok(GpuDevice {
            device_index: self.device_index,
            model_name,
            vendor,
            compute_units,
            max_clock_frequency,
            global_mem_size,
        })
    }

    /// Mine a block on GPU
    ///
    /// Returns `Some(nonce)` if valid proof found, `None` if batch exhausted
    pub fn mine_batch(
        &mut self,
        header: &BlockHeader,
        target: &MiningTarget,
        nonce_start: u32,
    ) -> Result<Option<u32>> {
        // Upload block data to GPU
        self.upload_header_data(header, target)?;

        // Initialize result to NOT_FOUND marker
        let not_found: cl_uint = 0xFFFFFFFF;
        unsafe {
            self.queue.enqueue_write_buffer(&mut self.result_buffer, CL_BLOCKING, 0, &[not_found], &[])
                .map_err(|e| anyhow!("Failed to write result buffer: {}", e))?;
        }

        // Calculate work sizes - query device for max work group size
        let max_work_group_size = self.kernel.get_work_group_size(self.device.id())
            .unwrap_or(64); // Fallback to 64 if query fails

        // Use smaller of: device max, our preferred (256), or kernel's max
        let local_work_size = std::cmp::min(max_work_group_size, WORK_GROUP_SIZE);

        // Ensure global work size is multiple of local work size
        let global_work_size = ((NONCES_PER_BATCH as usize + local_work_size - 1) / local_work_size) * local_work_size;

        // Set kernel arguments and execute
        let kernel_event = unsafe {
            ExecuteKernel::new(&self.kernel)
                .set_arg(&header.version)
                .set_arg(&self.prev_hash_buffer)
                .set_arg(&self.merkle_root_buffer)
                .set_arg(&header.timestamp)
                .set_arg(&header.bits)
                .set_arg(&nonce_start)
                .set_arg(&self.target_buffer)
                .set_arg(&self.result_buffer)
                .set_global_work_size(global_work_size)
                .set_local_work_size(local_work_size)
                .enqueue_nd_range(&self.queue)
                .map_err(|e| anyhow!("Failed to enqueue kernel: {}", e))?
        };

        // Wait for kernel to complete
        kernel_event.wait()
            .map_err(|e| anyhow!("Failed to wait for kernel: {}", e))?;

        // Read result
        let mut result = vec![0u32; 1];
        unsafe {
            self.queue.enqueue_read_buffer(&self.result_buffer, CL_BLOCKING, 0, &mut result, &[])
                .map_err(|e| anyhow!("Failed to read result: {}", e))?;
        }

        // Update hash count
        self.hashes_computed.fetch_add(NONCES_PER_BATCH as u64, Ordering::Relaxed);

        // Check if valid nonce found
        if result[0] != 0xFFFFFFFF {
            self.blocks_found.fetch_add(1, Ordering::Relaxed);
            Ok(Some(result[0]))
        } else {
            Ok(None)
        }
    }

    /// Upload header data to GPU buffers
    fn upload_header_data(&mut self, header: &BlockHeader, target: &MiningTarget) -> Result<()> {
        // Upload prev_hash
        let prev_hash_bytes = header.prev_hash.as_slice();
        unsafe {
            self.queue.enqueue_write_buffer(&mut self.prev_hash_buffer, CL_NON_BLOCKING, 0, prev_hash_bytes, &[])
                .map_err(|e| anyhow!("Failed to upload prev_hash: {}", e))?;
        }

        // Upload merkle_root
        let merkle_root_bytes = header.merkle_root.as_slice();
        unsafe {
            self.queue.enqueue_write_buffer(&mut self.merkle_root_buffer, CL_NON_BLOCKING, 0, merkle_root_bytes, &[])
                .map_err(|e| anyhow!("Failed to upload merkle_root: {}", e))?;
        }

        // Upload target
        let target_hash = target.as_hash();
        let target_bytes = target_hash.as_slice();
        unsafe {
            self.queue.enqueue_write_buffer(&mut self.target_buffer, CL_NON_BLOCKING, 0, target_bytes, &[])
                .map_err(|e| anyhow!("Failed to upload target: {}", e))?;
        }

        // Wait for uploads to complete
        self.queue.finish()
            .map_err(|e| anyhow!("Failed to finish queue: {}", e))?;

        Ok(())
    }

    /// Get total hashes computed
    pub fn get_hashes_computed(&self) -> u64 {
        self.hashes_computed.load(Ordering::Relaxed)
    }

    /// Get blocks found
    pub fn get_blocks_found(&self) -> u64 {
        self.blocks_found.load(Ordering::Relaxed)
    }

    /// Check if mining is active
    pub fn is_mining(&self) -> bool {
        self.is_mining.load(Ordering::Relaxed)
    }

    /// Set mining status
    pub fn set_mining(&self, active: bool) {
        self.is_mining.store(active, Ordering::Relaxed);
    }
}

/// Enumerate all available GPU devices
pub fn enumerate_gpu_devices() -> Result<Vec<GpuDevice>> {
    let platforms = get_platforms()
        .map_err(|e| anyhow!("Failed to get OpenCL platforms: {}", e))?;

    eprintln!("🔍 Found {} OpenCL platform(s)", platforms.len());

    // Try ALL platforms to find GPUs (don't assume rusticl has them)
    let mut all_devices = Vec::new();

    for (idx, platform) in platforms.iter().enumerate() {
        let platform_name = platform.name().unwrap_or_default();
        eprintln!("  Platform {}: {}", idx, platform_name);

        // Try GPU type first
        let device_ids = match platform.get_devices(CL_DEVICE_TYPE_GPU) {
            Ok(ids) if !ids.is_empty() => {
                eprintln!("    ✅ Found {} GPU device(s)", ids.len());
                ids
            }
            _ => {
                // Try ALL devices as fallback
                match platform.get_devices(CL_DEVICE_TYPE_ALL) {
                    Ok(ids) if !ids.is_empty() => {
                        eprintln!("    ⚠️ GPU query empty, CL_DEVICE_TYPE_ALL found {} device(s)", ids.len());
                        ids
                    }
                    _ => {
                        eprintln!("    ❌ No devices found on this platform");
                        continue;
                    }
                }
            }
        };

        // Collect devices from this platform
        for device_id in device_ids {
            all_devices.push(device_id);
        }
    }

    if all_devices.is_empty() {
        return Err(anyhow!("No GPU devices found on any platform"));
    }

    eprintln!("🎮 Found {} total GPU device(s) across all platforms", all_devices.len());

    let mut devices = Vec::new();

    for (device_index, device_id) in all_devices.iter().enumerate() {
        let device = Device::new(*device_id);
        let model_name = device.name().unwrap_or_else(|_| "Unknown".to_string());
        let vendor = device.vendor().unwrap_or_else(|_| "Unknown".to_string());
        let compute_units = device.max_compute_units().unwrap_or(0);
        let max_clock_frequency = device.max_clock_frequency().unwrap_or(0);
        let global_mem_size = device.global_mem_size().unwrap_or(0);

        eprintln!("  Device {}: {} ({} CUs, {}MHz, {} MB)", device_index, model_name, compute_units, max_clock_frequency, global_mem_size / 1024 / 1024);

        devices.push(GpuDevice {
            device_index: device_index as u32,
            model_name,
            vendor,
            compute_units,
            max_clock_frequency,
            global_mem_size,
        });
    }

    Ok(devices)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enumerate_devices() {
        // This test may fail on systems without GPUs
        match enumerate_gpu_devices() {
            Ok(devices) => {
                println!("Found {} GPU device(s)", devices.len());
                for device in devices {
                    println!("  - {} ({})", device.model_name, device.vendor);
                }
            }
            Err(e) => {
                println!("No GPUs found: {}", e);
            }
        }
    }
}