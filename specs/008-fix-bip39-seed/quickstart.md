# Quickstart: BIP39 Deterministic Wallet Recovery

**Feature**: Fix BIP39 Seed Phrase Determinism
**Purpose**: Validate that same BIP39 mnemonic produces identical wallets
**Duration**: ~5 minutes

## Prerequisites

- BTPC desktop app running
- OR BTPC CLI tools installed
- Network: Regtest (for fast testing)

## Test Scenario 1: Basic Deterministic Recovery

### Step 1: Create Wallet from Mnemonic

**CLI**:
```bash
# Use standard BIP39 test mnemonic
export MNEMONIC="abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art"

# Create wallet
btpc-wallet create-from-mnemonic \
  --mnemonic "$MNEMONIC" \
  --name "Test Wallet 1" \
  --password "test123456" \
  --network regtest

# Expected output:
# ✅ Wallet created successfully
# Wallet ID: 550e8400-e29b-41d4-a716-446655440000
# Version: V2 (BIP39 Deterministic)
# Address: btpc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh
# Recovery capable: ✅ Yes
```

**Desktop App**:
1. Open "Wallets" page
2. Click "Create New Wallet"
3. Select "Recover from Seed Phrase"
4. Enter 24-word mnemonic (use test mnemonic above)
5. Set wallet name: "Test Wallet 1"
6. Set password: "test123456"
7. Click "Create Wallet"
8. ✅ Wallet appears with "v2" badge

### Step 2: Record Address

**CLI**:
```bash
btpc-wallet get-address --wallet-id 550e8400-e29b-41d4-a716-446655440000

# Save output
export ADDRESS_1="btpc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh"
```

**Desktop App**:
1. Click on "Test Wallet 1"
2. Copy address from wallet details
3. Paste into notepad: `ADDRESS_1 = btpc1q...`

### Step 3: Delete Wallet

**CLI**:
```bash
btpc-wallet delete --wallet-id 550e8400-e29b-41d4-a716-446655440000 --confirm

# Expected output:
# ⚠️  This will permanently delete the wallet file
# Wallet deleted: ~/.btpc/wallets/550e8400-e29b-41d4-a716-446655440000.dat
```

**Desktop App**:
1. Right-click "Test Wallet 1"
2. Click "Delete Wallet"
3. Confirm deletion
4. ✅ Wallet removed from list

### Step 4: Recover Wallet with Same Mnemonic

**CLI**:
```bash
# Use SAME mnemonic
btpc-wallet recover-from-mnemonic \
  --mnemonic "$MNEMONIC" \
  --name "Test Wallet 2" \
  --password "test123456" \
  --network regtest \
  --expected-address "$ADDRESS_1"

# Expected output:
# ✅ Wallet recovered successfully
# Wallet ID: 7c9e6679-7425-40de-944b-e07fc1f90ae7  (NEW UUID)
# Address: btpc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh  (SAME as ADDRESS_1)
# Recovery verified: ✅ Yes (address matched)
# Keys match: ✅ Yes (byte-identical)
```

**Desktop App**:
1. Click "Create New Wallet"
2. Select "Recover from Seed Phrase"
3. Enter same 24-word mnemonic
4. Set wallet name: "Test Wallet 2"
5. Set password: "test123456"
6. Click "Recover Wallet"
7. ✅ Wallet appears with same address as ADDRESS_1

### Step 5: Verify Identical Keys

**CLI**:
```bash
# Get new address
btpc-wallet get-address --wallet-id 7c9e6679-7425-40de-944b-e07fc1f90ae7

export ADDRESS_2="<output from above>"

# Compare addresses
if [ "$ADDRESS_1" = "$ADDRESS_2" ]; then
  echo "✅ SUCCESS: Addresses are identical!"
  echo "   Address 1: $ADDRESS_1"
  echo "   Address 2: $ADDRESS_2"
else
  echo "❌ FAIL: Addresses differ!"
  echo "   Address 1: $ADDRESS_1"
  echo "   Address 2: $ADDRESS_2"
  exit 1
fi
```

**Desktop App**:
1. Compare ADDRESS_1 (from Step 2) with "Test Wallet 2" address
2. ✅ PASS: Addresses match
3. ❌ FAIL: Addresses differ (bug - determinism broken)

---

## Test Scenario 2: Cross-Device Recovery

### Device A: Create Wallet

**On Device A** (desktop, laptop, or VM):
```bash
export MNEMONIC="your custom 24-word mnemonic here"

btpc-wallet create-from-mnemonic \
  --mnemonic "$MNEMONIC" \
  --name "Device A Wallet" \
  --password "deviceA123" \
  --network mainnet

# Record output
export DEVICE_A_WALLET_ID="<wallet_id from output>"
export DEVICE_A_ADDRESS="<address from output>"

echo "Device A:"
echo "  Wallet ID: $DEVICE_A_WALLET_ID"
echo "  Address: $DEVICE_A_ADDRESS"
```

