# Authentication System - Manual Testing Guide

**Feature**: 006 - Application-Level Login/Logout System
**Date**: 2025-10-28
**Status**: Ready for Testing

## Overview

This guide provides step-by-step instructions for manually testing the complete authentication system implementation.

## Prerequisites

1. **Clean State**: Remove existing credentials file:
   ```bash
   rm ~/.btpc/credentials.enc
   ```

2. **Build Application**:
   ```bash
   cd /home/bob/BTPC/BTPC/btpc-desktop-app
   cargo build --release
   npm run tauri:dev
   ```

3. **Open Browser Console**: Press F12 to view event logs and debugging info

## Test Scenarios

### Test 1: First Launch - Create Master Password

**Objective**: Verify master password creation on first launch

**Steps**:
1. Launch the application (ensure `~/.btpc/credentials.enc` does not exist)
2. You should see the login page with title "Create Master Password"
3. Enter password: `TestPassword123` (8+ characters)
4. Enter confirm password: `TestPassword123`
5. Click "Create Master Password"

**Expected Results**:
- ✅ Button shows loading spinner with "Creating..." text
- ✅ Success toast notification appears: "Master password created successfully!"
- ✅ Backend event log shows: `[Event Manager] session:login event received`
- ✅ Application redirects to dashboard (index.html) after ~1.5 seconds
- ✅ Credentials file created at `~/.btpc/credentials.enc` with 0600 permissions
- ✅ Dashboard shows logout button in top-right corner

**Verify File Creation**:
```bash
ls -la ~/.btpc/credentials.enc
# Should show: -rw------- (0600 permissions)
```

### Test 2: Password Validation - Too Short

**Objective**: Verify password length validation

**Steps**:
1. Remove credentials file: `rm ~/.btpc/credentials.enc`
2. Relaunch application
3. Enter password: `short` (< 8 characters)
4. Enter confirm password: `short`
5. Click "Create Master Password"

**Expected Results**:
- ✅ Error message displayed: "Password must be at least 8 characters long"
- ✅ No credentials file created
- ✅ Remains on login page

### Test 3: Password Validation - Mismatch

**Objective**: Verify password confirmation validation

**Steps**:
1. Ensure on create password page (no credentials file)
2. Enter password: `TestPassword123`
3. Enter confirm password: `DifferentPassword456`
4. Click "Create Master Password"

**Expected Results**:
- ✅ Error message displayed: "Passwords do not match"
- ✅ No credentials file created
- ✅ Remains on login page

### Test 4: Navigation Guard - Unauthenticated Access

**Objective**: Verify protected pages redirect unauthenticated users

**Steps**:
1. Close application (logout if logged in)
2. Relaunch application
3. Login page should appear
4. Try to manually navigate to: `http://localhost:1420/index.html`

**Expected Results**:
- ✅ Console log: `[Navigation Guard] Authentication check: NOT AUTHENTICATED`
- ✅ Immediately redirected back to login.html
- ✅ Cannot access dashboard without authentication

### Test 5: Subsequent Launch - Login

**Objective**: Verify login with existing master password

**Steps**:
1. Ensure credentials file exists from Test 1
2. Launch application
3. You should see the login page with title "Welcome Back"
4. Enter password: `TestPassword123`
5. Click "Login"

**Expected Results**:
- ✅ Button shows loading spinner with "Logging in..." text
- ✅ Success toast notification appears: "Login successful!"
- ✅ Backend event log shows: `[Event Manager] session:login event received`
- ✅ Application redirects to dashboard (index.html) after ~1 second
- ✅ Dashboard shows logout button

### Test 6: Login - Wrong Password

**Objective**: Verify failed login with incorrect password

**Steps**:
1. Ensure logged out (on login page)
2. Enter password: `WrongPassword456`
3. Click "Login"

**Expected Results**:
- ✅ Error message displayed: "Authentication failed. Please check your password."
- ✅ Password field cleared
- ✅ Remains on login page
- ✅ Console log shows consistent error message (timing attack prevention)

### Test 7: Navigation Guard - Authenticated Access

**Objective**: Verify authenticated users can access all pages

**Steps**:
1. Login successfully (see Test 5)
2. On dashboard, click each navigation link:
   - Dashboard (index.html)
   - Wallet Manager (wallet-manager.html)
   - Transactions (transactions.html)
   - Mining (mining.html)
   - Node (node.html)
   - Settings (settings.html)

**Expected Results**:
- ✅ All pages load successfully
- ✅ Console log: `[Navigation Guard] Authentication check: AUTHENTICATED`
- ✅ Each page shows logout button in top-right corner
- ✅ No redirects to login page

### Test 8: Logout Functionality

**Objective**: Verify logout terminates session and redirects to login

**Steps**:
1. Login successfully (dashboard visible)
2. Click the "Logout" button in top-right corner

**Expected Results**:
- ✅ Button shows loading spinner with "Logging out..." text
- ✅ Success toast notification appears: "Logged out successfully"
- ✅ Backend event log shows: `[Event Manager] session:logout event received`
- ✅ Redirected to login.html after ~0.5 seconds
- ✅ Cannot access protected pages without logging in again

