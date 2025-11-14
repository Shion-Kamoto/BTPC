# UI Password Integration Plan

**Date**: 2025-10-18
**Status**: Planning Phase
**Priority**: HIGH (Next implementation task)

---

## Overview

Integrate encrypted wallet password protection into the btpc-desktop-app UI, completing the TDD GREEN implementation with user-facing functionality.

**Goal**: User must enter master password to unlock wallets on app startup.

---

## Current State Analysis

### Existing Flow (Insecure)
```
App Start
    ↓
WalletManager::new() [main.rs:396]
    ↓
load_wallets() [auto-loads plaintext JSON]
    ↓
UI displays wallets immediately
```

**Problem**: No password required, metadata in plaintext

### Target Flow (Secure)
```
App Start
    ↓
WalletManager::new() [checks for encrypted file]
    ↓
If encrypted.dat exists:
    UI shows password modal
    User enters password
    load_wallets_encrypted(password)
    ↓
If plaintext.json exists:
    UI prompts for new master password
    Migrate to encrypted format
    ↓
UI unlocks, displays wallets
```

---

## Implementation Phases

### Phase 1: Backend (Tauri Commands) ✅

**Estimated Time**: 1-2 hours

#### 1.1 Add Global Password State

**File**: `src-tauri/src/main.rs`
**Location**: AppState struct

```rust
pub struct AppState {
    // ... existing fields ...

    /// Wallet lock state (None = locked, Some(password) = unlocked)
    wallet_password: Arc<RwLock<Option<SecurePassword>>>,

    /// Whether wallets are currently locked
    wallets_locked: Arc<RwLock<bool>>,
}
```

**Initialization**:
```rust
// In AppState::new()
wallet_password: Arc::new(RwLock::new(None)),
wallets_locked: Arc::new(RwLock::new(true)),  // Start locked
```

#### 1.2 Modify WalletManager Initialization

**File**: `src-tauri/src/wallet_manager.rs`
**Lines**: 263-290

**Change**: Make auto-loading conditional

```rust
pub fn new(config: WalletManagerConfig, security: SecurityManager) -> BtpcResult<Self> {
    // ... directory creation ...

    let metadata_file = config.wallets_dir.join("wallets_metadata.json");
    let encrypted_file = config.wallets_dir.join("wallets_metadata.dat");

    let mut manager = Self {
        config,
        wallets: HashMap::new(),
        security,
        metadata_file,
    };

    // Auto-load ONLY if no encrypted file exists (backward compatibility)
    if !encrypted_file.exists() {
        manager.load_wallets()?;  // Load plaintext for migration
    }
    // If encrypted file exists, require password (don't auto-load)

    Ok(manager)
}
```

**Impact**:
- Backward compatible (plaintext JSON still auto-loads)
- Encrypted wallets require password
- Migration-friendly

#### 1.3 Create Tauri Commands

**File**: `src-tauri/src/main.rs`
**Location**: After existing commands

