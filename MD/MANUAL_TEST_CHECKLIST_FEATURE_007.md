# Manual Test Checklist - Feature 007

**Feature**: Transaction Sending Between Wallets
**Date**: 2025-11-04
**Tester**: ________________
**Environment**: Desktop App (Tauri)

---

## Prerequisites

### Setup
- [ ] Desktop app compiled: `cd btpc-desktop-app && npm run tauri:dev`
- [ ] btpc_node running (regtest mode): `cargo run --release --bin btpc_node -- --network regtest`
- [ ] Node synced and responsive
- [ ] Wallet created with test funds (or mining capability)

### Tools
- [ ] Browser console open (F12) to view events
- [ ] Node logs visible for transaction confirmation
- [ ] Test addresses ready (for external send testing)

---

## Test Scenario 1: Internal Wallet Send (Basic Flow)

**Objective**: Verify transaction sending between two wallets in the same app

### Steps
1. [ ] Create Wallet A ("Sender") with balance
   - Click "Create Wallet"
   - Set name: "Test Sender"
   - Note address: _______________________
   - Fund with 200 BTPC (via mining or RPC)

2. [ ] Create Wallet B ("Receiver")
   - Click "Create Wallet"
   - Set name: "Test Receiver"
   - Note address: _______________________

3. [ ] Send 50 BTPC from Wallet A → Wallet B
   - Select Wallet A
   - Click "Send"
   - Enter Wallet B address
   - Enter amount: 50.0 BTPC
   - Click "Create Transaction"

4. [ ] Verify transaction creation
   - [ ] Transaction ID displayed
   - [ ] Fee calculated (> 0 BTPC)
   - [ ] Status: "Creating" → "Validating"
   - [ ] Console shows: `transaction:initiated` event
   - [ ] Console shows: `transaction:validated` event
   - [ ] Console shows: `utxo:reserved` event

5. [ ] Sign transaction
   - Enter wallet password
   - Click "Sign Transaction"
   - [ ] Status: "Signing" → "Signed"
   - [ ] Console shows: `transaction:signed` event

6. [ ] Broadcast transaction
   - Click "Broadcast Transaction"
   - [ ] Status: "Broadcasting" → "Broadcast"
   - [ ] Console shows: `transaction:broadcast` event
   - [ ] Transaction ID returned

7. [ ] Verify confirmation
   - Wait for block confirmation (mine blocks if needed)
   - [ ] Status: "Confirming" → "Confirmed"
   - [ ] Wallet A balance decreased by (50 + fee)
   - [ ] Wallet B balance increased by 50
   - [ ] Console shows: `transaction:confirmed` event
   - [ ] Console shows: `utxo:released` event

**Expected Results**:
- ✅ Transaction completes successfully
- ✅ Balances updated correctly
- ✅ All events emitted in order

**Actual Results**: _______________

---

## Test Scenario 2: UTXO Reservation (Double-Spend Prevention)

**Objective**: Verify UTXO locking prevents concurrent transactions

### Steps
1. [ ] Use Wallet A from Scenario 1 (with remaining balance ~150 BTPC)

2. [ ] Start Transaction 1 (DO NOT BROADCAST)
   - Amount: 100 BTPC to Wallet B
   - Click "Create Transaction"
   - [ ] Transaction created, UTXOs reserved
   - [ ] Console shows: `utxo:reserved` event

3. [ ] Immediately start Transaction 2
   - Amount: 80 BTPC to Wallet B
   - Click "Create Transaction"

4. [ ] Verify UTXO locking
   - [ ] Transaction 2 FAILS with error
   - [ ] Error message: "UTXO locked" or "Insufficient funds"
   - [ ] Error includes: "UTXOs currently reserved by another transaction"
   - [ ] Console shows: `transaction:failed` event

5. [ ] Complete or cancel Transaction 1
   - Either broadcast OR click "Cancel Transaction"
   - [ ] Console shows: `utxo:released` event (if cancelled)
   - [ ] UTXOs now available

6. [ ] Retry Transaction 2
   - Click "Create Transaction" again
   - [ ] Transaction 2 now SUCCEEDS
   - [ ] UTXOs successfully reserved

**Expected Results**:
- ✅ Second transaction fails while first is pending
- ✅ Clear error message about UTXO locking
- ✅ Second transaction succeeds after first completes/cancels

**Actual Results**: _______________

---

## Test Scenario 3: Dynamic Fee Estimation

