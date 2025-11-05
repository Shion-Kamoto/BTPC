# Unused Files and Directories Analysis
**Generated**: 2025-10-10
**Project**: btpc-desktop-app
**Location**: `/home/bob/BTPC/BTPC/btpc-desktop-app`

---

## Summary

This document identifies all files and directories that are not actively used in the btpc-desktop-app project and can be safely removed to reduce clutter and disk usage.

---

## 🗑️ Files/Directories to Remove

### 1. Build Artifacts (LARGE - 23GB)
**Location**: `src-tauri/target/`
**Size**: ~23 GB
**Status**: ❌ **CAN BE DELETED**
**Reason**: Build artifacts that can be regenerated with `cargo build`
**Command**:
```bash
cd /home/bob/BTPC/BTPC/btpc-desktop-app/src-tauri
cargo clean
```

---

### 2. Deprecated UI Files
**Status**: ❌ **CAN BE DELETED**

| File | Reason |
|------|--------|
| `ui/explorer.html.deprecated` | Replaced by newer explorer implementation |
| `ui/login.html.deprecated` | Login functionality integrated into main app |
| `ui/index.html.backup` | Backup of old index.html - no longer needed |
| `ui/wallet-manager.html.backup` | Backup of old wallet manager - no longer needed |

**Command**:
```bash
rm ui/*.deprecated ui/*.backup
```

---

### 3. Test/Development Files (Temporary)

#### Test Scripts
- **`test_bip39_validation.rs`** (root directory)
  - Status: ❌ Temporary test file, never compiled
  - Location: Root directory (should be in examples/ or tests/)
  - Reason: Created for testing, not part of build

- **`test_mining_ui.sh`** (root directory)
  - Status: ⚠️ **KEEP IF USED FOR TESTING**
  - Purpose: Testing script for mining UI
  - Size: ~3KB

- **`reassign_utxos.sh`** (root directory)
  - Status: ⚠️ **KEEP IF USED FOR MAINTENANCE**
  - Purpose: UTXO reassignment utility
  - Size: ~2.8KB

---

### 4. Session/Status Documentation (Outdated)

These are historical session summaries and can be moved to a `/docs/archive/` directory or deleted:

**Status Documentation** (Can consolidate or archive):
```
API_MISMATCH_FIXES_2025-10-07.md
BALANCE_CACHE_SYNC_FIX_2025-10-07.md
COMPREHENSIVE_API_MISMATCH_FIXES_2025-10-07.md
COMPREHENSIVE_TEST_RESULTS_2025-10-07.md
COMPREHENSIVE_UI_AUDIT_2025-10-06.md
DESKTOP_APP_STATUS.md
DIRECTORY_CLEANUP.md
END_TO_END_TEST_RESULTS_2025-10-07.md
EVENT_SYSTEM_IMPLEMENTATION.md
INTEGRATION_STATUS.md
LOGIN_PAGE_UPDATE.md
LOW_PRIORITY_ENHANCEMENTS_2025-10-06.md
MINING_STATUS_AUDIT_2025-10-07.md
MINING_STATUS_ENHANCEMENT_2025-10-07.md
NODE_AUTOSTART_FIX.md
NODE_FIX_PROGRESS.md
NODE_FIX_STATUS.md
NODE_PERSISTENCE_FIX_PLAN.md
NODE_STABILITY_FIX.md
NODE_STATUS_FIX_2025-10-07.md
PERSISTENCE_FIX_SUMMARY.md
QR_CODE_STATUS.md
QUICK_FIX_PLAN.md
SESSION_SUMMARY.md
SOLID_NODE_FIX_PLAN.md
STORAGE_PERSISTENCE_GUIDE.md
UI_AUDIT.md
UI_CURRENT_STATE_2025-10-07.md
UI_ELEMENT_AUDIT.md
UI_ENHANCEMENT_SUMMARY_2025-10-07.md
UI_FIXES_APPLIED_2025-10-06.md
UI_REDESIGN_SUMMARY.md
UI_REFRESH_FIX_2025-10-06.md
UI_SESSION_SUMMARY.md
UPDATE_MANAGER_ROLLOUT.md
```

