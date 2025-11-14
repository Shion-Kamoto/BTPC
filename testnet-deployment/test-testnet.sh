#!/bin/bash
# Quick Testnet Verification Script

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "🧪 BTPC Testnet Verification"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Check binaries
echo "1️⃣  Checking binaries..."
if [ ! -f "$PROJECT_ROOT/target/debug/btpc_node" ]; then
    echo "❌ btpc_node not found. Run: cargo build"
    exit 1
fi
echo "✅ btpc_node binary found"

if [ ! -f "$PROJECT_ROOT/target/debug/btpc_wallet" ]; then
    echo "❌ btpc_wallet not found. Run: cargo build"
    exit 1
fi
echo "✅ btpc_wallet binary found"

if [ ! -f "$PROJECT_ROOT/target/debug/btpc_miner" ]; then
    echo "❌ btpc_miner not found. Run: cargo build"
    exit 1
fi
echo "✅ btpc_miner binary found"

echo ""

# Check genesis block
echo "2️⃣  Checking genesis block..."
if [ ! -f "$SCRIPT_DIR/data/genesis.json" ]; then
    echo "❌ Genesis block not found"
    exit 1
fi
echo "✅ Genesis block found"

# Parse genesis hash
GENESIS_HASH=$(grep -o '"hash":"[^"]*"' "$SCRIPT_DIR/data/genesis.json" | head -1 | cut -d'"' -f4)
echo "   Hash: $GENESIS_HASH"

echo ""

# Check configuration files
echo "3️⃣  Checking configuration files..."
for i in 1 2 3; do
    if [ -f "$SCRIPT_DIR/config/node$i.toml" ]; then
        echo "✅ node$i.toml found"
    else
        echo "⚠️  node$i.toml not found (optional)"
    fi
done

echo ""

# Check directory structure
echo "4️⃣  Checking directory structure..."
for dir in config data logs; do
    if [ -d "$SCRIPT_DIR/$dir" ]; then
        echo "✅ $dir/ directory exists"
    else
        echo "❌ $dir/ directory missing"
        mkdir -p "$SCRIPT_DIR/$dir"
        echo "   Created $dir/"
    fi
done

echo ""

# Check startup scripts
echo "5️⃣  Checking startup scripts..."
if [ -x "$SCRIPT_DIR/start-bootstrap-node.sh" ]; then
    echo "✅ start-bootstrap-node.sh (executable)"
else
    echo "⚠️  start-bootstrap-node.sh not executable"
    chmod +x "$SCRIPT_DIR/start-bootstrap-node.sh" 2>/dev/null || true
fi

if [ -x "$SCRIPT_DIR/start-mining-node.sh" ]; then
    echo "✅ start-mining-node.sh (executable)"
else
    echo "⚠️  start-mining-node.sh not executable"
    chmod +x "$SCRIPT_DIR/start-mining-node.sh" 2>/dev/null || true
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Testnet setup verification complete!"
echo ""
echo "Next steps:"
echo "  1. Start bootstrap node: ./start-bootstrap-node.sh"
echo "  2. In another terminal, start mining node: ./start-mining-node.sh 2"
echo "  3. Monitor logs: tail -f logs/node1.log"
echo ""
echo "For full documentation, see: README.md"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"