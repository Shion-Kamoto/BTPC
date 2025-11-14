#!/bin/bash
# Start BTPC Testnet Bootstrap Node (Node 1)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "🚀 Starting BTPC Testnet Bootstrap Node..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Create directories
mkdir -p "$SCRIPT_DIR/data/node1"
mkdir -p "$SCRIPT_DIR/logs"

# Copy genesis block to node data directory
if [ -f "$SCRIPT_DIR/data/genesis.json" ]; then
    echo "📦 Copying genesis block to node data directory..."
    cp "$SCRIPT_DIR/data/genesis.json" "$SCRIPT_DIR/data/node1/genesis.json"
fi

# Start node
echo "🔧 Network: Testnet"
echo "🔧 RPC Port: 18332"
echo "🔧 P2P Port: 18333"
echo "🔧 Data Dir: $SCRIPT_DIR/data/node1"
echo "🔧 Log File: $SCRIPT_DIR/logs/node1.log"
echo ""
echo "Starting node... (Press Ctrl+C to stop)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

"$PROJECT_ROOT/target/debug/btpc_node" \
    --network testnet \
    --datadir "$SCRIPT_DIR/data/node1" \
    --rpcport 18332 \
    --rpcbind 127.0.0.1 \
    --listen 0.0.0.0:18333 \
    2>&1 | tee "$SCRIPT_DIR/logs/node1.log"