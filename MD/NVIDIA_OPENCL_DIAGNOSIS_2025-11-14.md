# NVIDIA OpenCL Platform Detection Issue - Root Cause Analysis
**Date:** 2025-11-14
**System:** Ubuntu 24.04, NVIDIA RTX 3060 (x2), Driver 580.95.05

## Executive Summary

**Problem:** `clinfo -l` only shows Mesa platforms (Clover, rusticl) - NVIDIA OpenCL platform NOT detected.

**Root Cause:** `/dev/nvidia-uvm` device node returns I/O error (EIO) when NVIDIA OpenCL library attempts to open it, causing the NVIDIA ICD to fail initialization and be silently ignored by the ICD loader.

**Underlying Issue:** GPU1 (04:00.0) has hardware failure evidenced by `lspci` reporting "Unknown header type 7f" and nvidia-smi showing "Unable to determine the device handle for GPU1: Unknown Error"

## Detailed Findings

### 1. NVIDIA OpenCL Library IS Being Loaded

**Evidence from LD_DEBUG trace:**
```
find library=libnvidia-opencl.so.1 [0]; searching
  trying file=/lib/x86_64-linux-gnu/libnvidia-opencl.so.1
calling init: /lib/x86_64-linux-gnu/libnvidia-opencl.so.1
calling fini: /lib/x86_64-linux-gnu/libnvidia-opencl.so.1 [0]
```

**Interpretation:** The ICD loader successfully:
1. Reads `/etc/OpenCL/vendors/nvidia.icd` (verified via strace)
2. Loads `/lib/x86_64-linux-gnu/libnvidia-opencl.so.1`
3. Calls initialization function
4. **BUT** immediately calls finalization (fini) - indicating initialization FAILED

### 2. The Critical Failure Point

**Evidence from strace:**
```
openat(AT_FDCWD, "/etc/OpenCL/vendors/nvidia.icd", O_RDONLY) = 4
openat(AT_FDCWD, "/lib/x86_64-linux-gnu/libnvidia-opencl.so.1", O_RDONLY|O_CLOEXEC) = 6
[... NVIDIA reads config files successfully ...]
openat(AT_FDCWD, "/dev/nvidia-uvm", O_RDWR|O_CLOEXEC) = -1 EIO (Input/output error)
openat(AT_FDCWD, "/dev/nvidia-uvm", O_RDWR) = -1 EIO (Input/output error)
ioctl(-5, _IOC(_IOC_NONE, 0, 0x2, 0x3000), 0) = -1 EBADF (Bad file descriptor)
```

**Analysis:**
- NVIDIA OpenCL library loads successfully
- Reads driver configuration files without error
- Attempts to open `/dev/nvidia-uvm` (Unified Virtual Memory) device
- **Receives EIO error** - device exists but cannot be accessed
- Returns error code causing ICD loader to unload the library
- No NVIDIA platform appears in clinfo

### 3. Hardware Root Cause - Failed GPU1

**nvidia-smi output:**
```
GPU 00000000:03:00.0
    Product Name                          : NVIDIA GeForce RTX 3060
Unable to determine the device handle for GPU1: 0000:04:00.0: Unknown Error
```

**lspci -vvv output for GPU1:**
```
04:00.0 VGA compatible controller: NVIDIA Corporation GA106 [GeForce RTX 3060 Lite Hash Rate] (rev a1)
    !!! Unknown header type 7f
    Interrupt: pin ? routed to IRQ 47
    Kernel driver in use: nvidia
```