```rust
/// Check if wallets are locked (password required)
#[tauri::command]
async fn check_wallet_lock_status(
    state: tauri::State<'_, AppState>
) -> Result<bool, String> {
    let locked = state.wallets_locked.read().await;
    Ok(*locked)
}

/// Unlock wallets with password
#[tauri::command]
async fn unlock_wallets(
    password: String,
    state: tauri::State<'_, AppState>
) -> Result<String, String> {
    use btpc_core::crypto::SecurePassword;

    let secure_password = SecurePassword::new(password);

    // Try to load encrypted wallets
    let mut wallet_manager = state.wallet_manager.lock().await;

    match wallet_manager.load_wallets_encrypted(&secure_password) {
        Ok(_) => {
            // Store password for session
            let mut pw = state.wallet_password.write().await;
            *pw = Some(secure_password);

            // Mark as unlocked
            let mut locked = state.wallets_locked.write().await;
            *locked = false;

            Ok("Wallets unlocked successfully".to_string())
        },
        Err(e) => {
            Err(format!("Failed to unlock wallets: {}", e))
        }
    }
}

/// Lock wallets (clear password from memory)
#[tauri::command]
async fn lock_wallets(
    state: tauri::State<'_, AppState>
) -> Result<(), String> {
    // Clear password
    let mut pw = state.wallet_password.write().await;
    *pw = None;

    // Mark as locked
    let mut locked = state.wallets_locked.write().await;
    *locked = true;

    Ok(())
}

/// Change master password
#[tauri::command]
async fn change_master_password(
    old_password: String,
    new_password: String,
    state: tauri::State<'_, AppState>
) -> Result<String, String> {
    use btpc_core::crypto::SecurePassword;

    let old_pw = SecurePassword::new(old_password);
    let new_pw = SecurePassword::new(new_password);

    let wallet_manager = state.wallet_manager.lock().await;

    // Verify old password by attempting decrypt
    match wallet_manager.load_wallets_encrypted(&old_pw) {
        Ok(_) => {
            // Re-encrypt with new password
            wallet_manager.save_wallets_encrypted(&new_pw)
                .map_err(|e| format!("Failed to save with new password: {}", e))?;

            // Update session password
            let mut pw = state.wallet_password.write().await;
            *pw = Some(new_pw);

            Ok("Master password changed successfully".to_string())
        },
        Err(_) => {
            Err("Incorrect current password".to_string())
        }
    }
}

/// Migrate plaintext wallets to encrypted format
#[tauri::command]
async fn migrate_to_encrypted(
    new_password: String,
    state: tauri::State<'_, AppState>
) -> Result<String, String> {
    use btpc_core::crypto::SecurePassword;

    let secure_password = SecurePassword::new(new_password);
    let wallet_manager = state.wallet_manager.lock().await;

    // Save current wallets (loaded from plaintext) as encrypted
    wallet_manager.save_wallets_encrypted(&secure_password)
        .map_err(|e| format!("Migration failed: {}", e))?;

    // Backup old plaintext file
    let plaintext_path = wallet_manager.metadata_file.clone();
    if plaintext_path.exists() {
        let backup_path = plaintext_path.with_extension("json.bak");
        std::fs::rename(&plaintext_path, &backup_path)
            .map_err(|e| format!("Failed to backup plaintext file: {}", e))?;
    }

    // Store password for session
    let mut pw = state.wallet_password.write().await;
    *pw = Some(secure_password);

    // Mark as unlocked
    let mut locked = state.wallets_locked.write().await;
    *locked = false;

    Ok("Migration completed successfully. Plaintext file backed up.".to_string())
}
```

**Register Commands**:
```rust
// In main() function
.invoke_handler(tauri::generate_handler![
    // ... existing commands ...
    check_wallet_lock_status,
    unlock_wallets,
    lock_wallets,
    change_master_password,
    migrate_to_encrypted,
])
```

---

### Phase 2: Frontend (Password Modal UI) 🔨

**Estimated Time**: 2-3 hours

#### 2.1 Create Password Modal HTML

**File**: `btpc-desktop-app/ui/password-modal.html` (NEW)