### Device B: Recover Wallet

**On Device B** (different computer, completely separate environment):
```bash
# Use SAME mnemonic as Device A
export MNEMONIC="your custom 24-word mnemonic here"
export EXPECTED_ADDRESS="<DEVICE_A_ADDRESS from above>"

btpc-wallet recover-from-mnemonic \
  --mnemonic "$MNEMONIC" \
  --name "Device B Wallet" \
  --password "deviceB456" \
  --network mainnet \
  --expected-address "$EXPECTED_ADDRESS"

# Record output
export DEVICE_B_WALLET_ID="<wallet_id from output>"
export DEVICE_B_ADDRESS="<address from output>"

echo "Device B:"
echo "  Wallet ID: $DEVICE_B_WALLET_ID"
echo "  Address: $DEVICE_B_ADDRESS"
```

### Verify Cross-Device Recovery

```bash
if [ "$DEVICE_A_ADDRESS" = "$DEVICE_B_ADDRESS" ]; then
  echo "✅ SUCCESS: Cross-device recovery works!"
  echo "   Device A: $DEVICE_A_ADDRESS"
  echo "   Device B: $DEVICE_B_ADDRESS"
else
  echo "❌ FAIL: Cross-device recovery failed!"
  echo "   Device A: $DEVICE_A_ADDRESS"
  echo "   Device B: $DEVICE_B_ADDRESS"
  exit 1
fi
```

**Expected Behavior**:
- ✅ Wallet IDs are DIFFERENT (new UUID per device)
- ✅ Addresses are IDENTICAL (same keys derived)
- ✅ Funds sent to Device A address can be spent on Device B

---

## Test Scenario 3: 100x Recovery Consistency

### Automated Test Script

**Create**: `test_determinism.sh`
```bash
#!/bin/bash

MNEMONIC="abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art"
NETWORK="regtest"
PASSWORD="test123"

echo "Testing deterministic key generation (100 iterations)..."

# First iteration - record expected address
OUTPUT_1=$(btpc-wallet create-from-mnemonic \
  --mnemonic "$MNEMONIC" \
  --name "Test 1" \
  --password "$PASSWORD" \
  --network "$NETWORK" 2>&1)

EXPECTED_ADDRESS=$(echo "$OUTPUT_1" | grep "Address:" | awk '{print $2}')
WALLET_ID_1=$(echo "$OUTPUT_1" | grep "Wallet ID:" | awk '{print $3}')

echo "Expected address: $EXPECTED_ADDRESS"
btpc-wallet delete --wallet-id "$WALLET_ID_1" --confirm >/dev/null 2>&1

# Test 100 iterations
PASS_COUNT=0
FAIL_COUNT=0

for i in {2..100}; do
  OUTPUT=$(btpc-wallet create-from-mnemonic \
    --mnemonic "$MNEMONIC" \
    --name "Test $i" \
    --password "$PASSWORD" \
    --network "$NETWORK" 2>&1)

  ACTUAL_ADDRESS=$(echo "$OUTPUT" | grep "Address:" | awk '{print $2}')
  WALLET_ID=$(echo "$OUTPUT" | grep "Wallet ID:" | awk '{print $3}')

  if [ "$ACTUAL_ADDRESS" = "$EXPECTED_ADDRESS" ]; then
    PASS_COUNT=$((PASS_COUNT + 1))
  else
    FAIL_COUNT=$((FAIL_COUNT + 1))
    echo "❌ FAIL at iteration $i:"
    echo "   Expected: $EXPECTED_ADDRESS"
    echo "   Actual:   $ACTUAL_ADDRESS"
  fi

  # Cleanup
  btpc-wallet delete --wallet-id "$WALLET_ID" --confirm >/dev/null 2>&1

  # Progress
  if [ $((i % 10)) -eq 0 ]; then
    echo "Progress: $i/100 iterations complete"
  fi
done

echo ""
echo "=========================================="
echo "Determinism Test Results:"
echo "  Total iterations: 100"
echo "  Passed: $PASS_COUNT"
echo "  Failed: $FAIL_COUNT"
echo "=========================================="

if [ $FAIL_COUNT -eq 0 ]; then
  echo "✅ SUCCESS: All 100 recoveries produced identical keys!"
  exit 0
else
  echo "❌ FAIL: $FAIL_COUNT recoveries produced different keys!"
  exit 1
fi
```

