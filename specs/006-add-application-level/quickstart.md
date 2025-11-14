# Quickstart: Testing Authentication Flow

**Feature**: 006-add-application-level
**Date**: 2025-10-28

## Prerequisites

- Rust 1.75+ installed
- btpc-desktop-app project cloned
- Development environment set up

## Quick Test Commands

### 1. Run Contract Tests

```bash
cd btpc-desktop-app/src-tauri
cargo test auth_contract_test -- --nocapture
```

**Expected**: All tests FAIL (RED phase - no implementation yet)

### 2. Run Integration Tests

```bash
cd btpc-desktop-app/src-tauri
cargo test auth_integration_test -- --nocapture
```

**Expected**: Test creates password, logs in, logs out successfully

### 3. Manual UI Test - First Launch

```bash
cd btpc-desktop-app
npm run tauri:dev
```

**Steps**:
1. Application opens to password creation screen (login.html)
2. Enter password "testpass123" in both fields
3. Click "Create Master Password"
4. Dashboard (index.html) loads with logout button visible
5. ✅ PASS if: redirected to dashboard, logout button present

###  4. Manual UI Test - Subsequent Launch

```bash
# Kill app (Ctrl+C), restart
npm run tauri:dev
```

**Steps**:
1. Application opens to login screen (login.html displays login form)
2. Enter password "testpass123"
3. Click "Login"
4. Dashboard (index.html) loads
5. ✅ PASS if: redirected to dashboard after correct password

### 5. Manual UI Test - Wrong Password

**Steps** (from login screen):
1. Enter password "wrongpassword"
2. Click "Login"
3. ✅ PASS if: error message "Authentication failed" displayed
4. ✅ PASS if: stays on login screen (no redirect)

### 6. Manual UI Test - Logout

**Steps** (from any authenticated page):
1. Click logout button (unlock icon, top-right)
2. ✅ PASS if: redirected to login screen immediately
3. ✅ PASS if: session:logout event fired (check DevTools console)

### 7. Manual UI Test - Navigation Guard

**Steps** (from login screen, NOT logged in):
1. Manually type URL: http://localhost:1420/wallet-manager.html
2. Press Enter
3. ✅ PASS if: redirected back to login screen
4. ✅ PASS if: error message "Not authenticated" shown

### 8. Manual UI Test - Cross-Page Logout

**Steps** (logged in, two browser tabs open):
1. Tab 1: Dashboard (index.html)
2. Tab 2: Wallet Manager (wallet-manager.html)
3. In Tab 1: Click logout
4. ✅ PASS if: Tab 2 also redirects to login (event propagation)

### 9. Performance Test - Login Speed

```bash
# In DevTools console:
console.time('login');
await invoke('login', { password: 'testpass123' });
console.timeEnd('login');
```

✅ PASS if: <2000ms (per NFR-006)

### 10. Performance Test - Logout Speed

```bash
# In DevTools console:
console.time('logout');
await invoke('logout');
console.timeEnd('logout');
```

✅ PASS if: <100ms (per NFR-007)

---

## Validation Checklist

### Functional Requirements (from spec.md)

- [  ] FR-001: Password creation screen on first launch
- [ ] FR-002: Minimum 8 character password enforced
- [ ] FR-003: Password confirmation required (must match)
- [ ] FR-004: Argon2id + AES-256-GCM encryption used
- [ ] FR-005: Credentials stored in ~/.btpc/credentials.enc
- [ ] FR-006: Login screen on subsequent launches
- [ ] FR-007: Constant-time password validation
- [ ] FR-008: Session established on successful login
- [ ] FR-009: All features blocked when not logged in
- [ ] FR-010: Logout button visible on all pages
- [ ] FR-011: Session cleared completely on logout
- [ ] FR-012: Redirects to login after logout
- [ ] FR-013: Navigation guard prevents unauthorized access
- [ ] FR-014: Session persists across page navigation
- [ ] FR-015: Master password independent from wallet passwords
- [ ] FR-019-023: Dark theme with professional icons
- [ ] FR-024-027: Error messages for all failure cases

### Non-Functional Requirements

- [ ] NFR-001: Argon2id parameters (64MB, 3 iter, 4 par)
- [ ] NFR-002: Cryptographically secure salt (16+ bytes)
- [ ] NFR-003: Constant-time password comparison
- [ ] NFR-004: No password logging
- [ ] NFR-006: Login < 2s
- [ ] NFR-007: Logout < 100ms
- [ ] NFR-008: Navigation guard < 50ms
- [ ] NFR-011: Password visibility toggle present

### Article XI Compliance

- [ ] Backend Arc<RwLock> is single source of truth
- [ ] Frontend displays only, never authoritative
- [ ] Backend-first validation (login/logout)
- [ ] Event-driven UI updates (session:login, session:logout)
- [ ] Event listeners cleaned up on page unload
- [ ] No localStorage for authentication state

---

## Troubleshooting

### Issue: credentials.enc file not found

**Cause**: File deleted or never created
**Fix**: Delete ~/.btpc/credentials.enc and restart app (first launch flow)

### Issue: Login always fails with correct password

**Cause**: Corrupted credentials file
**Fix**: Delete ~/.btpc/credentials.enc and recreate password

### Issue: Logout button not visible

**Cause**: Missing icon or CSS issue
**Fix**: Check btpc-desktop-app/ui/src/assets/icons-svg/unlock.svg exists

### Issue: Navigation guard not working

**Cause**: check_session() not called before page load
**Fix**: Verify DOMContentLoaded handler calls invoke('check_session')

### Issue: Cross-page logout not propagating

**Cause**: Event listeners not registered
**Fix**: Verify btpc-event-manager.js subscribes to session:logout

---

## Success Criteria

✅ **Feature COMPLETE** when:
1. All contract tests pass (10/10)
2. All integration tests pass
3. All manual UI tests pass (8/8)
4. All FR/NFR checklist items checked
5. Article XI compliance verified
6. Performance targets met (login<2s, logout<100ms, guard<50ms)

**Estimated Test Time**: 30 minutes for full validation

---

## Status

✅ **COMPLETE** - Quickstart guide ready for implementation validation.