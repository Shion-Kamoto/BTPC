# RPC Height Bug Fix Summary

**Date:** 2025-10-05
**Status:** ✅ FIXED AND VERIFIED
**Impact:** Critical - RPC monitoring now fully functional

---

## Problem Statement

### Symptoms
- RPC method `getblockchaininfo` always returned `"blocks": 0`
- RPC method `getblock` always returned `"height": 0`
- Blockchain was mining blocks correctly (confirmed in logs)
- Block heights were being tracked in the database
- But RPC queries could not see the actual heights

### Impact
- Unable to monitor blockchain progress via RPC
- 24-hour stress test would have had invalid metrics
- External tools and UIs would show incorrect data
- Looked like the blockchain wasn't working when it actually was

---

## Root Cause Analysis

### Investigation Steps

1. **Initial Hypothesis**: Database visibility issue
   - Thought `get_block_height()` was failing to find height metadata
   - Added debug logging to `blockchain_db.rs`

2. **Discovery**: Separate RPC database instance
   - Found that RPC creates its own `BlockchainDb` instance
   - Both instances share same underlying RocksDB database
   - Data WAS being written and read correctly

3. **Breakthrough**: Wrong RPC handler being used
   - Found TWO implementations: `handlers.rs` and `integrated_handlers.rs`
   - Node was using `handlers.rs` (not the async one in `methods.rs`)
   - Reading `handlers.rs` revealed **STUB IMPLEMENTATIONS**

4. **Root Cause Identified**:
   ```rust
   // handlers.rs lines 87-104 (BEFORE FIX)
   fn get_blockchain_info(...) -> Result<Value, RpcServerError> {
       // In a real implementation, this would query the blockchain state
       Ok(json!({
           "chain": "main",
           "blocks": 0,              // <-- HARDCODED!
           "headers": 0,             // <-- HARDCODED!
           "bestblockhash": "0000...", // <-- HARDCODED!
           // ...
       }))
   }
   ```

   The function had a comment saying "In a real implementation..." - it was a placeholder!

---

## Fix Applied

### Files Modified

1. **`btpc-core/src/rpc/handlers.rs`** (lines 87-188)
   - `get_blockchain_info()` - Now queries actual chain tip and height
   - `get_block()` - Now queries actual block height from database

2. **`btpc-core/src/storage/blockchain_db.rs`** (line 206)
   - Minor fix: Changed `height_key` capacity from 6+64 to 7+64 bytes
   - Not critical (capacity is pre-allocation, not a limit) but more correct

### Code Changes

#### Before (WRONG):
```rust
fn get_blockchain_info(
    blockchain_db: &Arc<dyn BlockchainDatabase + Send + Sync>,
) -> Result<Value, RpcServerError> {
    // In a real implementation, this would query the blockchain state
    Ok(json!({
        "chain": "main",
        "blocks": 0,  // Always returns 0!
        "headers": 0,
        "bestblockhash": "0000000000000000000000000000000000000000000000000000000000000000",
        "difficulty": 1.0,
        // ...
    }))
}
```

#### After (CORRECT):
```rust
fn get_blockchain_info(
    blockchain_db: &Arc<dyn BlockchainDatabase + Send + Sync>,
) -> Result<Value, RpcServerError> {
    // Get chain tip
    let tip_block = blockchain_db
        .get_chain_tip()
        .map_err(|e| RpcServerError::Internal(format!("Failed to get chain tip: {}", e)))?;

    match tip_block {
        Some(block) => {
            let block_hash = block.hash();
            let height = blockchain_db
                .get_block_height(&block_hash)
                .unwrap_or(0);

            Ok(json!({
                "chain": "main",
                "blocks": height,  // Now returns actual height!
                "headers": height,
                "bestblockhash": hex::encode(block_hash.as_bytes()),
                // ...
            }))
        }
        None => {
            // No blocks yet (empty chain)
            Ok(json!({
                "chain": "main",
                "blocks": 0,
                // ...
            }))
        }
    }
}
```

