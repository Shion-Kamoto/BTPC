# BTPC Security Fix Checklist

**Quick reference for addressing consensus security vulnerabilities**

---

## 🔴 CRITICAL PRIORITY (Fix Before Testnet)

### [ ] 1. Implement Signature Verification
**File:** `btpc-core/src/consensus/storage_validation.rs`
**Lines:** 163-166, 376-387

**Tasks:**
- [ ] Add `verify_input_signature()` method to `StorageBlockValidator`
- [ ] Extract public key from UTXO `script_pubkey`
- [ ] Calculate signature hash (sighash) for transaction
- [ ] Call ML-DSA signature verification (constant-time)
- [ ] Add error variant `InvalidSignature` to `StorageValidationError`
- [ ] Remove TODO comments
- [ ] Add test cases for:
  - [ ] Valid signatures pass
  - [ ] Invalid signatures fail
  - [ ] Wrong public key fails
  - [ ] Modified transaction fails

**Estimated Time:** 4-8 hours

---

### [ ] 2. Constant-Time Hash Comparison
**File:** `btpc-core/src/crypto/hash.rs`
**Line:** 104-106

**Tasks:**
- [ ] Add `subtle` crate dependency to `Cargo.toml`
- [ ] Rewrite `meets_target()` using constant-time comparison
- [ ] Implement byte-by-byte comparison from MSB to LSB
- [ ] Add test cases:
  - [ ] Timing tests (ensure constant time)
  - [ ] Correctness tests (matches old behavior)
  - [ ] Edge cases (all zeros, all ones, boundary values)

**Estimated Time:** 2-4 hours

---

### [ ] 3. Median-Time-Past Validation
**File:** `btpc-core/src/consensus/storage_validation.rs`
**Lines:** 84-96

**Tasks:**
- [ ] Add `get_median_time_past()` method (11 block window)
- [ ] Replace simple timestamp check with MTP validation
- [ ] Update `validate_block_context()` to use MTP
- [ ] Add error variant `TimestampBelowMedian`
- [ ] Add test cases:
  - [ ] Genesis block (no MTP)
  - [ ] Normal progression
  - [ ] Time-warp attack blocked
  - [ ] Future time still enforced

**Estimated Time:** 4-6 hours

---

### [ ] 4. Checked Arithmetic Everywhere
**Files:** `difficulty.rs`, `rewards.rs`, `storage_validation.rs`

**Tasks:**
- [ ] Replace all `as u8` casts with `clamp()` + checked cast
- [ ] Replace all `as u64` casts with `try_into()` or bounds check
- [ ] Add `.checked_sub()` to all timestamp subtractions
- [ ] Replace `f64` math with `u128` integer math in rewards
- [ ] Add overflow tests for:
  - [ ] Difficulty edge cases
  - [ ] Reward at u32::MAX height
  - [ ] Timestamp underflow
  - [ ] Target calculations

**Estimated Time:** 6-10 hours

---

## 🟠 HIGH PRIORITY (Fix Before Mainnet - Week 2-3)

### [ ] 5. Fix Race Conditions
**File:** `btpc-core/src/consensus/storage_validation.rs`
**Lines:** 70-96, 262-317

**Tasks:**
- [ ] Use RocksDB `WriteBatch` for atomic UTXO updates
- [ ] Hold locks for entire validation sequences
- [ ] Add `commit_batch()` method to `UTXODatabase` trait
- [ ] Add transaction rollback on error
- [ ] Add stress tests:
  - [ ] Concurrent block validation
  - [ ] Rapid UTXO updates
  - [ ] Crash recovery

**Estimated Time:** 8-12 hours

---

### [ ] 6. Add Replay Protection
**Files:** Multiple

**Tasks:**
- [ ] Add `fork_id` field to `ConsensusParams`
- [ ] Include `fork_id` in signature hash calculation
- [ ] Update signature verification to check fork ID
- [ ] Mainnet fork_id = 0, Testnet = 1
- [ ] Add test cases:
  - [ ] Same transaction different forks (rejected)
  - [ ] Fork-specific transactions work

**Estimated Time:** 4-6 hours

---

### [ ] 7. Handle Nonce Exhaustion
**File:** `btpc-core/src/consensus/pow.rs`
**Lines:** 25-42

**Tasks:**
- [ ] Add `extra_nonce` field to `BlockHeader`
- [ ] Implement nested nonce loop (extra_nonce outer, nonce inner)
- [ ] Add `shutdown_signal: Arc<AtomicBool>` parameter
- [ ] Add `MiningAborted` error variant
- [ ] Or: Implement coinbase extra nonce (Bitcoin-style)
- [ ] Add test:
  - [ ] Mining with nonce exhaustion
  - [ ] Clean shutdown

**Estimated Time:** 4-8 hours

---

### [ ] 8. Strict Difficulty Validation
**File:** `btpc-core/src/consensus/storage_validation.rs`
**Lines:** 100-126

**Tasks:**
- [ ] Check if block height is adjustment boundary
- [ ] If adjustment: Calculate expected difficulty from last 2016 blocks
- [ ] If not adjustment: Require exact match with previous
- [ ] Add error variants:
  - [ ] `UnexpectedDifficultyChange`
  - [ ] `IncorrectDifficultyAdjustment`
- [ ] Add tests:
  - [ ] Valid adjustments pass
  - [ ] Invalid adjustments fail
  - [ ] Mid-period changes rejected

