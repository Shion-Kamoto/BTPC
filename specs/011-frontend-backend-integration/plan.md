# Feature 011: Complete Frontend-Backend Integration - Implementation Plan

## Executive Summary

This plan addresses critical frontend-backend integration gaps in the BTPC desktop app, focusing on three main issues:
1. Broken login system (0/3 commands working)
2. Missing GPU stats display (2 commands missing)
3. Missing transaction block details (1 command missing)

**Timeline:** 2-3 days
**Priority:** High (login system blocks new user onboarding)
**Risk Level:** Low (well-defined scope, existing patterns to follow)

## Phase 0: Research & Analysis ✅ COMPLETE

### Findings

**1. Authentication System Mismatch**
- Frontend (login.html) expects: `has_master_password`, `create_master_password`, `login`
- Backend has: SecurityManager with `check_wallet_lock_status`, `unlock_wallets`, `migrate_to_encrypted`
- **Root Cause:** login.html was created as a prototype but never integrated with SecurityManager
- **Solution:** Two options:
  - Option A: Implement missing commands wrapping SecurityManager
  - Option B: Update login.html to use password-modal.js pattern (used by other pages)
  - **Recommendation:** Option B (reuse existing tested code)

**2. GPU Stats Commands**
- Feature 009 implemented GPU mining but Phase 3 (stats display) was never completed
- MiningThreadPool has GPU stats available via `get_stats()` method
- Frontend already has UI components for GPU stats display
- **Solution:** Add two Tauri commands exposing MiningThreadPool GPU stats

**3. Block Details Command**
- Frontend calls `get_block_by_height` in transaction details modal
- Command doesn't exist in backend
- **Solution:** Add Tauri command querying UnifiedDatabase CF_BLOCKS column family

### Architecture Review

**Current State:**
```
Frontend (HTML/JS)
  ↓ window.invoke()
Tauri IPC Layer
  ↓ #[tauri::command]
Backend (Rust)
  ↓ Direct calls
  ├─ EmbeddedNode (blockchain state)
  ├─ MiningThreadPool (mining stats)
  ├─ UnifiedDatabase (RocksDB)
  ├─ SecurityManager (authentication)
  └─ WalletManager (wallet operations)
```

**Integration Points:**
- ✅ Wallet operations: 9/9 commands working
- ✅ Transaction operations: 13/14 commands working (1 missing)
- ✅ Node operations: 4/4 commands working
- ✅ Mining operations: 6/8 commands working (2 missing)
- ❌ Authentication: 0/3 commands working (complete mismatch)

## Phase 1: Design

### 1.1 Authentication System Design

**Approach: Refactor login.html to use password-modal.js pattern**

**Current Flow (Broken):**
```javascript
// login.html
await window.invoke('has_master_password') // ❌ doesn't exist
await window.invoke('create_master_password', { password }) // ❌ doesn't exist
await window.invoke('login', { password }) // ❌ doesn't exist
```

**New Flow (Aligned with existing system):**
```javascript
// login.html - updated to match password-modal.js
await window.invoke('check_wallet_lock_status') // ✅ exists
await window.invoke('migrate_to_encrypted', { password }) // ✅ exists (for new users)
await window.invoke('unlock_wallets', { password }) // ✅ exists (for existing users)
```

**Files to Modify:**
1. `ui/login.html` - Update JavaScript to use existing commands
2. No backend changes required (reuse existing SecurityManager integration)

**Benefits:**
- Reuses tested authentication code
- No new backend commands needed
- Consistent with app-wide authentication pattern
- Faster implementation (no new Rust code)

### 1.2 GPU Stats Commands Design

**Data Flow:**
```
Frontend (mining.html)
  ↓ window.invoke('is_gpu_stats_available')
Tauri Command
  ↓ AppState.mining_pool.lock()
MiningThreadPool
  ↓ opencl_manager.is_some()
OpenCL Manager
  ↓ true/false
```

**Command Signatures:**

