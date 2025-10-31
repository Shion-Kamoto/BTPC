# Phase 3: TODO Items Analysis

**Date**: 2025-10-31
**Total TODO Items Found**: 26 (21 in btpc-core, 5 in btpc-desktop-app)

## Summary by Priority

| Priority | Count | Impact |
|----------|-------|--------|
| 🔴 HIGH | 3 | Core functionality broken/missing |
| 🟡 MEDIUM | 18 | Enhanced functionality missing |
| 🟢 LOW | 4 | Nice-to-have features |
| ✅ DONE | 1 | Already implemented |

---

## HIGH PRIORITY (Critical Functionality) 🔴

### 1. storage_validation.rs:752 - Signature and Script Validation
**File**: `btpc-core/src/consensus/storage_validation.rs:752`
**Issue**: Transaction signature and script execution validation is not implemented
**Current Code**:
```rust
// TODO: Validate signature and script execution
```

**Impact**: **CRITICAL** - Transactions are not cryptographically verified!
**Risk**: Anyone could spend anyone else's UTXOs without proper signatures
**Complexity**: HIGH - Requires ML-DSA signature verification integration
**Priority**: **#1 - FIX IMMEDIATELY**

---

### 2. utxo_manager.rs:555 - Proper Transaction Signing
**File**: `btpc-desktop-app/src-tauri/src/utxo_manager.rs:555`
**Issue**: Transaction inputs have empty signature scripts
**Current Code**:
```rust
signature_script: Vec::new(), // TODO: Implement proper signing
```

**Impact**: **CRITICAL** - Created transactions cannot be validated by network
**Risk**: All transactions from desktop app will be rejected
**Complexity**: MEDIUM - Integration with existing signing code needed
**Priority**: **#2 - FIX IMMEDIATELY**

---

### 3. RPC Hex Deserialization (3 instances)
**Files**:
- `integrated_handlers.rs:499` - decoderawtransaction
- `integrated_handlers.rs:541` - sendrawtransaction
- `integrated_handlers.rs:615` - getblock (with verbosity)
- `integrated_handlers.rs:797` - submitblock

**Issue**: Cannot deserialize hex-encoded blocks/transactions
**Impact**: **HIGH** - RPC endpoints non-functional for raw data
**Complexity**: MEDIUM - Need to implement hex → struct deserialization
**Priority**: **#3 - FIX SOON**

---

## MEDIUM PRIORITY (Enhanced Functionality) 🟡

### RPC Calculations (16 instances in integrated_handlers.rs)

**Missing Calculations**:
1. Line 382: `difficulty = 1.0` (should calculate actual difficulty)
2. Line 402: `mediantime = 0` (should calculate median of last 11 blocks)
3. Line 406: `size_on_disk = 0` (should get actual RocksDB storage size)
4. Line 455: `confirmations = 1` (should count blocks since tx)
5. Line 456: `size = 0` (should calculate transaction byte size)
6. Line 457: `height = 0` (should get block height)
7. Line 464: `difficulty = 1.0` (transaction difficulty)
8. Line 648: `difficulty = 1.0` (mining info difficulty)
9. Line 649: `networkhashps = 0` (estimate network hashrate)
10. Line 741: `current_difficulty = 1.0` (getblocktemplate difficulty)
11. Line 832: Block count placeholder
12. Line 878: Current height placeholder

**Impact**: MEDIUM - RPC returns placeholder data, not accurate values
**Risk**: Third-party tools/explorers will show incorrect information
**Complexity**: LOW-MEDIUM - Most calculations straightforward
**Priority**: **Fix after HIGH items**

---

### Storage Enhancements (2 instances)

#### 1. storage/mod.rs:286 - UTXO Count
```rust
utxo_count: 0, // TODO: Implement UTXO count method
```
**Impact**: MEDIUM - Statistics endpoint returns incorrect UTXO count
**Complexity**: LOW - Simple RocksDB iterator count
**Priority**: Easy win

#### 2. storage/mempool.rs:126 - Fee Calculation
```rust
total_fees: 0, // TODO: Calculate fees when fee system is implemented
```
**Impact**: LOW - Fees not yet part of BTPC economics
**Complexity**: LOW - Simple sum once fee system exists
**Priority**: Low (fee system not implemented yet)

---

## LOW PRIORITY (Nice-to-Have) 🟢

### 1. utxo_manager.rs:527 & 600 - Script Decoding
**Issue**: Cannot decode script_pubkey to extract/verify addresses
**Impact**: LOW - Address matching uses alternative method
**Complexity**: MEDIUM - Bitcoin script parsing required
**Priority**: Enhancement

