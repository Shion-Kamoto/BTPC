# BTPC Send/Receive Testing Guide - 2025-11-05

## ✅ ALL FIXES APPLIED

The critical send/receive bugs have been fixed in the latest build:

### Fixed Issues
1. **fork_id Serialization** - Added fork_id byte to transaction signatures (transaction_commands.rs:520)
2. **Tauri Parameter Flattening** - create_transaction uses flattened params (transaction_commands.rs:48-55)
3. **UTXO Reservation** - Prevents double-spending with 5-minute locks (wallet_manager.rs)
4. **Dynamic Fee Estimation** - RPC-based fees with conservative fallback (fee_estimator.rs)
5. **Wallet Integrity Validation** - Pre-signing ML-DSA key checks (transaction_commands.rs:534-600)
6. **RocksDB Schema Migration** - Fixed fork_id compatibility, 17 transactions migrated

### Binary Info
- **Path**: `/home/bob/BTPC/BTPC/btpc-desktop-app/src-tauri/target/release/btpc-desktop-app`
- **Size**: 19MB
- **Build Date**: 2025-11-05 13:18
- **Status**: ✅ Running (PID: 1922890)

---

## Pre-Test Setup

### 1. Verify Node is Running
```bash
ps aux | grep btpc_node
# Expected: Process running on port 18360
```

### 2. Check Wallet Balance
Current wallet address: `n3UWGnEV1LQPJuFvnvnBpSLBipxoZavtHW`

Open browser console in desktop app:
```javascript
const balance = await window.invoke('get_wallet_balance', {
    address: 'n3UWGnEV1LQPJuFvnvnBpSLBipxoZavtHW'
});
console.log('Balance:', balance);
// Expected: ~550 BTPC (from 17 mined blocks)
```

### 3. Create Second Test Wallet (if needed)
In Wallet Manager tab:
1. Click "Create New Wallet"
2. Set password (remember it!)
3. Copy the new address for testing

---

## Test 1: Send BTPC Between Wallets

### Step 1: Open Transactions Tab
Navigate to: **Transactions** → **Send** sub-tab

### Step 2: Fill Send Form
- **From Wallet**: Select wallet with balance (default wallet)
- **To Address**: Paste second wallet address OR use: `n1TestAddressForReceivingBTPC123456`
- **Amount**: `1.5` BTPC (1.5 × 10^8 = 150,000,000 credits)
- **Fee**: Leave auto (dynamic estimation)

### Step 3: Monitor Console Output
Open browser DevTools (F12) and watch for:
```
🔨 Creating transaction:
  Wallet: <wallet_id>
  From: n3UWGnEV1LQPJuFvnvnBpSLBipxoZavtHW
  To: <recipient_address>
  Amount: 150000000 credits

📝 Step 1: Creating transaction...
✅ Transaction created: <txid>

🔐 Step 2: Signing transaction...
✅ Transaction signed successfully

📡 Step 3: Broadcasting transaction...
✅ Transaction broadcast: <txid>
```

### Step 4: Check Backend Logs
```bash
# In terminal where app is running, should see:
✅ Transaction validated
✅ Signature verification passed
✅ Transaction accepted by node
```

### Step 5: Verify Transaction Status
In browser console:
```javascript
const status = await window.invoke('get_transaction_status', {
    txid: '<transaction_id>'
});
console.log('Status:', status);
// Expected: "pending" → "confirmed" (after block mined)
```

---

## Test 2: Verify Receiving Wallet

### Step 1: Switch to Receiving Wallet
In Wallet Manager:
1. Select the wallet that received funds
2. Unlock with password

### Step 2: Check Balance Updated
```javascript
const balance = await window.invoke('get_wallet_balance', {
    address: '<receiving_wallet_address>'
});
console.log('Received:', balance);
// Expected: 1.5 BTPC (150,000,000 credits)
```

### Step 3: Check Transaction History
Navigate to: **Transactions** → **History** sub-tab
- Should show incoming transaction
- Amount: +1.5 BTPC
- Status: Confirmed
- From: n3UWGnEV1LQPJuFvnvnBpSLBipxoZavtHW

---

## Test 3: Mine Block to Confirm Transaction