**Estimated Time:** 6-10 hours

---

## 🟡 MEDIUM PRIORITY (Fix Before Mainnet - Week 4-6)

### [ ] 9. Coinbase Maturity Enforcement
**File:** `btpc-core/src/consensus/storage_validation.rs`

**Tasks:**
- [ ] Add `get_current_tip_height()` method
- [ ] Check UTXO `is_coinbase` flag in validation
- [ ] Enforce 100 block maturity
- [ ] Add error variant `ImmatureCoinbase`
- [ ] Add test: Spending coinbase before/after maturity

**Estimated Time:** 2-4 hours

---

### [ ] 10. Randomize Mining Nonces
**File:** `btpc-core/src/consensus/pow.rs`

**Tasks:**
- [ ] Add `rand` crate dependency
- [ ] Start mining from random nonce value
- [ ] Wrap around to cover all nonces
- [ ] Add test: Verify all nonces eventually checked

**Estimated Time:** 1-2 hours

---

### [ ] 11. Transaction ID Tracking
**Files:** `btpc-core/src/storage/*`

**Tasks:**
- [ ] Add `has_transaction()` to `BlockchainDatabase` trait
- [ ] Add `store_transaction()` to track txid → block_hash
- [ ] Check for duplicate transactions in validation
- [ ] Add error variant `DuplicateTransaction`
- [ ] Add test: Same transaction in multiple blocks

**Estimated Time:** 4-6 hours

---

### [ ] 12. Remove Floating-Point from Consensus
**Files:** `difficulty.rs`, `rewards.rs`

**Tasks:**
- [ ] Replace `work()` f64 with `work_integer()` u128
- [ ] Rewrite reward calculation using only integer math
- [ ] Use u128 intermediate values to prevent overflow
- [ ] Add determinism tests:
  - [ ] Same input → same output (1M iterations)
  - [ ] Cross-platform consistency

**Estimated Time:** 6-8 hours

---

### [ ] 13. Block Size Validation During Mining
**File:** `btpc-core/src/consensus/pow.rs`

**Tasks:**
- [ ] Add `validate_size()` call before mining
- [ ] Check against `MAX_BLOCK_SIZE` constant
- [ ] Fail fast if block too large
- [ ] Add test: Refuse to mine oversized blocks

**Estimated Time:** 1-2 hours

---

### [ ] 14. Implement Mempool
**File:** New file `btpc-core/src/mempool/mod.rs`

**Tasks:**
- [ ] Create `Mempool` struct with transaction pool
- [ ] Add size/count limits
- [ ] Require signature verification before accepting
- [ ] Implement fee-based prioritization
- [ ] Add rate limiting per peer
- [ ] Add memory limits (max 300MB)
- [ ] Implement transaction eviction policy

**Estimated Time:** 16-24 hours

---

## Testing Requirements

### Unit Tests (Add to each fix)
- [ ] Happy path tests
- [ ] Error condition tests
- [ ] Edge case tests
- [ ] Boundary value tests

### Integration Tests (After all fixes)
- [ ] Full blockchain validation
- [ ] Attack scenario tests
- [ ] Concurrent operation tests
- [ ] Chain reorganization tests

### Adversarial Tests (Before mainnet)
- [ ] Fuzzing of all consensus code
- [ ] Property-based testing
- [ ] Simulated attacks:
  - [ ] Double-spend attempts
  - [ ] Time-warp attacks
  - [ ] 51% attacks
  - [ ] Replay attacks

---

## Progress Tracking

### Week 1: Critical Fixes
- [ ] Day 1-2: Signature verification (#1)
- [ ] Day 3: Constant-time comparison (#2)
- [ ] Day 4-5: Median-time-past (#3)

### Week 2: Critical Fixes Continued
- [ ] Day 1-3: Checked arithmetic (#4)
- [ ] Day 4-5: Testing + bug fixes

### Week 3: High Priority
- [ ] Day 1-3: Race conditions (#5)
- [ ] Day 4: Replay protection (#6)
- [ ] Day 5: Nonce exhaustion (#7)

### Week 4: High Priority Continued
- [ ] Day 1-3: Difficulty validation (#8)
- [ ] Day 4-5: Integration testing

### Week 5-6: Medium Priority
- [ ] All medium priority issues (#9-14)
- [ ] Comprehensive testing
- [ ] Documentation updates

---

## Verification Checklist

Before declaring a fix complete:
- [ ] Code compiles without warnings
- [ ] All existing tests pass
- [ ] New tests added for the fix
- [ ] Code reviewed by another developer
- [ ] Documentation updated
- [ ] CHANGELOG entry added
- [ ] No new TODO comments added

---

## Resources

### Documentation
- Bitcoin's median-time-past: BIP-113
- Time-warp attack: Bitcoin Wiki
- Constant-time programming: `subtle` crate docs
- RocksDB transactions: RocksDB wiki

### Tools
- `cargo-fuzz`: Fuzzing framework
- `proptest`: Property-based testing
- `miri`: Detect undefined behavior
- `cargo-audit`: Security audit

### External Reviews
- [ ] Schedule security audit (after all fixes)
- [ ] Bug bounty program setup
- [ ] Community testing period

---

## Contact

**Security Issues:** Report privately to development team
**Questions:** Development Discord/Slack
**Progress Updates:** Weekly security meeting

---

**Last Updated:** 2025-10-11
**Total Estimated Time:** 70-120 hours (2-3 weeks full-time)