```html
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>Unlock Wallets</title>
    <link rel="stylesheet" href="btpc-styles.css">
    <style>
        .modal-overlay {
            position: fixed;
            top: 0;
            left: 0;
            width: 100%;
            height: 100%;
            background: rgba(0, 0, 0, 0.7);
            display: flex;
            align-items: center;
            justify-content: center;
            z-index: 10000;
        }

        .password-modal {
            background: var(--bg-primary, #1a1a1a);
            border: 2px solid var(--border-color, #333);
            border-radius: 12px;
            padding: 32px;
            max-width: 450px;
            width: 90%;
            box-shadow: 0 10px 40px rgba(0, 0, 0, 0.5);
        }

        .modal-header {
            margin-bottom: 24px;
        }

        .modal-title {
            font-size: 24px;
            font-weight: 600;
            color: var(--text-primary, #fff);
            margin-bottom: 8px;
        }

        .modal-subtitle {
            font-size: 14px;
            color: var(--text-secondary, #aaa);
        }

        .password-input-group {
            margin-bottom: 24px;
        }

        .password-input-wrapper {
            position: relative;
        }

        .password-input {
            width: 100%;
            padding: 12px 40px 12px 16px;
            background: var(--bg-secondary, #2a2a2a);
            border: 2px solid var(--border-color, #333);
            border-radius: 8px;
            color: var(--text-primary, #fff);
            font-size: 16px;
            font-family: monospace;
        }

        .password-input:focus {
            outline: none;
            border-color: var(--primary-color, #3b82f6);
        }

        .password-toggle {
            position: absolute;
            right: 12px;
            top: 50%;
            transform: translateY(-50%);
            background: none;
            border: none;
            color: var(--text-secondary, #aaa);
            cursor: pointer;
            padding: 4px;
        }

        .password-toggle:hover {
            color: var(--text-primary, #fff);
        }

        .error-message {
            color: var(--error-color, #ef4444);
            font-size: 14px;
            margin-top: 8px;
            display: none;
        }

        .error-message.show {
            display: block;
        }

        .modal-actions {
            display: flex;
            gap: 12px;
            margin-top: 24px;
        }

        .btn {
            flex: 1;
            padding: 12px 24px;
            border-radius: 8px;
            font-size: 16px;
            font-weight: 500;
            cursor: pointer;
            transition: all 0.2s;
        }

        .btn-primary {
            background: var(--primary-color, #3b82f6);
            color: white;
            border: none;
        }

        .btn-primary:hover {
            background: var(--primary-hover, #2563eb);
        }

        .btn-primary:disabled {
            background: #444;
            cursor: not-allowed;
            opacity: 0.5;
        }

        .btn-secondary {
            background: transparent;
            color: var(--text-secondary, #aaa);
            border: 2px solid var(--border-color, #333);
        }

        .btn-secondary:hover {
            border-color: var(--text-primary, #fff);
            color: var(--text-primary, #fff);
        }

        .loading-spinner {
            display: none;
            width: 20px;
            height: 20px;
            border: 3px solid rgba(255, 255, 255, 0.3);
            border-top-color: white;
            border-radius: 50%;
            animation: spin 0.8s linear infinite;
            margin: 0 auto;
        }

        @keyframes spin {
            to { transform: rotate(360deg); }
        }

        .migration-notice {
            background: var(--warning-bg, #451a03);
            border: 1px solid var(--warning-border, #78350f);
            border-radius: 8px;
            padding: 16px;
            margin-bottom: 24px;
        }

        .migration-notice-title {
            font-weight: 600;
            color: var(--warning-color, #fbbf24);
            margin-bottom: 8px;
        }

        .migration-notice-text {
            font-size: 14px;
            color: var(--text-secondary, #aaa);
        }
    </style>
</head>
<body>
    <div class="modal-overlay" id="passwordModal">
        <div class="password-modal">
            <!-- Migration Notice (shown only for migration) -->
            <div class="migration-notice" id="migrationNotice" style="display: none;">
                <div class="migration-notice-title">⚠️ Migration Required</div>
                <div class="migration-notice-text">
                    Your wallet data will be encrypted with a master password for enhanced security.
                    This is a one-time migration.
                </div>
            </div>

            <div class="modal-header">
                <h2 class="modal-title" id="modalTitle">Unlock Wallets</h2>
                <p class="modal-subtitle" id="modalSubtitle">Enter your master password to access your wallets</p>
            </div>

            <div class="password-input-group">
                <div class="password-input-wrapper">
                    <input
                        type="password"
                        id="passwordInput"
                        class="password-input"
                        placeholder="Enter master password"
                        autocomplete="off"
                    >
                    <button class="password-toggle" id="togglePassword" title="Show password">
                        👁️
                    </button>
                </div>
                <div class="error-message" id="errorMessage"></div>
            </div>

            <div class="modal-actions">
                <button class="btn btn-primary" id="unlockButton">
                    <span id="buttonText">Unlock</span>
                    <div class="loading-spinner" id="loadingSpinner"></div>
                </button>
            </div>
        </div>
    </div>

    <script src="password-modal.js"></script>
</body>
</html>
```