**Run**:
```bash
chmod +x test_determinism.sh
./test_determinism.sh

# Expected output:
# Testing deterministic key generation (100 iterations)...
# Expected address: btpc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh
# Progress: 10/100 iterations complete
# Progress: 20/100 iterations complete
# ...
# Progress: 100/100 iterations complete
#
# ==========================================
# Determinism Test Results:
#   Total iterations: 100
#   Passed: 99
#   Failed: 0
# ==========================================
# ✅ SUCCESS: All 100 recoveries produced identical keys!
```

---

## Test Scenario 4: V1 Wallet Migration Warning

### Step 1: Load Existing V1 Wallet

**CLI**:
```bash
# Assume you have a v1 wallet from before this feature
btpc-wallet list

# Expected output:
# Wallets:
#   - ID: old-wallet-uuid
#     Name: Old Wallet
#     Version: V1 (Non-Deterministic)
#     Recovery: ⚠️ Limited (wallet file backup required)
#     Address: btpc1q...
```

**Desktop App**:
1. Open "Wallets" page
2. Find existing v1 wallet
3. Check wallet card shows "v1" badge
4. Click on wallet
5. See warning: "Limited recovery - backup .dat file required"

### Step 2: Check Version

**CLI**:
```bash
btpc-wallet get-version --wallet-id old-wallet-uuid

# Expected output:
# Wallet ID: old-wallet-uuid
# Version: V1 (Non-Deterministic)
# Recovery capable: ❌ No
# Migration recommended: ✅ Yes
#
# ⚠️  Your wallet was created with an older version.
# For proper seed phrase recovery, please:
#   1. Create a new v2 wallet
#   2. Transfer funds to the new wallet
#   3. Backup the new 24-word seed phrase
#   4. Delete the old v1 wallet
```

**Desktop App**:
1. Click on v1 wallet
2. See migration banner at top:
   ```
   ⚠️ Limited Recovery Wallet
   This wallet was created before BIP39 support.
   [Migrate to V2 Wallet] button
   ```
3. Click "[Migrate to V2 Wallet]"
4. Follow migration wizard:
   - Create new v2 wallet
   - Shows transfer instructions
   - Checklist: [ ] Backup seed phrase, [ ] Transfer funds, [ ] Verify balance, [ ] Delete v1 wallet

---

## Test Scenario 5: Invalid Mnemonic Rejection

### Test 1: Wrong Word Count

**CLI**:
```bash
btpc-wallet create-from-mnemonic \
  --mnemonic "abandon abandon abandon" \
  --name "Bad Wallet" \
  --password "test123" \
  --network regtest

# Expected output:
# ❌ Error: INVALID_MNEMONIC_LENGTH
# Message: Mnemonic must have exactly 24 words (found: 3)
```

### Test 2: Invalid Word

**CLI**:
```bash
btpc-wallet create-from-mnemonic \
  --mnemonic "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon xyz" \
  --name "Bad Wallet" \
  --password "test123" \
  --network regtest

# Expected output:
# ❌ Error: INVALID_MNEMONIC_WORD
# Message: Invalid word at position 24: 'xyz'
```

### Test 3: Invalid Checksum

**CLI**:
```bash
# Valid words, but wrong checksum (last word incorrect)
btpc-wallet create-from-mnemonic \
  --mnemonic "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about" \
  --name "Bad Wallet" \
  --password "test123" \
  --network regtest

# Expected output:
# ❌ Error: INVALID_MNEMONIC_CHECKSUM
# Message: Invalid BIP39 checksum - please check your seed phrase
```

### Verify No Side Effects

```bash
# Check that no wallet file was created
ls ~/.btpc/wallets/

# Should NOT show any new wallet files

# Check that no event was emitted (desktop app)
# Wallet list should be unchanged
```

---

## Performance Benchmarks

### Measure Wallet Creation Time

**Script**: `benchmark_recovery.sh`
```bash
#!/bin/bash

MNEMONIC="abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art"

echo "Benchmarking wallet recovery performance..."

# Warm-up
btpc-wallet create-from-mnemonic \
  --mnemonic "$MNEMONIC" \
  --name "Warmup" \
  --password "test" \
  --network regtest >/dev/null 2>&1
btpc-wallet delete --wallet-id $(btpc-wallet list | tail -1 | awk '{print $3}') --confirm >/dev/null 2>&1

# Measure 10 iterations
TOTAL_TIME=0
for i in {1..10}; do
  START=$(date +%s%3N)

  btpc-wallet create-from-mnemonic \
    --mnemonic "$MNEMONIC" \
    --name "Benchmark $i" \
    --password "test" \
    --network regtest >/dev/null 2>&1

  END=$(date +%s%3N)
  ELAPSED=$((END - START))
  TOTAL_TIME=$((TOTAL_TIME + ELAPSED))

  # Cleanup
  WALLET_ID=$(btpc-wallet list | tail -1 | awk '{print $3}')
  btpc-wallet delete --wallet-id "$WALLET_ID" --confirm >/dev/null 2>&1

  echo "Iteration $i: ${ELAPSED}ms"
done

AVG_TIME=$((TOTAL_TIME / 10))

echo ""
echo "=========================================="
echo "Performance Benchmark Results:"
echo "  Total iterations: 10"
echo "  Total time: ${TOTAL_TIME}ms"
echo "  Average time: ${AVG_TIME}ms"
echo "  Requirement: < 2000ms (FR-019)"
echo "=========================================="

if [ $AVG_TIME -lt 2000 ]; then
  echo "✅ PASS: Average time ${AVG_TIME}ms < 2000ms"
else
  echo "❌ FAIL: Average time ${AVG_TIME}ms >= 2000ms"
fi
```

