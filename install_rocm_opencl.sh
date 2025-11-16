#!/bin/bash
# ROCm OpenCL Installation Script for AMD Radeon RX 580
# This replaces Mesa OpenCL with AMD's proper OpenCL implementation

set -e

echo "=== ROCm OpenCL Installation ==="
echo ""
echo "⚠️  WARNING: This will modify your OpenCL runtime"
echo "    - Mesa OpenCL will be replaced with ROCm OpenCL"
echo "    - Your display drivers should NOT be affected"
echo "    - If issues occur, you can revert by uninstalling rocm packages"
echo ""
read -p "Continue? (y/N) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Installation cancelled."
    exit 1
fi

echo ""
echo "Step 1: Adding ROCm repository..."
# Add ROCm repository
wget -q -O - https://repo.radeon.com/rocm/rocm.gpg.key | sudo apt-key add -
echo 'deb [arch=amd64] https://repo.radeon.com/rocm/apt/6.2.4 ubuntu main' | sudo tee /etc/apt/sources.list.d/rocm.list

echo ""
echo "Step 2: Updating package lists..."
sudo apt update

echo ""
echo "Step 3: Installing ROCm OpenCL runtime..."
# Install only OpenCL runtime (not full ROCm stack to avoid conflicts)
sudo apt install -y rocm-opencl-runtime rocm-clinfo

echo ""
echo "Step 4: Adding user to 'video' and 'render' groups..."
sudo usermod -a -G video $USER
sudo usermod -a -G render $USER

echo ""
echo "✅ ROCm OpenCL installation complete!"
echo ""
echo "⚠️  IMPORTANT: You must LOG OUT and LOG BACK IN for group changes to take effect"
echo ""
echo "After logging back in, run:"
echo "  clinfo | grep -i 'platform name'"
echo ""
echo "Expected output:"
echo "  Platform Name: AMD Accelerated Parallel Processing"
echo ""
echo "Then test GPU mining with:"
echo "  cd ~/BTPC/BTPC/test_opencl_diagnostic && cargo run"