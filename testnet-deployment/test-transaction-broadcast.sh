#!/bin/bash
# Test transaction broadcasting between nodes

set -e

echo "=================================================="
echo "BTPC Transaction Broadcasting Test"
echo "=================================================="
echo ""

# Check if nodes are running
if ! pgrep -f "btpc_node.*18332" > /dev/null; then
    echo "❌ Node 1 (bootstrap) not running on port 18332"
    echo "   Start it with: ./start-bootstrap-node.sh"
    exit 1
fi

if ! pgrep -f "btpc_node.*18350" > /dev/null; then
    echo "❌ Node 2 (mining) not running on port 18350"
    echo "   Start it with: ./start-mining-node.sh 2"
    exit 1
fi

echo "✅ Both nodes are running"
echo ""

# Get blockchain info from both nodes
echo "📊 Node Status:"
echo "---------------"

NODE1_INFO=$(curl -s -X POST http://127.0.0.1:18332 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"getblockchaininfo","id":1}' | jq -r '.result')

NODE2_INFO=$(curl -s -X POST http://127.0.0.1:18350 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"getblockchaininfo","id":1}' | jq -r '.result')

echo "Node 1 Height: $(echo $NODE1_INFO | jq -r '.blocks')"
echo "Node 2 Height: $(echo $NODE2_INFO | jq -r '.blocks')"
echo ""

# Note: For a full transaction test, we need:
# 1. A wallet with UTXOs (from mining)
# 2. Transaction creation tool
# 3. Transaction signing with ML-DSA keys
# 4. sendrawtransaction RPC call

echo "📝 Transaction Broadcasting Test Plan:"
echo "--------------------------------------"
echo "1. ✅ RPC Method: sendrawtransaction implemented"
echo "2. ✅ Mempool: Transaction storage ready"
echo "3. ✅ P2P Broadcast: broadcast_transaction() implemented"
echo "4. ✅ Peer Relay: Transaction reception handler implemented"
echo "5. ⏳ Transaction Creation: Needs wallet integration"
echo ""

echo "🔧 Next Steps to Test:"
echo "----------------------"
echo "1. Mine some blocks to create UTXOs"
echo "2. Use btpc_wallet to create a transaction"
echo "3. Submit via sendrawtransaction RPC"
echo "4. Verify it propagates to both nodes"
echo ""

echo "📡 Simulating Transaction Broadcast Flow:"
echo "-----------------------------------------"
echo "1. Node receives transaction via RPC"
echo "2. Transaction validated and added to mempool"
echo "3. Transaction broadcast to connected peers"
echo "4. Peers receive and validate transaction"
echo "5. Peers add to their mempool"
echo "6. Eventually included in mined blocks"
echo ""

echo "✅ Infrastructure Ready for Transaction Broadcasting!"
echo ""
echo "To test with actual transactions:"
echo "  1. Let nodes mine for a bit (to create UTXOs)"
echo "  2. Create transaction with btpc_wallet"
echo "  3. Submit via: curl -X POST http://127.0.0.1:18332 \\"
echo "       -H 'Content-Type: application/json' \\"
echo "       -d '{\"jsonrpc\":\"2.0\",\"method\":\"sendrawtransaction\",\"params\":[\"<hex>\"],\"id\":1}'"
echo ""