#### 2.2 Create Password Modal JavaScript

**File**: `btpc-desktop-app/ui/password-modal.js` (NEW)

```javascript
const { invoke } = window.__TAURI__.tauri;

// DOM elements
const passwordInput = document.getElementById('passwordInput');
const togglePassword = document.getElementById('togglePassword');
const unlockButton = document.getElementById('unlockButton');
const buttonText = document.getElementById('buttonText');
const loadingSpinner = document.getElementById('loadingSpinner');
const errorMessage = document.getElementById('errorMessage');
const modalTitle = document.getElementById('modalTitle');
const modalSubtitle = document.getElementById('modalSubtitle');
const migrationNotice = document.getElementById('migrationNotice');

let isMigration = false;

// Check if this is a migration scenario
async function checkMigrationStatus() {
    // Check if encrypted file exists
    try {
        const response = await invoke('check_wallet_lock_status');
        // If true (locked), check if we need migration
        // This would need a separate backend call to detect plaintext JSON
        return false;  // Placeholder
    } catch (error) {
        console.error('Error checking migration status:', error);
        return false;
    }
}

// Toggle password visibility
togglePassword.addEventListener('click', () => {
    const type = passwordInput.type === 'password' ? 'text' : 'password';
    passwordInput.type = type;
    togglePassword.textContent = type === 'password' ? '👁️' : '🙈';
});

// Show error message
function showError(message) {
    errorMessage.textContent = message;
    errorMessage.classList.add('show');
    passwordInput.style.borderColor = 'var(--error-color, #ef4444)';
}

// Hide error message
function hideError() {
    errorMessage.classList.remove('show');
    passwordInput.style.borderColor = '';
}

// Set loading state
function setLoading(loading) {
    if (loading) {
        buttonText.style.display = 'none';
        loadingSpinner.style.display = 'block';
        unlockButton.disabled = true;
        passwordInput.disabled = true;
    } else {
        buttonText.style.display = 'block';
        loadingSpinner.style.display = 'none';
        unlockButton.disabled = false;
        passwordInput.disabled = false;
    }
}

// Attempt to unlock wallets
async function attemptUnlock() {
    const password = passwordInput.value.trim();

    if (!password) {
        showError('Please enter a password');
        return;
    }

    hideError();
    setLoading(true);

    try {
        let result;

        if (isMigration) {
            // Migration flow
            result = await invoke('migrate_to_encrypted', { newPassword: password });
        } else {
            // Normal unlock flow
            result = await invoke('unlock_wallets', { password });
        }

        // Success! Redirect to main dashboard
        window.location.href = 'index.html';

    } catch (error) {
        console.error('Unlock failed:', error);
        showError(error || 'Incorrect password. Please try again.');
        setLoading(false);
        passwordInput.value = '';
        passwordInput.focus();
    }
}

// Event listeners
unlockButton.addEventListener('click', attemptUnlock);

passwordInput.addEventListener('keypress', (e) => {
    if (e.key === 'Enter') {
        attemptUnlock();
    }
});

passwordInput.addEventListener('input', () => {
    hideError();
});

// Auto-focus password input
passwordInput.focus();

// Initialize
(async function init() {
    // Check if migration is needed
    // This would call a backend function to detect plaintext JSON
    // For now, assume normal unlock
    isMigration = false;

    if (isMigration) {
        modalTitle.textContent = 'Set Master Password';
        modalSubtitle.textContent = 'Choose a strong password to encrypt your wallet data';
        buttonText.textContent = 'Encrypt & Continue';
        migrationNotice.style.display = 'block';
    }
})();
```

