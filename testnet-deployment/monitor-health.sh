#!/bin/bash
# Enhanced testnet health monitoring

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo "=================================================="
echo "BTPC Testnet Health Monitor"
echo "=================================================="
echo ""

# Function to get RPC response
get_rpc() {
    local port=$1
    local method=$2
    local params=${3:-"[]"}

    curl -s -X POST http://127.0.0.1:$port \
        -H "Content-Type: application/json" \
        -d "{\"jsonrpc\":\"2.0\",\"method\":\"$method\",\"params\":$params,\"id\":1}" 2>/dev/null
}

# Check node processes
echo "📊 Node Status:"
echo "==============="
NODE1_PID=$(pgrep -f "btpc_node.*18332" || echo "")
NODE2_PID=$(pgrep -f "btpc_node.*18350" || echo "")

if [ -n "$NODE1_PID" ]; then
    echo -e "${GREEN}✅ Node 1 (Bootstrap):${NC} Running (PID: $NODE1_PID)"
else
    echo -e "${RED}❌ Node 1 (Bootstrap):${NC} Not running"
fi

if [ -n "$NODE2_PID" ]; then
    echo -e "${GREEN}✅ Node 2 (Mining):${NC} Running (PID: $NODE2_PID)"
else
    echo -e "${RED}❌ Node 2 (Mining):${NC} Not running"
fi
echo ""

# Exit if nodes aren't running
if [ -z "$NODE1_PID" ] && [ -z "$NODE2_PID" ]; then
    echo -e "${RED}No nodes running. Start them first.${NC}"
    exit 1
fi

# Get blockchain info
echo "⛓️  Blockchain Status:"
echo "====================="

if [ -n "$NODE1_PID" ]; then
    NODE1_INFO=$(get_rpc 18332 "getblockchaininfo")
    if [ -n "$NODE1_INFO" ]; then
        NODE1_HEIGHT=$(echo "$NODE1_INFO" | jq -r '.result.blocks // 0')
        NODE1_HASH=$(echo "$NODE1_INFO" | jq -r '.result.bestblockhash // "unknown"')
        echo -e "${BLUE}Node 1:${NC}"
        echo "  Height: $NODE1_HEIGHT"
        echo "  Best Block: ${NODE1_HASH:0:16}..."
    fi
fi

if [ -n "$NODE2_PID" ]; then
    NODE2_INFO=$(get_rpc 18350 "getblockchaininfo")
    if [ -n "$NODE2_INFO" ]; then
        NODE2_HEIGHT=$(echo "$NODE2_INFO" | jq -r '.result.blocks // 0')
        NODE2_HASH=$(echo "$NODE2_INFO" | jq -r '.result.bestblockhash // "unknown"')
        echo -e "${BLUE}Node 2:${NC}"
        echo "  Height: $NODE2_HEIGHT"
        echo "  Best Block: ${NODE2_HASH:0:16}..."
    fi
fi
echo ""

# Check if nodes are in sync
if [ -n "$NODE1_HEIGHT" ] && [ -n "$NODE2_HEIGHT" ]; then
    if [ "$NODE1_HEIGHT" -eq "$NODE2_HEIGHT" ]; then
        echo -e "${GREEN}✅ Nodes in sync${NC} (both at height $NODE1_HEIGHT)"
    else
        DIFF=$((NODE2_HEIGHT - NODE1_HEIGHT))
        echo -e "${YELLOW}⚠️  Nodes out of sync${NC} (difference: $DIFF blocks)"
    fi
    echo ""
fi

# Get peer information
echo "🌐 P2P Network:"
echo "==============="

if [ -n "$NODE1_PID" ]; then
    PEER_INFO=$(get_rpc 18332 "getpeerinfo")
    PEER_COUNT=$(echo "$PEER_INFO" | jq -r '.result | length // 0')
    echo -e "${BLUE}Node 1:${NC} $PEER_COUNT peer(s) connected"
fi

if [ -n "$NODE2_PID" ]; then
    PEER_INFO=$(get_rpc 18350 "getpeerinfo")
    PEER_COUNT=$(echo "$PEER_INFO" | jq -r '.result | length // 0')
    echo -e "${BLUE}Node 2:${NC} $PEER_COUNT peer(s) connected"
fi
echo ""

# Check database sizes
echo "💾 Storage:"
echo "==========="
if [ -d ~/.btpc/blockchain ]; then
    NODE1_SIZE=$(du -sh ~/.btpc/blockchain 2>/dev/null | cut -f1)
    echo -e "${BLUE}Node 1:${NC} $NODE1_SIZE"
fi

if [ -d ~/.btpc2/blockchain ]; then
    NODE2_SIZE=$(du -sh ~/.btpc2/blockchain 2>/dev/null | cut -f1)
    echo -e "${BLUE}Node 2:${NC} $NODE2_SIZE"
fi
echo ""

# System resources
echo "🖥️  System Resources:"
echo "===================="
CPU=$(top -bn1 | grep "Cpu(s)" | sed "s/.*, *\([0-9.]*\)%* id.*/\1/" | awk '{print 100 - $1"%"}')
MEM=$(free -h | awk '/^Mem:/ {print $3 "/" $2}')
echo "CPU Usage: $CPU"
echo "Memory: $MEM"
echo ""

# Mining stats (if applicable)
if [ -n "$NODE2_PID" ]; then
    echo "⛏️  Mining Stats (Node 2):"
    echo "========================="
    MINING_INFO=$(get_rpc 18350 "getmininginfo")
    if [ -n "$MINING_INFO" ]; then
        HASHRATE=$(echo "$MINING_INFO" | jq -r '.result.networkhashps // "N/A"')
        DIFFICULTY=$(echo "$MINING_INFO" | jq -r '.result.difficulty // "N/A"')
        echo "Network Hashrate: $HASHRATE"
        echo "Difficulty: $DIFFICULTY"
    fi
    echo ""
fi

# Transaction pool
echo "💳 Mempool:"
echo "==========="
if [ -n "$NODE1_PID" ]; then
    # Note: Need to implement getmempoolinfo RPC
    echo -e "${BLUE}Node 1:${NC} Check logs for mempool activity"
fi
if [ -n "$NODE2_PID" ]; then
    echo -e "${BLUE}Node 2:${NC} Check logs for mempool activity"
fi
echo ""

# Recent log activity
echo "📝 Recent Activity (last 5 lines):"
echo "===================================="
if [ -f logs/node1.log ]; then
    echo -e "${BLUE}Node 1:${NC}"
    tail -5 logs/node1.log | sed 's/^/  /'
    echo ""
fi

if [ -f logs/node2.log ]; then
    echo -e "${BLUE}Node 2:${NC}"
    tail -5 logs/node2.log | sed 's/^/  /'
    echo ""
fi

echo "=================================================="
echo "Monitoring complete. Run this script periodically"
echo "or check logs with: tail -f logs/node*.log"
echo "=================================================="
