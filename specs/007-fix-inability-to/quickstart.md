# Quickstart: Testing Transaction Sending Fix

**Feature**: Fix Transaction Sending Between Wallets
**Time to Test**: ~10 minutes
**Prerequisites**: BTPC desktop app running, at least 2 wallets created with balance

## Test Environment Setup

### 1. Start Required Services
```bash
# Terminal 1: Start the node
cd /home/bob/BTPC/BTPC
./target/release/btpc_node --network regtest

# Terminal 2: Start the desktop app
cd /home/bob/BTPC/BTPC/btpc-desktop-app
npm run tauri:dev
```

### 2. Prepare Test Wallets
```bash
# Create two test wallets if needed
./target/release/btpc_wallet create --name TestWallet1
./target/release/btpc_wallet create --name TestWallet2

# Mine some test coins to Wallet1 (regtest only)
./target/release/btpc_miner --wallet TestWallet1 --blocks 10
```

## Test Scenarios

### Scenario 1: Send Between Internal Wallets ✅

**Objective**: Verify funds can be transferred between user's own wallets

1. **Open Desktop App**
   - Navigate to http://localhost:1420 (or app window)
   - Login with master password

2. **Select Source Wallet**
   - Go to Wallet Manager
   - Select TestWallet1 (should show 100+ BTPC from mining)
   - Note the balance

3. **Create Transaction**
   - Go to Transactions → Send tab
   - Enter recipient: TestWallet2's address (copy from Wallet Manager)
   - Amount: 50 BTPC
   - Click "Estimate Fee"
   - Verify fee shows (should be ~0.001 BTPC)

4. **Send Transaction**
   - Click "Send Transaction"
   - Enter wallet password when prompted
   - **Expected**: Progress indicators for:
     - ✅ Creating transaction
     - ✅ Signing with ML-DSA
     - ✅ Broadcasting to network
     - ✅ Transaction sent successfully

5. **Verify Balances**
   - Switch to TestWallet2
   - **Expected**: Balance shows +50 BTPC (minus fee)
   - Switch back to TestWallet1
   - **Expected**: Balance reduced by 50 BTPC + fee

### Scenario 2: Send to External Address ✅

**Objective**: Verify sending to addresses outside user's wallets

1. **Generate External Address**
   ```bash
   # In terminal, create external wallet
   ./target/release/btpc_wallet create --name ExternalWallet
   ./target/release/btpc_wallet address --wallet ExternalWallet
   # Copy the address shown
   ```

2. **Send from Desktop App**
   - Select TestWallet1
   - Go to Transactions → Send
   - Paste external address
   - Amount: 10 BTPC
   - Send transaction
   - **Expected**: Transaction completes with all status updates

3. **Verify on External Wallet**
   ```bash
   ./target/release/btpc_wallet balance --wallet ExternalWallet
   # Expected: Shows 10 BTPC received
   ```

### Scenario 3: Error Handling ✅

**Objective**: Verify proper error messages and state management

#### Test 3.1: Insufficient Funds
1. Select wallet with low balance
2. Try to send more than available
3. **Expected**: Clear error "Insufficient funds. Available: X BTPC, Required: Y BTPC"
4. **Verify**: Wallet state unchanged, no UTXOs locked

#### Test 3.2: Invalid Address
1. Enter malformed address: "btpc_invalid_123"
2. Try to send
3. **Expected**: Error "Invalid BTPC address format"
4. **Verify**: Transaction not created

#### Test 3.3: Network Disconnection
1. Stop the node (Ctrl+C in Terminal 1)
2. Try to send transaction
3. **Expected**: Error "Cannot connect to node. Please check your connection."
4. **Verify**: Option to retry when node restarted

### Scenario 4: Concurrent Transactions ⚠️

**Objective**: Verify UTXO locking prevents double-spending

1. **Open Two Transaction Windows**
   - Open app in two browser tabs
   - Select same wallet in both

2. **Send Simultaneously**
   - Tab 1: Send 40 BTPC to TestWallet2
   - Tab 2: Immediately send 40 BTPC to ExternalWallet
   - **Expected**:
     - First transaction proceeds
     - Second shows "UTXOs locked by another transaction"

3. **After First Completes**
   - Retry second transaction
   - **Expected**: Now succeeds if balance sufficient

### Scenario 5: Transaction Status Events 🔄

**Objective**: Verify real-time status updates via events

1. **Open Browser Console**
   - Press F12 → Console tab
   - Clear console

2. **Send Transaction**
   - Create and send any transaction
   - **Watch console for events**:
   ```javascript
   // Expected event sequence:
   transaction:initiated
   transaction:validated
   transaction:signing_started
   transaction:signed
   transaction:broadcast
   transaction:confirmed
   wallet:balance_updated
   ```

3. **Verify No Polling**
   - Check Network tab
   - **Expected**: No repeated status requests, only event-driven updates

## Performance Validation

### Transaction Creation Speed
- Start timer when clicking "Send"
- Stop when "Creating transaction" appears
- **Target**: < 500ms

### ML-DSA Signing Speed
- Time from "Signing" to "Signed" status
- **Target**: < 100ms per input

### UI Responsiveness
- During transaction processing:
  - Can switch tabs
  - Can open other wallets
  - No freezing or lag
- **Target**: All UI interactions < 50ms response

## Debugging Guide

### If Transaction Fails

1. **Check Desktop App Logs**
   ```bash
   # Linux/Mac
   tail -f ~/.btpc/logs/desktop-app.log

   # Look for:
   - UTXO selection errors
   - Signature generation failures
   - Network broadcast errors
   ```

2. **Verify Wallet Has Seed**
   ```bash
   # Check wallet file structure
   cat ~/.btpc/wallets/TestWallet1.dat | jq '.keys[0].seed'
   # Should show seed bytes, not null
   ```

3. **Check Node Connectivity**
   ```bash
   # Test RPC connection
   curl -X POST http://localhost:8332 \
     -H "Content-Type: application/json" \
     -d '{"jsonrpc":"2.0","method":"getblockcount","params":[],"id":1}'
   ```

4. **Inspect UTXO State**
   ```bash
   # List available UTXOs
   ./target/release/btpc_wallet utxos --wallet TestWallet1
   ```

### Common Issues and Fixes

| Issue | Cause | Fix |
|-------|-------|-----|
| "Signature creation failed" | Missing seed in wallet | Restore wallet with seed phrase |
| "No UTXOs available" | All locked/spent | Wait for pending tx or check balance |
| "Invalid address" | Wrong network type | Ensure mainnet/testnet/regtest match |
| "Network unavailable" | Node not running | Start node before sending |
| "Fee too high" | Small UTXO set | Consolidate UTXOs first |

## Success Criteria

✅ **All tests pass if**:
1. Can send between internal wallets
2. Can send to external addresses
3. Proper error messages displayed
4. No double-spending possible
5. Events fire in correct sequence
6. Performance targets met
7. UI remains responsive

## Rollback Plan

If issues occur after deployment:

1. **Immediate**: Disable send button in UI
2. **Quick Fix**: Revert to previous transaction code
3. **Data Recovery**: UTXOs auto-unlock after 5 minutes
4. **User Communication**: Show maintenance message

## Next Steps

After successful testing:
1. Run integration test suite: `cargo test --test transaction_flow`
2. Deploy to testnet for broader testing
3. Monitor error rates and performance metrics
4. Collect user feedback on error messages

---

**Support**: Report issues to btpc-dev@example.com or file bug report
**Documentation**: See `/docs/transactions.md` for detailed transaction lifecycle