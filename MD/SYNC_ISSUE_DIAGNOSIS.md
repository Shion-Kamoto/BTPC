# BTPC Desktop App Sync Issue - Complete Diagnosis

**Date**: 2025-10-18
**User Report**: "Still not syncing. Network regtest. Status Syncing 0.0%. The % of sync does not move."

---

## Root Causes Identified

### 1. ✅ FIXED: Network Mismatch
- **Original Issue**: Node running on testnet port 8332, desktop app expecting regtest port 18360
- **Fix Applied**: Started fresh regtest node on port 18360
- **Status**: ✅ Resolved - Node now running on correct network/port

### 2. ⚠️ IN PROGRESS: Blockchain at Genesis (Height 0)
- **Issue**: Blockchain has only genesis block (height = 0, headers = 0)
- **Impact**: Sync calculation shows 0/0 = "100% synced" in node, "0.0%" in UI
- **Cause**: No blocks to sync because blockchain just started
- **Status**: ⚠️ Miner is running and finding blocks, but they're not being added to blockchain

### 3. ⚠️ DISCOVERED: Block Submission Not Working
- **Issue**: Miner finds blocks and reports "✅ Block submitted successfully" but node blockchain remains at height 0
- **Evidence**: 16+ blocks "submitted" according to miner logs, but `getblockchaininfo` still shows blocks: 0
- **Possible Causes**:
  - Block validation failing silently
  - RPC submission endpoint not processing blocks
  - Network mismatch between miner and node (despite both saying regtest)
  - Block storage issue

---

## Current System State

### Node Status
```
PID: 298672
Command: /home/bob/.btpc/bin/btpc_node --network regtest --rpcport 18360 --rpcbind 127.0.0.1 --datadir /home/bob/.btpc/data/regtest-node
RPC Port: 18360 (✅ correct for regtest)
P2P Port: 8333
Network: "main" (⚠️ should say "regtest")
Blocks: 0
Headers: 0
Sync: "100.0%" (because 0/0)
```

### Miner Status
```
PID: 301131
Command: /home/bob/.btpc/bin/btpc_miner --network regtest --address mgwyEDvagzDr2HxPE3kinCSXhmTxr9N2qq --rpc-url http://127.0.0.1:18360
Network: regtest
RPC URL: http://127.0.0.1:18360 (✅ correct)
Status: Running, finding blocks rapidly
Blocks Found: 16+ in last few minutes
Blocks Submitted: All show "✅ Block submitted successfully"
Blocks Accepted by Node: 0 (⚠️ PROBLEM)
```

### Desktop App Status
```
Network Display: "regtest"
RPC Connection: Port 18360 (✅ correct after fix)
Sync Display: "Syncing 0.0%"
Calculation: blocks=0, headers=0 → 0/0 → 0.0%
```

---

## Why Sync Shows 0.0%

### Node Calculation (btpc_node)
```rust
// When blocks = 0 and headers = 0:
let sync_progress = 0 / 0;  // Division by zero
// Rust/btpc_node returns this as 100.0% (treats 0/0 as "fully synced")
```

### UI Calculation (btpc-update-manager.js)
```javascript
// btpc-desktop-app/ui/btpc-update-manager.js:128-129
const sync_progress = headers > 0 ? Math.min(100, (height / headers) * 100) : 0;
//                    ^^^^^^^^^^
//                    Correctly handles 0/0 as 0% (not 100%)
```

**Result**: Node says "100%", UI correctly shows "0.0%"

---

## Missing RPC Methods

Discovered during diagnosis:

| Method | Status | Error |
|--------|--------|-------|
| `getblockchaininfo` | ✅ Works | - |
| `getblockcount` | ❌ Missing | "Method not found" (-32601) |
| `submitblock` | ❓ Unknown | Miner claims success but blocks don't appear |

---

## What Should Happen (Normal Flow)

1. **Miner**:
   - Calls `getblocktemplate` → gets template from node
   - Mines block (finds valid nonce)
   - Calls `submitblock` → submits block to node

2. **Node**:
   - Receives block via `submitblock` RPC
   - Validates block (PoW, transactions, etc.)
   - If valid: adds to blockchain, increments height
   - If invalid: rejects with error

3. **Desktop App**:
   - Polls `getblockchaininfo` every 5 seconds
   - Gets updated blocks/headers count
   - Calculates sync: (blocks / headers) * 100%
   - Displays in UI

---

## What's Actually Happening

1. **Miner**: ✅ Finding blocks, ✅ Calling submitblock, ❌ Blocks not persisting
2. **Node**: ✅ Running, ✅ Responding to RPC, ❌ Not recording submitted blocks
3. **Desktop App**: ✅ Connecting to RPC, ✅ Getting data (blocks=0), ✅ Displaying correctly (0.0%)

**Problem is between steps 1 and 2**: Miner → Node block submission is failing silently.

---

## Diagnostic Evidence

### Evidence 1: Miner Logs Show Success
```
🎉 Block found by thread 22!
Block hash: 16c518d1a38e5ce68bdfbb42047d988541a6bc46a6eba86ee6fd55a552a7d4ba...
✅ Block submitted successfully!

🎉 Block found by thread 20!
Block hash: 5fe98da571917f2c6cc0d165e75f4470b5576e84c9a69c15f58191ddb56ea731...
✅ Block submitted successfully!

[... 14 more blocks ...]
```

