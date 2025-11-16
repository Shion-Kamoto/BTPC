# OpenCL Kernel Compilation Fix Required

## Issue
GPU mining feature (Feature 012) fails with kernel compilation error:
```
fatal error: 'clc/clcfunc.h' file not found
```

## Root Cause
Ubuntu's `libclc-20-dev` package has a known packaging bug where critical header files are missing:
- `/usr/include/clc/clcfunc.h` (missing)
- `/usr/include/clc/clctypes.h` (missing)

The main header `/usr/include/clc/clc.h:19` includes these missing files, causing compilation to fail.

## Diagnostic Test Results
Tested on AMD Radeon RX 580 with Mesa OpenCL 25.0.7:
- Empty build options: ❌ FAILED (missing headers)
- `-cl-std=CL1.2`: ❌ FAILED (empty build log - different Mesa bug)
- `-cl-fast-relaxed-math`: ❌ FAILED (missing headers)
- `-cl-no-stdinc`: ❌ FAILED (option not supported by Mesa)

## Solution
Run the provided script with sudo:
```bash
sudo ./install_libclc_stub_headers.sh
```

This creates stub header files with required macro definitions:
- `clcfunc.h`: Defines `_CLC_OVERLOAD`, `_CLC_DECL`, `_CLC_DEF`, `_CLC_INLINE`, `_CLC_CONVERGENT`
- `clctypes.h`: Empty stub (types provided by compiler)

## Alternative (if sudo not available)
Contact system administrator to install the headers, or use CPU-only mining until headers are installed.

## File References
- Diagnostic test: `test_opencl_diagnostic/src/main.rs`
- Install script: `install_libclc_stub_headers.sh`
- GPU miner code: `btpc-desktop-app/src-tauri/src/gpu_miner.rs:107`
- Kernel source: `btpc-desktop-app/src-tauri/src/sha512_kernel.cl`

## Verification
After installing headers, rerun GPU mining or diagnostic test:
```bash
cd /home/bob/BTPC/BTPC/test_opencl_diagnostic
cargo run
```

Expected success message:
```
✅ SUCCESS: Kernel compiled with empty build options!
```

## Status
- **Feature 012**: BLOCKED until headers installed
- **GPU mining**: NON-FUNCTIONAL until fix applied
- **Desktop app build**: ✅ SUCCEEDS (OpenCL is optional dependency)