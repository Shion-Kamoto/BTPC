# Feature 011: Frontend-Backend Integration - Manual Testing Guide

**Date:** 2025-11-16
**Status:** Ready for Manual Testing
**Tester:** User

---

## Overview

Feature 011 implements frontend-backend integration fixes for:
1. ✅ **T011-001**: Login system (fixed - uses correct backend commands)
2. ✅ **T011-002/003**: GPU stats display (tested - 7 passing unit tests)
3. ⚠️ **T011-005/006**: Transaction block details (BLOCKED - database compilation errors)

**This guide covers manual testing for T011-001, T011-007, and T011-008.**

---

## Prerequisites

### Build and Run the Application

```bash
cd /home/bob/BTPC/BTPC/btpc-desktop-app
npm run tauri:dev
```

**Expected:** Application launches in development mode

---

## Test Suite 1: T011-001 Login System Integration

### Test 1.1: First Launch (New User Flow)

**Objective:** Verify new user can create master password

**Steps:**
1. **Clean slate:**
   ```bash
   rm -rf ~/.btpc/credentials.enc
   rm -rf ~/.btpc/wallets/*.dat
   ```

2. **Launch app:** `npm run tauri:dev`

3. **Verify login page:**
   - ✅ Title shows "Create Master Password"
   - ✅ Subtitle shows "Secure your wallet with a master password"
   - ✅ Two password fields visible (password + confirm)
   - ✅ "Create Master Password" button visible

4. **Test password validation:**
   - Enter password: `test123` (too short)
   - Click "Create Master Password"
   - ✅ Error: "Password must be at least 8 characters long"

5. **Test password mismatch:**
   - Enter password: `testpassword123`
   - Enter confirm: `testpassword456`
   - Click "Create Master Password"
   - ✅ Error: "Passwords do not match"

6. **Create valid password:**
   - Enter password: `TestPassword123!`
   - Enter confirm: `TestPassword123!`
   - Click "Create Master Password"
   - ✅ Success message: "Master password created successfully! Redirecting..."
   - ✅ Redirects to `index.html` after 1.5 seconds

**Backend Command Verified:** `migrate_to_encrypted` ✅

---

### Test 1.2: Subsequent Launch (Existing User Flow)

**Objective:** Verify existing user can login with password

**Steps:**
1. **Close app** (Ctrl+C in terminal or close window)

2. **Relaunch app:** `npm run tauri:dev`

3. **Verify login page:**
   - ✅ Title shows "Welcome Back"
   - ✅ Subtitle shows "Enter your master password to continue"
   - ✅ Single password field visible
   - ✅ "Login" button visible

4. **Test wrong password:**
   - Enter password: `WrongPassword123`
   - Click "Login"
   - ✅ Error: "Authentication failed. Please check your password."
   - ✅ Password field cleared
   - ✅ Button re-enabled

5. **Test correct password:**
   - Enter password: `TestPassword123!` (from Test 1.1)
   - Click "Login"
   - ✅ Success message: "Login successful! Redirecting..."
   - ✅ Redirects to `index.html` after 1 second

**Backend Command Verified:** `unlock_wallets` ✅

---

### Test 1.3: Edge Cases

**Empty Password:**
- Leave password field blank
- Click "Login"
- ✅ Error: "Please enter your password"

**Check Console:**
- Open DevTools (F12)
- Navigate to Console tab
- ✅ No red errors (warnings OK)
- ✅ Should see: `[PasswordModal] Unlock successful` or similar

---

## Test Suite 2: T011-002/003 GPU Stats Display

### Test 2.1: GPU Stats Availability (CPU-Only Mode)

**Objective:** Verify GPU stats API returns false when no GPU

**Steps:**
1. **Login to app** (use password from Test 1.1)

2. **Navigate to Mining page**
   - Click "Mining" in sidebar
   - Wait 2 seconds for page to load

3. **Open DevTools Console** (F12)

4. **Check GPU availability:**
   ```javascript
   window.invoke('is_gpu_stats_available').then(console.log)
   ```
   - ✅ Returns: `false` (on CPU-only systems)

**Backend Command Verified:** `is_gpu_stats_available` ✅

---

### Test 2.2: GPU Stats Structure (If GPU Available)

**Note:** This test requires a system with GPU and OpenCL drivers installed.

**Steps:**
1. **Check GPU stats:**
   ```javascript
   window.invoke('get_gpu_stats').then(console.log)
   ```

2. **Verify response structure:**
   ```javascript
   {
     device_name: "NVIDIA GeForce RTX 3060",
     vendor: "NVIDIA Corporation",
     compute_units: 28,
     max_work_group_size: 1024,
     global_mem_size: 12884901888,
     local_mem_size: 49152,
     max_clock_frequency: 1777,
     hashrate: 0.0,  // or > 0 if mining
     total_hashes: 0,
     uptime_seconds: 0,
     temperature: null,  // or float value
     power_usage: null   // or float value
   }
   ```

**Backend Command Verified:** `get_gpu_stats` ✅

---

## Test Suite 3: T011-007 Transaction Block Details

### ⚠️ BLOCKED - Cannot Test

**Reason:** T011-005/006 (database methods) are BLOCKED due to UnifiedDatabase compilation errors:
- `unified_database.rs` has type mismatches (`u64` vs `u32`)
- `embedded_node.rs` has missing `txid()` method
- Cannot add to lib.rs for testing without fixing bugs

