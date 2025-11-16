# GPU Mining Blocked - Mesa OpenCL Compiler Issue

**Date**: 2025-11-13
**Status**: ❌ BLOCKED - Mesa OpenCL incompatibility

---

## Issue Summary

GPU mining for Feature 012 is **blocked** by a fundamental Mesa OpenCL compiler incompatibility. The issue cannot be resolved with header stubs alone.

---

## Problem Details

### Initial Issue (RESOLVED)
- **Problem**: Missing libclc headers (`clcfunc.h`, `clctypes.h`)
- **Solution**: Installed stub headers via `install_libclc_stub_headers.sh`
- **Result**: ✅ Headers now present

### Current Issue (BLOCKING)
- **Problem**: Mesa's OpenCL compiler automatically includes `/usr/include/clc/convert.h`
- **Error**: Hundreds of "unknown type name" errors for OpenCL vector types:
  - `char2`, `char3`, `char4`, `char8`, `char16`
  - `uchar2`, `uchar3`, `uchar4`, `uchar8`, `uchar16`
  - `int2`, `int3`, `int4`, `int8`, `int16`
  - `uint`, `uint2`, `uint3`, `uint4`, `uint8`, `uint16`
  - Similar errors for `short`, `ushort`, `long`, `ulong`, `float`, `double` vectors

### Root Cause
Mesa OpenCL (version 25.0.7) on AMD Radeon RX 580:
- Automatically includes system headers with vector type definitions
- Does NOT provide the actual vector type definitions (`clctypes.h` is just a stub)
- Our kernel (`sha512_kernel.cl`) is self-contained and doesn't use these types
- But Mesa forces inclusion anyway

---

## Why This Blocks GPU Mining

1. **OpenCL kernel won't compile**: Every kernel compilation fails with 300+ type errors
2. **No workaround available**:
   - `-cl-no-stdinc` not supported by Mesa
   - Cannot override system include path
   - Cannot prevent automatic header inclusion
3. **Feature 012 depends on working GPU mining**: Dashboard shows GPU mining stats

---

## Possible Solutions

### Option 1: Use NVIDIA GPU (Recommended if available)
- NVIDIA's OpenCL implementation is mature and well-tested
- libclc headers work correctly with NVIDIA drivers
- Would require access to system with NVIDIA GPU

### Option 2: Use ROCm for AMD GPUs
- **ROCm** (Radeon Open Compute) is AMD's proper OpenCL stack
- Replace Mesa OpenCL with ROCm OpenCL
- Requires: `sudo apt install rocm-opencl-runtime rocm-clinfo`
- **Warning**: May conflict with existing Mesa drivers

### Option 3: Implement CPU-Only Mining Dashboard (Workaround)
- Modify Feature 012 to show "GPU mining unavailable" message
- Implement dashboard with mock/placeholder data
- Document as "GPU mining not supported on Mesa OpenCL"
- **Downside**: Defeats purpose of Feature 012

### Option 4: Switch to CUDA (NVIDIA only)
- Use CUDA instead of OpenCL for GPU mining
- Requires NVIDIA GPU and CUDA toolkit
- Would need to rewrite kernel in CUDA C
- **Incompatible with AMD GPUs**

### Option 5: Defer Feature 012
- Document GPU mining as blocked
- Focus on other features (wallet, transactions, networking)
- Revisit when proper OpenCL runtime available

---

## Recommendation

**SHORT TERM**: Defer Feature 012 implementation until proper OpenCL runtime available

**RATIONALE**:
1. Mesa OpenCL is fundamentally broken for custom kernels
2. Cannot guarantee GPU mining will work on user systems
3. AMD GPU users would need ROCm (complex installation)
4. NVIDIA GPU users could work, but that's a small subset
5. Better to have solid core features than broken GPU mining

**ALTERNATIVE**: Implement Feature 012 UI with graceful degradation:
- Show "GPU mining unavailable (OpenCL compiler issue)" message
- Display placeholder stats for testing
- Document known limitation in user guide
- Revisit when OpenCL runtime situation improves

---

## Technical Details

### System Info
```
GPU: AMD Radeon RX 580 Series (polaris10)
OpenCL: Mesa OpenCL 25.0.7
OpenCL C: 1.1
Platforms: 2 (Mesa, CUDA stub)
Devices: 1 GPU device
```

### Error Sample
```
error: unknown type name 'char2'; did you mean 'char'?
  /usr/include/clc/convert.h:74:1
  (repeated for all vector types)
```

### Files Involved
- `/usr/include/clc/clc.h` - Main header (auto-included by Mesa)
- `/usr/include/clc/convert.h` - Contains vector type references
- `/usr/include/clc/clctypes.h` - Stub (missing actual type definitions)
- `btpc-desktop-app/src-tauri/src/sha512_kernel.cl` - Our kernel (self-contained)

---

## Impact on Project

### What Still Works
- ✅ Desktop app compiles successfully
- ✅ UI doesn't freeze (non-blocking GPU init fix applied)
- ✅ All spec/plan issues resolved
- ✅ CPU mining works
- ✅ Wallet, transactions, blockchain core

### What's Blocked
- ❌ GPU mining functionality
- ❌ Feature 012 implementation (GPU Mining Dashboard)
- ❌ GPU performance optimization
- ❌ Multi-GPU mining

### Workaround Available
- Show "GPU mining unavailable" in UI
- Implement dashboard with graceful degradation
- Document limitation in release notes

---

## Decision Needed

**User must decide**:
1. Install ROCm (complex, may break display drivers)
2. Switch to NVIDIA GPU (hardware requirement)
3. Defer Feature 012 entirely
4. Implement Feature 012 with "unavailable" message

---

## Session Summary

**Fixed Today**:
- ✅ Compilation errors (main.rs module paths)
- ✅ UI freezing (non-blocking GPU init)
- ✅ All CRITICAL + HIGH spec issues
- ✅ Installed libclc stub headers

**Current Blocker**:
- ❌ Mesa OpenCL compiler incompatibility (cannot be fixed without changing OpenCL runtime)

**Recommendation**: Focus on other features, defer GPU mining until proper OpenCL available

---

**Status**: Documented blocker, awaiting user decision