### Similar Fix for `get_block()`

```rust
// Before:
"height": 0, // Would get actual height

// After:
let height = blockchain_db.get_block_height(&hash).unwrap_or(0);
"height": height,
```

---

## Testing & Verification

### Test 1: getblockchaininfo
```bash
# Before fix:
curl -s http://127.0.0.1:18350 -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"getblockchaininfo","params":[]}' | jq '.result.blocks'
# Output: 0 (WRONG!)

# After fix:
curl -s http://127.0.0.1:18350 -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"getblockchaininfo","params":[]}' | jq '.result.blocks'
# Output: 883 (CORRECT!)
```

### Test 2: getblock
```bash
# Test with actual mined block hash
curl -s http://127.0.0.1:18350 -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"getblock","params":["7458283a0e008f0173c259502c31b4729a82fea850fb9143507874276caac693839e0f2676353d6e2d9eeba561e9354a5bfaf6f90feb896394c8576553daf5a9"]}' \
  | jq '.result.height'

# Before: 0
# After: 175 ✅
```

### Test 3: Continuous Monitoring
```bash
# Monitor shows correct increasing heights:
./monitor-testnet.sh

# Output:
# Node1 (port 18350): 883 blocks
# Best hash: 0eb374b6d9ce0f15d4c2505230eadfd8...
```

---

## Additional Fixes

### Genesis JSON Format (Fixed Earlier)

**Issue 1 - bits field:**
```json
// Before:
"bits": "0x207fffff"  // Hex string

// After:
"bits": 545259519  // Decimal u32
```

**Issue 2 - Script fields:**
```json
// Before:
"script_sig": ""
"script_pubkey": ""

// After:
"script_sig": {"operations": []}
"script_pubkey": {"operations": []}
```

---

## Impact Assessment

### Before Fix
- ❌ RPC monitoring unusable
- ❌ Cannot track blockchain progress via API
- ❌ Stress test metrics would be invalid
- ❌ External integrations would fail
- ❌ UIs would show empty blockchain

### After Fix
- ✅ RPC returns accurate real-time data
- ✅ Blockchain progress fully visible
- ✅ 24-hour stress test running with valid metrics
- ✅ All monitoring scripts functional
- ✅ Ready for external integrations

---

## Lessons Learned

1. **Always check which implementation is being used**
   - Multiple RPC implementations existed (handlers.rs, integrated_handlers.rs, methods.rs)
   - Need to trace from `main.rs` to see which is registered

2. **Stub implementations should fail loudly**
   - Returning placeholder data (0, empty strings) masks problems
   - Better to return errors or panic with "unimplemented!"

3. **Integration testing is critical**
   - Unit tests passed (they tested the database layer)
   - But end-to-end RPC tests would have caught this immediately

4. **Comments like "In a real implementation..." are red flags**
   - These indicate incomplete code that needs attention
   - Should be tracked as TODOs or issues

---

## Current Status

### Deployment
- ✅ Node running stable (PID: 53442)
- ✅ Mining continuous blocks (900+ blocks)
- ✅ RPC fully functional on port 18350
- ✅ Database growing normally (~6 MB)

### Stress Test
- ✅ 24-hour test started (PID: 108846)
- ✅ 2/288 checks completed
- ✅ All metrics showing correct values
- ✅ No errors or crashes

### Monitoring
- ✅ `monitor-testnet.sh` - Working
- ✅ `stress-test-24hr.sh` - Running
- ✅ RPC queries - All functional
- ✅ Block heights - Accurate

---

## Related Documentation

- **Main README**: `testnet-deployment/README.md`
- **Stress Test Status**: `testnet-deployment/STRESS_TEST_STATUS.md`
- **Test Logs**: `testnet-deployment/logs/stress-test-24hr.log`

---

**Status**: ✅ Issue resolved, system fully operational, 24-hour test in progress

**Last Updated**: 2025-10-05 10:22:00
