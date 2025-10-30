#!/bin/bash

# Test Mining UI Script
# This script helps you test the mining functionality through the BTPC desktop app

echo "========================================="
echo "BTPC Mining UI Test"
echo "========================================="
echo ""

# Check if BTPC app is running
echo "1. Checking if BTPC desktop app is running..."
if pgrep -f "btpc-desktop-app" > /dev/null; then
    echo "   ✅ BTPC desktop app is running"
else
    echo "   ❌ BTPC desktop app is NOT running"
    echo "   Please start the app first with: npm run tauri:dev"
    exit 1
fi

# Check if node is running
echo ""
echo "2. Checking if BTPC node is running..."
if pgrep -f "btpc_node.*regtest" > /dev/null; then
    NODE_PID=$(pgrep -f "btpc_node.*regtest")
    echo "   ✅ BTPC node is running (PID: $NODE_PID)"
else
    echo "   ❌ BTPC node is NOT running"
    echo "   The mining page will show 'Node is offline'"
    exit 1
fi

# Check if node is responding
echo ""
echo "3. Testing node connectivity..."
RESPONSE=$(curl -s http://127.0.0.1:18360 -H 'Content-Type: application/json' -d '{"jsonrpc":"2.0","id":"1","method":"getblockchaininfo","params":[]}')
if [ $? -eq 0 ]; then
    BLOCKS=$(echo "$RESPONSE" | grep -o '"blocks":[0-9]*' | cut -d':' -f2)
    echo "   ✅ Node is responding"
    echo "   Current blockchain height: $BLOCKS blocks"
else
    echo "   ❌ Node is not responding"
    exit 1
fi

# Stop any standalone miners
echo ""
echo "4. Stopping standalone miners (if any)..."
STANDALONE_MINERS=$(pgrep -f "btpc_miner" | grep -v defunct || true)
if [ -n "$STANDALONE_MINERS" ]; then
    echo "   Found standalone miners: $STANDALONE_MINERS"
    echo "   Killing them..."
    kill $STANDALONE_MINERS 2>/dev/null
    sleep 1
    echo "   ✅ Standalone miners stopped"
else
    echo "   ✅ No standalone miners found"
fi

# Instructions
echo ""
echo "========================================="
echo "READY TO TEST!"
echo "========================================="
echo ""
echo "Now follow these steps in the BTPC desktop app:"
echo ""
echo "1. Open the BTPC desktop application window"
echo "2. Navigate to the 'Mining' page (click Mining in sidebar)"
echo "3. Open DevTools to see debug logs:"
echo "   - Right-click anywhere → 'Inspect Element'"
echo "   - Or press F12"
echo "   - Click the 'Console' tab"
echo "4. Click 'Start Mining' or 'Quick Start Mining' button"
echo "5. Watch the console for these debug messages:"
echo "   🔍 Quick Start Mining clicked"
echo "   📋 Wallets loaded: ..."
echo "   ✅ Using wallet: ..."
echo ""
echo "6. If successful, you should see:"
echo "   - Live hashrate display at top of page"
echo "   - Mining activity cards appearing as blocks are mined"
echo "   - Block details with UTXO information"
echo ""
echo "========================================="
echo "Press Enter to continue..."
read

echo "Monitoring backend logs for mining activity..."
echo "Watch this terminal for mining output from the backend"
echo "Press Ctrl+C to stop monitoring"
echo ""

# This will just keep the script running so you can monitor
tail -f /dev/null