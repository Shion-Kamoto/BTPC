# BTPC Desktop App - Quick Start Guide

## Prerequisites

1. **Testnet node must be running** on port 18350
2. Desktop app binaries built in `~/.btpc/bin/`

## Starting the Desktop App

### Option 1: Development Mode (Recommended)
```bash
cd /home/bob/BTPC/BTPC/btpc-desktop-app
npm run tauri:dev
```

### Option 2: Direct Binary
```bash
./src-tauri/target/debug/btpc-desktop-app
```

### Option 3: Release Build
```bash
./src-tauri/target/release/btpc-desktop-app
```

## Verify Everything is Working

### 1. Check Testnet Node
```bash
# Should return blockchain info
curl -s http://127.0.0.1:18350 -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"getblockchaininfo","params":[]}'
```

### 2. Check No Desktop Node Running
```bash
# Should return empty (no desktop node)
ps aux | grep "btpc_node.*desktop" | grep -v grep
```

### 3. Check Desktop App Running
```bash
# Should show the app process
ps aux | grep btpc-desktop-app | grep -v grep
```

## Configuration

### Desktop App (Wallet Client)
- **Mode**: Wallet-only (no local node)
- **RPC Host**: 127.0.0.1:18350 (connects to testnet)
- **Data Dir**: `~/.btpc/data/wallet/`
- **Logs**: None (wallet client doesn't create logs)

### Testnet Node (External)
- **RPC Port**: 18350
- **P2P Port**: 18351
- **Data Dir**: `testnet-deployment/data/node1/`
- **Mining**: Active
- **Logs**: `testnet-deployment/data/node1/*.log`

## Common Issues

### Issue: "RPC connection failed"
**Solution**: Start the testnet node first
```bash
cd /home/bob/BTPC/BTPC/testnet-deployment
# Check if node is running
ps aux | grep btpc_node | grep 18350
```

### Issue: "Address already in use"
**Solution**: Kill any desktop nodes that shouldn't be running
```bash
pkill -f "btpc_node.*desktop-node"
```

### Issue: App won't start
**Solution**: Rebuild the app
```bash
cd /home/bob/BTPC/BTPC/btpc-desktop-app
cargo build
npm run tauri:dev
```

## Available Features

✅ **Wallet Management**
- Create wallets
- View balances
- Generate addresses
- Display QR codes

✅ **Transactions**
- Send BTPC
- View transaction history
- Check UTXOs

✅ **Mining**
- Start/stop mining to wallet address
- View mining status
- Track mining rewards

✅ **Blockchain Explorer**
- View recent blocks
- Search transactions
- Check blockchain info

## File Locations

### Wallet Data
```
~/.btpc/data/wallet/
├── wallet.json                    # Wallet database (5 wallets)
├── wallet_utxos.json             # UTXO cache
└── wallet_transactions.json      # Transaction history
```

### Application
```
/home/bob/BTPC/BTPC/btpc-desktop-app/
├── src-tauri/                    # Rust backend
│   ├── src/main.rs              # Main application logic
│   └── target/debug/            # Built binaries
├── ui/                          # HTML/CSS/JS frontend
│   ├── index.html              # Dashboard
│   ├── wallet-manager.html     # Wallet management
│   ├── transactions.html       # Transactions
│   ├── mining.html             # Mining controls
│   ├── node.html               # Node status (shows testnet)
│   └── settings.html           # App settings
└── NODE_AUTOSTART_FIX.md       # Fix documentation
```

## Quick Commands

### Start Everything
```bash
# 1. Verify testnet node is running
ps aux | grep "btpc_node.*18350" | grep -v grep

# 2. Start desktop app
cd /home/bob/BTPC/BTPC/btpc-desktop-app
npm run tauri:dev
```

### Stop Everything
```bash
# Stop desktop app (Ctrl+C in terminal or close window)

# Testnet node keeps running (don't stop it)
```

### Check Status
```bash
# Testnet node status
curl -s http://127.0.0.1:18350 -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"getblockchaininfo","params":[]}' | jq

# Desktop app process
ps aux | grep btpc-desktop-app | grep -v grep

# Wallet count
cat ~/.btpc/data/wallet/wallet.json | jq 'keys | length'
```

## Build Commands

### Debug Build (Fast)
```bash
cd /home/bob/BTPC/BTPC/btpc-desktop-app
cargo build
```

### Release Build (Optimized)
```bash
cargo build --release
```

### Clean Build
```bash
cargo clean
cargo build
```

## Success Indicators

✅ Desktop app starts without creating `~/.btpc/logs/node.log`
✅ No port conflicts on 18350
✅ Wallet data loads correctly
✅ RPC connection to testnet works
✅ No "node.err" files created

## Support

For issues, check:
1. `NODE_AUTOSTART_FIX.md` - Detailed fix documentation
2. `DESKTOP_APP_STATUS.md` - Complete status and history
3. `TROUBLESHOOTING.md` - Debug guide
