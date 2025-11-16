#!/bin/bash
# NVIDIA OpenCL Platform Detection Fix Script
# For Ubuntu 24.04 with NVIDIA RTX 3060
# Created: 2025-11-14

set -e

echo "=========================================="
echo "NVIDIA OpenCL Diagnostic and Fix Script"
echo "=========================================="
echo ""

# Function to check if command succeeded
check_success() {
    if [ $? -eq 0 ]; then
        echo "[OK] $1"
    else
        echo "[FAIL] $1"
        return 1
    fi
}

# Step 1: Verify current state
echo "Step 1: Checking current system state..."
echo ""

echo "1.1 Checking NVIDIA driver version:"
nvidia-smi --query-gpu=driver_version --format=csv,noheader | head -1
check_success "NVIDIA driver loaded"
echo ""

echo "1.2 Checking GPU status:"
nvidia-smi --query-gpu=index,name,pci.bus_id --format=csv
echo ""
if nvidia-smi 2>&1 | grep -q "Unable to determine"; then
    echo "[WARNING] One or more GPUs have errors - this is likely the root cause!"
    echo ""
fi

echo "1.3 Checking kernel modules:"
lsmod | grep nvidia
check_success "NVIDIA modules loaded"
echo ""

echo "1.4 Checking device nodes:"
ls -la /dev/nvidia* 2>&1 | grep -E "nvidia0|nvidiactl|nvidia-uvm"
check_success "Device nodes exist"
echo ""

echo "1.5 Checking OpenCL ICD files:"
ls -la /etc/OpenCL/vendors/
check_success "ICD files present"
echo ""

echo "1.6 Testing /dev/nvidia-uvm access:"
if python3 -c "import os; fd = os.open('/dev/nvidia-uvm', os.O_RDWR); os.close(fd)" 2>/dev/null; then
    echo "[OK] nvidia-uvm device accessible"
else
    echo "[FAIL] nvidia-uvm device returns I/O error - THIS IS THE PROBLEM"
    echo ""
    echo "Root cause: nvidia-uvm device exists but cannot be accessed."
    echo "Common cause: Failed GPU hardware preventing module initialization"
fi
echo ""

echo "1.7 Current OpenCL platforms:"
clinfo -l 2>&1
echo ""

# Step 2: Quick diagnostic
echo "=========================================="
echo "Step 2: Quick Diagnostic"
echo "=========================================="
echo ""

echo "2.1 Checking for problematic GPUs:"
echo ""
for pci in $(lspci | grep VGA | grep NVIDIA | awk '{print $1}'); do
    echo "Checking GPU at $pci:"
    lspci -vvv -s $pci 2>&1 | grep -E "Memory|Kernel driver|Unknown header" | head -5
    echo ""
done

echo "2.2 Testing NVIDIA OpenCL library load:"
if python3 -c "import ctypes; ctypes.CDLL('/lib/x86_64-linux-gnu/libnvidia-opencl.so.1')" 2>/dev/null; then
    echo "[OK] NVIDIA OpenCL library loads"
else
    echo "[FAIL] NVIDIA OpenCL library cannot load"
fi
echo ""

# Step 3: Suggested fixes
echo "=========================================="
echo "Step 3: Recommended Fixes"
echo "=========================================="
echo ""

if nvidia-smi 2>&1 | grep -q "Unable to determine"; then
    echo "DETECTED: GPU hardware error"
    echo ""
    echo "RECOMMENDED FIX: Disable or remove the failed GPU"
    echo ""
    echo "Option A (Best): Physically remove failed GPU"
    echo "  1. Shut down system"
    echo "  2. Remove GPU showing 'Unable to determine' error"
    echo "  3. Boot and test with: clinfo -l"
    echo ""
    echo "Option B: Disable via kernel parameter (if removal not possible)"
    echo "  1. Edit /etc/default/grub"
    echo "  2. Add to GRUB_CMDLINE_LINUX_DEFAULT:"

    # Get failed GPU device ID
    failed_pci=$(nvidia-smi 2>&1 | grep -oP '0000:\K[0-9a-f:\.]+' | tail -1)
    if [ -n "$failed_pci" ]; then
        device_id=$(lspci -n -s "$failed_pci" | awk '{print $3}' | sed 's/:/./')
        echo "     pci-stub.ids=$device_id"
    else
        echo "     pci-stub.ids=<get from lspci -n>"
    fi
    echo "  3. Run: sudo update-grub"
    echo "  4. Reboot"
    echo ""

    echo "Option C: Try module reload (temporary fix for testing)"
    echo "  Run: sudo bash $0 reload-modules"
    echo "  WARNING: This will kill your GUI session!"
    echo ""