**Interpretation:**
- "Unknown header type 7f" = PCI device returning 0xFF (all 1's) when config space is read
- Classic symptom: Device is unpowered, disabled, or hardware-failed
- nvidia-uvm module cannot initialize properly because GPU1 is not responding
- When nvidia-uvm fails to initialize ALL GPUs, the entire module enters error state

### 4. Why Mesa Platforms Work But NVIDIA Doesn't

**Mesa (Clover/Rusticl):**
- Software-based OpenCL implementations
- Can run on CPU or GPU
- Don't require nvidia-uvm device
- Work even with failed GPUs

**NVIDIA OpenCL:**
- Requires working nvidia-uvm kernel module
- nvidia-uvm needs ALL attached NVIDIA GPUs to respond correctly
- One failed GPU causes nvidia-uvm to enter error state
- Error state propagates to ALL OpenCL attempts (even on working GPU0)

### 5. System State Verification

**Kernel modules loaded correctly:**
```
nvidia_uvm             2076672  0 - Live (loaded)
nvidia_drm             135168  14
nvidia_modeset         1638400  15 nvidia_drm
nvidia                 104071168 191 nvidia_uvm,nvidia_modeset
```

**Device nodes exist with correct permissions:**
```
crw-rw-rw- 1 root root 234,   0 Jul 12 04:28 /dev/nvidia-uvm
crw-rw-rw- 1 root root 234,   1 Jul 12 04:28 /dev/nvidia-uvm-tools
```

**Library files correct:**
```
lrwxrwxrwx 1 root root 29 Oct  1 18:47 /lib/x86_64-linux-gnu/libnvidia-opencl.so.1 -> libnvidia-opencl.so.580.95.05
-rw-r--r-- 1 root root 89858384 Oct  1 18:47 /lib/x86_64-linux-gnu/libnvidia-opencl.so.580.95.05
```

**ICD file correct:**
```
/etc/OpenCL/vendors/nvidia.icd contains: libnvidia-opencl.so.1
```

## Solution Priority Order

### PRIORITY 1: Fix or Disable Failed GPU1 (CRITICAL)

**Option A: Physical Removal (Recommended if GPU1 truly failed)**
1. Power down system
2. Physically remove GPU at PCIe slot 04:00.0
3. Boot and verify: `nvidia-smi` should show only GPU0
4. Test OpenCL: `clinfo -l` should now show NVIDIA platform

**Option B: Disable via Kernel Parameter**
```bash
# Add to /etc/default/grub:
GRUB_CMDLINE_LINUX_DEFAULT="quiet splash pci=noaer pci-stub.ids=10de:2504"
# (replace 2504 with device ID from lspci -n)

sudo update-grub
sudo reboot
```

**Option C: Disable via BIOS**
- Enter BIOS/UEFI
- Disable the PCIe slot containing GPU1
- Save and reboot

### PRIORITY 2: Test nvidia-uvm Reset (Quick Test)

**Attempt to reset the nvidia-uvm module:**
```bash
# Stop all processes using NVIDIA
sudo systemctl stop display-manager
sudo pkill -9 Xorg

# Unload modules (requires no processes using them)
sudo modprobe -r nvidia_uvm
sudo modprobe -r nvidia_drm
sudo modprobe -r nvidia_modeset
sudo modprobe -r nvidia

# Reload modules
sudo modprobe nvidia
sudo modprobe nvidia_modeset
sudo modprobe nvidia_drm
sudo modprobe nvidia_uvm

# Test
clinfo -l

# Restart display manager
sudo systemctl start display-manager
```

**WARNING:** This will kill your GUI session. Only do this if you can work from TTY (Ctrl+Alt+F3).

### PRIORITY 3: Disable Mesa OpenCL (Workaround - doesn't fix root cause)

**Temporarily disable Mesa to isolate NVIDIA issues:**
```bash
sudo dpkg-divert --divert /etc/OpenCL/vendors/mesa.icd.disabled --rename /etc/OpenCL/vendors/mesa.icd
sudo dpkg-divert --divert /etc/OpenCL/vendors/rusticl.icd.disabled --rename /etc/OpenCL/vendors/rusticl.icd

# Test
clinfo -l

# Restore if needed:
sudo dpkg-divert --remove /etc/OpenCL/vendors/mesa.icd
sudo dpkg-divert --remove /etc/OpenCL/vendors/rusticl.icd
```

**Note:** This won't fix nvidia-uvm EIO error, but confirms Mesa isn't interfering.

### PRIORITY 4: Install Missing Packages (Unlikely but Worth Checking)

```bash
# Install nvidia-modprobe if missing
sudo apt install nvidia-modprobe-580

# Install OpenCL ICD package explicitly
sudo apt install nvidia-opencl-icd-580

# Verify installation
dpkg -l | grep nvidia | grep -E "opencl|modprobe"
```

## Verification Steps After Fix

### Step 1: Verify nvidia-smi Shows Only Working GPUs
```bash
nvidia-smi --query-gpu=index,name,driver_version,pci.bus_id --format=csv
# Should NOT show "Unable to determine device handle" error
```

### Step 2: Test nvidia-uvm Device Access
```bash
# Should succeed without EIO error
python3 -c "import os; fd = os.open('/dev/nvidia-uvm', os.O_RDWR); print('SUCCESS'); os.close(fd)"
```

### Step 3: Verify OpenCL Platforms
```bash
clinfo -l
# Should show:
# Platform #0: Clover
# Platform #1: rusticl
# Platform #2: NVIDIA CUDA  <-- NEW!
```

### Step 4: Verify NVIDIA Platform Details
```bash
clinfo | grep -A10 "Platform #2"
# Should show NVIDIA devices
```

### Step 5: Test OpenCL Program
```bash
# Create test program
cat > /tmp/test_opencl.c << 'EOF'
#include <CL/cl.h>
#include <stdio.h>
int main() {
    cl_platform_id platforms[10];
    cl_uint num_platforms;
    clGetPlatformIDs(10, platforms, &num_platforms);
    printf("Found %d platforms\n", num_platforms);

    for (int i = 0; i < num_platforms; i++) {
        char name[256];
        clGetPlatformInfo(platforms[i], CL_PLATFORM_NAME, sizeof(name), name, NULL);
        printf("Platform %d: %s\n", i, name);
    }
    return 0;
}
EOF

gcc /tmp/test_opencl.c -o /tmp/test_opencl -lOpenCL
/tmp/test_opencl
```

## Technical Notes

### Why OCL_ICD_VENDORS Doesn't Help

Setting `OCL_ICD_VENDORS=/etc/OpenCL/vendors/nvidia.icd` doesn't fix the issue because:
1. The ICD loader successfully reads nvidia.icd
2. The ICD loader successfully loads libnvidia-opencl.so.1
3. The problem occurs INSIDE the NVIDIA library during initialization
4. nvidia-uvm device EIO error causes init failure
5. Failed init causes ICD loader to unload the library

### Why CUDA_VISIBLE_DEVICES Doesn't Help

`CUDA_VISIBLE_DEVICES=0` only affects CUDA runtime API, not:
- OpenCL platform enumeration
- nvidia-uvm kernel module initialization
- PCIe device detection

The nvidia-uvm module initializes for ALL NVIDIA GPUs at module load time, before any user-space program runs.

### Kernel Module Dependency Chain

```
nvidia (base driver)
  |
  +-- nvidia_modeset (mode setting)
  |     |
  |     +-- nvidia_drm (DRM/KMS)
  |
  +-- nvidia_uvm (Unified Virtual Memory) <-- FAILING HERE
        |
        +-- OpenCL (requires working nvidia_uvm)
        +-- CUDA (requires working nvidia_uvm)
```

## References

1. **strace evidence**: `/dev/nvidia-uvm` returns EIO on open()
2. **LD_DEBUG evidence**: libnvidia-opencl.so.1 calls fini immediately after init
3. **lspci evidence**: GPU1 shows "Unknown header type 7f"
4. **nvidia-smi evidence**: "Unable to determine device handle for GPU1"
5. **Web research**: EIO on nvidia-uvm typically indicates hardware or module state error

## Recommended Action

**Immediately try PRIORITY 1, Option A:**
1. Shut down the system
2. Physically remove the GPU at PCIe slot 04:00.0 (GPU1)
3. Boot and verify nvidia-smi only shows GPU0
4. Test: `clinfo -l` should now show "NVIDIA CUDA" platform
5. Test Rust opencl3 mining code

This is the fastest and most reliable fix if GPU1 is indeed hardware-failed (as evidence strongly suggests).

If GPU1 is critical for mining, investigate why it's showing "Unknown header type 7f" - this could indicate:
- Insufficient PSU power
- Faulty PCIe riser/cable
- Motherboard PCIe slot failure
- GPU hardware failure
- BIOS/UEFI PCIe configuration issue