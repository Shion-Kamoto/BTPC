#!/bin/bash
# Register NVIDIA OpenCL ICD for RTX 3060
# Run with: sudo bash register_nvidia_opencl.sh

set -e

echo "=== Registering NVIDIA OpenCL ICD ==="

# Create nvidia.icd file
echo "libnvidia-opencl.so.1" > /etc/OpenCL/vendors/nvidia.icd

echo "✅ NVIDIA OpenCL ICD registered"
echo ""
echo "Verifying OpenCL platforms:"
clinfo | grep -A5 "Platform Name"