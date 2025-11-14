#!/bin/bash
# Start a test node on different ports to validate RPC fix

NODE_NUM=${1:-3}
DATA_DIR="$HOME/.btpc${NODE_NUM}"
RPC_PORT=$((18332 + (NODE_NUM - 1) * 2))
P2P_PORT=$((18333 + (NODE_NUM - 1)))

echo "Starting Test Node $NODE_NUM"
echo "  Data directory: $DATA_DIR"
echo "  RPC port: $RPC_PORT"
echo "  P2P port: $P2P_PORT"

# Create data directory
mkdir -p "$DATA_DIR"

# Copy genesis block if it doesn't exist
if [ ! -f "$DATA_DIR/genesis.json" ]; then
    if [ -f "testnet-genesis.json" ]; then
        cp testnet-genesis.json "$DATA_DIR/genesis.json"
        echo "  Genesis block copied"
    else
        echo "  Warning: testnet-genesis.json not found"
    fi
fi

# Start node with mining enabled
../target/release/btpc_node \
    --network testnet \
    --datadir "$DATA_DIR" \
    --rpc-port $RPC_PORT \
    --p2p-port $P2P_PORT \
    --bootstrap 127.0.0.1:18333 \
    --mine \
    --miner-address btpc_test_address_12345 \
    2>&1 | tee logs/node${NODE_NUM}.log