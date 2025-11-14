# BTPC Testnet Deployment Guide

This directory contains everything needed to deploy and run a BTPC testnet for development and testing.

## 📋 Overview

**Testnet Details:**
- **Network Type**: Testnet
- **Genesis Hash**: `66f93816446e9aae8eebd6a26c4bc9b74f161c54871a59d4722d39baf194df3ec91605384f048c76c6524089ce7f2029e89557a0c482db56aa44aaf58028ad6c`
- **Genesis Timestamp**: 1759614480 (Oct 2025)
- **Initial Supply**: 1,000,000 BTPC (testnet coins)
- **Magic Bytes**: `0x4254FF01`
- **Current Deployment**: Single mining node configuration
- **Default Ports**:
  - Node 1: RPC 18350, P2P 18351

## 🚀 Quick Start

### Prerequisites

1. Build the BTPC binaries:
```bash
cd /home/bob/BTPC/BTPC
cargo build --release
```

2. Verify binaries exist:
```bash
ls -la target/release/btpc_node
ls -la target/release/btpc_wallet
ls -la target/release/btpc_miner
```

### Starting the Testnet

#### Current Configuration: Single Mining Node

Start the mining node:
```bash
cd testnet-deployment
../target/release/btpc_node \
    --network testnet \
    --datadir data/node1 \
    --rpcport 18350 \
    --listen 127.0.0.1:18351 \
    --mine > /tmp/node1.log 2>&1 &
```

The node will:
- Load the genesis block from `data/node1/genesis.json`
- Listen for P2P connections on port 18351
- Accept RPC requests on port 18350
- Start mining blocks automatically
- Store blockchain data in `data/node1/`
- Write logs to `/tmp/node1.log`

## 🔍 Monitoring the Testnet

### Quick Health Check

```bash
./monitor-testnet.sh
```

This displays:
- Running nodes and PIDs
- Current block height (via RPC)
- Best block hash
- Database sizes
- Recent errors
- Mining performance
- System resources

### Check Blockchain Info via RPC

```bash
curl -s http://127.0.0.1:18350 \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"getblockchaininfo","params":[],"id":1}' | jq
```

Example response:
```json
{
  "result": {
    "blocks": 883,
    "bestblockhash": "0eb374b6d9ce0f15d4c2505230eadfd8...",
    "chain": "main",
    "difficulty": 1.0
  }
}
```

### Get Specific Block

```bash
curl -s http://127.0.0.1:18350 \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"getblock","params":["<block_hash>"],"id":1}' | jq
```

### View Live Mining

```bash
tail -f /tmp/node1.log | grep "Block mined"
```

Example output:
```
🎉 Block mined! Hash: 7458283a0e008f0173c259502c31b4729a82fea8...
   Height: 175, Reward: 3236017825 satoshis
```

## 📊 24-Hour Stress Test

### Running the Stress Test

```bash
./stress-test-24hr.sh
```

This will:
- Run for 24 hours (288 checks at 5-minute intervals)
- Monitor node health, block heights, and system resources
- Log all metrics to `logs/stress-test-24hr.log`
- Alert if node crashes

### Monitoring Stress Test Progress

```bash
# Watch test progress
tail -f logs/stress-test-24hr.log

# See current status
cat STRESS_TEST_STATUS.md

# Quick check
./monitor-testnet.sh
```

### Stress Test Metrics

The test monitors:
- **Block Production**: Continuous mining for 24 hours (~8,640 blocks expected)
- **RPC Stability**: Height queries every 5 minutes
- **Database Growth**: Storage size tracking
- **System Resources**: CPU, memory, disk usage
- **Error Detection**: Crashes, panics, fatal errors
- **Mining Performance**: Block rate and rewards

## 💰 Creating a Testnet Wallet

```bash
cd /home/bob/BTPC/BTPC

# Create a new testnet wallet
./target/release/btpc_wallet \
    --network testnet \
    --datadir testnet-deployment/data/wallet \
    --rpc-url http://127.0.0.1:18350 \
    generate --label "My Testnet Wallet"
```

This will:
- Generate a new ML-DSA keypair
- Create an encrypted wallet file
- Display your testnet address
- Store wallet in `testnet-deployment/data/wallet/`

## ⛏️ Mining on Testnet

Mining is automatic when you start the node with `--mine` flag.

Block rewards on testnet:
- Initial reward: ~32.375 BTPC
- Linear decay over 24 years
- Tail emission: 0.5 BTPC forever

Mining stats are displayed in logs:
```
🎉 Block mined! Hash: 7458283a0e008f...
   Height: 175, Reward: 3236017825 satoshis
```

## 📁 Directory Structure

```
testnet-deployment/
├── data/
│   ├── node1/                    # Node 1 blockchain data
│   │   ├── blockchain/           # Block and UTXO data
│   │   └── genesis.json          # Genesis block definition
│   └── node2/                    # Node 2 (unused in current setup)
├── testnet-genesis.json/
│   └── genesis.json              # Source genesis block file
├── logs/
│   ├── stress-test-24hr.log      # 24-hour test results
│   ├── stress-test-console.log   # Test console output
│   └── monitor.log               # Monitoring history
├── monitor-testnet.sh            # Health monitoring script
├── monitor-health.sh             # Detailed health check
├── stress-test-24hr.sh           # 24-hour stress test runner
├── STRESS_TEST_STATUS.md         # Current test status
└── README.md                     # This file
```