else
    echo "No obvious GPU hardware errors detected."
    echo ""
    echo "Try these fixes in order:"
    echo ""
    echo "Fix 1: Reload nvidia-uvm module"
    echo "  Run: sudo bash $0 reload-modules"
    echo ""
    echo "Fix 2: Install missing packages"
    echo "  Run: sudo bash $0 install-packages"
    echo ""
    echo "Fix 3: Disable Mesa OpenCL (to isolate issue)"
    echo "  Run: sudo bash $0 disable-mesa"
    echo ""
fi

# Step 4: Execute fix if requested
if [ "$1" == "reload-modules" ]; then
    echo "=========================================="
    echo "Reloading NVIDIA Modules"
    echo "=========================================="
    echo ""
    echo "WARNING: This will kill your display manager!"
    echo "Press Ctrl+C within 5 seconds to cancel..."
    sleep 5

    echo "Stopping display manager..."
    sudo systemctl stop display-manager 2>/dev/null || true

    echo "Killing X and GPU processes..."
    sudo pkill -9 Xorg 2>/dev/null || true
    sudo pkill -9 nvidia-persiste 2>/dev/null || true

    echo "Unloading NVIDIA modules..."
    sudo modprobe -r nvidia_uvm 2>/dev/null || echo "nvidia_uvm already unloaded"
    sudo modprobe -r nvidia_drm 2>/dev/null || echo "nvidia_drm already unloaded"
    sudo modprobe -r nvidia_modeset 2>/dev/null || echo "nvidia_modeset already unloaded"
    sudo modprobe -r nvidia 2>/dev/null || echo "nvidia already unloaded"

    sleep 2

    echo "Reloading NVIDIA modules..."
    sudo modprobe nvidia
    sudo modprobe nvidia_modeset
    sudo modprobe nvidia_drm
    sudo modprobe nvidia_uvm

    sleep 2

    echo "Testing OpenCL..."
    clinfo -l

    echo ""
    echo "Restart display manager with: sudo systemctl start display-manager"
    exit 0
fi

if [ "$1" == "install-packages" ]; then
    echo "=========================================="
    echo "Installing Missing Packages"
    echo "=========================================="
    echo ""

    DRIVER_VERSION=$(nvidia-smi --query-gpu=driver_version --format=csv,noheader | head -1 | cut -d. -f1)

    echo "Installing packages for driver version: $DRIVER_VERSION"
    sudo apt update
    sudo apt install -y nvidia-modprobe-$DRIVER_VERSION nvidia-opencl-icd-$DRIVER_VERSION ocl-icd-libopencl1 clinfo

    echo ""
    echo "Testing OpenCL..."
    clinfo -l
    exit 0
fi

if [ "$1" == "disable-mesa" ]; then
    echo "=========================================="
    echo "Disabling Mesa OpenCL"
    echo "=========================================="
    echo ""

    if [ -f /etc/OpenCL/vendors/mesa.icd ]; then
        echo "Disabling mesa.icd..."
        sudo dpkg-divert --divert /etc/OpenCL/vendors/mesa.icd.disabled --rename /etc/OpenCL/vendors/mesa.icd
    fi

    if [ -f /etc/OpenCL/vendors/rusticl.icd ]; then
        echo "Disabling rusticl.icd..."
        sudo dpkg-divert --divert /etc/OpenCL/vendors/rusticl.icd.disabled --rename /etc/OpenCL/vendors/rusticl.icd
    fi

    echo ""
    echo "Testing OpenCL..."
    clinfo -l

    echo ""
    echo "To restore Mesa, run: sudo bash $0 enable-mesa"
    exit 0
fi

if [ "$1" == "enable-mesa" ]; then
    echo "=========================================="
    echo "Re-enabling Mesa OpenCL"
    echo "=========================================="
    echo ""

    sudo dpkg-divert --remove /etc/OpenCL/vendors/mesa.icd 2>/dev/null || true
    sudo dpkg-divert --remove /etc/OpenCL/vendors/rusticl.icd 2>/dev/null || true

    echo "Mesa OpenCL re-enabled"
    exit 0
fi

echo "=========================================="
echo "Diagnostic Complete"
echo "=========================================="
echo ""
echo "For detailed analysis, see: MD/NVIDIA_OPENCL_DIAGNOSIS_2025-11-14.md"
echo ""