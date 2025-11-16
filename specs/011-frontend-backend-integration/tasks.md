# Feature 011: Complete Frontend-Backend Integration - Task List

## Overview

This task list implements the complete frontend-backend integration plan to fix:
- Login system (0/3 commands working → 3/3)
- GPU stats display (6/8 commands → 8/8)
- Transaction block details (13/14 commands → 14/14)

**Total Tasks:** 9
**Estimated Time:** 12-18 hours (2-3 days)

## ⚠️ CONSTITUTION COMPLIANCE WARNING (Article VI.3)

**TDD METHODOLOGY MANDATORY**: All tasks MUST follow RED-GREEN-REFACTOR cycle:
1. **RED**: Write failing test FIRST
2. **GREEN**: Implement minimum code to pass test
3. **REFACTOR**: Improve code while keeping tests green

**Current Status**: Backend code exists for T011-002 through T011-006, but was implemented WITHOUT following TDD. Tests must be added retroactively to achieve constitutional compliance.

**Evidence Required for Completion**:
- Test file paths and test names
- Proof of RED phase (test written first or added retroactively)
- Proof of GREEN phase (all tests pass)
- Test execution output (cargo test results)

---

## T011-001: Update login.html to use existing authentication ⏳ TODO

**Priority:** P0 (Critical - blocks new user onboarding)
**Estimated Time:** 2-3 hours
**Dependencies:** None
**Status:** NOT STARTED - Awaiting TDD implementation

### Description
Refactor `login.html` to use existing SecurityManager commands instead of non-existent authentication commands. This aligns the login page with the password-modal.js pattern used throughout the app.

### Current State
```javascript
// ❌ BROKEN - These commands don't exist
await window.invoke('has_master_password')
await window.invoke('create_master_password', { password })
await window.invoke('login', { password })
```

### Target State
```javascript
// ✅ WORKING - These commands exist in backend
const status = await window.invoke('check_wallet_lock_status')
// New users:
await window.invoke('migrate_to_encrypted', { password })
// Existing users:
await window.invoke('unlock_wallets', { password })
```

### Files to Modify
- `btpc-desktop-app/ui/login.html`

### Implementation Steps
1. [ ] Update `checkMasterPassword()` function to use `check_wallet_lock_status`
2. [ ] Update `createMasterPassword()` to use `migrate_to_encrypted`
3. [ ] Update `login()` to use `unlock_wallets`
4. [ ] Update error handling to match command response formats
5. [ ] Test new user flow (create password)
6. [ ] Test existing user flow (login with password)
7. [ ] Test error cases (wrong password, empty password)

### Acceptance Criteria
- [ ] New users can create master password
- [ ] Existing users can login with correct password
- [ ] Wrong password shows clear error message
- [ ] Successful login redirects to dashboard (index.html)
- [ ] No console errors
- [ ] Session persists across page navigation

### Testing
```bash
# Manual test
1. Delete ~/.btpc/security/* (simulate new user)
2. Launch app with: npm run tauri:dev
3. Verify login page shows
4. Create password → should succeed
5. Close app and relaunch
6. Login with correct password → should succeed
7. Login with wrong password → should show error
```

---

## T011-002: Add GPU availability check command ⚠️ PARTIALLY IMPLEMENTED (Backend exists, TDD tests missing)

**Priority:** P1 (Important)
**Estimated Time:** 1 hour
**Dependencies:** None

### Description
Implement `is_gpu_stats_available` command to detect if GPU mining hardware is available. This is Phase 1 of completing Feature 009 GPU stats display.

### Current State
```javascript
// ❌ BROKEN - Command doesn't exist
const available = await window.invoke('is_gpu_stats_available')
```

### Target State
```rust
// ✅ NEW COMMAND
#[tauri::command]
pub async fn is_gpu_stats_available(state: State<'_, AppState>) -> Result<bool, String>
```

### Files to Modify
1. `btpc-desktop-app/src-tauri/src/mining_thread_pool.rs`
   - Add `is_gpu_available()` method
2. `btpc-desktop-app/src-tauri/src/mining_commands.rs`
   - Add `is_gpu_stats_available` command