```rust
// src-tauri/src/mining_commands.rs

#[tauri::command]
pub async fn is_gpu_stats_available(
    state: State<'_, AppState>
) -> Result<bool, String> {
    let pool = state.mining_pool.lock()
        .map_err(|_| "Failed to lock mining pool")?;
    Ok(pool.is_gpu_available())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GpuStats {
    pub device_name: String,
    pub hashrate: f64,           // H/s
    pub temperature: Option<f32>,  // Celsius
    pub utilization: Option<f32>,  // Percentage (0-100)
    pub memory_used: Option<u64>,  // Bytes
    pub memory_total: Option<u64>, // Bytes
    pub power_usage: Option<f32>,  // Watts
}

#[tauri::command]
pub async fn get_gpu_stats(
    state: State<'_, AppState>
) -> Result<Vec<GpuStats>, String> {
    let pool = state.mining_pool.lock()
        .map_err(|_| "Failed to lock mining pool")?;

    let stats = pool.get_stats();

    if !stats.gpu_enabled {
        return Ok(vec![]);
    }

    // TODO: Implement in MiningThreadPool
    pool.get_gpu_stats()
        .map_err(|e| format!("Failed to get GPU stats: {}", e))
}
```

**Frontend Integration:**
```javascript
// mining.html - already has UI components, just needs working commands

async function updateGPUStats() {
    try {
        const available = await window.invoke('is_gpu_stats_available');
        if (!available) {
            document.getElementById('gpu-stats-section').style.display = 'none';
            return;
        }

        const gpuStats = await window.invoke('get_gpu_stats');
        displayGPUStats(gpuStats); // Function already exists in mining.html
    } catch (e) {
        console.error('GPU stats error:', e);
    }
}
```

**Files to Modify:**
1. `src-tauri/src/mining_commands.rs` - Add two commands
2. `src-tauri/src/mining_thread_pool.rs` - Add `is_gpu_available()` and `get_gpu_stats()` methods
3. `src-tauri/src/main.rs` - Register new commands in invoke_handler
4. `ui/mining.html` - Already has UI, just needs working command calls

### 1.3 Block Details Command Design

**Data Flow:**
```
Frontend (transactions.html)
  ↓ window.invoke('get_block_by_height', { height: 12345 })
Tauri Command
  ↓ UnifiedDatabase.get_block(height)
RocksDB CF_BLOCKS
  ↓ Block { height, hash, timestamp, tx_count, ... }
```

**Command Signature:**

```rust
// src-tauri/src/main.rs (or new src-tauri/src/blockchain_commands.rs)

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BlockInfo {
    pub height: u64,
    pub hash: String,
    pub prev_hash: String,
    pub timestamp: u64,
    pub transaction_count: usize,
    pub difficulty: String,
    pub nonce: u64,
    pub confirmations: u64, // Current height - block height
}

#[tauri::command]
pub async fn get_block_by_height(
    height: u64,
    state: State<'_, AppState>
) -> Result<BlockInfo, String> {
    let node = state.embedded_node.read().await;
    let node_ref = node.as_ref()
        .ok_or("Embedded node not initialized")?;

    let node_lock = node_ref.read().await;
    let database = node_lock.database();

    // Query CF_BLOCKS for block at height
    let block = database.get_block(height)
        .map_err(|e| format!("Failed to get block: {}", e))?
        .ok_or_else(|| format!("Block {} not found", height))?;

    // Get current height for confirmations
    let current_height = node_lock.get_blockchain_state()
        .await
        .map_err(|e| format!("Failed to get current height: {}", e))?
        .current_height;

    Ok(BlockInfo {
        height: block.header.height,
        hash: hex::encode(&block.hash()),
        prev_hash: hex::encode(&block.header.prev_hash),
        timestamp: block.header.timestamp,
        transaction_count: block.transactions.len(),
        difficulty: block.header.difficulty.to_string(),
        nonce: block.header.nonce,
        confirmations: if current_height >= height {
            current_height - height + 1
        } else {
            0
        },
    })
}
```

