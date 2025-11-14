#!/bin/bash
# Start BTPC Testnet Mining Node (Node 2)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

NODE_NUM="${1:-2}"
RPC_PORT=$((18330 + NODE_NUM * 10))
P2P_PORT=$((18333 + NODE_NUM - 1))

echo "🚀 Starting BTPC Testnet Mining Node $NODE_NUM..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Create directories
mkdir -p "$SCRIPT_DIR/data/node$NODE_NUM"
mkdir -p "$SCRIPT_DIR/logs"

# Copy genesis block to node data directory
if [ -f "$SCRIPT_DIR/data/genesis.json" ]; then
    echo "📦 Copying genesis block to node data directory..."
    cp "$SCRIPT_DIR/data/genesis.json" "$SCRIPT_DIR/data/node$NODE_NUM/genesis.json"
fi

# Start node with mining
echo "🔧 Network: Testnet"
echo "🔧 RPC Port: $RPC_PORT"
echo "🔧 P2P Port: $P2P_PORT"
echo "🔧 Data Dir: $SCRIPT_DIR/data/node$NODE_NUM"
echo "🔧 Log File: $SCRIPT_DIR/logs/node$NODE_NUM.log"
echo "⛏️  Mining: ENABLED"
echo ""
echo "Starting node... (Press Ctrl+C to stop)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

"$PROJECT_ROOT/target/debug/btpc_node" \
    --network testnet \
    --datadir "$SCRIPT_DIR/data/node$NODE_NUM" \
    --rpcport $RPC_PORT \
    --rpcbind 127.0.0.1 \
    --listen "0.0.0.0:$P2P_PORT" \
    --connect 127.0.0.1:18333 \
    --mine \
    2>&1 | tee "$SCRIPT_DIR/logs/node$NODE_NUM.log"