3. `btpc-desktop-app/src-tauri/src/main.rs`
   - Register command in `invoke_handler`

### Implementation Steps
1. [ ] Add `is_gpu_available()` method to MiningThreadPool
   ```rust
   pub fn is_gpu_available(&self) -> bool {
       self.opencl_manager.is_some()
   }
   ```
2. [ ] Add Tauri command in mining_commands.rs
   ```rust
   #[tauri::command]
   pub async fn is_gpu_stats_available(
       state: State<'_, AppState>
   ) -> Result<bool, String> {
       let pool = state.mining_pool.lock()
           .map_err(|_| "Failed to lock mining pool".to_string())?;
       Ok(pool.is_gpu_available())
   }
   ```
3. [ ] Register command in main.rs invoke_handler
4. [ ] Add unit test for command
5. [ ] Test with GPU present (if available)
6. [ ] Test with GPU absent

### Acceptance Criteria
- [ ] Command returns `true` when GPU mining hardware available
- [ ] Command returns `false` when GPU mining hardware not available
- [ ] No panics or errors when called
- [ ] Unit test passes
- [ ] Backend logs show command being called

### Testing
```rust
#[tokio::test]
async fn test_is_gpu_stats_available() {
    let temp_dir = tempdir().unwrap();
    let pool = MiningThreadPool::new(temp_dir.path().to_path_buf());

    // Test without GPU
    assert!(!pool.is_gpu_available());

    // Test with GPU (if OpenCL available)
    // This test may be skipped on systems without GPU
}
```

---

## T011-003: Add GPU stats command ⚠️ PARTIALLY IMPLEMENTED (Backend exists, TDD tests missing)

**Priority:** P1 (Important)
**Estimated Time:** 2-3 hours
**Dependencies:** T011-002

### Description
Implement `get_gpu_stats` command to retrieve GPU mining statistics (hashrate, temperature, utilization, etc.). This completes Feature 009 Phase 3.

### Current State
```javascript
// ❌ BROKEN - Command doesn't exist
const gpuStats = await window.invoke('get_gpu_stats')
```

### Target State
```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GpuStats {
    pub device_name: String,
    pub hashrate: f64,
    pub temperature: Option<f32>,
    pub utilization: Option<f32>,
    pub memory_used: Option<u64>,
    pub memory_total: Option<u64>,
}

#[tauri::command]
pub async fn get_gpu_stats(state: State<'_, AppState>) -> Result<Vec<GpuStats>, String>
```

### Files to Modify
1. `btpc-desktop-app/src-tauri/src/mining_thread_pool.rs`
   - Add `get_gpu_stats()` method
2. `btpc-desktop-app/src-tauri/src/mining_commands.rs`
   - Define `GpuStats` struct
   - Add `get_gpu_stats` command
3. `btpc-desktop-app/src-tauri/src/main.rs`
   - Register command in `invoke_handler`

### Implementation Steps
1. [ ] Define `GpuStats` struct in mining_commands.rs
2. [ ] Add `get_gpu_stats()` method to MiningThreadPool
   - Query OpenCL manager for device info
   - Extract hashrate from current stats
   - Get temperature/utilization if available
3. [ ] Add Tauri command wrapper
4. [ ] Handle case when GPU not available (return empty vec)
5. [ ] Add unit test
6. [ ] Test with real GPU (if available)
7. [ ] Test without GPU (should return empty array)

### Acceptance Criteria
- [ ] Command returns array of GPU stats
- [ ] Each stat includes: device_name, hashrate
- [ ] Optional fields (temp, utilization) handled gracefully
- [ ] Returns empty array when GPU not available
- [ ] No panics when GPU missing
- [ ] Unit test passes

### Testing
```rust
#[tokio::test]
async fn test_get_gpu_stats_without_gpu() {
    let state = create_test_state_without_gpu();
    let result = get_gpu_stats(State::from(state)).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0);
}

#[tokio::test]
async fn test_get_gpu_stats_with_gpu() {
    let state = create_test_state_with_gpu();
    let result = get_gpu_stats(State::from(state)).await;
    assert!(result.is_ok());
    let stats = result.unwrap();
    assert!(!stats.is_empty());
    assert!(stats[0].hashrate >= 0.0);
}
```

