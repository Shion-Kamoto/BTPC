# Quick Start: Testing All Bug Fixes

**Date**: 2025-10-25
**Status**: All fixes implemented and ready for testing

## What Was Fixed

1. ✅ **Transaction Signing** - "Failed to sign input 0" error fixed
2. ✅ **Wallet Backup** - Backup button now creates directory and handles errors
3. ✅ **UTXO Locking** - Prevents race conditions in concurrent transactions
4. ✅ **Wallet ID** - Backups now include wallet_id for restoration

## Prerequisites

- Rust 1.75+ installed
- BTPC source code in `/home/bob/BTPC/BTPC`
- Clean terminal session

## Step 1: Run Automated Tests

```bash
cd /home/bob/BTPC/BTPC/btpc-desktop-app/src-tauri

# Run comprehensive verification tests
cargo test --test verify_all_fixes -- --nocapture

# Expected output:
#   test test_wallet_creation_includes_seed_and_wallet_id ... ok
#   test test_transaction_signing_after_wallet_load ... ok
#   test test_multi_input_transaction_signing ... ok
#   test test_wallet_backup_completeness ... ok
#   test test_utxo_reservation_system ... ok
#   test test_complete_wallet_lifecycle ... ok
#   test test_summary ... ok
#
#   test result: ok. 7 passed; 0 failed; 0 ignored
```

✅ **All 7 tests should pass** if fixes are correct!

## Step 2: Build Desktop App

```bash
cd /home/bob/BTPC/BTPC/btpc-desktop-app/src-tauri

# Build the app (takes ~2-3 minutes)
cargo build --release

# Expected output:
#   Finished `release` profile [optimized] target(s) in X.XXs
```

## Step 3: Start Desktop App

```bash
# Run the built app
./target/release/btpc-desktop-app

# Or use development mode:
npm run tauri:dev
```

## Step 4: Test Wallet Creation (NEW WALLET REQUIRED!)

**IMPORTANT**: Your old wallet doesn't have a seed! You MUST create a new wallet.

1. **Click "Wallet Manager"** in the sidebar
2. **Click "Create New Wallet"**
3. **Fill in details**:
   - Nickname: "Test Wallet 2025"
   - Password: (choose a strong password)
   - Category: "Testing"
4. **Click "Create Wallet"**
5. **Save the seed phrase** (24 words) - write it down!

✅ **New wallet will have**:
- 32-byte seed (for signing)
- wallet_id (for backups)
- Argon2id encryption

## Step 5: Test Backup Function

1. **Click "View"** on your new wallet
2. **Click "Backup"** button
3. **Confirm the backup dialog**

✅ **Expected Result**:
```
Wallet backed up to: /home/bob/.btpc/wallet-backups/backup_Test_Wallet_2025_20251025_HHMMSS.dat

Your wallet has been safely backed up!
```

✅ **Verify backup file exists**:
```bash
ls -lh ~/.btpc/wallet-backups/
# Should show backup_Test_Wallet_2025_*.dat
```

## Step 6: Test Transaction Signing

**Prerequisites**:
- New wallet created (with seed)
- Wallet has some balance (mine or receive funds)

### 6.1: Mine Some Blocks to New Wallet

```bash
# In the desktop app:
1. Click "Mining" in sidebar
2. Select your new wallet from dropdown
3. Click "Start Mining"
4. Mine 5-10 blocks
5. Wait for confirmations (100 blocks for coinbase maturity)
```

### 6.2: Send Transaction

```bash
# In the desktop app:
1. Click "Wallet Manager"
2. Click "View" on your wallet
3. Click "Send" button
4. Enter recipient address (can be another wallet you create)
5. Enter amount (e.g., 1.0 BTP)
6. Enter your password
7. Click "Send Transaction"
```

✅ **Expected Result**:
```
Transaction signed and broadcast successfully from wallet 'Test Wallet 2025'
Transaction ID: tx_1729876543210
Sent 1.00000000 BTP to btpc1...
Fee: 0.00010000 BTP
Inputs: 1 UTXOs (signed with ML-DSA)
Outputs: 2 outputs
Status: Broadcast to network
```

❌ **OLD ERROR (should NOT see this)**:
```
Failed to send transaction: Failed to sign input 0: Signature creation failed
```

## Step 7: Verify UTXO Locking (Advanced)

This requires testing concurrent transactions. Skip if not needed.

```bash
# In the desktop app, try to send two transactions quickly:
1. Open wallet in two browser tabs (if using tauri dev)
2. Initiate transaction in Tab 1
3. Immediately initiate another transaction in Tab 2
4. Both should succeed without UTXO conflicts
```

✅ **Expected**: Second transaction uses different UTXOs (or waits)
❌ **Old Bug**: Both transactions would try to use same UTXOs

## Troubleshooting

### Test Failure: "Signing should succeed with seed!"

**Cause**: Using old wallet without seed
**Fix**: Create NEW wallet (Step 4)

### Backup Button Does Nothing

**Cause**: Check browser console for errors
**Fix**:
```bash
# Rebuild app:
cd /home/bob/BTPC/BTPC/btpc-desktop-app/src-tauri
cargo build --release
./target/release/btpc-desktop-app
```

### Transaction Signing Still Fails

**Causes**:
1. Using old wallet (no seed)
2. App not rebuilt after fix
3. Password incorrect

**Fixes**:
1. Create NEW wallet
2. Rebuild: `cargo build --release`
3. Try password again

### Can't Find Backup File

**Check**:
```bash
# List all backups:
ls -lh ~/.btpc/wallet-backups/

# Check app logs:
tail -f ~/.btpc/logs/btpc-desktop-app.log
```

## Verification Checklist

- [ ] All 7 automated tests pass
- [ ] Desktop app builds without errors
- [ ] New wallet created successfully
- [ ] Seed phrase displayed (24 words)
- [ ] Backup button works
- [ ] Backup file created in `~/.btpc/wallet-backups/`
- [ ] Transaction signing works (after mining/receiving funds)
- [ ] No "SigningFailed" errors

## Success Criteria

✅ **Minimum for Success**:
1. All 7 tests pass: `cargo test --test verify_all_fixes`
2. Backup file created successfully
3. Transaction signing works with new wallet

✅ **Full Success**:
- All minimum criteria met
- Wallet lifecycle complete (create → backup → sign → restore)
- No errors in app logs

## Next Steps After Testing

If all tests pass:
1. Delete old wallets (no seed, can't sign)
2. Use new wallet for all transactions
3. Set new wallet as default
4. Create periodic backups

If tests fail:
1. Check which test failed
2. Review error messages
3. Verify you created NEW wallet (not using old one)
4. Rebuild app: `cargo build --release`

## Support

For issues, check:
- `MD/SESSION_COMPLETE_2025-10-25_UTXO_LOCKING.md` - Full implementation details
- `MD/ML_DSA_LIBRARY_LIMITATION.md` - Technical background on ML-DSA
- Test output: `cargo test --test verify_all_fixes -- --nocapture`

---

**Remember**: The critical requirement is to **create a NEW wallet** after rebuilding the app. Old wallets don't have seeds and cannot sign transactions!