### 2. main.rs:1270 - GPU Miner Flag Comment
**Issue**: Code commented out waiting for --gpu flag support
**Impact**: LOW - Manual GPU configuration still works
**Complexity**: LOW - Just uncomment when ready
**Priority**: Very low

---

## ALREADY IMPLEMENTED ✅

### transaction_commands.rs:515 - UTXO Reservation Release
```rust
// TODO: Release UTXO reservations after confirmation
```

**Status**: ✅ **ALREADY DONE** in Feature 007
**Implementation**: `transaction_monitor.rs` automatically releases reservations on confirmation
**Action**: Remove this TODO comment

---

## ALREADY DOCUMENTED (Not a Bug) 📝

### crypto/keys.rs:595 - Deterministic Key Generation
```rust
#[ignore] // TODO: Implement true deterministic key generation from seed
```

**Status**: Documented in Phase 2 as library limitation
**Impact**: None (BTPC uses file-based wallet storage)
**Action**: Keep as ignored test with documentation

---

## Recommended Fix Order

### Phase 3A: Critical Fixes (Session 1)
1. ✅ Signature and script validation (storage_validation.rs:752)
2. ✅ Transaction signing (utxo_manager.rs:555)
3. ✅ Remove completed TODO (transaction_commands.rs:515)

### Phase 3B: RPC Hex Deserialization (Session 2)
4. Implement hex → Transaction deserialization
5. Implement hex → Block deserialization
6. Update all affected RPC endpoints

### Phase 3C: RPC Calculations (Session 3)
7. Implement difficulty calculations
8. Implement confirmation counting
9. Implement height lookups
10. Implement size calculations
11. Implement network hashrate estimation

### Phase 3D: Storage Enhancements (Session 4)
12. Implement UTXO count
13. Implement fee calculations (when fee system ready)

### Phase 3E: Polish (Optional)
14. Script decoding for addresses
15. GPU miner flag support

---

## Risk Assessment

| Item | Current Risk | After Fix | Blocker? |
|------|-------------|-----------|----------|
| Signature validation | 🔴 CRITICAL | ✅ NONE | YES |
| Transaction signing | 🔴 CRITICAL | ✅ NONE | YES |
| RPC hex deser | 🟡 MEDIUM | ✅ NONE | NO |
| RPC calculations | 🟢 LOW | ✅ NONE | NO |
| Storage stats | 🟢 LOW | ✅ NONE | NO |

**Blockers**: Items #1 and #2 are critical and should be fixed before any production use.

---

## Testing Strategy

### Phase 3A Tests
- [ ] Signature verification test with invalid signatures (should fail)
- [ ] Signature verification test with valid signatures (should pass)
- [ ] Transaction signing test (desktop app creates valid tx)
- [ ] End-to-end transaction test (create → sign → validate → broadcast)

### Phase 3B Tests
- [ ] Hex deserialization round-trip (tx → hex → tx)
- [ ] RPC decoderawtransaction with test vector
- [ ] RPC sendrawtransaction with valid/invalid txs

### Phase 3C Tests
- [ ] Difficulty calculation matches manual calculation
- [ ] Confirmation counting across chain reorgs
- [ ] Network hashrate estimation with known block times

---

## Files Requiring Changes

**btpc-core**:
- `src/consensus/storage_validation.rs` (signature validation)
- `src/rpc/integrated_handlers.rs` (16 TODOs - calculations + deserialization)
- `src/storage/mod.rs` (UTXO count)
- `src/storage/mempool.rs` (fee calculation)

**btpc-desktop-app**:
- `src-tauri/src/utxo_manager.rs` (transaction signing, script decoding)
- `src-tauri/src/transaction_commands.rs` (remove completed TODO)
- `src-tauri/src/main.rs` (GPU flag comment)

**Total**: 7 files need modifications

---

## Estimated Effort

| Phase | Items | Estimated Time | Complexity |
|-------|-------|----------------|------------|
| 3A | 3 critical | 2-3 hours | HIGH |
| 3B | 4 deserialization | 1-2 hours | MEDIUM |
| 3C | 12 calculations | 2-3 hours | LOW-MEDIUM |
| 3D | 2 storage | 30 minutes | LOW |
| 3E | 2 polish | 1 hour | LOW |
| **Total** | **23 items** | **6-9 hours** | **MIXED** |

---

## Success Criteria

- [x] All 26 TODO items analyzed and categorized
- [ ] All HIGH priority items fixed
- [ ] All MEDIUM priority items addressed or deferred with reason
- [ ] All tests passing
- [ ] Documentation updated
- [ ] No new TODO items added without tracking