**Files to Modify:**
1. `src-tauri/src/unified_database.rs` - Add `get_block(height)` method
2. `src-tauri/src/main.rs` - Add `get_block_by_height` command
3. `ui/transactions.html` - Already calls command, just needs it to work

## Phase 2: Implementation

### Task Breakdown

#### T011-001: Update login.html to use existing authentication [2-3 hours]
**Priority:** P0 (blocks new user onboarding)
**Dependencies:** None
**Files:**
- `ui/login.html`

**Steps:**
1. Replace `has_master_password` with `check_wallet_lock_status`
2. Replace `create_master_password` with `migrate_to_encrypted`
3. Replace `login` with `unlock_wallets`
4. Update error handling to match existing command responses
5. Test new user flow (create password)
6. Test existing user flow (login)

**Acceptance:**
- New users can create master password
- Existing users can login
- Failed login shows error message
- Successful login redirects to dashboard
- No console errors

---

#### T011-002: Add GPU availability check command [1 hour]
**Priority:** P1
**Dependencies:** None
**Files:**
- `src-tauri/src/mining_thread_pool.rs`
- `src-tauri/src/mining_commands.rs`
- `src-tauri/src/main.rs`

**Steps:**
1. Add `is_gpu_available()` method to MiningThreadPool
2. Add `is_gpu_stats_available` Tauri command
3. Register command in invoke_handler
4. Add unit test for command
5. Test with GPU present
6. Test with GPU absent

**Acceptance:**
- Command returns `true` when GPU mining is available
- Command returns `false` when GPU mining is not available
- No panics or errors
- Unit test passes

---

#### T011-003: Add GPU stats command [2-3 hours]
**Priority:** P1
**Dependencies:** T011-002
**Files:**
- `src-tauri/src/mining_thread_pool.rs`
- `src-tauri/src/mining_commands.rs`
- `src-tauri/src/main.rs`

**Steps:**
1. Define `GpuStats` struct with all required fields
2. Add `get_gpu_stats()` method to MiningThreadPool
3. Query OpenCL manager for GPU metrics
4. Add `get_gpu_stats` Tauri command
5. Register command in invoke_handler
6. Add unit test for command
7. Test with real GPU (if available)

**Acceptance:**
- Command returns array of GPU stats
- Stats include: device_name, hashrate, temperature (optional)
- Returns empty array when GPU not available
- No panics or errors
- Unit test passes

---

#### T011-004: Update mining.html to display GPU stats [1-2 hours]
**Priority:** P1
**Dependencies:** T011-002, T011-003
**Files:**
- `ui/mining.html`

**Steps:**
1. Update `updateGPUStats()` to call new commands
2. Add periodic refresh (every 5 seconds)
3. Test GPU stats display with mock data
4. Test graceful fallback when GPU not available
5. Verify no console errors

**Acceptance:**
- GPU stats section shows when GPU available
- GPU stats section hides when GPU not available
- Stats update every 5 seconds
- No flickering or UI glitches
- No console errors

---

#### T011-005: Add get_block method to UnifiedDatabase [1 hour]
**Priority:** P1
**Dependencies:** None
**Files:**
- `src-tauri/src/unified_database.rs`

**Steps:**
1. Add `get_block(height: u64)` method
2. Query CF_BLOCKS column family
3. Deserialize block data
4. Add error handling for missing blocks
5. Add unit test
6. Test with existing blocks
7. Test with non-existent blocks

**Acceptance:**
- Returns `Some(Block)` when block exists
- Returns `None` when block doesn't exist
- No panics on invalid height
- Unit test passes

---

#### T011-006: Add get_block_by_height command [1-2 hours]
**Priority:** P1
**Dependencies:** T011-005
**Files:**
- `src-tauri/src/main.rs`

**Steps:**
1. Define `BlockInfo` struct
2. Implement `get_block_by_height` command
3. Calculate confirmations from current height
4. Register command in invoke_handler
5. Add unit test
6. Test with various block heights

**Acceptance:**
- Command returns block information
- Confirmations calculated correctly
- Error message when block not found
- No panics on invalid height
- Unit test passes

