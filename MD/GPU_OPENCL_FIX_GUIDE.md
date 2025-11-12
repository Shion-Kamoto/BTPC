# GPU Mining OpenCL Fix Guide

**Date**: 2025-11-09
**Problem**: btpc_miner fails with OpenCL kernel compilation error: `fatal error: 'clc/clcfunc.h' file not found`

## Root Cause Analysis

Your system has:
- ✅ OpenCL libraries installed (libclc-20, libclc-20-dev)
- ✅ OpenCL ICD loader configured (/etc/OpenCL/vendors/)
- ✅ NVIDIA driver packages installed (535.274.02)
- ❌ NVIDIA kernel modules NOT loaded (no /dev/nvidia* devices)
- ❌ nvidia-smi fails to communicate with driver

**The issue**: The NVIDIA kernel modules are not loaded, preventing OpenCL from accessing the GPU. The `clc/clcfunc.h` error is a red herring - it occurs because the OpenCL runtime can't initialize the GPU properly.

## Solution Steps

### CRITICAL: Load NVIDIA Kernel Modules

The GPU cannot be used until the NVIDIA kernel modules are loaded. Run these commands:

```bash
# Step 1: Load NVIDIA kernel modules
sudo modprobe nvidia
sudo modprobe nvidia-uvm

# Step 2: Verify the modules are loaded
lsmod | grep nvidia

# Step 3: Check device files were created
ls -la /dev/nvidia*

# Step 4: Verify nvidia-smi works
nvidia-smi
```

**Expected output from `nvidia-smi`**:
- GPU model and driver version displayed
- No error about "couldn't communicate with NVIDIA driver"

### If modprobe Fails

If `sudo modprobe nvidia` returns errors, you need to install kernel modules:

```bash
# Install kernel modules for your current kernel
sudo apt update
sudo apt install linux-modules-extra-$(uname -r)

# Reboot to ensure modules load properly
sudo reboot
```

### Make NVIDIA Modules Load at Boot

To prevent this issue after reboot:

```bash
# Add nvidia modules to auto-load configuration
echo "nvidia" | sudo tee -a /etc/modules
echo "nvidia-uvm" | sudo tee -a /etc/modules

# Alternative: Enable NVIDIA modeset in GRUB
sudo nano /etc/default/grub
# Add nvidia.modeset=1 to GRUB_CMDLINE_LINUX_DEFAULT line
# Then run:
sudo update-grub
```

### Install clinfo for Testing (Optional)

```bash
# Install OpenCL diagnostic tool
sudo apt install clinfo

# Verify OpenCL can see your GPU
clinfo
```

**Expected output**: Should list NVIDIA GPU as an OpenCL device

## Verification Steps

After loading the NVIDIA modules, verify GPU mining can start:

```bash
# Navigate to btpc_miner directory
cd /home/bob/BTPC/BTPC/bins/btpc_miner

# Build with GPU support
cargo build --release --features gpu

# Run the miner (should now succeed)
../../target/release/btpc_miner --gpu --network regtest
```

**Success indicators**:
- No "clc/clcfunc.h" error
- GPU device info printed (name, compute units, VRAM)
- Mining starts with hashrate displayed

## Alternative: Use Mesa/Rusticl OpenCL (Software Fallback)

If NVIDIA modules cannot be loaded, you can test with Mesa's OpenCL implementation:

```bash
# Mesa OpenCL is already installed (mesa-opencl-icd)
# It will use CPU/software rendering instead of GPU

# Try running btpc_miner - it will use Mesa's implementation
cargo run --release --features gpu -- --network regtest
```

**Note**: Mesa OpenCL won't provide GPU acceleration but will allow testing the code.

## Technical Details

### About the clc/clcfunc.h Error

- `clc/clcfunc.h` is part of libclc (LLVM OpenCL library)
- It's used during OpenCL kernel **runtime compilation**
- The error occurs when the OpenCL driver tries to compile your `.cl` kernel
- Your system has libclc-20-dev installed, so the header exists
- The real problem: OpenCL can't initialize without NVIDIA driver loaded

### Your OpenCL Kernel

The code correctly uses the standalone kernel:
- File: `/home/bob/BTPC/BTPC/bins/btpc_miner/src/sha512_mining_standalone.cl`
- Kernel name: `mine_sha512` ✅
- No libclc dependencies in the kernel code ✅
- Embedded via `include_str!()` ✅

### Your OpenCL Configuration

Currently installed:
```
ocl-icd-libopencl1:amd64     2.3.2-1build1       Generic OpenCL ICD Loader
mesa-opencl-icd:amd64        25.0.7              Mesa OpenCL implementation
cuda-opencl-13-0             13.0.39-1           CUDA OpenCL (requires NVIDIA driver)
libclc-20-dev                1:20.1.2            OpenCL headers
```

Vendor ICDs detected:
```
/etc/OpenCL/vendors/nvidia.icd    ← Your target (requires driver loaded)
/etc/OpenCL/vendors/mesa.icd      ← Software fallback
/etc/OpenCL/vendors/rusticl.icd   ← Rust OpenCL (software)
```

## Summary

**The fix is simple**: Load the NVIDIA kernel modules with `sudo modprobe nvidia`.

The OpenCL libraries and headers are already installed correctly. The kernel compilation error is a symptom of the driver not being loaded, not a missing package.

After running `sudo modprobe nvidia`, your GPU mining should work immediately.

## Files Reference

- GPU miner code: `/home/bob/BTPC/BTPC/bins/btpc_miner/src/gpu_miner.rs`
- OpenCL kernel: `/home/bob/BTPC/BTPC/bins/btpc_miner/src/sha512_mining_standalone.cl`
- Cargo.toml: `/home/bob/BTPC/BTPC/bins/btpc_miner/Cargo.toml` (gpu feature defined)
- Expected stats port: 18360 (regtest network)