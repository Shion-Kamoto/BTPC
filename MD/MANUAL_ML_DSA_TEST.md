# Manual ML-DSA Signature Generation Test

## Objective
Test ML-DSA (Dilithium5) signature generation by sending a BTPC transaction from testingW1 to testingW2.

## Prerequisites
- ✅ Desktop app is running (PID 333712)
- ✅ testingW1 has balance of 102,434.50 BTP
- ✅ testingW2 address: `mwjfUDDmMYdDyN54FyFKsNCCJMrYk3GCWY`
- ✅ Signing code location: `btpc-desktop-app/src-tauri/src/wallet_commands.rs` line 251

## Test Procedure

### Step 1: Open Desktop App
The app should already be running. If not, start it with:
```bash
cd btpc-desktop-app && npm run tauri:dev
```

### Step 2: Navigate to Transactions Page
1. Click on "Transactions" in the left sidebar
2. Ensure you're on the "Send" tab

### Step 3: Fill in Send Form
1. **From Wallet**: Select "testingW1" from the dropdown
2. **Recipient Address**: Enter `mwjfUDDmMYdDyN54FyFKsNCCJMrYk3GCWY`
3. **Amount**: Enter `1000.0` BTP (for easy verification)
4. **Fee**: Default (10000 credits = 0.0001 BTP)

### Step 4: Submit Transaction
1. Click the "Send BTPC" button
2. A password modal will appear
3. Enter the wallet password (likely "test")
4. Click "Confirm"

### Step 5: Monitor ML-DSA Signing
Watch the terminal where the app is running for console output. Look for:
- `🔧 DEBUG` messages about the transaction
- Messages about signing
- Transaction ID confirmation
- Any error messages

### Expected Console Output (from wallet_commands.rs):
```
🔧 DEBUG (send_btpc_from_wallet): Recipient address: 'mwjfUDDmMYdDyN54FyFKsNCCJMrYk3GCWY' -> clean: 'mwjfUDDmMYdDyN54FyFKsNCCJMrYk3GCWY'
🔧 DEBUG (send_btpc_from_wallet): Wallet address: '...' -> clean: '...'
✅ Transaction broadcast successfully! TXID: [some hash]
✅ Marked UTXO as spent: [txid]:[vout]
✅ Marked X/X UTXOs as spent
```

### Step 6: Verify ML-DSA Signature Generation
The ML-DSA signature generation happens at:
**File**: `btpc-desktop-app/src-tauri/src/wallet_commands.rs`
**Line 251**: `let signature = private_key.sign(message_bytes)`

This calls the `btpc_core::crypto::PrivateKey::sign()` method which uses Dilithium5.

### Step 7: Check Transaction History
1. Switch to the "History" tab in the Transactions page
2. Verify the transaction appears in the history
3. Note the transaction ID

### Step 8: Verify on testingW2
1. Navigate to "Wallet" page
2. Select testingW2
3. Check that it has received the 1000.0 BTP (may need to refresh or mine a block)

## Success Criteria
✅ Transaction was created without errors
✅ Password modal appeared and accepted credentials
✅ ML-DSA signing completed (line 251 executed without error)
✅ Transaction was broadcast successfully
✅ Transaction ID was returned
✅ UTXOs were marked as spent
✅ Transaction appears in history

## Monitoring Rust Backend Logs

To see the backend logs in real-time, monitor the terminal where you ran `npm run tauri:dev`.

You can also check for any error logs:
```bash
journalctl --user -f | grep -i btpc
```

## Code Reference: ML-DSA Signing

The actual ML-DSA signing happens in `wallet_commands.rs` line 244-256:

```rust
// Sign each input with ML-DSA
for (i, input) in transaction.inputs.iter_mut().enumerate() {
    // Create the signing message (transaction hash without signatures)
    // For simplicity, we'll sign the transaction ID + input index
    let signing_message = format!("{}:{}", transaction.txid, i);
    let message_bytes = signing_message.as_bytes();

    // Sign with ML-DSA (THIS IS WHERE POST-QUANTUM SIGNING HAPPENS)
    let signature = private_key.sign(message_bytes)
        .map_err(|e| format!("Failed to sign input {}: {}", i, e))?;

    // Store signature in signature_script field
    input.signature_script = signature.to_bytes().to_vec();
}
```

The `private_key.sign()` method is implemented in `btpc-core/src/crypto/mod.rs` and uses the `ml_dsa_87` crate (Dilithium5).

## Alternative: Monitor with strace

If you want to see system calls during signing:
```bash
# Find the PID
ps aux | grep btpc-desktop-app

# Monitor system calls (replace PID)
sudo strace -p 333712 -e trace=open,read,write -o /tmp/btpc-strace.log
```

Then perform the transaction and check `/tmp/btpc-strace.log` for activity.
