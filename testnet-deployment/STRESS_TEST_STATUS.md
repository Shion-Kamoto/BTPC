# BTPC 24-Hour Stress Test

## Status: ✅ RUNNING

**Start Time:** 2025-10-05 10:15:18  
**Expected End:** 2025-10-06 10:15:18  
**Current Time:** $(date '+%Y-%m-%d %H:%M:%S')

## Test Configuration

- **Duration:** 24 hours (86,400 seconds)
- **Check Interval:** 5 minutes (300 seconds)  
- **Total Checks:** 288
- **Node:** Single mining node on port 18350
- **Network:** Testnet
- **Log Files:**
  - Main log: `logs/stress-test-24hr.log`
  - Console log: `logs/stress-test-console.log`
  - Node log: `/tmp/node1_final.log`

## Current Status

- **Node PID:** 53442
- **Test PID:** 108846
- **Current Height:** 900+ blocks (increasing)
- **Database Size:** ~6 MB
- **Uptime:** Running stable
- **Last Check:** 2/288 completed

## Live Monitoring

Monitor progress:
```bash
# Watch live mining
tail -f /tmp/node1_final.log | grep "Block mined"

# Check test progress
tail -f logs/stress-test-24hr.log

# Quick status
./monitor-testnet.sh

# Current height
curl -s http://127.0.0.1:18350 -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"getblockchaininfo","params":[]}' | jq '.result.blocks'
```

## What's Being Tested

1. **Block Production:** Continuous mining for 24 hours
2. **RPC Stability:** Height queries every 5 minutes (all returning correct values)
3. **Database Growth:** Tracking storage requirements (~5-10 MB per 1000 blocks)
4. **System Resources:** CPU (~5-10%) and memory usage stable
5. **Error Detection:** Monitoring for crashes or errors (none so far)

## Bug Fixes Applied Before Test

### 2025-10-05: RPC Height Bug

**Issue**: RPC methods returned hardcoded `height: 0` instead of actual blockchain height

**Root Cause**: RPC handlers in `btpc-core/src/rpc/handlers.rs` were stub implementations:
```rust
// Before (WRONG):
fn get_blockchain_info(...) -> Result<Value, RpcServerError> {
    Ok(json!({
        "blocks": 0,  // <-- HARDCODED!
        // ...
    }))
}
```

**Fix Applied**:
- Updated `get_blockchain_info()` (handlers.rs:87-133)
- Updated `get_block()` (handlers.rs:172-188)
- Both now call `blockchain_db.get_chain_tip()` and `blockchain_db.get_block_height()`

```rust
// After (CORRECT):
fn get_blockchain_info(...) -> Result<Value, RpcServerError> {
    let tip_block = blockchain_db.get_chain_tip()?;
    match tip_block {
        Some(block) => {
            let height = blockchain_db.get_block_height(&block.hash())?;
            Ok(json!({"blocks": height, ...}))
        }
        // ...
    }
}
```

**Files Modified**:
- `btpc-core/src/rpc/handlers.rs` (RPC method implementations)
- `btpc-core/src/storage/blockchain_db.rs` (minor capacity fix)

**Verification**:
```bash
# Before fix: always returned 0
# After fix: returns actual height
curl -s http://127.0.0.1:18350 -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"getblockchaininfo","params":[]}' | jq '.result.blocks'
# Output: 883 (correct!)
```

### Additional Fixes

1. **Genesis JSON Format** - Fixed `bits` field (hex→decimal: 545259519) and Script format
2. **Height Tracking** - Ensured proper database key format (`height:` prefix = 7 bytes)

## Expected Outcomes

- **Blocks Mined:** ~8,640 blocks (1 block per 10 seconds avg)
- **Database Growth:** ~50-100 MB (estimated)
- **Uptime:** 100% with no crashes
- **RPC Accuracy:** All height queries return correct values ✅

## Success Criteria

Progress:
- [x] Node runs without crashing (ongoing)
- [x] RPC returns correct heights (verified)
- [x] Block heights increase continuously (verified)
- [x] No critical errors in logs (verified)
- [x] Stress test started successfully
- [ ] All 288 monitoring checks complete (2/288 so far)
- [ ] Full 24-hour uptime achieved

## Real-Time Metrics

**Latest Check Results** (Check 2/288 at 2025-10-05 10:20:19):
```
Height: 883 blocks
Database: 5.8M
CPU: 9.4%
Memory: 7.1Gi / 125Gi
Mining Rate: ~40 blocks per 5 minutes
Errors: None
```

**Mining Examples**:
```
🎉 Block mined! Hash: 7458283a0e008f0173c259502c31b4729a82fea8...
   Height: 175, Reward: 3236017825 satoshis

🎉 Block mined! Hash: 0eb374b6d9ce0f15d4c2505230eadfd8...
   Height: 883, Reward: 3235270425 satoshis
```

## Test Progress

The stress test will:
1. Run 288 checks over 24 hours (every 5 minutes)
2. Record blockchain height, database size, and system metrics
3. Alert if node crashes or errors occur
4. Generate final summary report at completion

Check `logs/stress-test-24hr.log` for detailed results.

---

**Status**: ✅ Running smoothly, all systems operational

Last updated: 2025-10-05 10:21:00
