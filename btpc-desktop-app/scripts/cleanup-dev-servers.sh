#!/bin/bash
#
# BTPC Dev Server Cleanup Script
#
# Kills all orphaned dev server processes and their children
# Constitution Compliance: Article XI, Section 11.5 - No Orphaned Processes
#

set -e

echo "🧹 BTPC Dev Server Cleanup"
echo "================================"

# Count processes before cleanup
echo ""
echo "📊 Current process count:"
TAURI_COUNT=$(ps aux | grep -E "(tauri:dev|tauri dev)" | grep -v grep | wc -l)
NODE_COUNT=$(ps aux | grep "btpc-desktop-app" | grep "node_modules/.bin/tauri" | grep -v grep | wc -l)
APP_COUNT=$(ps aux | grep "target/debug/btpc-desktop-app" | grep -v grep | wc -l)
ZOMBIE_COUNT=$(ps aux | grep defunct | grep -E "(btpc_node|btpc_miner|btpc_wallet)" | grep -v grep | wc -l)

echo "  - npm run tauri:dev: $TAURI_COUNT"
echo "  - tauri dev (node): $NODE_COUNT"
echo "  - btpc-desktop-app: $APP_COUNT"
echo "  - zombie processes: $ZOMBIE_COUNT"

if [ "$TAURI_COUNT" -eq 0 ] && [ "$NODE_COUNT" -eq 0 ] && [ "$APP_COUNT" -eq 0 ] && [ "$ZOMBIE_COUNT" -eq 0 ]; then
    echo ""
    echo "✅ No dev server processes found - system is clean!"
    exit 0
fi

echo ""
echo "🛑 Stopping processes..."

# Kill npm run tauri:dev processes
if [ "$TAURI_COUNT" -gt 0 ]; then
    echo "  - Killing npm run tauri:dev processes..."
    pkill -f "npm run tauri:dev" || true
fi

# Kill tauri dev (node) processes
if [ "$NODE_COUNT" -gt 0 ]; then
    echo "  - Killing tauri dev (node) processes..."
    pkill -f "node.*tauri dev" || true
fi

# Kill btpc-desktop-app debug builds
if [ "$APP_COUNT" -gt 0 ]; then
    echo "  - Killing btpc-desktop-app processes..."
    pkill -f "target/debug/btpc-desktop-app" || true
fi

# Clean up zombie processes (defunct btpc_node, btpc_miner, etc.)
if [ "$ZOMBIE_COUNT" -gt 0 ]; then
    echo "  - Reaping zombie processes..."
    ps aux | grep defunct | grep -E "(btpc_node|btpc_miner|btpc_wallet)" | grep -v grep | awk '{print $2}' | xargs -r kill -9 2>/dev/null || true
fi

# Wait a second for cleanup
sleep 1

# Verify cleanup
echo ""
echo "📊 Process count after cleanup:"
REMAINING=$(ps aux | grep -E "(tauri:dev|tauri dev|btpc-desktop-app)" | grep -v grep | wc -l)
ZOMBIES=$(ps aux | grep defunct | grep -E "(btpc_node|btpc_miner|btpc_wallet)" | grep -v grep | wc -l)

echo "  - Dev processes remaining: $REMAINING"
echo "  - Zombie processes remaining: $ZOMBIES"

if [ "$REMAINING" -eq 0 ] && [ "$ZOMBIES" -eq 0 ]; then
    echo ""
    echo "✅ All dev server processes cleaned up successfully!"
    exit 0
else
    echo ""
    echo "⚠️  Warning: Some processes may still be running"
    echo "   Try running this script again or manually kill:"
    ps aux | grep -E "(tauri:dev|tauri dev|btpc-desktop-app|defunct)" | grep -v grep
    exit 1
fi