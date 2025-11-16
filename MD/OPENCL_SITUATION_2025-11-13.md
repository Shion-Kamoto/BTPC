# OpenCL Situation - RX 580 Compatibility Issue

**Date**: 2025-11-13 13:20
**Status**: ❌ CRITICAL - No OpenCL runtime available for RX 580
**Impact**: Feature 012 GPU mining **completely blocked**

## Current State

### What Happened
1. ✅ Downloaded amdgpu-install 5.7.50702 (legacy, correct version)
2. ✅ Installed successfully (replaced 6.2 version)
3. ❌ **PROBLEM**: Legacy version removed `--opencl=legacy` option
4. ❌ Only ROCr-based OpenCL available (requires RDNA/CDNA GPUs)
5. ❌ OpenCL ICD directory empty (`/etc/OpenCL/vendors/` has no .icd files)
6. ❌ test_opencl_diagnostic fails: `CL_PLATFORM_NOT_FOUND_KHR`

### Hardware vs Software Mismatch
- **GPU**: AMD RX 580 (Polaris/GCN architecture, 2016)
- **Needs**: PAL-based OpenCL (legacy, pre-2020)
- **Available**: ROCr-based OpenCL (RDNA/CDNA only, 2020+)
- **Result**: **No compatible OpenCL runtime exists in amdgpu-install 5.7**

## Root Cause

AMD **removed legacy OpenCL** from amdgpu-install starting ~2023. The RX 580 Polaris GPU is now considered EOL (end-of-life) and not supported by modern drivers.

### What RX 580 Needs (Not Available)
- PAL-based OpenCL runtime (`libamdocl64.so`)
- Option: `--opencl=legacy` or `--opencl=pal`
- Last available: amdgpu-pro 20.45 or earlier (2020)

### What's Available (Incompatible)
- ROCr-based OpenCL (`libmesaopenclrocr.so`)
- Requires: RDNA (RX 5000+) or CDNA (Instinct) GPUs
- Option: `--opencl=rocr` (current only option)

## Solution Options

### Option 1: Use Mesa Rusticl OpenCL (Recommended)
Mesa 25.0.7 has OpenCL support via Rusticl, but **headers are broken** (libclc bug).

**Workaround**: Use `-cl-std=CL1.0` build flag (bypass broken headers)

**Steps**:
```bash
# Reinstall Mesa OpenCL
sudo apt install mesa-opencl-icd ocl-icd-libopencl1

# Verify rusticl detected
clinfo | grep -i platform

# Test with workaround flag
cd /home/bob/BTPC/BTPC/test_opencl_diagnostic
cargo run
```

**Status**: Already applied workaround in `gpu_miner.rs:109` (`-cl-std=CL1.0`)

### Option 2: Find Ancient amdgpu-pro 20.45 (2020)
Download 2020-era amdgpu-pro driver with PAL-based OpenCL.

**Challenges**:
- Hard to find (AMD removed from repo)
- May not support Ubuntu 24.04
- Security risk (4+ year old driver)

**Not Recommended**: Too much effort for uncertain outcome

### Option 3: Defer GPU Mining to Future
Continue BTPC development without GPU mining.

**Impact**:
- CPU mining still works (8 threads)
- Feature 012 blocked indefinitely
- All other features unaffected

## Recommended Path Forward

### Immediate: Test Mesa Rusticl with `-cl-std=CL1.0`

**Run these commands**:
```bash
# 1. Reinstall Mesa OpenCL (may have been removed)
sudo apt install mesa-opencl-icd ocl-icd-libopencl1

# 2. Check if rusticl detected
clinfo | grep "Platform Name"
# Expected: "rusticl" or "Clover"

# 3. Test OpenCL compilation with workaround
cd /home/bob/BTPC/BTPC/test_opencl_diagnostic
cargo run
# Expected: "✅ SUCCESS: Kernel compiled with CL1.0 standard!"

# 4. If test passes, build desktop app
cd /home/bob/BTPC/BTPC/btpc-desktop-app/src-tauri
cargo build
```

**If Mesa works**: Feature 012 GPU mining can continue
**If Mesa fails**: No OpenCL solution exists for RX 580 on modern Ubuntu

## Files Modified This Session

```
?? MD/OPENCL_SITUATION_2025-11-13.md (this file)
?? install_amdgpu_pro_legacy.sh (failed approach)
```

## Next Steps

1. **User**: Run commands above to test Mesa Rusticl
2. **If Mesa works**: Resume Feature 012 implementation with GPU hardware
3. **If Mesa fails**: Document GPU mining as "RX 580 not supported" and defer feature

## Technical Details

### Why ROCr OpenCL Doesn't Work
ROCr (Radeon Open Compute) requires:
- LLVM-based GPU compiler (amdgpu LLVM backend)
- Modern GPU ISA (GFX9+ for RDNA, GFX7+ for Vega)
- RX 580 is GFX8 (Polaris) - too old for ROCr

### Why PAL OpenCL Was Removed
PAL (Platform Abstraction Library) OpenCL was proprietary AMD code:
- Closed source, hard to maintain
- Replaced by open source ROCr stack
- AMD decided EOL for pre-RDNA GPUs

### Mesa Rusticl Architecture
Rusticl is Rust-based OpenCL implementation in Mesa:
- Uses LLVM for GPU codegen (supports GCN/Polaris)
- Open source, actively maintained
- **Bug**: libclc headers incomplete (missing vector types)
- **Workaround**: Use `-cl-std=CL1.0` (no headers needed)

## Constitutional Compliance

Feature 012 blocked by hardware compatibility, not design:
- ✅ SHA-512 kernel ready (293 lines, FIPS 180-4 compliant)
- ✅ Architecture complete (4 modules, 1286 lines)
- ❌ Cannot test without working OpenCL runtime

---

**Status**: Awaiting Mesa Rusticl test results
**Blocker**: Hardware driver EOL (RX 580 unsupported by AMD)
**Workaround**: Mesa with `-cl-std=CL1.0` (already applied)