---

#### T011-007: Verify transaction details modal displays block info [30 min]
**Priority:** P1
**Dependencies:** T011-006
**Files:**
- `ui/transactions.html`

**Steps:**
1. Open transaction details modal
2. Verify block height displayed
3. Verify block hash displayed
4. Verify confirmations displayed
5. Test with multiple transactions
6. Verify error handling

**Acceptance:**
- Block info displays in modal
- All fields populated correctly
- Error message when block data unavailable
- No console errors

---

#### T011-008: Integration testing [1-2 hours]
**Priority:** P0
**Dependencies:** All previous tasks
**Files:**
- All modified files

**Steps:**
1. Test complete login flow (new + existing user)
2. Test GPU stats display (with/without GPU)
3. Test transaction block details
4. Test page navigation (state persistence)
5. Check for console errors
6. Verify no regression in existing features

**Acceptance:**
- All three integration issues resolved
- No console errors on any page
- No regression in existing functionality
- All acceptance criteria met

---

#### T011-009: Documentation [1 hour]
**Priority:** P2
**Dependencies:** T011-008
**Files:**
- `specs/011-frontend-backend-integration/completion-report.md`
- `CLAUDE.md` (update Recent Changes)

**Steps:**
1. Document all command signatures
2. Update CLAUDE.md with Feature 011 summary
3. Create completion report
4. Document any remaining issues
5. Document unused commands for future reference

**Acceptance:**
- Completion report created
- CLAUDE.md updated
- All commands documented
- Known issues documented

## Phase 3: Testing

### Test Plan

#### 3.1 Unit Tests

**Authentication (login.html):**
```javascript
// Manual test cases
1. New user flow
   - Launch app → login page shows
   - Enter password → success
   - Redirect to dashboard → success

2. Existing user flow
   - Launch app → login page shows
   - Enter correct password → success
   - Enter wrong password → error message

3. Edge cases
   - Empty password → validation error
   - Special characters in password → accepted
   - Very long password → accepted
```

**GPU Stats:**
```rust
// src-tauri/src/mining_commands.rs tests
#[tokio::test]
async fn test_is_gpu_stats_available_with_gpu() {
    // Create AppState with GPU enabled
    // Call is_gpu_stats_available
    // Assert returns true
}

#[tokio::test]
async fn test_is_gpu_stats_available_without_gpu() {
    // Create AppState with GPU disabled
    // Call is_gpu_stats_available
    // Assert returns false
}

#[tokio::test]
async fn test_get_gpu_stats() {
    // Create AppState with GPU enabled
    // Call get_gpu_stats
    // Assert returns non-empty array
    // Assert all required fields present
}
```

**Block Details:**
```rust
// src-tauri/src/unified_database.rs tests
#[test]
fn test_get_block_existing() {
    // Create temp database
    // Insert test block
    // Call get_block(height)
    // Assert returns Some(block)
}

#[test]
fn test_get_block_missing() {
    // Create temp database
    // Call get_block(999999)
    // Assert returns None
}

// src-tauri/src/main.rs tests
#[tokio::test]
async fn test_get_block_by_height() {
    // Create test state
    // Call get_block_by_height
    // Assert BlockInfo populated correctly
    // Assert confirmations calculated correctly
}
```

#### 3.2 Integration Tests

**End-to-End Flow:**
1. Launch app
2. Create master password (new user)
3. Navigate to mining page
4. Start mining
5. Verify GPU stats display (if GPU present)
6. Navigate to transactions page
7. View transaction details
8. Verify block info displays
9. Navigate to node page
10. Verify sync status persists
11. Logout
12. Login with password
13. Verify session restored

#### 3.3 Regression Tests

**Verify no breaks in:**
- Wallet creation/import
- Transaction sending
- Mining start/stop
- Node sync start/stop
- Settings changes
- Page navigation
- Session persistence

### Manual Testing Checklist

