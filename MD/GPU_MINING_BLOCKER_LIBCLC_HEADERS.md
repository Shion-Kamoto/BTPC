# GPU Mining Blocker: Missing libclc Headers (Ubuntu Packaging Bug)

**Date**: 2025-11-09
**Priority**: P0 - BLOCKER
**Status**: External dependency issue (not BTPC code bug)
**Affected**: Feature 009 GPU Mining Phase 2 testing

## Problem Statement

GPU mining kernel compilation fails with:
```
/usr/include/clc/clc.h:19:10: fatal error: 'clc/clcfunc.h' file not found
```

**Impact**: Cannot test GPU mining implementation (code complete, runtime blocked)

## Root Cause

**Ubuntu Packaging Bug**: `libclc-20-dev` package missing required headers
- Missing: `clcfunc.h`, `clctypes.h`
- Required by: `/usr/include/clc/clc.h` (lines 19-22)
- Affects: Mesa Rusticl OpenCL runtime kernel JIT compilation
- Upstream: https://github.com/llvm/llvm-project/issues/119967

**Installed Packages**:
```
libclc-20              1:20.1.2-0ubuntu1~24.04.2
libclc-20-dev          1:20.1.2-0ubuntu1~24.04.2
mesa-opencl-icd        25.0.7-0ubuntu0.24.04.2
```

**File Verification**:
```bash
$ ls /usr/include/clc/
as_type.h  async/  atomic/  clc.h  clcmacros.h  common/  convert.h  ...
# Missing: clcfunc.h, clctypes.h
```

## Workaround Options

### Option A: Create Minimal Stub Headers (FASTEST)
Create missing headers with minimal definitions to satisfy Mesa compiler:

```bash
# Create clcfunc.h stub
sudo tee /usr/include/clc/clcfunc.h > /dev/null << 'EOF'
#ifndef __CLC_CLCFUNC_H__
#define __CLC_CLCFUNC_H__
#define _CLC_OVERLOAD __attribute__((overloadable))
#define _CLC_DECL
#define _CLC_DEF
#define _CLC_INLINE __attribute__((always_inline)) inline
#define _CLC_CONVERGENT __attribute__((convergent))
#endif
EOF

# Create clctypes.h stub
sudo tee /usr/include/clc/clctypes.h > /dev/null << 'EOF'
#ifndef __CLC_CLCTYPES_H__
#define __CLC_CLCTYPES_H__
/* OpenCL built-in types already provided by compiler */
#endif
EOF
```

**Pros**: Quick fix, minimal risk
**Cons**: May not work for all kernels (our SHA-512 kernel doesn't use clc library functions)
**Estimated Time**: 5 minutes
**Success Probability**: 70% (our kernel is self-contained)

### Option B: Use Pre-compiled SPIR-V Binary (RECOMMENDED)
Compile kernel offline, embed binary in Rust code:

```bash
# Install SPIR-V tools
sudo apt-get install spirv-tools llvm-spirv

# Compile SHA-512 kernel to SPIR-V
clang -cl-std=CL2.0 -target spir64 -O3 \
  -o sha512_mining.bc \
  bins/btpc_miner/src/sha512_mining.cl

llvm-spirv sha512_mining.bc -o sha512_mining.spv

# Embed in Rust (modify gpu_miner.rs)
const KERNEL_SPIRV: &[u8] = include_bytes!("sha512_mining.spv");
program = Program::with_binary(&context, &devices, &[KERNEL_SPIRV])?;
```

**Pros**: Avoids runtime compilation entirely, production-ready
**Cons**: Requires toolchain setup, 1-2 hours implementation
**Estimated Time**: 2 hours
**Success Probability**: 95%

### Option C: Build libclc from Source (THOROUGH)
Compile complete libclc with all headers:

```bash
git clone https://github.com/llvm/llvm-project.git
cd llvm-project/libclc
mkdir build && cd build
cmake .. -DCMAKE_INSTALL_PREFIX=/usr/local
make -j$(nproc)
sudo make install
sudo ldconfig
```

**Pros**: Complete solution, all headers available
**Cons**: 30+ minute compile time, large disk usage
**Estimated Time**: 1 hour
**Success Probability**: 99%

### Option D: Switch to NVIDIA CUDA Backend (ALTERNATIVE)
Use CUDA instead of OpenCL (system has NVIDIA drivers):

**Pros**: CUDA more mature, better performance
**Cons**: AMD GPU won't work, CUDA-only solution
**Estimated Time**: 4-6 hours (rewrite kernel)
**Success Probability**: 95%
**Recommendation**: ❌ Not viable (AMD GPU hardware present)

## Recommendation

**Immediate**: Try **Option A** (stub headers) - 5 minutes
- Our SHA-512 kernel is self-contained (no libclc dependencies)
- If it works, unblocks testing immediately
- Low risk (can revert by deleting 2 files)

**If Option A Fails**: Use **Option B** (SPIR-V binary) - 2 hours
- Production-ready solution
- Eliminates all runtime compilation issues
- Portable across systems

**Avoid**: Option C (time-consuming), Option D (wrong hardware)

## Testing After Fix

Once headers resolved or SPIR-V used:

```bash
# Restart desktop app to pick up new binary
pkill btpc-desktop-app
npm run tauri:dev

# Start mining with GPU
# Should see: "Mining thread 1: GPU mode (1048576 nonces/iteration)"

# Verify GPU utilization
watch -n 1 'nvidia-smi'  # or 'radeontop' for AMD

# Check mining logs for hashrate
# Expected: 100-500 MH/s (vs 2-10 MH/s CPU)
```

## Constitutional Compliance

✅ Article II.1: SHA-512 PoW unchanged (kernel implements SHA-512)
✅ Article II.2: ML-DSA signatures unaffected
✅ Article VI.3: TDD - tests exist, blocked by runtime
✅ Article X: No prohibited features

**Blocker Status**: External dependency, not BTPC code issue

## Files Affected

**BTPC Code** (✅ Complete):
- `bins/btpc_miner/src/sha512_mining.cl` (275 lines, SHA-512 kernel)
- `bins/btpc_miner/src/gpu_miner.rs` (GPU buffer management)
- `bins/btpc_miner/src/main.rs` (mining loop integration)

**System Dependencies** (❌ Missing):
- `/usr/include/clc/clcfunc.h` (Ubuntu packaging bug)
- `/usr/include/clc/clctypes.h` (Ubuntu packaging bug)

## Next Actions

**Immediate** (Choose one):
1. Create stub headers (Option A) - 5 min
2. Pre-compile to SPIR-V (Option B) - 2 hours
3. Build libclc from source (Option C) - 1 hour

**After Fix**:
1. Test GPU mining functional
2. Benchmark hashrate (target: 100-500 MH/s)
3. Validate blocks accepted
4. Mark Phase 2 testing complete

---

**Session Handoff**: Next session should attempt Option A (stub headers) first. If successful, proceed to GPU testing. If fails, implement Option B (SPIR-V binary).