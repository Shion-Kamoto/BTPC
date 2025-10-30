#!/bin/bash
# UTXO Reassignment Script
# Reassigns all existing UTXOs to the current wallet address

set -e

WALLET_DIR="$HOME/.btpc/data/wallet"
UTXO_FILE="$WALLET_DIR/wallet_utxos.json"
WALLET_FILE="$WALLET_DIR/wallet.json"
BACKUP_DIR="$WALLET_DIR/backups"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}======================================${NC}"
echo -e "${BLUE}   BTPC UTXO Reassignment Script${NC}"
echo -e "${BLUE}======================================${NC}"
echo

# Check if required files exist
if [ ! -f "$WALLET_FILE" ]; then
    echo -e "${RED}Error: Wallet file not found at $WALLET_FILE${NC}"
    exit 1
fi

if [ ! -f "$UTXO_FILE" ]; then
    echo -e "${RED}Error: UTXO file not found at $UTXO_FILE${NC}"
    exit 1
fi

# Get current wallet address
CURRENT_ADDRESS=$(jq -r '.address' "$WALLET_FILE")
echo -e "${GREEN}Current wallet address:${NC}"
echo -e "${YELLOW}${CURRENT_ADDRESS:0:64}...${NC}"
echo

# Count existing UTXOs
UTXO_COUNT=$(jq '. | length' "$UTXO_FILE")
echo -e "${BLUE}Found ${UTXO_COUNT} UTXOs to reassign${NC}"
echo

# Create backup directory
mkdir -p "$BACKUP_DIR"

# Create backup with timestamp
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_FILE="$BACKUP_DIR/wallet_utxos_backup_${TIMESTAMP}.json"
cp "$UTXO_FILE" "$BACKUP_FILE"
echo -e "${GREEN}✓ Backup created:${NC} $BACKUP_FILE"
echo

# Show sample of UTXOs before reassignment
echo -e "${BLUE}Sample of UTXOs before reassignment:${NC}"
jq -r '.[0:3] | .[] | "  TXID: \(.txid) | Address: \(.address[0:32])... | Value: \(.value_credits) credits"' "$UTXO_FILE"
echo

# Reassign all UTXOs to current wallet address
echo -e "${YELLOW}Reassigning UTXOs to current wallet address...${NC}"
jq --arg addr "$CURRENT_ADDRESS" 'map(.address = $addr)' "$UTXO_FILE" > "${UTXO_FILE}.tmp"
mv "${UTXO_FILE}.tmp" "$UTXO_FILE"

echo -e "${GREEN}✓ All UTXOs reassigned successfully!${NC}"
echo

# Show sample after reassignment
echo -e "${BLUE}Sample of UTXOs after reassignment:${NC}"
jq -r '.[0:3] | .[] | "  TXID: \(.txid) | Address: \(.address[0:32])... | Value: \(.value_credits) credits"' "$UTXO_FILE"
echo

# Calculate total balance
TOTAL_CREDITS=$(jq '[.[] | select(.spent == false) | .value_credits] | add' "$UTXO_FILE")
TOTAL_BTP=$(echo "scale=8; $TOTAL_CREDITS / 100000000" | bc)

echo -e "${BLUE}======================================${NC}"
echo -e "${GREEN}   Reassignment Complete!${NC}"
echo -e "${BLUE}======================================${NC}"
echo -e "${GREEN}Total unspent UTXOs:${NC} $(jq '[.[] | select(.spent == false)] | length' "$UTXO_FILE")"
echo -e "${GREEN}Total balance:${NC} ${TOTAL_CREDITS} credits (${TOTAL_BTP} BTP)"
echo -e "${GREEN}Backup location:${NC} $BACKUP_FILE"
echo

echo -e "${YELLOW}Note:${NC} Restart the desktop app to see the updated balance."
echo