#### 2.3 Integration into App Startup

**File**: `btpc-desktop-app/ui/index.html`
**Location**: After `<script>` tags

```javascript
// Add to existing initialization code
async function checkWalletLockStatus() {
    try {
        const isLocked = await invoke('check_wallet_lock_status');

        if (isLocked) {
            // Redirect to password modal
            window.location.href = 'password-modal.html';
        } else {
            // Wallets unlocked, continue normally
            initializeDashboard();
        }
    } catch (error) {
        console.error('Error checking wallet lock status:', error);
    }
}

// Call on page load
document.addEventListener('DOMContentLoaded', checkWalletLockStatus);
```

---

### Phase 3: Migration & Safety 🛡️

**Estimated Time**: 1 hour

#### 3.1 Auto-Detection

**File**: `src-tauri/src/main.rs`

```rust
/// Detect if migration is needed
#[tauri::command]
async fn needs_migration(
    state: tauri::State<'_, AppState>
) -> Result<bool, String> {
    let wallet_manager = state.wallet_manager.lock().await;

    let plaintext_path = wallet_manager.metadata_file.clone();
    let encrypted_path = wallet_manager.metadata_file.with_extension("dat");

    // Migration needed if plaintext exists but encrypted doesn't
    Ok(plaintext_path.exists() && !encrypted_path.exists())
}
```

#### 3.2 Safe Migration Process

**Steps**:
1. ✅ Detect plaintext JSON
2. ✅ Prompt user for new password
3. ✅ Encrypt and save to `.dat` file
4. ✅ Verify encrypted file loads correctly
5. ✅ Rename plaintext to `.json.bak` (backup)
6. ✅ Delete backup only after user confirmation (manual)

**Rollback**: If anything fails, keep `.json.bak` and log error

---

### Phase 4: Security Features 🔐

**Estimated Time**: 1-2 hours (optional enhancements)

#### 4.1 Auto-Lock After Inactivity

```rust
/// Auto-lock after N minutes of inactivity
#[tauri::command]
async fn start_auto_lock_timer(
    minutes: u64,
    state: tauri::State<'_, AppState>
) -> Result<(), String> {
    // Spawn background task that checks last activity
    // Lock wallets if inactive > minutes

    Ok(())
}
```

#### 4.2 Password Strength Validator

```rust
/// Validate password strength
#[tauri::command]
fn validate_password_strength(password: String) -> Result<PasswordStrength, String> {
    // Check length, complexity, common patterns
    // Return: Weak, Medium, Strong

    Ok(PasswordStrength::Strong)
}
```

**Frontend**: Show strength meter during migration

#### 4.3 Maximum Attempts Protection

```javascript
let failedAttempts = 0;
const MAX_ATTEMPTS = 5;
const LOCKOUT_TIME = 300; // 5 minutes in seconds

async function attemptUnlock() {
    if (failedAttempts >= MAX_ATTEMPTS) {
        showError(`Too many failed attempts. Try again in ${LOCKOUT_TIME/60} minutes.`);
        return;
    }

    try {
        await invoke('unlock_wallets', { password });
        failedAttempts = 0; // Reset on success
    } catch (error) {
        failedAttempts++;
        showError(`Incorrect password (${failedAttempts}/${MAX_ATTEMPTS} attempts)`);
    }
}
```

---

## Testing Plan

### Manual Testing Checklist

**New User (Fresh Install)**:
- [ ] App starts without password prompt
- [ ] Create first wallet
- [ ] App continues to work normally (no encryption yet)
- [ ] Manually trigger migration via settings

**Existing User (Plaintext JSON)**:
- [ ] App detects plaintext wallets
- [ ] Shows migration prompt on startup
- [ ] User enters new master password
- [ ] Migration completes successfully
- [ ] `.json.bak` backup created
- [ ] Restart app - password prompt appears
- [ ] Correct password unlocks wallets
- [ ] Wrong password shows error