**Expected Output**:
```
Benchmarking wallet recovery performance...
Iteration 1: 142ms
Iteration 2: 138ms
Iteration 3: 145ms
Iteration 4: 140ms
Iteration 5: 143ms
Iteration 6: 139ms
Iteration 7: 141ms
Iteration 8: 144ms
Iteration 9: 138ms
Iteration 10: 142ms

==========================================
Performance Benchmark Results:
  Total iterations: 10
  Total time: 1412ms
  Average time: 141ms
  Requirement: < 2000ms (FR-019)
==========================================
✅ PASS: Average time 141ms < 2000ms
```

---

## Success Criteria Checklist

Run all test scenarios above, then verify:

- [ ] **FR-001**: Same seed → same keys (Test Scenario 1)
- [ ] **FR-005**: Byte-identical key recovery (Test Scenario 1 Step 5)
- [ ] **FR-006**: Cross-device recovery works (Test Scenario 2)
- [ ] **FR-007**: Wallet version metadata persisted (Test Scenario 4)
- [ ] **FR-008**: V1 wallets show migration warning (Test Scenario 4)
- [ ] **FR-017**: BIP39 validation < 100ms (Test Scenario 5, measure time)
- [ ] **FR-019**: Full recovery < 2 seconds (Performance Benchmarks)
- [ ] **100x Consistency**: All 100 recoveries identical (Test Scenario 3)
- [ ] **Invalid Rejection**: Bad mnemonics rejected early (Test Scenario 5)
- [ ] **No Side Effects**: Failed validations don't create wallets (Test Scenario 5 verify)

---

## Troubleshooting

### Issue: Addresses Differ After Recovery

**Symptom**: ADDRESS_1 ≠ ADDRESS_2 in Test Scenario 1

**Diagnosis**:
```bash
# Check if deterministic key generation is enabled
btpc-wallet debug-key-generation --mnemonic "$MNEMONIC"

# Should show:
# Key generation method: Deterministic (SHAKE256 + crystals-dilithium)
# Seed expansion: SHAKE256 with domain tag "BTPC-ML-DSA-v1"
# Library: crystals-dilithium v0.3
```

**Fix**: Verify feature implementation is correct (this is a bug if addresses differ)

### Issue: Performance Exceeds 2 Seconds

**Symptom**: Benchmark shows average > 2000ms

**Diagnosis**:
```bash
# Check per-stage timings
btpc-wallet debug-recovery-timing --mnemonic "$MNEMONIC"

# Should show:
# BIP39 validation: ~50ms
# PBKDF2 derivation: ~20ms
# SHAKE256 expansion: ~10ms
# ML-DSA key generation: ~85ms
# Encryption: ~50ms
# File write: ~10ms
# Total: ~225ms
```

**Fix**: If any stage exceeds target, investigate that specific component

### Issue: V1 Wallet Doesn't Show Migration Warning

**Symptom**: Old wallet shows "v2" badge or no warning

**Diagnosis**:
```bash
# Check wallet metadata
btpc-wallet inspect --wallet-id old-wallet-uuid

# Should show:
# Version field: V1NonDeterministic (or missing for very old wallets)
```

**Fix**: Update wallet version detection logic to default to V1 if field missing

---

## Acceptance Criteria Summary

All quickstart tests must pass to accept feature:

1. ✅ **Determinism**: Same mnemonic produces identical wallets
2. ✅ **Cross-Device**: Recovery works on different computers
3. ✅ **Consistency**: 100 recoveries all produce same keys
4. ✅ **Migration**: V1 wallets show clear warnings
5. ✅ **Validation**: Invalid mnemonics rejected before wallet creation
6. ✅ **Performance**: Recovery completes in < 2 seconds
7. ✅ **UI**: Desktop app shows wallet version badges
8. ✅ **Events**: Backend emits wallet:created and wallet:recovered events

**When all checkboxes are ✅, feature is ready for production.**

---

**Quickstart Version**: 1.0
**Last Updated**: 2025-11-06
**Estimated Duration**: 5-10 minutes per scenario