**Objective**: Verify fee calculation adapts to transaction size and network rate

### Steps
1. [ ] **Test 1: Simple transaction** (2 inputs, 2 outputs)
   - Amount: 10 BTPC
   - [ ] Note fee: _______ BTPC
   - [ ] Fee > 0 (not hardcoded 0.001)

2. [ ] **Test 2: Complex transaction** (many inputs)
   - Use wallet with many small UTXOs (e.g., from multiple mining rewards)
   - Amount: 100 BTPC (forces use of many UTXOs)
   - [ ] Note fee: _______ BTPC
   - [ ] Fee > Test 1 fee (more inputs = higher fee)

3. [ ] **Test 3: Node offline** (fee fallback)
   - Stop btpc_node
   - Try to create transaction
   - [ ] Fee estimation uses fallback (conservative estimate)
   - [ ] Transaction still creates successfully
   - [ ] Console shows warning about using fallback fee rate

4. [ ] **Test 4: Fee rate verification**
   - Restart btpc_node
   - Check node RPC: `estimatefee` command
   - Create transaction
   - [ ] Fee matches (or is conservative vs) RPC estimate

**Expected Results**:
- ✅ Fees vary with transaction size
- ✅ Offline fallback works (no crash)
- ✅ Fee estimation reasonable (not 0, not excessive)

**Actual Results**: _______________

---

## Test Scenario 4: Error Handling

**Objective**: Verify proper error messages and recovery guidance

### Sub-test 4a: Insufficient Funds
1. [ ] Wallet with 10 BTPC balance
2. [ ] Try to send 50 BTPC
3. [ ] Verify error:
   - [ ] Error type: "Insufficient Funds"
   - [ ] Shows available: 10 BTPC
   - [ ] Shows required: 50 BTPC + fee
   - [ ] Suggested action: "Add more funds or reduce amount"

### Sub-test 4b: Invalid Address
1. [ ] Enter invalid address: "invalid_btpc_address_123"
2. [ ] Click "Create Transaction"
3. [ ] Verify error:
   - [ ] Error type: "Invalid Address"
   - [ ] Shows the invalid address
   - [ ] Reason: "Invalid format"
   - [ ] Suggested action: "Check address format"

### Sub-test 4c: Dust Output
1. [ ] Try to send 0.00000500 BTPC (500 satoshis, below 1000 dust limit)
2. [ ] Click "Create Transaction"
3. [ ] Verify error:
   - [ ] Error type: "Dust Output"
   - [ ] Shows amount: 500 satoshis
   - [ ] Shows dust limit: 1000 satoshis
   - [ ] Suggested action: "Increase amount above dust limit"

### Sub-test 4d: Wallet Corruption Detection
1. [ ] Manually corrupt wallet file (truncate or modify .dat file)
2. [ ] Try to sign transaction
3. [ ] Verify error:
   - [ ] Error type: "Wallet Corrupted"
   - [ ] Reason: "Checksum mismatch" or "Invalid format"
   - [ ] Suggested action: "Restore from backup" or "Re-create wallet"

**Expected Results**:
- ✅ All error messages are user-friendly
- ✅ Errors include specific details (amounts, addresses)
- ✅ Suggested actions provided for recovery

**Actual Results**: _______________

---

## Test Scenario 5: Event Sequence Verification

**Objective**: Verify transaction events emit in correct order

### Steps
1. [ ] Open browser console (F12)
2. [ ] Filter for "transaction:" events
3. [ ] Execute full transaction flow (create → sign → broadcast)

4. [ ] Verify event order:
   - [ ] 1. `transaction:initiated` (wallet_id, recipient, amount)
   - [ ] 2. `transaction:validated` (tx_id, inputs_count, outputs_count, fee)
   - [ ] 3. `utxo:reserved` (tx_id, utxo_keys, reservation_token)
   - [ ] 4. `transaction:signing_started` (tx_id, inputs_to_sign)
   - [ ] 5. `transaction:input_signed` (tx_id, input_index) - per input
   - [ ] 6. `transaction:signed` (tx_id, signatures_count)
   - [ ] 7. `transaction:broadcast` (tx_id, broadcast_to_peers)
   - [ ] 8. `transaction:confirmed` (tx_id, block_height, confirmations)
   - [ ] 9. `utxo:released` (reservation_token, release_reason)

