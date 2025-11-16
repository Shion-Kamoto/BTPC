#!/bin/bash
#
# NVIDIA Driver Installation Script
# Version: 1.0
# Driver: NVIDIA-Linux-x86_64-580.105.08
#
# Usage: sudo ./install_nvidia_driver.sh
#

set -e  # Exit on error

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}================================${NC}"
echo -e "${GREEN}NVIDIA Driver Installation${NC}"
echo -e "${GREEN}Version: 580.105.08${NC}"
echo -e "${GREEN}================================${NC}"
echo ""

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}ERROR: This script must be run as root${NC}"
    echo "Usage: sudo ./install_nvidia_driver.sh"
    exit 1
fi

# Verify driver file exists
DRIVER_PATH="/home/bob/Downloads/NVIDIA-Linux-x86_64-580.105.08.run"
if [ ! -f "$DRIVER_PATH" ]; then
    echo -e "${RED}ERROR: Driver file not found: $DRIVER_PATH${NC}"
    exit 1
fi

echo -e "${GREEN}✓${NC} Driver file found: $DRIVER_PATH"

# Check if driver is executable
if [ ! -x "$DRIVER_PATH" ]; then
    echo -e "${YELLOW}Making driver executable...${NC}"
    chmod +x "$DRIVER_PATH"
fi

echo -e "${GREEN}✓${NC} Driver file is executable"
echo ""

# Ask user for confirmation
echo -e "${YELLOW}WARNING:${NC} This will install NVIDIA driver 580.105.08"
echo "The installation will:"
echo "  1. Stop the display manager (GDM)"
echo "  2. Install the NVIDIA driver"
echo "  3. Register kernel modules"
echo "  4. Restart the display manager"
echo ""
read -p "Continue with installation? (yes/no): " -r
echo ""
if [[ ! $REPLY =~ ^[Yy][Ee][Ss]$ ]]; then
    echo -e "${YELLOW}Installation cancelled by user${NC}"
    exit 0
fi

# Check for existing NVIDIA drivers
echo "Checking for existing NVIDIA drivers..."
if lsmod | grep -q nvidia; then
    echo -e "${YELLOW}⚠${NC} NVIDIA modules currently loaded"
    echo "Unloading existing NVIDIA modules..."
    rmmod nvidia_uvm 2>/dev/null || true
    rmmod nvidia_drm 2>/dev/null || true
    rmmod nvidia_modeset 2>/dev/null || true
    rmmod nvidia 2>/dev/null || true
    echo -e "${GREEN}✓${NC} Existing modules unloaded"
else
    echo -e "${GREEN}✓${NC} No existing NVIDIA modules loaded"
fi
echo ""

# Stop display manager
echo "Stopping display manager (GDM)..."
systemctl stop gdm
echo -e "${GREEN}✓${NC} Display manager stopped"
echo ""

# Install the driver
echo -e "${GREEN}Installing NVIDIA driver...${NC}"
echo "This may take 2-5 minutes..."
echo ""

# Run installer with automatic options:
# --silent: Non-interactive mode
# --accept-license: Accept EULA
# --dkms: Use DKMS for kernel module management
# --run-nvidia-xconfig: Update X configuration (optional, disabled for safety)
# --no-questions: Don't ask questions

"$DRIVER_PATH" \
    --silent \
    --accept-license \
    --dkms \
    --no-questions \
    2>&1 | tee /tmp/nvidia_install.log

INSTALL_STATUS=$?

# Check installation result
if [ $INSTALL_STATUS -eq 0 ]; then
    echo ""
    echo -e "${GREEN}✓${NC} NVIDIA driver installed successfully"
else
    echo ""
    echo -e "${RED}✗${NC} Driver installation failed with exit code: $INSTALL_STATUS"
    echo "Installation log saved to: /tmp/nvidia_install.log"

    # Restart display manager before exiting
    echo "Restarting display manager..."
    systemctl start gdm
    exit 1
fi

# Load NVIDIA modules
echo ""
echo "Loading NVIDIA kernel modules..."
modprobe nvidia
modprobe nvidia_uvm
modprobe nvidia_drm
modprobe nvidia_modeset
echo -e "${GREEN}✓${NC} NVIDIA modules loaded"

# Verify modules loaded
if lsmod | grep -q nvidia; then
    echo -e "${GREEN}✓${NC} NVIDIA modules verified"
else
    echo -e "${RED}✗${NC} Failed to load NVIDIA modules"
fi

# Restart display manager
echo ""
echo "Restarting display manager..."
systemctl start gdm
echo -e "${GREEN}✓${NC} Display manager restarted"

# Test nvidia-smi
echo ""
echo "Testing NVIDIA driver..."
sleep 2  # Give driver a moment to initialize

if nvidia-smi &>/dev/null; then
    echo -e "${GREEN}✓${NC} nvidia-smi working!"
    echo ""
    nvidia-smi
else
    echo -e "${RED}✗${NC} nvidia-smi failed - driver may not be working correctly"
    echo "Check /tmp/nvidia_install.log for details"
    exit 1
fi

# Success summary
echo ""
echo -e "${GREEN}================================${NC}"
echo -e "${GREEN}Installation Complete!${NC}"
echo -e "${GREEN}================================${NC}"
echo ""
echo "Driver version: 580.105.08"
echo "Installation log: /tmp/nvidia_install.log"
echo ""
echo "Next steps:"
echo "  1. Verify GPU detected: nvidia-smi"
echo "  2. Test GPU mining: /home/bob/.btpc/bin/btpc_miner --gpu --address btpc1q..."
echo ""