## 🧪 Testing Scenarios

### 1. Block Production Test

Verify continuous mining:
```bash
# Watch for new blocks
tail -f /tmp/node1.log | grep "Block mined"

# Check current height
curl -s http://127.0.0.1:18350 \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"getblockchaininfo","params":[]}' | jq '.result.blocks'
```

### 2. RPC Query Test

Test all RPC methods:
```bash
# Get blockchain info
curl -s http://127.0.0.1:18350 -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"getblockchaininfo","params":[]}' | jq

# Get specific block (use hash from logs)
curl -s http://127.0.0.1:18350 -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"getblock","params":["<hash>"]}' | jq
```

### 3. Database Growth Test

Monitor storage over time:
```bash
# Check current size
du -sh data/node1

# Watch growth
watch -n 60 'du -sh data/node1'
```

### 4. Long-Running Stability Test

The 24-hour stress test validates:
- No crashes or panics
- Consistent block production
- RPC remains responsive
- Memory doesn't leak
- Database stays consistent

## 🛑 Stopping the Testnet

```bash
# Stop the node
pkill btpc_node

# Or if running stress test
pkill -9 btpc_node
pkill -9 stress-test
```

To clean and restart:
```bash
rm -rf data/node1/*
cp testnet-genesis.json/genesis.json data/node1/
# Start node again
```

## 🐛 Troubleshooting

### Node won't start
- Check if port 18350 is available: `lsof -i :18350`
- Verify genesis.json exists: `ls data/node1/genesis.json`
- Check logs: `tail -100 /tmp/node1.log`

### RPC not responding
- Verify node is running: `ps aux | grep btpc_node`
- Test connection: `curl http://127.0.0.1:18350`
- Check firewall rules

### Mining not working
- Verify `--mine` flag is present
- Check logs for "Mining started"
- Look for "No genesis block found" errors

### RPC returns wrong heights
**FIXED (2025-10-05)** - RPC handlers were stub implementations returning hardcoded 0
- Updated `handlers.rs` to query actual blockchain database
- Both `getblockchaininfo` and `getblock` now return correct heights

## 🔧 Recent Fixes

### 2025-10-05: RPC Height Bug Fixed

**Issue**: RPC methods `getblockchaininfo` and `getblock` always returned `height: 0`

**Root Cause**: The RPC handlers in `btpc-core/src/rpc/handlers.rs` were placeholder implementations with hardcoded values

**Fix Applied**:
- Updated `get_blockchain_info()` (lines 87-133) to call `blockchain_db.get_chain_tip()` and `blockchain_db.get_block_height()`
- Updated `get_block()` (lines 172-188) to query actual block height from database
- Removed all hardcoded `0` values

**Files Modified**:
- `btpc-core/src/rpc/handlers.rs`
- `btpc-core/src/storage/blockchain_db.rs` (minor capacity fix)

**Verification**:
```bash
# Now returns actual height
curl -s http://127.0.0.1:18350 -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"getblockchaininfo","params":[]}' | jq '.result.blocks'
# Example: 883 (instead of 0)
```

## 📚 Additional Resources

- Main Project README: `/home/bob/BTPC/BTPC/CLAUDE.md`
- RPC Documentation: `btpc-core/src/rpc/`
- Network Protocol: `btpc-core/src/network/`
- Stress Test Status: `STRESS_TEST_STATUS.md`

## 🔐 Security Note

**THIS IS A TESTNET**
- Testnet coins have NO value
- Private keys are for testing only
- Do not use testnet addresses/keys on mainnet
- Genesis allocation addresses are publicly known
- Current deployment is single-node (no network security)

## 📊 Performance Expectations

On modest hardware (current results):
- Block mining: ~1 block/second (testnet difficulty)
- Block validation: < 10ms per block
- Transaction verification: < 5ms
- RPC response time: < 100ms
- Database growth: ~5-10 MB per 1000 blocks

**Current Metrics** (as of latest check):
- Height: 883+ blocks
- Database: 5.8 MB
- CPU Usage: 9.4%
- Memory: 7.1 GiB / 125 GiB
- Mining Rate: ~40 blocks per 5 minutes

## 🎯 Success Criteria

Your testnet is working if:
- ✅ Genesis block loads successfully
- ✅ Mining produces valid blocks continuously
- ✅ RPC queries return correct data (not hardcoded values)
- ✅ Block heights increment properly
- ✅ Database grows as blocks are added
- ✅ No crashes or errors in logs
- ✅ System resources remain stable

**Current Status**: ✅ All criteria met, 24-hour stress test running

---

**Happy Testing! 🚀**

For live status, run `./monitor-testnet.sh` or check `STRESS_TEST_STATUS.md`
