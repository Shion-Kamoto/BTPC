# URGENT: Transaction Monitoring & Fee Issues

**Date**: 2025-11-06 15:00:00
**Status**: BLOCKING - Transactions not confirming, fees too high
**Priority**: P0 - CRITICAL

---

## Issue 1: Transaction Monitoring Not Working

### Problem
Transaction monitor calls `getrawtransaction` RPC method which doesn't exist in btpc_node.

**Error Message**:
```
⚠️  Transaction tx_1762421584546672750 not found in node: RPC error -32601: Method not found
```

### Root Cause
- Desktop app calls: `getrawtransaction` (line 246 in rpc_client.rs)
- Node implements: `gettransaction` (line 168 in integrated_handlers.rs)
- **Mismatch!**

### Fix Required
Change `rpc_client.rs:246` from:
```rust
self.call("getrawtransaction", Some(params)).await
```

To:
```rust
self.call("gettransaction", Some(params)).await
```

**File**: `btpc-desktop-app/src-tauri/src/rpc_client.rs`
**Line**: 246
**Estimated Time**: 1 minute

---

## Issue 2: Fees Still Too High (4.81 BTPC for 3 BTPC transfer)

### Problem
Fee estimator falls back to 1000 crd/byte because RPC `estimatefee` method doesn't exist.

**Log Output**:
```
⚠️  RPC fee estimation not yet implemented, using fallback
💰 Dynamic fee rate: 1000 crd/byte (from FeeEstimator)
📊 Fee calculation:
  Estimated size: 4810 bytes
  Fee rate: 1000 crd/byte
  Total fee: 4810000 credits (0.0481 BTPC)
```

### Root Cause Analysis

**Desktop App Code** (`fee_estimator.rs:156-180`):
```rust
pub async fn estimate_fee_for_transaction(
    &self,
    inputs_count: usize,
    outputs_count: usize,
) -> Result<u64, String> {
    // Get RPC client
    match self.get_rpc_client().await {
        Ok(rpc_client) => {
            // Try RPC fee estimation first
            match rpc_client.estimate_fee(1).await {  // <-- This is failing!
                Ok(fee_rate) => {
                    println!("💰 Dynamic fee rate: {} crd/byte (from RPC)", fee_rate);
                    // ...
                }
                Err(e) => {
                    println!("⚠️  RPC fee estimation not yet implemented, using fallback");
                    // Falls back to 1000 crd/byte
                }
            }
        }
        Err(_) => {
            // Falls back to 1000 crd/byte
        }
    }
}
```

**RPC Client** (`rpc_client.rs:280-283`):
```rust
pub async fn estimate_fee(&self, num_blocks: u32) -> Result<u64> {
    let params = json!([num_blocks]);
    let result = self.call("estimatefee", Some(params)).await?;
    // ...
}
```

**Problem**: btpc_node doesn't have `estimatefee` RPC method!

### Available RPC Methods (from integrated_handlers.rs)
```
getblockchaininfo
getbestblockhash
getblock
getblockheader
getblockcount
gettransaction          <-- Have this
getrecenttransactions
gettxout
sendrawtransaction
validatetransaction
getblocktemplate
submitblock
getmininginfo
getpeerinfo
getsyncinfo
getnetworkinfo
getconsensusinfo
getdifficultyinfo
getrewardinfo
validateblock
help
uptime
```

**No `estimatefee` or `getrawtransaction` methods exist!**

### Solution Options

**Option A: Implement `estimatefee` RPC method (RECOMMENDED)**
1. Add RPC method to btpc_node
2. Calculate fee rate based on recent blocks
3. Return satoshis/byte rate
4. **Estimated Time**: 2-3 hours

**Option B: Use reasonable hardcoded fallback (QUICK FIX)**
1. Change fallback from 1000 crd/byte to 100 crd/byte
2. **Estimated Time**: 1 minute
3. **Trade-off**: Transactions might confirm slower but fees reasonable

**Option C: Use mempool-based estimation**
1. Query mempool for recent transaction fees
2. Calculate median fee rate
3. **Estimated Time**: 3-4 hours

---

## Recommended Actions

### Immediate (Fix Transaction Monitoring)
```bash
cd btpc-desktop-app/src-tauri
# Edit rpc_client.rs line 246
# Change "getrawtransaction" to "gettransaction"
cargo build --release
```

### Short Term (Fix High Fees - Quick)
```bash
cd btpc-desktop-app/src-tauri
# Edit fee_estimator.rs line 177
# Change fallback from 1000 to 100
cargo build --release
```

### Long Term (Proper Fee Estimation)
1. Implement `estimatefee` RPC method in btpc_node
2. Use recent block data to calculate fee rates
3. Return dynamic rates based on network congestion
4. Test with various block fill levels

---

## Testing After Fix

### Test 1: Transaction Monitoring
1. Send transaction
2. Wait 30 seconds (polling interval)
3. Check logs for: ✅ Transaction confirmed (not ⚠️ Method not found)

### Test 2: Fee Calculation
1. Estimate fee for 3 BTPC transfer
2. Expected (with 100 crd/byte): ~0.005 BTPC (reasonable)
3. Current (with 1000 crd/byte): ~0.048 BTPC (too high)

---

## Files to Modify

### Fix 1: Transaction Monitoring
**File**: `btpc-desktop-app/src-tauri/src/rpc_client.rs`
**Line**: 246
**Change**: `"getrawtransaction"` → `"gettransaction"`

### Fix 2: Fee Fallback (Quick)
**File**: `btpc-desktop-app/src-tauri/src/fee_estimator.rs`
**Line**: 177
**Change**: `1000` → `100`

### Fix 3: Implement estimatefee RPC (Proper Solution)
**File**: `btpc-core/src/rpc/integrated_handlers.rs`
**Add**: New `estimatefee` method (~50-100 lines)

---

## Impact Assessment

### Current State (Broken)
- ❌ Transactions appear stuck (monitoring fails)
- ❌ Fees 10x too high (0.0481 BTPC for 3 BTPC transfer)
- ❌ Users confused about transaction status
- ❌ Makes wallet unusable for small transfers

### After Fix 1 + 2 (Quick Fixes)
- ✅ Transaction monitoring works
- ✅ Fees reasonable (0.005 BTPC)
- ✅ Users see confirmation status
- ⚠️  Still using fallback (not dynamic)

### After Fix 3 (Proper Solution)
- ✅ Transaction monitoring works
- ✅ Fees dynamically adjusted
- ✅ Network-aware fee estimation
- ✅ Production-ready

---

## Priority

**P0 - CRITICAL**: Fix 1 (Transaction Monitoring) - 1 minute
**P1 - HIGH**: Fix 2 (Fee Fallback) - 1 minute
**P2 - MEDIUM**: Fix 3 (estimatefee RPC) - 2-3 hours

---

## Notes

- Feature 008 (BIP39) is complete and unrelated to this issue
- This is a separate bug from Feature 007 transaction sending
- Quick fixes (1-2 minutes total) will make wallet immediately usable
- Proper solution (Fix 3) should be implemented before production deployment

---

**Status**: Documented, awaiting fix
**Blocking**: Wallet usability
**Assignee**: Next session
**Est. Time**: 2 minutes (quick fixes) or 3 hours (proper solution)