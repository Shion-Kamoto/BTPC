# ROCm OpenCL Installation Guide

**Date**: 2025-11-13
**System**: Ubuntu with AMD Radeon RX 580
**Goal**: Replace Mesa OpenCL with ROCm OpenCL for GPU mining support

---

## Quick Start

```bash
cd ~/BTPC/BTPC
./install_rocm_opencl.sh
```

**After installation**:
1. Log out and log back in (required for group membership)
2. Verify: `clinfo | grep -i 'platform name'`
3. Test: `cd test_opencl_diagnostic && cargo run`

---

## What This Does

**Installs**:
- `rocm-opencl-runtime` - AMD's proper OpenCL implementation
- `rocm-clinfo` - OpenCL information utility

**Configures**:
- Adds you to `video` and `render` groups (required for GPU access)
- Adds ROCm 6.2.4 APT repository

**Does NOT Touch**:
- Display drivers (amdgpu, radeon kernel modules stay unchanged)
- Mesa graphics libraries (for desktop rendering)
- X11/Wayland configuration

---

## Verification Steps

### Step 1: Check OpenCL Platform
```bash
clinfo | grep -i "platform name"
```

**Expected Output**:
```
  Platform Name                                   AMD Accelerated Parallel Processing
```

**If you see "Clover" or "Mesa"**: ROCm didn't take priority. See troubleshooting below.

### Step 2: Check Device Recognition
```bash
clinfo | grep -i "device name"
```

**Expected Output**:
```
  Device Name                                     gfx803
```
(gfx803 is the ROCm codename for Polaris10/RX 580)

### Step 3: Test Kernel Compilation
```bash
cd ~/BTPC/BTPC/test_opencl_diagnostic
cargo run
```

**Expected Output**:
```
=== OpenCL Diagnostic Test ===
✅ Found 2 platform(s)
✅ Found 1 GPU device(s)
🔨 Test 1: Building kernel with empty build options...
✅ SUCCESS: Kernel compiled!
```

---

## Troubleshooting

### Issue 1: "Platform Name" still shows "Clover" or "Mesa"

**Cause**: Mesa OpenCL still taking priority over ROCm

**Solution 1 - Environment Variable** (temporary):
```bash
export OCL_ICD_VENDORS=/opt/rocm/share/OpenCL/vendors
clinfo | grep -i "platform name"
```

**Solution 2 - System-wide Configuration** (permanent):
```bash
sudo sh -c 'echo "/opt/rocm/opencl/lib" > /etc/ld.so.conf.d/rocm-opencl.conf'
sudo ldconfig
```

Then log out and back in.

### Issue 2: "No platforms found" after installation

**Cause**: Groups not applied yet

**Solution**: Log out and log back in (not just close terminal - full logout/login)

Verify groups:
```bash
groups | grep -E "(video|render)"
```

Should show both `video` and `render`.

### Issue 3: Kernel compilation still fails with vector type errors

**Cause**: Wrong OpenCL platform being used

**Solution**: Force ROCm platform in test:
```bash
cd ~/BTPC/BTPC/test_opencl_diagnostic
# Edit src/main.rs to select platform by name "AMD Accelerated Parallel Processing"
```

Or set environment variable:
```bash
export RUSTICL_ENABLE=0  # Disable Mesa Rusticl
export OCL_ICD_VENDORS=/opt/rocm/share/OpenCL/vendors
cargo run
```

### Issue 4: Display issues after ROCm installation

**Unlikely** - ROCm OpenCL doesn't touch display drivers. But if you experience issues:

**Check driver status**:
```bash
lsmod | grep amdgpu
glxinfo | grep "OpenGL renderer"
```

**Revert if needed**:
```bash
sudo apt remove rocm-opencl-runtime rocm-clinfo
sudo apt autoremove
sudo ldconfig
```

Then reboot.

---

## How ROCm Differs from Mesa

| Aspect | Mesa OpenCL | ROCm OpenCL |
|--------|-------------|-------------|
| **Compiler** | Clover (incomplete) | AMD LLVM-based |
| **Vector Types** | Missing definitions | Full OpenCL 1.2 support |
| **GPU Support** | Generic | AMD-optimized |
| **Performance** | Lower | Higher (native) |
| **Compatibility** | Limited | NIST-compliant SHA-512 works |

---

## Expected Behavior After Installation

**Before ROCm**:
```
clinfo output:
  Platform Name: Clover
  Device Name: AMD Radeon RX 580 Series (radeonsi, polaris10, ACO, DRM 3.61, 6.14.0-35-generic)
  OpenCL C Version: OpenCL C 1.1

Kernel compilation: ❌ FAILS with 300+ vector type errors
```

**After ROCm**:
```
clinfo output:
  Platform Name: AMD Accelerated Parallel Processing
  Device Name: gfx803
  OpenCL C Version: OpenCL C 2.0

Kernel compilation: ✅ SUCCESS
```

---

## Testing BTPC GPU Mining

After verification succeeds:

```bash
cd ~/BTPC/BTPC/btpc-desktop-app
npm run tauri:dev
```

1. Navigate to Mining page
2. Enable "GPU Mining" checkbox
3. Check logs for "GPU mining started" (not "GPU mining unavailable")
4. Verify no UI freezing

**Expected behavior**:
- UI stays responsive
- GPU mining initializes successfully
- No vector type compilation errors in logs

---

## Rollback Plan

If ROCm causes issues:

```bash
# Remove ROCm packages
sudo apt remove rocm-opencl-runtime rocm-clinfo
sudo apt autoremove

# Remove repository
sudo rm /etc/apt/sources.list.d/rocm.list
sudo apt update

# Reboot (optional, recommended)
sudo reboot
```

Mesa OpenCL will automatically become default again.

---

## Additional Resources

- **ROCm Documentation**: https://rocm.docs.amd.com/
- **Supported GPUs**: https://rocm.docs.amd.com/projects/install-on-linux/en/latest/reference/system-requirements.html
- **RX 580 (gfx803)**: Officially supported in ROCm 6.x

---

## Next Steps After Successful Installation

1. ✅ Verify ROCm with `clinfo`
2. ✅ Test kernel compilation with diagnostic
3. ✅ Run desktop app GPU mining
4. Update `MD/GPU_MINING_BLOCKED_MESA_ISSUE.md` status to RESOLVED
5. Proceed with Feature 012 implementation (36 tasks)

---

**Status**: Ready for installation
**Risk Level**: LOW (only affects OpenCL runtime, not display drivers)
**Estimated Time**: 5-10 minutes + log out/in