#!/bin/bash
# Monitor BTPC Testnet Health
# Checks node status, block heights, and system resources

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOG_DIR="$SCRIPT_DIR/logs"
MONITORING_LOG="$LOG_DIR/monitor.log"

echo "=== BTPC Testnet Monitor ===" | tee -a "$MONITORING_LOG"
echo "Timestamp: $(date '+%Y-%m-%d %H:%M:%S')" | tee -a "$MONITORING_LOG"
echo "" | tee -a "$MONITORING_LOG"

# Check running nodes
echo "--- Running Nodes ---" | tee -a "$MONITORING_LOG"
ps aux | grep btpc_node | grep -v grep | awk '{print $2, $11, $12, $13, $14, $15, $16}' | tee -a "$MONITORING_LOG"
echo "" | tee -a "$MONITORING_LOG"

# Check block heights via RPC
echo "--- Block Heights (RPC) ---" | tee -a "$MONITORING_LOG"
NODE1_RPC=$(curl -s http://127.0.0.1:18350 -H 'Content-Type: application/json' -d '{"jsonrpc":"2.0","id":"1","method":"getblockchaininfo","params":[]}' 2>/dev/null)
if [ ! -z "$NODE1_RPC" ]; then
    NODE1_HEIGHT=$(echo "$NODE1_RPC" | jq -r '.result.blocks // 0')
    NODE1_HASH=$(echo "$NODE1_RPC" | jq -r '.result.bestblockhash // "unknown"')
    echo "Node1 (port 18350): ${NODE1_HEIGHT} blocks" | tee -a "$MONITORING_LOG"
    echo "  Best hash: ${NODE1_HASH:0:32}..." | tee -a "$MONITORING_LOG"
else
    echo "Node1: RPC unavailable" | tee -a "$MONITORING_LOG"
fi
echo "" | tee -a "$MONITORING_LOG"

# Database sizes
echo "--- Database Sizes ---" | tee -a "$MONITORING_LOG"
du -sh "$SCRIPT_DIR/data/node"* 2>/dev/null | tee -a "$MONITORING_LOG"
echo "" | tee -a "$MONITORING_LOG"

# Recent errors
echo "--- Recent Errors ---" | tee -a "$MONITORING_LOG"
tail -500 "$LOG_DIR"/*.log 2>/dev/null | grep -i "error\|fatal\|panic" | tail -5 | tee -a "$MONITORING_LOG"
echo "" | tee -a "$MONITORING_LOG"

# Mining rate (check current node log)
if [ -f "/tmp/node1_final.log" ]; then
    echo "--- Mining Performance ---" | tee -a "$MONITORING_LOG"
    BLOCK_COUNT=$(tail -200 "/tmp/node1_final.log" | grep "Block mined" | wc -l)
    echo "Recent blocks mined: $BLOCK_COUNT" | tee -a "$MONITORING_LOG"
    LATEST_REWARD=$(tail -100 "/tmp/node1_final.log" | grep -oP 'Reward: \K[0-9]+' | tail -1)
    if [ ! -z "$LATEST_REWARD" ]; then
        echo "Latest block reward: $LATEST_REWARD satoshis" | tee -a "$MONITORING_LOG"
    fi
    echo "" | tee -a "$MONITORING_LOG"
fi

# System resources
echo "--- System Resources ---" | tee -a "$MONITORING_LOG"
echo "CPU: $(top -bn1 | grep "Cpu(s)" | sed "s/.*, *\([0-9.]*\)%* id.*/\1/" | awk '{print 100 - $1"%"}')" | tee -a "$MONITORING_LOG"
echo "Memory: $(free -h | awk '/^Mem:/ {print $3 "/" $2}')" | tee -a "$MONITORING_LOG"
echo "Disk: $(df -h "$SCRIPT_DIR" | awk 'NR==2 {print $3 "/" $2 " (" $5 " used)"}')" | tee -a "$MONITORING_LOG"
echo "" | tee -a "$MONITORING_LOG"

echo "===========================================" | tee -a "$MONITORING_LOG"
echo "" | tee -a "$MONITORING_LOG"