---

## T011-004: Update mining.html to display GPU stats ⚠️ PARTIALLY IMPLEMENTED (Backend exists, integration tests missing)

**Priority:** P1 (Important)
**Estimated Time:** 1-2 hours
**Dependencies:** T011-002, T011-003

### Description
Update mining.html to call the new GPU stats commands and display GPU information in the existing UI components.

### Current State
```javascript
// ❌ Calls missing commands
const available = await window.invoke('is_gpu_stats_available') // fails
const gpuStats = await window.invoke('get_gpu_stats') // fails
```

### Target State
```javascript
// ✅ Calls working commands
const available = await window.invoke('is_gpu_stats_available') // succeeds
if (available) {
    const gpuStats = await window.invoke('get_gpu_stats') // succeeds
    displayGPUStats(gpuStats) // existing function
}
```

### Files to Modify
- `btpc-desktop-app/ui/mining.html`

### Implementation Steps
1. [ ] Update `updateGPUStats()` function to call working commands
2. [ ] Add periodic refresh (every 5 seconds)
3. [ ] Show GPU stats section when GPU available
4. [ ] Hide GPU stats section when GPU not available
5. [ ] Display all GPU metrics in existing UI
6. [ ] Test with GPU present
7. [ ] Test without GPU (verify graceful fallback)

### Acceptance Criteria
- [ ] GPU stats section visible when GPU available
- [ ] GPU stats section hidden when GPU not available
- [ ] Stats update every 5 seconds automatically
- [ ] All metrics displayed: name, hashrate, temp (if available)
- [ ] No flickering or UI glitches
- [ ] No console errors

### Testing
```bash
# Manual test
1. Launch app with GPU: npm run tauri:dev
2. Navigate to mining page
3. Verify GPU stats section appears
4. Verify stats update every 5 seconds
5. Launch app without GPU (or disable OpenCL)
6. Verify GPU stats section hidden
7. Verify no console errors
```

---

## T011-005: Add get_block method to UnifiedDatabase ⚠️ PARTIALLY IMPLEMENTED (Backend exists, TDD tests missing)

**Priority:** P1 (Important)
**Estimated Time:** 1 hour
**Dependencies:** None

### Description
Add `get_block(height)` method to UnifiedDatabase to query blocks from the CF_BLOCKS column family. This is the backend foundation for transaction block details.

### Current State
```rust
// ❌ Method doesn't exist
let block = database.get_block(height)?
```

### Target State
```rust
// ✅ NEW METHOD
pub fn get_block(&self, height: u64) -> Result<Option<Block>> {
    let key = height.to_be_bytes();
    match self.db.get_cf(&self.cf_blocks, &key)? {
        Some(bytes) => Ok(Some(bincode::deserialize(&bytes)?)),
        None => Ok(None),
    }
}
```

### Files to Modify
- `btpc-desktop-app/src-tauri/src/unified_database.rs`

### Implementation Steps
1. [ ] Add `get_block(height: u64)` method
2. [ ] Query CF_BLOCKS column family with height as key
3. [ ] Deserialize block data using bincode
4. [ ] Return `Ok(Some(Block))` if found, `Ok(None)` if not
5. [ ] Add error handling for deserialization failures
6. [ ] Add unit test
7. [ ] Test with existing blocks
8. [ ] Test with non-existent block heights

### Acceptance Criteria
- [ ] Method returns `Some(Block)` when block exists
- [ ] Method returns `None` when block doesn't exist
- [ ] No panics on invalid height values
- [ ] Deserialization errors handled gracefully
- [ ] Unit test passes

### Testing
```rust
#[test]
fn test_get_block() {
    let temp_dir = tempdir().unwrap();
    let db = UnifiedDatabase::open(temp_dir.path()).unwrap();

    // Insert test block
    let block = create_test_block(height: 100);
    db.put_block(&block).unwrap();

    // Test existing block
    let result = db.get_block(100).unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().header.height, 100);

    // Test missing block
    let result = db.get_block(999999).unwrap();
    assert!(result.is_none());
}
```

---

## T011-006: Add get_block_by_height command ⚠️ PARTIALLY IMPLEMENTED (Backend exists, TDD tests missing)

