#!/bin/bash
# Install AMD Legacy OpenCL Driver for RX 580 (GCN/Polaris)
# Fix for OpenCL Feature 012 GPU Mining

set -e

echo "=== AMD Legacy OpenCL Installation for RX 580 ==="
echo ""

# Step 1: Uninstall conflicting ROCm packages
echo "[1/4] Removing ROCm 6.2+ packages (incompatible with RX 580)..."
sudo apt remove --purge -y rocm-* amdgpu-core 2>/dev/null || true

# Step 2: Install legacy amdgpu-install tool (v5.7)
echo "[2/4] Installing amdgpu-install 5.7 (legacy)..."
sudo dpkg -i /home/bob/Downloads/amdgpu-install_5.7.50702-1_all.deb

# Step 3: Install legacy OpenCL runtime
echo "[3/4] Installing legacy OpenCL runtime for GCN GPUs..."
sudo amdgpu-install --opencl=legacy --no-dkms -y

# Step 4: Verify installation
echo "[4/4] Verifying OpenCL platforms..."
echo ""
clinfo | grep -E "Platform Name|Device Name" || echo "⚠️ No OpenCL platforms - may need reboot"

echo ""
echo "=== Installation Complete ==="
echo ""
echo "Next steps:"
echo "1. If clinfo shows 0 platforms: sudo reboot"
echo "2. After reboot: cd /home/bob/BTPC/BTPC/test_opencl_diagnostic && cargo run"
echo "3. If test passes: Resume Feature 012 implementation"
echo ""