**Encrypted User (Normal Flow)**:
- [ ] App shows password prompt on startup
- [ ] Correct password unlocks wallets
- [ ] Wrong password shows error
- [ ] Multiple wrong attempts handled correctly
- [ ] Lock wallets from settings
- [ ] Password prompt reappears
- [ ] Change master password works
- [ ] Old password still in memory during session

**Edge Cases**:
- [ ] Network offline (wallets still unlock)
- [ ] Corrupted encrypted file (fallback to backup)
- [ ] Empty password rejected
- [ ] Very long password (1000+ chars)
- [ ] Special characters in password
- [ ] Unicode characters in password
- [ ] Password with spaces

---

## File Summary

### New Files (6 files)
1. `ui/password-modal.html` - Password prompt UI
2. `ui/password-modal.js` - Password prompt logic
3. `MD/UI_PASSWORD_INTEGRATION_PLAN.md` - This document

### Modified Files (2 files)
1. `src-tauri/src/main.rs` - Add Tauri commands, AppState fields
2. `src-tauri/src/wallet_manager.rs` - Conditional auto-loading
3. `ui/index.html` - Add lock status check on startup

**Total Estimated Lines**: ~500 lines (300 frontend, 200 backend)

---

## Timeline

| Phase | Task | Time | Priority |
|-------|------|------|----------|
| 1 | Backend Tauri commands | 1-2h | HIGH |
| 2 | Frontend password modal | 2-3h | HIGH |
| 3 | Migration & safety | 1h | HIGH |
| 4 | Security enhancements | 1-2h | MEDIUM |
| **Total** | **Full implementation** | **5-8h** | - |

**Minimum Viable Product (MVP)**: Phases 1-3 (4-6 hours)

---

## Risk Mitigation

### Risk 1: User Forgets Password ⚠️

**Mitigation**:
- Document seed phrase recovery flow
- Add "Forgot Password?" link (requires seed phrase)
- Warn user during migration about password importance

### Risk 2: Migration Fails 🛡️

**Mitigation**:
- Always backup plaintext as `.json.bak`
- Verify encrypted file loads before deleting plaintext
- Log all migration steps
- Provide manual rollback instructions

### Risk 3: Password in Memory 🔒

**Mitigation**:
- Use `SecurePassword` with `Zeroize` on drop
- Auto-lock after inactivity (optional)
- Clear password on app close
- Don't log/print passwords

### Risk 4: UI/Backend Desync ⚙️

**Mitigation**:
- Single source of truth (`wallets_locked` state)
- Check lock status on every wallet operation
- Redirect to password modal if locked
- Tauri events for lock state changes

---

## Success Criteria

### Must Have ✅
- [ ] Password prompt on app startup (if encrypted)
- [ ] Correct password unlocks wallets
- [ ] Wrong password shows clear error
- [ ] Migration from plaintext works
- [ ] Backup created during migration
- [ ] Lock/unlock functionality works
- [ ] Zero compilation errors
- [ ] Professional UI matching BTPC theme

### Nice to Have 🌟
- [ ] Auto-lock after inactivity
- [ ] Password strength meter
- [ ] Maximum attempts protection
- [ ] Biometric unlock (future)
- [ ] Password recovery via seed phrase

---

## Next Actions

1. **Review this plan** with user for approval
2. **Begin Phase 1**: Implement Tauri backend commands
3. **Test backend**: Verify unlock/lock/migrate commands work
4. **Begin Phase 2**: Create password modal UI
5. **Integration testing**: Full end-to-end flow
6. **Documentation**: Update user guide

**Estimated Start**: 2025-10-18 (after plan approval)
**Estimated Completion**: 2025-10-18 to 2025-10-19 (1-2 days)

---

**Status**: **READY FOR IMPLEMENTATION** ✅

---

**Document Owner**: Claude Code
**Last Updated**: 2025-10-18
**Version**: 1.0