5. [ ] Verify event payloads:
   - [ ] All events include timestamp
   - [ ] All events include transaction_id (after creation)
   - [ ] No duplicate events
   - [ ] Events originate from backend only (not triggered by UI)

**Expected Results**:
- ✅ All 9 event types emitted
- ✅ Correct order maintained
- ✅ Event payloads contain expected data

**Actual Results**: _______________

---

## Test Scenario 6: Transaction Cancellation

**Objective**: Verify transaction cancellation releases UTXOs

### Steps
1. [ ] Create transaction (DO NOT BROADCAST)
   - Amount: 25 BTPC
   - [ ] Transaction in "Creating" or "Signed" status
   - [ ] Console shows: `utxo:reserved` event

2. [ ] Cancel transaction
   - Click "Cancel Transaction"
   - [ ] Status changes to "Cancelled"
   - [ ] Console shows: `transaction:failed` event (stage: "Cancelled")
   - [ ] Console shows: `utxo:released` event (reason: "Cancelled")

3. [ ] Verify UTXO release
   - Create new transaction with same amount
   - [ ] Transaction succeeds (UTXOs available)
   - [ ] No "UTXO locked" error

4. [ ] Attempt to cancel broadcast transaction
   - Create and broadcast transaction
   - Try to cancel after broadcast
   - [ ] Cancel fails with error: "Cannot cancel broadcast transaction"

**Expected Results**:
- ✅ Pending transactions can be cancelled
- ✅ UTXOs released after cancellation
- ✅ Broadcast transactions cannot be cancelled

**Actual Results**: _______________

---

## Test Scenario 7: External Address Send

**Objective**: Verify sending to address not in app wallet list

### Steps
1. [ ] Prepare external address (from another wallet or testnet faucet)
   - External address: _______________________

2. [ ] Send 10 BTPC from Wallet A to external address
   - Click "Send"
   - Paste external address
   - Amount: 10 BTPC
   - Complete flow (create → sign → broadcast)

3. [ ] Verify transaction
   - [ ] Transaction completes without errors
   - [ ] Wallet A balance decreases
   - [ ] Transaction appears on blockchain explorer (if available)

**Expected Results**:
- ✅ External sends work identically to internal sends
- ✅ Address validation works for all BTPC formats

**Actual Results**: _______________

---

## Performance Validation

### Transaction Creation Speed
1. [ ] Record time for transaction creation (click → response)
   - Time: _______ ms
   - [ ] Target: < 500ms
   - [ ] Result: PASS / FAIL

### ML-DSA Signing Speed
1. [ ] Record time for signing (click → response)
   - Time: _______ ms
   - [ ] Target: < 100ms
   - [ ] Result: PASS / FAIL

### UI Responsiveness
1. [ ] During transaction processing:
   - [ ] UI remains responsive (not frozen)
   - [ ] Loading indicators shown
   - [ ] Can navigate to other pages during processing

**Expected Results**:
- ✅ Transaction creation < 500ms
- ✅ Signing < 100ms
- ✅ UI remains responsive

**Actual Results**: _______________

---

## Security Checks

### Private Key Exposure
1. [ ] Check browser console logs
   - [ ] No private keys logged
   - [ ] No seeds logged
   - [ ] Passwords not in plaintext

2. [ ] Check error messages
   - [ ] Wallet corruption errors don't expose keys
   - [ ] Signing failures don't expose seeds

**Expected Results**:
- ✅ No sensitive data in logs or errors

**Actual Results**: _______________

---

## Test Summary

### Overall Results
- Total Tests: 7 scenarios
- Passed: _____ / 7
- Failed: _____ / 7
- Blocked: _____ / 7

### Critical Issues Found
1. ________________________________________________
2. ________________________________________________
3. ________________________________________________

### Non-Critical Issues Found
1. ________________________________________________
2. ________________________________________________
3. ________________________________________________

### Performance Summary
- Transaction creation: PASS / FAIL (_____ ms)
- ML-DSA signing: PASS / FAIL (_____ ms)
- UI responsiveness: PASS / FAIL

### Recommendation
- [ ] ✅ APPROVED: Ready for production deployment
- [ ] ⚠️  CONDITIONAL: Approved with minor issues (list above)
- [ ] ❌ REJECTED: Critical issues must be fixed (list above)

---

## Notes & Observations

_____________________________________________________________

_____________________________________________________________

_____________________________________________________________

---

**Tester Signature**: ________________  **Date**: ________
**Reviewer**: ________________  **Date**: ________