**Recommendation**:
- **KEEP**: `ARCHITECTURE.md`, `QUICKSTART.md`, `QUICK_TEST_GUIDE.md`, `TROUBLESHOOTING.md`
- **ARCHIVE**: All dated session summaries (move to `docs/archive/`)
- **DELETE**: Duplicate/superseded status files

---

### 5. Unused Test Infrastructure

#### Cypress E2E Testing (Not Configured)
**Status**: ❌ **PARTIALLY UNUSED**

Files:
- `cypress.config.js` - Cypress configuration
- `cypress/` directory (if it exists - may be hidden)
- `.eslintrc.js` - ESLint config for Cypress

**Issue**: Cypress is in `package.json` but may not have test files created
**Recommendation**:
- If no E2E tests exist, remove Cypress dependencies
- If planning to use it, keep the configuration

#### Jest Testing (Configured but Limited)
**Status**: ⚠️ **KEEP - ACTIVELY CONFIGURED**

Files:
- `jest.config.js` - Jest configuration
- `tests/setup.js` - Jest setup
- `tests/ui.test.js` - UI tests

**Recommendation**: KEEP - This is actively configured for UI testing

---

### 6. Nested/Duplicate Directories

#### `src-tauri/src-tauri/`
**Status**: ❌ **APPEARS UNUSED**
**Contents**: Only `examples/` subdirectory
**Issue**: Unusual nested structure - likely created by mistake
**Recommendation**: Verify it's not referenced, then delete

#### `examples/` (root level)
**Status**: ⚠️ **VERIFY BEFORE DELETING**
**Contents**: `test_bip39.rs` - Created during development
**Issue**: Not in `Cargo.toml` examples section
**Recommendation**:
- If useful, move to `src-tauri/examples/`
- Otherwise delete

---

### 7. Hidden/Config Files to Review

- **`.claude/settings.local.json`** - KEEP (Claude Code settings)
- **`.github/workflows/`** - KEEP IF using GitHub Actions, otherwise DELETE
- **`node_modules/`** - KEEP (required dependencies, can regenerate)

---

## ✅ Files to KEEP

### Essential Configuration
```
src-tauri/Cargo.toml          # Rust project config
src-tauri/tauri.conf.json     # Tauri app config
src-tauri/build.rs            # Tauri build script
src-tauri/deny.toml           # Cargo deny config
package.json                  # NPM dependencies
package-lock.json             # NPM lock file
```

### Essential Documentation
```
ARCHITECTURE.md               # Architecture overview
QUICKSTART.md                 # Getting started guide
QUICK_TEST_GUIDE.md          # Testing guide
TROUBLESHOOTING.md           # Troubleshooting help
```

### Source Code
```
src-tauri/src/               # All Rust source code
ui/                          # All HTML/CSS/JS (except .deprecated/.backup)
```

### Active Testing
```
src-tauri/tests/integration_tests.rs
src-tauri/benches/performance_benchmarks.rs
tests/setup.js
tests/ui.test.js
jest.config.js
```

---

## 📋 Cleanup Commands

### Quick Cleanup (Safe)
```bash
cd /home/bob/BTPC/BTPC/btpc-desktop-app

# Remove build artifacts (saves 23GB!)
cd src-tauri && cargo clean && cd ..

# Remove deprecated UI files
rm -f ui/*.deprecated ui/*.backup

# Remove temporary test file from root
rm -f test_bip39_validation.rs

# Optional: Archive old documentation
mkdir -p docs/archive
mv *_2025-10-*.md docs/archive/
mv *FIX*.md *STATUS*.md *SUMMARY*.md docs/archive/ 2>/dev/null
```