```markdown
- [ ] Login as new user works
- [ ] Login as existing user works
- [ ] Wrong password shows error
- [ ] GPU stats display when GPU present
- [ ] GPU stats hidden when GPU absent
- [ ] Transaction block details show
- [ ] Block confirmations accurate
- [ ] No console errors on any page
- [ ] Page navigation preserves state
- [ ] Logout/login cycle works
- [ ] All existing features still work
```

## Phase 4: Deployment

### 4.1 Pre-Deployment Checklist

```markdown
- [ ] All unit tests passing
- [ ] All integration tests passing
- [ ] No console errors in production build
- [ ] Documentation complete
- [ ] Code reviewed (self-review)
- [ ] CLAUDE.md updated
- [ ] Completion report written
```

### 4.2 Rollout Plan

**Step 1:** Build production binary
```bash
npm run tauri:build
```

**Step 2:** Test production build
- Test on clean environment (no existing data)
- Test on existing environment (with data)
- Verify all features work

**Step 3:** Create release
- Tag commit: `feature-011-frontend-backend-integration`
- Document changes in release notes
- Note breaking changes (if any)

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Authentication refactor breaks existing sessions | Low | High | Test with existing user data, add migration logic if needed |
| GPU stats not available on all systems | High | Low | Graceful fallback already designed |
| Block data format mismatch | Medium | Medium | Add schema validation, test with real blockchain data |
| Performance impact from new commands | Low | Low | All commands use existing optimized code paths |

## Timeline

**Total Estimate:** 12-18 hours (2-3 days)

| Phase | Duration | Dependencies |
|-------|----------|--------------|
| Authentication (T011-001) | 2-3 hours | None |
| GPU Stats Commands (T011-002, T011-003) | 3-4 hours | None |
| GPU Stats UI (T011-004) | 1-2 hours | T011-002, T011-003 |
| Block Details Backend (T011-005, T011-006) | 2-3 hours | None |
| Block Details Frontend (T011-007) | 30 min | T011-006 |
| Integration Testing (T011-008) | 1-2 hours | All tasks |
| Documentation (T011-009) | 1 hour | T011-008 |

**Critical Path:** Authentication → Integration Testing → Documentation

## Success Metrics

### Quantitative
- 100% of frontend commands have backend implementation
- 0 console errors on any page
- All 7 pages fully functional
- Test coverage ≥ 80% for new code

### Qualitative
- New users can onboard successfully
- Mining page shows complete information
- Transaction details are informative
- No user-facing errors or broken features

## Future Considerations

### Code Cleanup (Future Feature)
- Audit ~60 unused backend commands
- Create deprecation plan
- Document commands intended for future use
- Remove truly obsolete commands

### Enhanced Features (Future)
- Add more GPU metrics (memory bandwidth, clock speed)
- Add block explorer functionality
- Implement advanced transaction filtering
- Add authentication session timeout settings

## Appendix

### A. Command Reference

**Authentication Commands (Existing):**
- `check_wallet_lock_status() -> Result<WalletLockStatus, String>`
- `migrate_to_encrypted(password: String) -> Result<String, String>`
- `unlock_wallets(password: String) -> Result<String, String>`
- `lock_wallets() -> Result<(), String>`

**New Commands:**
- `is_gpu_stats_available() -> Result<bool, String>`
- `get_gpu_stats() -> Result<Vec<GpuStats>, String>`
- `get_block_by_height(height: u64) -> Result<BlockInfo, String>`

### B. File Modifications Summary

**Backend:**
- `src-tauri/src/mining_thread_pool.rs` - Add GPU stats methods
- `src-tauri/src/mining_commands.rs` - Add 2 commands
- `src-tauri/src/unified_database.rs` - Add get_block method
- `src-tauri/src/main.rs` - Add block command, register new commands

**Frontend:**
- `ui/login.html` - Update to use existing auth commands
- `ui/mining.html` - Update GPU stats calls (already has UI)
- `ui/transactions.html` - Already calls get_block_by_height (just needs backend)

**Total:** 7 files modified, 0 files added

### C. Breaking Changes

**None** - All changes are additive or fix existing broken functionality.