If transaction stays "pending":

### Option A: Mine via Desktop App
1. Go to **Mining** tab
2. Click "Start Mining"
3. Wait for block (should be fast on regtest)
4. Check transaction status again

### Option B: Mine via CLI
```bash
/home/bob/BTPC/BTPC/target/release/btpc_miner \
  --network regtest \
  --address n3UWGnEV1LQPJuFvnvnBpSLBipxoZavtHW \
  --threads 4
```

---

## Common Errors & Solutions

### Error: "No UTXOs available"
**Cause**: Wallet has no unspent outputs
**Solution**: Mine blocks to the sending wallet first

### Error: "Failed to sign input 0"
**Cause**: Wallet file corruption OR missing seed
**Solution**:
1. Check wallet integrity validation logs
2. Restore from seed phrase if needed
3. Create new wallet if corrupted

### Error: "Signature verification failed"
**Cause**: fork_id mismatch (should be fixed now!)
**Solution**: Already fixed in transaction_commands.rs:520

### Error: "Insufficient balance"
**Cause**: Amount + Fee > Available Balance
**Solution**: Reduce amount or mine more blocks

### Transaction Stuck "Pending"
**Cause**: No blocks being mined
**Solution**: Mine 1 block to confirm transactions

---

## Verification Checklist

After successful send/receive:

- [ ] Sending wallet balance decreased by (amount + fee)
- [ ] Receiving wallet balance increased by amount
- [ ] Transaction appears in both wallets' history
- [ ] Transaction status changed: pending → confirmed
- [ ] UTXO set updated correctly (check with get_wallet_utxos)
- [ ] No error messages in console
- [ ] RocksDB persisted transaction (survives app restart)

---

## Advanced Testing

### Test 4: Multiple Concurrent Transactions
Test UTXO reservation system:
1. Create transaction A (don't broadcast yet)
2. Create transaction B (should reserve different UTXOs)
3. Broadcast both
4. Verify no double-spending

### Test 5: Large Transaction
Test with maximum inputs:
1. Send 50+ BTPC (requires many UTXOs)
2. Verify signature calculation works with large ML-DSA signatures
3. Check fee estimation scales correctly

### Test 6: App Restart Persistence
1. Send transaction
2. Close app before confirmation
3. Restart app
4. Verify transaction still tracked in RocksDB

---

## Debug Commands

### Check RocksDB Transaction Count
```javascript
const history = await window.invoke('get_mining_history_from_storage');
console.log(`Transactions in RocksDB: ${history.length}`);
```

### List All UTXOs
```javascript
const utxos = await window.invoke('get_wallet_utxos', {
    address: 'n3UWGnEV1LQPJuFvnvnBpSLBipxoZavtHW'
});
console.log('UTXOs:', utxos);
```

### Check Node RPC Connection
```bash
curl -s http://127.0.0.1:18360 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getblockchaininfo","params":[]}'
```

---

## Expected Test Results

### Successful Transaction Flow
```
1. create_transaction: 50ms - 200ms
2. sign_transaction: 100ms - 500ms (ML-DSA signing)
3. broadcast_transaction: 50ms - 100ms
4. Transaction pending: Until next block
5. Transaction confirmed: After 1 block mined
6. Balance updated: Immediately after confirmation
```

### Typical Timings (Regtest)
- Transaction creation: < 1 second
- Signature generation: < 1 second
- Block mining: 1-60 seconds (depending on CPU)
- Confirmation: Instant after block found

---

## Success Criteria

✅ **Send/Receive is WORKING** if:
1. Transaction created without errors
2. Signature verification passes
3. Node accepts transaction
4. Transaction confirmed in next block
5. Balances updated correctly
6. No fork_id errors
7. No double-spending
8. Data persists after restart

---

## Report Issues

If any test fails, collect:
1. Browser console logs
2. Backend terminal output
3. Transaction ID
4. Wallet addresses
5. Error message (full text)
6. Steps to reproduce

**File**: `MD/SEND_RECEIVE_BUG_REPORT_<date>.md`

---

**Testing Guide Created**: 2025-11-05
**Last Updated**: 2025-11-05 13:50
**Status**: ✅ READY FOR TESTING