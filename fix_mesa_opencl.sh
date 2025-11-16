#!/bin/bash
# Fix Mesa OpenCL ICD Registration
# After amdgpu-install removed ICD files

set -e

echo "=== Restoring Mesa OpenCL ICD Registration ==="

# Create ICD directory if missing
sudo mkdir -p /etc/OpenCL/vendors

# Register Rusticl OpenCL
echo "Registering Rusticl (Mesa Rust-based OpenCL)..."
echo "libRusticlOpenCL.so.1" | sudo tee /etc/OpenCL/vendors/rusticl.icd

# Register Mesa legacy OpenCL (fallback)
echo "Registering Mesa OpenCL (fallback)..."
echo "libMesaOpenCL.so.1" | sudo tee /etc/OpenCL/vendors/mesa.icd

# Test registration
echo ""
echo "Testing OpenCL platform detection..."
clinfo | grep -E "Platform Name|Device Name" || echo "⚠️ No platforms detected"

echo ""
echo "=== Mesa OpenCL Restored ==="
echo ""
echo "Next: cd /home/bob/BTPC/BTPC/test_opencl_diagnostic && cargo run"