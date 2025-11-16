# OpenCL Driver Blocker - Feature 012

**Date**: 2025-11-13 10:04
**Status**: ❌ BLOCKED - OpenCL drivers broken
**Impact**: GPU mining testing blocked, architecture implementation continues

## Problem

After ROCm 6.2.4 installation and reboot, all OpenCL programs segfault:
- `clinfo` produces no output (hangs/fails silently)
- `cargo run` (opencl3 crate) segfaults immediately
- Test diagnostic tool crashes with core dump

## Root Cause

**Hardware**: AMD RX 580 (Polaris/GCN architecture)
**Issue**: ROCm 6.2.4 packages installed but OpenCL libraries missing
- `libamdocl64.so` not found anywhere in `/usr` or `/opt/rocm*`
- RX 580 requires legacy **amdgpu-pro OpenCL**, not modern ROCm 6.2+
- ROCm 6.2+ dropped support for GCN GPUs (focuses on RDNA/CDNA only)

## Installed Packages (Conflicting)

```
rocm-opencl                   2.0.0.60204-139~20.04
rocm-opencl-runtime           6.2.4.60204-139~20.04
mesa-opencl-icd               25.0.7 (with broken libclc headers)
cuda-opencl-13-0              13.0.39-1 (NVIDIA, irrelevant)
```

## ICDs Registered

```
/etc/OpenCL/vendors/
├── amdocl64_60204_139.icd → libamdocl64.so (NOT FOUND - causes segfault)
├── mesa.icd → libMesaOpenCL.so.1 (broken libclc headers)
├── rusticl.icd → libRusticlOpenCL.so.1 (Mesa fallback)
└── nvidia.icd → libnvidia-opencl.so.1 (irrelevant)
```

## Attempted Fixes

1. ✅ System reboot (completed)
2. ❌ Force ROCm library (`OCL_ICD_FILENAMES`) - library doesn't exist
3. ❌ Disable ROCm ICD, use Mesa - still segfaults
4. ❌ Test with clinfo natively - hangs/no output

## Solution Required (User Action)

**Option 1: Install amdgpu-pro OpenCL (Recommended for RX 580)**
```bash
# Uninstall broken ROCm
sudo amdgpu-install --uninstall
sudo apt remove --purge rocm-* amdgpu-core

# Install legacy amdgpu-pro with OpenCL
wget https://repo.radeon.com/amdgpu-install/latest/ubuntu/jammy/amdgpu-install_*_all.deb
sudo dpkg -i amdgpu-install_*_all.deb
sudo amdgpu-install --opencl=legacy --no-dkms
```

**Option 2: Fix Mesa OpenCL (Workaround)**
```bash
# Use pre-compiled SPIR-V kernel (bypass broken libclc headers)
# Requires: clang, llvm-spirv, pre-compile SHA-512 kernel
```

**Option 3: Defer GPU Mining**
- Continue Feature 012 implementation with architecture only
- Test GPU mining when OpenCL fixed
- All non-GPU code can be completed now

## Impact on Feature 012

**BLOCKED** ❌:
- T013: GPU enumeration (requires working OpenCL)
- T015: NVML/sensor polling (may work, but can't test without enum)
- T020: Thermal throttling (depends on health metrics)
- T030-T031: Performance tests (require GPU mining)
- End-to-end GPU mining testing

**NOT BLOCKED** ✅:
- T001: Create module files (done)
- T002: Add dependencies (done)
- T003: Persistent storage structure (filesystem only)
- T004: HTML sub-tab structure (frontend only)
- T005-T012: Contract tests (can write, will fail until OpenCL fixed)
- T016: Backend data structures (Rust structs, no hardware)
- T017-T019: Tauri commands (mock data for testing)
- T021: Event emission (mock GPU data)
- T022: Persistence logic (JSON I/O only)
- T023-T028: Frontend UI (mock data from backend)
- T032: Clippy (code quality)
- T036: Documentation updates

## Current Strategy

**Implement Feature 012 architecture WITHOUT GPU hardware**:
1. Create all Rust modules and data structures
2. Write contract tests (expected to fail)
3. Implement Tauri commands with mock/stub GPU data
4. Build frontend UI with event listeners
5. Test event-driven architecture with mock events
6. **Document**: "GPU hardware blocked - architecture complete, awaiting OpenCL fix"

**User resolves OpenCL** → Plug in real GPU enumeration + health monitoring → Feature complete

## Files Modified This Session

```
MD/OPENCL_BLOCKER_2025-11-13.md (this file)
```

## Next Steps

1. ✅ User resolves OpenCL (Option 1 recommended)
2. ⏳ Continue Feature 012 with mock data (architecture only)
3. ⏳ Test GPU hardware when OpenCL working

---

**Blocker Owner**: User
**Implementation**: Continues with mock data
**ETA**: OpenCL fix required before GPU mining testing