### Archive Documentation (Recommended)
```bash
cd /home/bob/BTPC/BTPC/btpc-desktop-app

# Create archive directory
mkdir -p docs/archive

# Move dated session documents
mv API_MISMATCH_FIXES_2025-10-07.md docs/archive/
mv BALANCE_CACHE_SYNC_FIX_2025-10-07.md docs/archive/
mv COMPREHENSIVE_API_MISMATCH_FIXES_2025-10-07.md docs/archive/
mv COMPREHENSIVE_TEST_RESULTS_2025-10-07.md docs/archive/
mv COMPREHENSIVE_UI_AUDIT_2025-10-06.md docs/archive/
mv END_TO_END_TEST_RESULTS_2025-10-07.md docs/archive/
mv LOW_PRIORITY_ENHANCEMENTS_2025-10-06.md docs/archive/
mv MINING_STATUS_AUDIT_2025-10-07.md docs/archive/
mv MINING_STATUS_ENHANCEMENT_2025-10-07.md docs/archive/
mv NODE_STATUS_FIX_2025-10-07.md docs/archive/
mv UI_CURRENT_STATE_2025-10-07.md docs/archive/
mv UI_ENHANCEMENT_SUMMARY_2025-10-07.md docs/archive/
mv UI_FIXES_APPLIED_2025-10-06.md docs/archive/

# Move fix/plan documents
mv DIRECTORY_CLEANUP.md docs/archive/
mv NODE_AUTOSTART_FIX.md docs/archive/
mv NODE_FIX_PROGRESS.md docs/archive/
mv NODE_FIX_STATUS.md docs/archive/
mv NODE_PERSISTENCE_FIX_PLAN.md docs/archive/
mv NODE_STABILITY_FIX.md docs/archive/
mv PERSISTENCE_FIX_SUMMARY.md docs/archive/
mv QR_CODE_STATUS.md docs/archive/
mv QUICK_FIX_PLAN.md docs/archive/
mv SESSION_SUMMARY.md docs/archive/
mv SOLID_NODE_FIX_PLAN.md docs/archive/
mv UI_AUDIT.md docs/archive/
mv UI_ELEMENT_AUDIT.md docs/archive/
mv UI_REDESIGN_SUMMARY.md docs/archive/
mv UI_REFRESH_FIX_2025-10-06.md docs/archive/
mv UI_SESSION_SUMMARY.md docs/archive/
mv UPDATE_MANAGER_ROLLOUT.md docs/archive/

# Keep these active documentation files:
# - ARCHITECTURE.md
# - DESKTOP_APP_STATUS.md
# - EVENT_SYSTEM_IMPLEMENTATION.md
# - INTEGRATION_STATUS.md
# - LOGIN_PAGE_UPDATE.md
# - QUICKSTART.md
# - QUICK_TEST_GUIDE.md
# - STORAGE_PERSISTENCE_GUIDE.md
# - TROUBLESHOOTING.md
```

### Verify Unused Directories
```bash
# Check if nested src-tauri is empty/unused
ls -la src-tauri/src-tauri/

# If only contains examples/, verify examples aren't referenced
grep -r "test_bip39" . --include="*.toml"

# If no references, remove
rm -rf src-tauri/src-tauri/
```

---

## 💾 Disk Space Savings

| Item | Size | Impact |
|------|------|--------|
| `src-tauri/target/` | ~23 GB | **HUGE** |
| `ui/*.deprecated` + `*.backup` | ~100 KB | Minimal |
| `test_bip39_validation.rs` | ~3 KB | Minimal |
| Old documentation (43 files) | ~500 KB | Small |
| **Total Potential Savings** | **~23 GB** | **Significant** |

---

## ⚠️ Caution Areas

### DO NOT DELETE:
- `node_modules/` - Required dependencies (can regenerate with `npm install`)
- `src-tauri/Cargo.lock` - Dependency lock file
- `src-tauri/src/` - All source code
- `ui/*.html` (non-deprecated) - Active UI files
- `ui/*.js`, `ui/*.css` - Active frontend code

### Verify Before Deleting:
- Shell scripts (`*.sh`) - May be used for deployment/testing
- `.github/workflows/` - May be used for CI/CD
- `cypress/` directory - May have E2E tests

---

## 📊 Summary

### Definite Deletions:
1. ✅ Build artifacts (`cargo clean`) - **23 GB saved**
2. ✅ Deprecated UI files (4 files)
3. ✅ Temporary test file (`test_bip39_validation.rs`)

### Recommended Actions:
1. ✅ Archive old documentation (43 files) to `docs/archive/`
2. ✅ Review and remove unused test infrastructure (Cypress if unused)
3. ✅ Clean up nested `src-tauri/src-tauri/` directory

### Total Impact:
- **Disk Space**: ~23 GB freed
- **Files**: ~50+ files removed/archived
- **Organization**: Much cleaner project structure

---

**Generated by**: Claude Code AI Assistant
**Review**: Recommended before executing cleanup
**Backup**: Suggest creating backup before mass deletion