**Priority:** P1 (Important)
**Estimated Time:** 1-2 hours
**Dependencies:** T011-005

### Description
Implement `get_block_by_height` Tauri command to expose block data to the frontend. Includes calculating confirmations based on current blockchain height.

### Current State
```javascript
// ❌ BROKEN - Command doesn't exist
const block = await window.invoke('get_block_by_height', { height: 12345 })
```

### Target State
```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BlockInfo {
    pub height: u64,
    pub hash: String,
    pub prev_hash: String,
    pub timestamp: u64,
    pub transaction_count: usize,
    pub difficulty: String,
    pub nonce: u64,
    pub confirmations: u64,
}

#[tauri::command]
pub async fn get_block_by_height(height: u64, state: State<'_, AppState>) -> Result<BlockInfo, String>
```

### Files to Modify
1. `btpc-desktop-app/src-tauri/src/main.rs`
   - Define `BlockInfo` struct
   - Add `get_block_by_height` command
   - Register in `invoke_handler`

### Implementation Steps
1. [ ] Define `BlockInfo` struct with all required fields
2. [ ] Implement `get_block_by_height` command
   - Get embedded node from AppState
   - Query database using `get_block(height)`
   - Get current height for confirmations calculation
   - Build BlockInfo response
3. [ ] Calculate confirmations: `current_height - block_height + 1`
4. [ ] Handle missing blocks (return error)
5. [ ] Register command in invoke_handler
6. [ ] Add unit test
7. [ ] Test with various block heights

### Acceptance Criteria
- [ ] Command returns block information
- [ ] All fields populated correctly
- [ ] Confirmations calculated accurately
- [ ] Error message when block not found
- [ ] No panics on invalid heights
- [ ] Unit test passes

### Testing
```rust
#[tokio::test]
async fn test_get_block_by_height() {
    let state = create_test_state_with_blocks();

    // Test existing block
    let result = get_block_by_height(100, State::from(&state)).await;
    assert!(result.is_ok());
    let block = result.unwrap();
    assert_eq!(block.height, 100);
    assert!(block.confirmations > 0);

    // Test missing block
    let result = get_block_by_height(999999, State::from(&state)).await;
    assert!(result.is_err());
}
```

---

## T011-007: Verify transaction details modal displays block info ⏳ TODO (Manual testing required)

**Priority:** P1 (Important)
**Estimated Time:** 30 minutes
**Dependencies:** T011-006

### Description
Verify that transactions.html transaction details modal correctly displays block information now that the backend command exists.

### Current State
```javascript
// ❌ Command call fails, block info not displayed
const block = await window.invoke('get_block_by_height', { height: tx.block_height })
```

### Target State
```javascript
// ✅ Command succeeds, block info displayed
const block = await window.invoke('get_block_by_height', { height: tx.block_height })
// Block height, hash, confirmations shown in modal
```

### Files to Check
- `btpc-desktop-app/ui/transactions.html` (no changes needed, just verification)

### Implementation Steps
1. [ ] Open transaction details modal in UI
2. [ ] Verify block height displays
3. [ ] Verify block hash displays
4. [ ] Verify confirmations display
5. [ ] Test with multiple transactions
6. [ ] Test error handling (block not found)
7. [ ] Verify no console errors

### Acceptance Criteria
- [ ] Block height shown in modal
- [ ] Block hash shown in modal
- [ ] Confirmations shown correctly
- [ ] Error message when block data unavailable
- [ ] No console errors
- [ ] UI displays gracefully

### Testing
```bash
# Manual test
1. Launch app: npm run tauri:dev
2. Navigate to transactions page
3. Click on a transaction
4. Verify transaction details modal opens
5. Verify block information section displays:
   - Block Height: 12345
   - Block Hash: 0x1234...abcd
   - Confirmations: 42
6. Close modal
7. Check console for errors (should be none)
```

---

## T011-008: Integration testing ⏳ TODO (Comprehensive testing required)

**Priority:** P0 (Critical)
**Estimated Time:** 1-2 hours
**Dependencies:** T011-001, T011-004, T011-007

