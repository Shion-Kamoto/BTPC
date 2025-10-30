# BTPC Desktop App - Rebuild Instructions

**Issue**: "state not managed for field `session` on command `create_master_password`"

**Cause**: Cached build artifacts from before the authentication system was implemented.

**Solution**: Clean rebuild with latest code.

## Quick Rebuild (Recommended)

```bash
# 1. Navigate to project root
cd /home/bob/BTPC/BTPC/btpc-desktop-app

# 2. Kill any running processes
pkill -9 -f btpc-desktop-app || true

# 3. Clean all build artifacts
cargo clean

# 4. Rebuild in release mode
cargo build --release

# 5. Launch the application
npm run tauri:dev
```

## Verification Checklist

After rebuilding, verify the following:

✅ **Backend Module Exports** (`src-tauri/src/lib.rs`):
```rust
pub mod auth_commands;
pub mod auth_crypto;
pub mod auth_state;
```

✅ **SessionState Managed** (`src-tauri/src/main.rs`):
```rust
let auth_session = Arc::new(RwLock::new(auth_state::SessionState::new()));
// ...
.manage(auth_session)
```

✅ **Commands Registered** (`src-tauri/src/main.rs`):
```rust
invoke_handler![
    // ... other commands ...
    auth_commands::has_master_password,
    auth_commands::create_master_password,
    auth_commands::login,
    auth_commands::logout,
    auth_commands::check_session
]
```

## Testing The Build

### 1. Verify Compilation
```bash
cd /home/bob/BTPC/BTPC/btpc-desktop-app
cargo check
# Should complete without errors
```

### 2. Run Auth Tests
```bash
cargo test --test auth_contract_test
# All 15 tests should pass
```

###  3. Test Login Flow
```bash
# Remove existing credentials
rm ~/.btpc/credentials.enc 2>/dev/null || true

# Launch app
npm run tauri:dev

# Expected behavior:
# - Login page appears (create master password form)
# - Can create password (8+ characters)
# - Redirects to dashboard after creation
# - Shows logout button
```

## Common Issues

### Issue: "Module not found" errors during compile
**Solution**:
```bash
# Ensure lib.rs exposes auth modules
grep "pub mod auth" src-tauri/src/lib.rs
# Should show:
# pub mod auth_commands;
# pub mod auth_crypto;
# pub mod auth_state;
```

### Issue: "state not managed" error persists
**Solution**:
```bash
# Full clean rebuild
cd /home/bob/BTPC/BTPC/btpc-desktop-app
cargo clean
rm -rf target/
rm -rf node_modules/.tauri/
npm run tauri:dev
```

### Issue: Login page doesn't appear
**Solution**:
```bash
# Check console for errors
# Ensure btpc-navigation-guard.js is loaded
# Verify check_session command is registered
cargo build --release && npm run tauri:dev
```

### Issue: App crashes on startup
**Solution**:
```bash
# Check for conflicting processes
pkill -9 -f btpc-desktop-app
pkill -9 -f "tauri dev"

# Remove lock files
rm ~/.btpc/*.lock 2>/dev/null || true

# Rebuild and launch
cargo clean && npm run tauri:dev
```

## Development Mode vs Production

**Development** (`npm run tauri:dev`):
- Hot reload enabled
- Console debugging available
- Slower startup

**Production** (`npm run tauri:build`):
- Optimized binary
- No console
- Faster performance

For testing authentication, use **development mode** for easier debugging.

## File Locations

**Backend**:
- Commands: `src-tauri/src/auth_commands.rs`
- Crypto: `src-tauri/src/auth_crypto.rs`
- State: `src-tauri/src/auth_state.rs`
- Main: `src-tauri/src/main.rs`
- Lib: `src-tauri/src/lib.rs`

**Frontend**:
- Login page: `ui/login.html`
- Navigation guard: `ui/btpc-navigation-guard.js`
- Event listeners: `ui/btpc-event-listeners.js`
- Logout button: `ui/btpc-logout.js`

**Tests**:
- Contract tests: `src-tauri/tests/auth_contract_test.rs`
- Crypto tests: `src-tauri/tests/auth_crypto_test.rs`
- Integration tests: `src-tauri/tests/auth_integration_test.rs`

**Documentation**:
- Testing guide: `AUTH_TESTING_GUIDE.md`
- Implementation summary: `AUTH_IMPLEMENTATION_SUMMARY.md`

## Build Verification

After rebuilding, run the full test suite:

```bash
# 1. Backend compilation
cargo check
# Expected: No errors

# 2. Auth tests
cargo test --test auth_contract_test
# Expected: 15 tests passed

# 3. Crypto tests
cargo test --test auth_crypto_test
# Expected: All tests passed

# 4. Launch app
npm run tauri:dev
# Expected: Login page appears
```

## Success Indicators

✅ **Compilation succeeds** without "state not managed" error
✅ **All 15 contract tests pass**
✅ **Login page appears** on first launch
✅ **Can create master password** (8+ characters)
✅ **Redirects to dashboard** after successful login
✅ **Logout button visible** on all authenticated pages
✅ **Navigation guard works** (redirects when unauthenticated)

## Still Having Issues?

If problems persist after following this guide:

1. Check the detailed testing guide: `AUTH_TESTING_GUIDE.md`
2. Review implementation summary: `AUTH_IMPLEMENTATION_SUMMARY.md`
3. Verify all files listed in "File Locations" section exist
4. Run: `cargo clean && cargo build --release && npm run tauri:dev`

## Next Steps

Once the app launches successfully:

1. Follow `AUTH_TESTING_GUIDE.md` for complete manual testing
2. Test all 14 scenarios in the testing guide
3. Verify Article XI compliance
4. Check performance (<50ms for check_session)

---

**Last Updated**: 2025-10-28
**Feature**: 006 - Application-Level Login/Logout System
**Status**: Implementation Complete ✅