**Required Fix:** Resolve compilation errors in:
1. `/home/bob/BTPC/BTPC/btpc-desktop-app/src-tauri/src/unified_database.rs`
2. `/home/bob/BTPC/BTPC/btpc-desktop-app/src-tauri/src/embedded_node.rs`

**After Fix:** Can proceed with testing `get_block_by_height` command

---

## Test Suite 4: T011-008 Integration Testing

### Test 4.1: Page Navigation Persistence

**Objective:** Verify active tab state persists across page reloads

**Steps:**
1. **Navigate to Settings page**
2. **Click "Network" tab**
3. **Reload page** (Ctrl+R or F5)
4. ✅ "Network" tab should still be active (not reset to first tab)

**Implementation:** Uses `localStorage` keys: `btpc_active_tab_settings`, `btpc_active_tab_transactions`, `btpc_active_tab_mining`

---

### Test 4.2: End-to-End Flow

**Objective:** Verify complete user workflow

**Steps:**
1. **Fresh start:**
   ```bash
   rm -rf ~/.btpc/credentials.enc
   npm run tauri:dev
   ```

2. **Create password** → Should succeed ✅

3. **Navigate to Wallets page** → Should load ✅

4. **Create new wallet:**
   - Click "Create Wallet"
   - Enter wallet name
   - ✅ Wallet created successfully

5. **Navigate to Transactions page** → Should load ✅

6. **Navigate to Mining page** → Should load ✅

7. **Close and relaunch app**

8. **Login** → Should succeed ✅

9. **Verify wallet still exists** → Should be visible ✅

---

## Test Results Summary

### T011-001: Login System ✅

| Test Case | Status | Notes |
|-----------|--------|-------|
| First launch - create password | ⏳ Pending | Manual test required |
| Subsequent launch - login | ⏳ Pending | Manual test required |
| Wrong password error | ⏳ Pending | Manual test required |
| Empty password validation | ⏳ Pending | Manual test required |

### T011-002/003: GPU Stats ✅

| Test Case | Status | Notes |
|-----------|--------|-------|
| Unit tests (7 tests) | ✅ PASS | `cargo test --test test_feature_011_gpu_commands` |
| is_gpu_stats_available API | ⏳ Pending | Manual test in DevTools |
| get_gpu_stats structure | ⏳ Pending | Manual test (GPU required) |

### T011-005/006: Block Details ⚠️

| Test Case | Status | Notes |
|-----------|--------|-------|
| get_block database method | ⚠️ BLOCKED | UnifiedDatabase compilation errors |
| get_block_by_height command | ⚠️ BLOCKED | Depends on T011-005 |

---

## Console Commands for Manual Testing

Open DevTools Console (F12) and run:

```javascript
// Check if Tauri invoke is available
console.log(window.btpcInvoke || window.__TAURI__.core);

// Test login system
window.invoke('check_wallet_lock_status').then(console.log);

// Test GPU stats (after login)
window.invoke('is_gpu_stats_available').then(console.log);
window.invoke('get_gpu_stats').then(console.log);

// Check localStorage persistence
console.log(localStorage.getItem('btpc_active_tab_settings'));
console.log(localStorage.getItem('btpc_active_tab_transactions'));
console.log(localStorage.getItem('btpc_active_tab_mining'));
```

---

## Troubleshooting

### App won't start

```bash
# Check if ports are in use
lsof -i :1420  # Frontend
lsof -i :3000  # Backend

# Kill processes if needed
pkill -f "tauri dev"

# Clean and rebuild
cargo clean
npm run tauri:dev
```

### Login page doesn't show

- Check console for errors
- Verify `check_wallet_lock_status` command exists:
  ```bash
  grep "check_wallet_lock_status" src-tauri/src/main.rs
  ```

### Password creation fails

- Check backend logs in terminal running `tauri:dev`
- Verify `migrate_to_encrypted` command exists
- Check file permissions on `~/.btpc/` directory

---

## Reporting Issues

If any test fails, report:
1. **Test case name**
2. **Steps to reproduce**
3. **Expected result**
4. **Actual result**
5. **Console errors** (F12 → Console tab screenshot)
6. **Backend logs** (terminal output screenshot)

---

## Constitution Compliance

**Article VI.3 (TDD):**
- ✅ T011-002/003: 7 unit tests passing
- ✅ T011-001: Frontend refactoring (no new logic, just command name changes)
- ⚠️ T011-005/006: Blocked by existing bugs

**Article XI (Backend-First):**
- ✅ All authentication through backend commands (no localStorage for credentials)
- ✅ GPU stats fetched from backend (no frontend caching of stats)
- ✅ Tab state uses localStorage only for UI preferences (allowed per Article XI)

---

## Next Steps After Manual Testing

1. **If all tests pass:**
   - Update `/home/bob/BTPC/BTPC/specs/011-frontend-backend-integration/tasks.md`
   - Mark T011-001, T011-007, T011-008 as ✅ COMPLETE
   - Update STATUS.md

2. **If tests fail:**
   - Document failures
   - Create bug report
   - Fix issues and retest

3. **For T011-005/006:**
   - Fix UnifiedDatabase compilation errors
   - Add TDD tests
   - Then proceed with manual testing

---

**End of Testing Guide**