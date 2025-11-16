#!/bin/bash
# BTPC GPU Mining - NVIDIA Driver Fix Script
# Fixes "clc/clcfunc.h not found" OpenCL error by loading NVIDIA kernel modules

set -e

echo "========================================"
echo "BTPC GPU Mining - NVIDIA Driver Fix"
echo "========================================"
echo ""

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo "❌ This script requires sudo privileges"
    echo "   Please run: sudo bash fix_nvidia_gpu.sh"
    exit 1
fi

echo "Step 1: Checking current NVIDIA module status..."
if lsmod | grep -q nvidia; then
    echo "✅ NVIDIA modules already loaded"
else
    echo "⚠️  NVIDIA modules not loaded"
fi

echo ""
echo "Step 2: Loading NVIDIA kernel modules..."
modprobe nvidia || {
    echo "❌ Failed to load nvidia module"
    echo "   Attempting to install linux-modules-extra..."
    apt update
    apt install -y linux-modules-extra-$(uname -r)
    echo "✅ Installed linux-modules-extra, please reboot and try again"
    exit 0
}

modprobe nvidia-uvm || echo "⚠️  nvidia-uvm not available (optional)"

echo ""
echo "Step 3: Verifying NVIDIA modules loaded..."
if lsmod | grep nvidia > /dev/null; then
    echo "✅ NVIDIA modules loaded successfully:"
    lsmod | grep nvidia | awk '{print "   - " $1}'
else
    echo "❌ NVIDIA modules failed to load"
    exit 1
fi

echo ""
echo "Step 4: Checking NVIDIA device files..."
if ls /dev/nvidia* &> /dev/null; then
    echo "✅ NVIDIA device files created:"
    ls -la /dev/nvidia* | awk '{print "   " $0}'
else
    echo "❌ No /dev/nvidia* device files found"
    exit 1
fi

echo ""
echo "Step 5: Verifying nvidia-smi..."
if nvidia-smi &> /dev/null; then
    echo "✅ nvidia-smi working:"
    nvidia-smi --query-gpu=name,driver_version --format=csv,noheader | while read line; do
        echo "   $line"
    done
else
    echo "⚠️  nvidia-smi not working, but driver loaded"
fi

echo ""
echo "Step 6: Configuring auto-load at boot..."
if grep -q "^nvidia$" /etc/modules; then
    echo "✅ nvidia already in /etc/modules"
else
    echo "nvidia" >> /etc/modules
    echo "✅ Added nvidia to /etc/modules"
fi

if grep -q "^nvidia-uvm$" /etc/modules; then
    echo "✅ nvidia-uvm already in /etc/modules"
else
    echo "nvidia-uvm" >> /etc/modules
    echo "✅ Added nvidia-uvm to /etc/modules"
fi

echo ""
echo "Step 7: Checking OpenCL configuration..."
if [ -d /etc/OpenCL/vendors ]; then
    echo "✅ OpenCL ICD loader configured:"
    ls -1 /etc/OpenCL/vendors/*.icd | while read icd; do
        echo "   - $(basename $icd)"
    done
else
    echo "⚠️  OpenCL ICD loader not configured"
fi

echo ""
echo "========================================"
echo "✅ NVIDIA GPU Setup Complete!"
echo "========================================"
echo ""
echo "Next steps:"
echo "1. Build btpc_miner with GPU support:"
echo "   cd /home/bob/BTPC/BTPC"
echo "   cargo build --release --features gpu"
echo ""
echo "2. Run the GPU miner:"
echo "   ./target/release/btpc_miner --gpu --network regtest"
echo ""
echo "3. Check GPU stats at:"
echo "   http://localhost:18360/stats"
echo ""