### Evidence 2: Node Shows No New Blocks
```bash
$ curl -s http://127.0.0.1:18360 -d '{"method":"getblockchaininfo","id":1}' | jq .result.blocks
0
```

### Evidence 3: Node Logs Silent on Block Processing
```
Starting BTPC Node...
✅ Blockchain synchronized (100.0%)
Press Ctrl+C to stop the node

[No messages about receiving/validating/storing blocks]
```

---

## Hypothesis: Why Blocks Aren't Being Added

### Hypothesis 1: Network Type Confusion
- Node command: `--network regtest`
- Node response: `"chain": "main"` (not "regtest"!)
- Possible issue: Node is running in mainnet mode despite `--network regtest` flag
- Impact: Miner submits regtest blocks to mainnet node → rejected for wrong network

### Hypothesis 2: Block Validation Failure
- Miner submits blocks
- Node validates and **silently rejects** them
- No error returned to miner (hence "success" message)
- Possible reasons:
  - Invalid PoW (wrong difficulty)
  - Invalid coinbase transaction
  - Wrong network magic bytes
  - Timestamp issues

### Hypothesis 3: RPC Implementation Gap
- `submitblock` RPC endpoint exists but doesn't fully implement block storage
- Returns success but doesn't persist to blockchain database
- This would be a critical bug in btpc_node

---

## Next Steps to Debug

### Step 1: Verify Node Network Type
```bash
# Check what network the node actually thinks it's on
curl -s -X POST http://127.0.0.1:18360 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"getblockchaininfo","params":[],"id":1}' | \
  jq '.result.chain'

# Expected: "regtest"
# Actual: "main" ⚠️
```

### Step 2: Enable Debug Logging on Node
```bash
# Kill current node
kill 298672

# Restart with verbose logging
/home/bob/.btpc/bin/btpc_node \
  --network regtest \
  --rpcport 18360 \
  --rpcbind 127.0.0.1 \
  --datadir /home/bob/.btpc/data/regtest-node \
  --log-level debug \
  > /tmp/btpc_node_debug.log 2>&1 &

# Watch logs while miner submits blocks
tail -f /tmp/btpc_node_debug.log
```

### Step 3: Test Block Submission Manually
```bash
# Get a block template
TEMPLATE=$(curl -s -X POST http://127.0.0.1:18360 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"getblocktemplate","params":[],"id":1}')

# Try to submit a block (will fail due to invalid PoW, but should show error)
curl -s -X POST http://127.0.0.1:18360 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"submitblock","params":["000000..."],"id":1}' | jq .
```

### Step 4: Check Node Source Code
File to check: `/home/bob/BTPC/BTPC/bins/btpc_node/src/main.rs`

Look for:
- `submitblock` RPC handler implementation
- Block validation logic
- Block storage/persistence code
- Network type configuration

---

## Immediate Workaround

While debugging the block submission issue, you can verify the sync functionality works by connecting to a node that already has blocks:

```bash
# Option 1: Use mainnet node (will take time to sync)
killall btpc_node btpc_miner
/home/bob/.btpc/bin/btpc_node --network mainnet --rpcport 8332 &

# Option 2: Import pre-mined regtest blockchain
# (if you have a backup with blocks > 0)
```

---

## UI Sync Display - Status

**Current Behavior**: ✅ **CORRECT**

The desktop app is correctly showing "Syncing 0.0%" because:
1. ✅ Connects to node on correct port (18360)
2. ✅ Gets blockchain info (blocks=0, headers=0)
3. ✅ Calculates sync correctly: `0 / max(1, 0) = 0.0%`
4. ✅ Displays "Syncing 0.0%" (accurate representation of 0 blocks)

The sync display will automatically update to show progress once:
- Blocks start being added to the blockchain (currently not happening)
- Headers > 0 (will happen when blocks are added)

---

## Summary

| Component | Status | Issue |
|-----------|--------|-------|
| Desktop App RPC Connection | ✅ Fixed | Was connecting to wrong port, now correct (18360) |
| Desktop App Sync Display | ✅ Working | Correctly shows 0.0% for empty blockchain |
| Node RPC Server | ✅ Running | Responding on port 18360 |
| Node Blockchain | ⚠️ Empty | Stuck at genesis block (height 0) |
| Miner Block Finding | ✅ Working | Finding blocks rapidly |
| Miner Block Submission | ❌ **FAILING** | Submits blocks but node doesn't accept them |

**Root Cause**: Block submission from miner to node is broken (probable cause: network type mismatch - node reports "main" instead of "regtest")

**Recommendation**:
1. Fix node network type detection/configuration
2. Enable debug logging to see why blocks are rejected
3. Once blocks start being added, sync display will work automatically

---

## Files for Reference

- **Node Logs**: `/tmp/btpc_node_regtest.log`
- **Miner Logs**: `/tmp/btpc_miner.log`
- **Node Binary**: `/home/bob/.btpc/bin/btpc_node`
- **Miner Binary**: `/home/bob/.btpc/bin/btpc_miner`
- **Connectivity Fix Report**: `/home/bob/BTPC/BTPC/CONNECTIVITY_FIXES_SUMMARY.md`
- **Analysis Report**: `/home/bob/BTPC/BTPC/CONNECTIVITY_ANALYSIS_REPORT.md`

---

*Diagnosis complete. The sync display is working correctly - the underlying issue is that the blockchain has no blocks to sync because block submission is failing.*