### Test 9: Logout from Multiple Pages

**Objective**: Verify logout works from all authenticated pages

**Steps**:
1. Login successfully
2. Navigate to each page and click logout:
   - From Wallet Manager → Logout → Verify redirect to login.html
   - Login again → Navigate to Transactions → Logout → Verify redirect
   - Login again → Navigate to Mining → Logout → Verify redirect
   - Login again → Navigate to Node → Logout → Verify redirect
   - Login again → Navigate to Settings → Logout → Verify redirect

**Expected Results**:
- ✅ Logout works from all pages
- ✅ Toast notification appears on each logout
- ✅ Consistent redirect behavior

### Test 10: Session Persistence

**Objective**: Verify session does NOT persist across app restarts (per Article XI)

**Steps**:
1. Login successfully
2. Close the application (Alt+F4 or close window)
3. Relaunch the application

**Expected Results**:
- ✅ Application shows login page (not dashboard)
- ✅ Must enter password again to access protected pages
- ✅ Session state is NOT persisted to localStorage (Article XI compliance)

### Test 11: Event System - Console Logs

**Objective**: Verify event system logs are present for debugging

**Steps**:
1. Open browser console (F12)
2. Login successfully
3. Navigate to different pages
4. Logout

**Expected Results** (in console):
```
[Event Manager] Event listeners initialized successfully
[Navigation Guard] Authentication check: NOT AUTHENTICATED
[Event Manager] session:login event received: {session_token: "...", timestamp: ...}
[Navigation Guard] Authentication check: AUTHENTICATED
[Event Manager] session:logout event received: {timestamp: ...}
```

### Test 12: Article XI Compliance - Backend First

**Objective**: Verify backend is single source of truth for authentication state

**Steps**:
1. Login successfully
2. Open browser console
3. Run: `localStorage.getItem('btpc_authenticated')`
4. Run: `sessionStorage.getItem('btpc_authenticated')`

**Expected Results**:
- ✅ Both return `null` (authentication state NOT stored in frontend)
- ✅ Only backend `check_session` command determines authentication state
- ✅ Article XI Section 11.5 compliance verified

### Test 13: Cryptography - File Encryption

**Objective**: Verify credentials file is properly encrypted

**Steps**:
1. Create master password: `TestPassword123`
2. View credentials file content:
   ```bash
   xxd ~/.btpc/credentials.enc | head -20
   ```

**Expected Results**:
- ✅ File content is binary (not plaintext)
- ✅ Password is NOT visible in file
- ✅ File contains encrypted data + AES-GCM tag + nonce + salt
- ✅ Cannot recover password without correct master password

### Test 14: Performance - check_session Speed

**Objective**: Verify navigation guard performance (<50ms per NFR-006)

**Steps**:
1. Login successfully
2. Open browser console
3. Run:
   ```javascript
   const start = performance.now();
   await window.__TAURI__.core.invoke('check_session');
   const end = performance.now();
   console.log(`check_session took ${end - start}ms`);
   ```

**Expected Results**:
- ✅ Response time < 50ms
- ✅ No noticeable UI lag during page navigation
- ✅ Performance requirement NFR-006 met

## Success Criteria

All 14 test scenarios must pass for the authentication system to be considered production-ready.

**Checklist**:
- [ ] Test 1: First launch password creation works
- [ ] Test 2: Password too short validation works
- [ ] Test 3: Password mismatch validation works
- [ ] Test 4: Unauthenticated users redirected to login
- [ ] Test 5: Login with correct password works
- [ ] Test 6: Login with wrong password fails gracefully
- [ ] Test 7: Authenticated users can access all pages
- [ ] Test 8: Logout functionality works
- [ ] Test 9: Logout works from all pages
- [ ] Test 10: Session does NOT persist across restarts
- [ ] Test 11: Event system logs are present
- [ ] Test 12: Article XI compliance (no localStorage auth state)
- [ ] Test 13: Credentials file is encrypted
- [ ] Test 14: check_session performance < 50ms

## Troubleshooting

### Application Won't Start
```bash
# Check for existing processes
pkill -9 -f btpc-desktop-app
# Rebuild
cd /home/bob/BTPC/BTPC/btpc-desktop-app
cargo clean
cargo build --release
npm run tauri:dev
```

### Console Shows "Tauri API not available"
- Ensure btpc-tauri-context.js is loaded before other scripts
- Check browser console for script loading errors
- Verify npm run tauri:dev is running (not just serving HTML)

### Login Button Does Nothing
- Check browser console for JavaScript errors
- Verify btpc-tauri-context.js loaded successfully
- Ensure backend commands are registered in main.rs

### Credentials File Permissions Issue (Unix)
```bash
# Verify 0600 permissions
ls -la ~/.btpc/credentials.enc
# Should show: -rw-------

# If incorrect, fix:
chmod 600 ~/.btpc/credentials.enc
```

## Next Steps After Testing

Once all 14 tests pass:
1. Update auth_contract_test.rs to convert from RED to GREEN phase
2. Run cargo test to verify all tests pass
3. Complete Phase 3.5: Polish & Documentation
4. Create final documentation and handoff notes