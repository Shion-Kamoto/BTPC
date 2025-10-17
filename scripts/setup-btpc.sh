#!/bin/bash
# BTPC Setup Script
# This script builds the BTPC binaries and sets up the required directory structure

set -e  # Exit on error

echo "=========================================="
echo "BTPC Setup Script"
echo "=========================================="
echo ""

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Configuration
BTPC_HOME="${HOME}/.btpc"
BTPC_BIN="${BTPC_HOME}/bin"
BTPC_DATA="${BTPC_HOME}/data"
BTPC_CONFIG="${BTPC_HOME}/config"
BTPC_LOGS="${BTPC_HOME}/logs"
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo -e "${BLUE}Project root: ${PROJECT_ROOT}${NC}"
echo -e "${BLUE}BTPC home: ${BTPC_HOME}${NC}"
echo ""

# Step 1: Create directory structure
echo -e "${GREEN}Step 1: Creating directory structure...${NC}"
mkdir -p "${BTPC_BIN}"
mkdir -p "${BTPC_DATA}"/{wallet,blockchain,tx_storage,address_book,node,desktop-node}
mkdir -p "${BTPC_CONFIG}/security"
mkdir -p "${BTPC_LOGS}"
echo "✓ Directory structure created"
echo ""

# Step 2: Build binaries
echo -e "${GREEN}Step 2: Building BTPC binaries (this may take a few minutes)...${NC}"
cd "${PROJECT_ROOT}"

if cargo build --release --bin btpc_node --bin btpc_miner --bin btpc_wallet; then
    echo "✓ Binaries built successfully"
else
    echo -e "${RED}✗ Build failed${NC}"
    exit 1
fi
echo ""

# Step 3: Install binaries
echo -e "${GREEN}Step 3: Installing binaries to ${BTPC_BIN}...${NC}"
cp "${PROJECT_ROOT}/target/release/btpc_node" "${BTPC_BIN}/"
cp "${PROJECT_ROOT}/target/release/btpc_miner" "${BTPC_BIN}/"
cp "${PROJECT_ROOT}/target/release/btpc_wallet" "${BTPC_BIN}/"
chmod +x "${BTPC_BIN}"/*
echo "✓ Binaries installed"
echo ""

# Step 4: Create template configuration files
echo -e "${GREEN}Step 4: Creating template configuration files...${NC}"

# Create empty UTXO file
if [ ! -f "${BTPC_DATA}/wallet/wallet_utxos.json" ]; then
    echo "[]" > "${BTPC_DATA}/wallet/wallet_utxos.json"
    echo "✓ Created wallet_utxos.json"
fi

# Create empty transactions file
if [ ! -f "${BTPC_DATA}/wallet/wallet_transactions.json" ]; then
    echo "[]" > "${BTPC_DATA}/wallet/wallet_transactions.json"
    echo "✓ Created wallet_transactions.json"
fi

# Create empty address book
if [ ! -f "${BTPC_DATA}/address_book/address_book.json" ]; then
    echo "[]" > "${BTPC_DATA}/address_book/address_book.json"
    echo "✓ Created address_book.json"
fi

# Create mining stats file
if [ ! -f "${BTPC_DATA}/mining_stats.json" ]; then
    cat > "${BTPC_DATA}/mining_stats.json" << 'EOF'
{
  "total_blocks_found": 0,
  "total_hashes_computed": 0,
  "average_hashrate": 0.0,
  "sessions": []
}
EOF
    echo "✓ Created mining_stats.json"
fi
echo ""

# Step 5: Verify installation
echo -e "${GREEN}Step 5: Verifying installation...${NC}"
ERRORS=0

for binary in btpc_node btpc_miner btpc_wallet; do
    if [ -x "${BTPC_BIN}/${binary}" ]; then
        echo "✓ ${binary} installed and executable"
    else
        echo -e "${RED}✗ ${binary} missing or not executable${NC}"
        ERRORS=$((ERRORS + 1))
    fi
done
echo ""

# Summary
echo "=========================================="
echo "Setup Summary"
echo "=========================================="
echo ""
echo "BTPC Home: ${BTPC_HOME}"
echo "Binaries: ${BTPC_BIN}"
echo "Data: ${BTPC_DATA}"
echo "Config: ${BTPC_CONFIG}"
echo "Logs: ${BTPC_LOGS}"
echo ""

if [ $ERRORS -eq 0 ]; then
    echo -e "${GREEN}✓ Setup completed successfully!${NC}"
    echo ""
    echo "Next steps:"
    echo "  1. Add BTPC binaries to PATH (optional):"
    echo "     export PATH=\"\${HOME}/.btpc/bin:\${PATH}\""
    echo ""
    echo "  2. Create a wallet:"
    echo "     btpc_wallet generate"
    echo ""
    echo "  3. Start the node:"
    echo "     btpc_node --network mainnet"
    echo ""
    echo "  4. Start mining:"
    echo "     btpc_miner --address YOUR_ADDRESS"
    echo ""
    echo "  5. Or use the desktop app:"
    echo "     cd btpc-desktop-app && npm run tauri:dev"
    exit 0
else
    echo -e "${RED}✗ Setup completed with ${ERRORS} error(s)${NC}"
    exit 1
fi