### Description
Perform comprehensive integration testing to verify all three integration issues are resolved and no regressions occurred.

### Scope
- Login flow (new + existing users)
- GPU stats display (with/without GPU)
- Transaction block details
- Page navigation state persistence
- All existing features

### Test Plan

#### Authentication Tests
1. [ ] **New User Flow**
   - Delete ~/.btpc/security/*
   - Launch app
   - Verify login page shows
   - Create master password
   - Verify redirects to dashboard
   - Close and relaunch
   - Verify can login

2. [ ] **Existing User Flow**
   - Launch app with existing credentials
   - Enter correct password
   - Verify login succeeds
   - Enter wrong password
   - Verify error message shows

3. [ ] **Edge Cases**
   - Empty password → validation error
   - Very long password (100+ chars) → accepted
   - Special characters → accepted
   - Session timeout → requires re-login

#### GPU Stats Tests
1. [ ] **With GPU Available**
   - Navigate to mining page
   - Verify GPU stats section appears
   - Verify stats show: name, hashrate
   - Wait 10 seconds
   - Verify stats update

2. [ ] **Without GPU Available**
   - Disable OpenCL or test on system without GPU
   - Navigate to mining page
   - Verify GPU stats section hidden
   - Verify no errors in console

#### Block Details Tests
1. [ ] **Transaction with Block**
   - Navigate to transactions page
   - Click on confirmed transaction
   - Verify modal shows block info
   - Verify confirmations > 0

2. [ ] **Transaction without Block** (pending)
   - Click on pending transaction
   - Verify graceful error handling
   - No console errors

#### Regression Tests
1. [ ] **Wallet Operations**
   - Create wallet → succeeds
   - Import wallet → succeeds
   - Delete wallet → succeeds
   - Backup wallet → succeeds

2. [ ] **Transaction Operations**
   - Create transaction → succeeds
   - Sign transaction → succeeds
   - Broadcast transaction → succeeds
   - View history → succeeds

3. [ ] **Mining Operations**
   - Start CPU mining → succeeds
   - Stop mining → succeeds
   - View mining logs → succeeds

4. [ ] **Node Operations**
   - Start blockchain sync → succeeds
   - Stop sync → succeeds
   - Sync status persists across pages

5. [ ] **Settings**
   - Change network → succeeds
   - Settings persist

#### Cross-Page Navigation
1. [ ] Navigate Dashboard → Wallet → Transactions → Mining → Node → Settings
2. [ ] Verify no console errors
3. [ ] Verify state persists (mining status, sync status, etc.)

### Acceptance Criteria
- [ ] All authentication tests pass
- [ ] All GPU stats tests pass
- [ ] All block details tests pass
- [ ] No regressions in existing features
- [ ] Zero console errors
- [ ] All pages fully functional

### Defects Log
```markdown
| Test | Status | Issue | Resolution |
|------|--------|-------|------------|
| Login new user | ✅ PASS | - | - |
| Login existing user | ✅ PASS | - | - |
| GPU stats with GPU | ⏭️ SKIP | No GPU available | Expected |
| GPU stats without GPU | ✅ PASS | - | - |
| Transaction block details | ✅ PASS | - | - |
| Wallet create | ✅ PASS | - | - |
| Wallet backup | ✅ PASS | - | - |
| Transaction send | ✅ PASS | - | - |
| Mining start | ✅ PASS | - | - |
| Node sync | ✅ PASS | - | - |
```

---

## T011-009: Documentation ⏳ TODO (Pending completion of all tasks)

**Priority:** P2 (Important)
**Estimated Time:** 1 hour
**Dependencies:** T011-008

### Description
Create comprehensive documentation for Feature 011, update CLAUDE.md, and create completion report.

### Deliverables
1. Completion report
2. CLAUDE.md update
3. Command reference
4. Known issues documentation

### Files to Create/Modify
1. `specs/011-frontend-backend-integration/completion-report.md`
2. `CLAUDE.md`
3. `specs/011-frontend-backend-integration/command-reference.md`

### Implementation Steps

#### 1. Create Completion Report
- [ ] Document all implemented features
- [ ] List all modified files
- [ ] Include before/after command counts
- [ ] Document test results
- [ ] Note any deviations from plan
- [ ] Document remaining work (if any)

#### 2. Update CLAUDE.md
- [ ] Add Feature 011 to Recent Changes
- [ ] Update command counts
- [ ] Document new authentication flow
- [ ] List new GPU stats commands
- [ ] Note block details command

#### 3. Create Command Reference
- [ ] Document all new command signatures
- [ ] Include usage examples
- [ ] Document error responses
- [ ] Add frontend code samples

#### 4. Document Known Issues
- [ ] List any unresolved issues
- [ ] Document workarounds
- [ ] Note future improvements
- [ ] Document ~60 unused commands

### Acceptance Criteria
- [ ] Completion report created and comprehensive
- [ ] CLAUDE.md updated with Feature 011
- [ ] Command reference complete
- [ ] All commands documented with examples
- [ ] Known issues logged

### Completion Report Template
```markdown
# Feature 011: Complete Frontend-Backend Integration - Completion Report

## Summary
Successfully integrated all frontend pages with backend commands, resolving 3 critical integration issues.

## Changes Implemented

### Authentication (T011-001)
- Refactored login.html to use SecurityManager commands
- Files modified: ui/login.html
- Commands aligned: check_wallet_lock_status, migrate_to_encrypted, unlock_wallets

### GPU Stats (T011-002, T011-003, T011-004)
- Added is_gpu_stats_available command
- Added get_gpu_stats command
- Updated mining.html to display GPU stats
- Files modified:
  - src-tauri/src/mining_thread_pool.rs
  - src-tauri/src/mining_commands.rs
  - src-tauri/src/main.rs
  - ui/mining.html

### Block Details (T011-005, T011-006, T011-007)
- Added get_block method to UnifiedDatabase
- Added get_block_by_height command
- Verified transactions.html displays block info
- Files modified:
  - src-tauri/src/unified_database.rs
  - src-tauri/src/main.rs

## Metrics

| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| Broken pages | 1 (login.html) | 0 | 100% |
| Partially broken pages | 2 | 0 | 100% |
| Working commands | 34/40 (85%) | 40/40 (100%) | +15% |
| Console errors | Multiple | 0 | 100% |

## Testing Results
- ✅ All unit tests passing
- ✅ All integration tests passing
- ✅ All acceptance criteria met
- ✅ Zero regression issues

## Known Issues
- None critical
- ~60 unused backend commands (documented for future cleanup)

## Future Work
- Code cleanup: Remove/deprecate unused commands
- Enhanced GPU metrics (memory bandwidth, clock speed)
- Block explorer functionality
- Advanced transaction filtering
```

---

## Summary

| Task | Priority | Time | Dependencies | Status |
|------|----------|------|--------------|--------|
| T011-001: Update login.html | P0 | 2-3h | None | ⏳ TODO (TDD required) |
| T011-002: GPU availability command | P1 | 1h | None | ⚠️ PARTIAL (Backend exists, tests missing) |
| T011-003: GPU stats command | P1 | 2-3h | T011-002 | ⚠️ PARTIAL (Backend exists, tests missing) |
| T011-004: Update mining.html | P1 | 1-2h | T011-002, T011-003 | ⚠️ PARTIAL (Backend exists, integration tests missing) |
| T011-005: Add get_block to database | P1 | 1h | None | ⚠️ PARTIAL (Backend exists, tests missing) |
| T011-006: Add get_block_by_height | P1 | 1-2h | T011-005 | ⚠️ PARTIAL (Backend exists, tests missing) |
| T011-007: Verify transaction details | P1 | 30m | T011-006 | ⏳ TODO (Manual testing) |
| T011-008: Integration testing | P0 | 1-2h | T011-001, T011-004, T011-007 | ⏳ TODO (All features) |
| T011-009: Documentation | P2 | 1h | T011-008 | ⏳ TODO (Pending completion) |

**Total Estimated Time:** 12-18 hours (2-3 days)
**Critical Path:** T011-001 → T011-008 → T011-009

## Getting Started

To begin implementation, start with T011-001 (login system) as it's the highest priority and blocks new user onboarding. Then proceed in parallel with GPU stats (T011-002-004) and block details (T011-005-007).