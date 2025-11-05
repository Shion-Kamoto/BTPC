# Tasks: Node and Backend Stability Fixes

**Input**: Design documents from `/home/bob/BTPC/BTPC/specs/003-fix-node-and/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md
**Constitution**: Article XI patterns for desktop features (MANDATORY)

## Feature Overview

This feature fixes critical stability and reliability issues in btpc-desktop-app:
- **50+ mutex poison vulnerabilities** causing cascading crashes
- **UTXO balance calculation bug** (displays 0.00 instead of 226.625 BTP)
- **Zombie process leaks** from `std::mem::forget()` usage
- **Unsafe file locking** (`libc::flock()` without error handling)
- **Poor error UX** (panics instead of user-friendly progressive disclosure)

**Technical Approach**: Systematic refactoring of `btpc-desktop-app/src-tauri/src/` focusing on:
1. Error propagation (`thiserror` with `AppError` enum)
2. Process lifecycle (`tokio::process` with async reaping)
3. State management (`StateManager<T>` wrapper for Article XI compliance)
4. Progressive disclosure error UI (HTML `<details>` + Tauri clipboard)

---

## Phase 3.1: Setup & Configuration

- [ ] **T001** Add `thiserror = "1.0"` and `fs2 = "0.4"` to `btpc-desktop-app/src-tauri/Cargo.toml`
  - **Files**: `btpc-desktop-app/src-tauri/Cargo.toml`
  - **Why**: Required for error handling and safe file locking

- [ ] **T002** [P] Create error module structure in `btpc-desktop-app/src-tauri/src/error.rs`
  - **Files**: `btpc-desktop-app/src-tauri/src/error.rs` (NEW)
  - **Defines**: `AppError` enum with variants from research.md
  - **Dependencies**: T001 (thiserror dependency)

- [ ] **T003** [P] Create state management module in `btpc-desktop-app/src-tauri/src/state_management.rs`
  - **Files**: `btpc-desktop-app/src-tauri/src/state_management.rs` (NEW)
  - **Defines**: `StateManager<T>` wrapper type for Article XI auto-event emission
  - **Dependencies**: None (can run parallel with T002)

- [ ] **T004** [P] Create process health monitor module in `btpc-desktop-app/src-tauri/src/process_health.rs`
  - **Files**: `btpc-desktop-app/src-tauri/src/process_health.rs` (NEW)
  - **Defines**: Health check loop, crash tracking, auto-restart logic
  - **Dependencies**: None (can run parallel with T002, T003)

- [ ] **T005** [P] Create lock manager module in `btpc-desktop-app/src-tauri/src/lock_manager.rs`
  - **Files**: `btpc-desktop-app/src-tauri/src/lock_manager.rs` (NEW)
  - **Defines**: `LockFile` wrapper using `fs2` crate
  - **Dependencies**: T001 (fs2 dependency)

---

## Phase 3.2: Tests First (TDD) ⚠️ MUST COMPLETE BEFORE 3.3
**CRITICAL: These tests MUST be written and MUST FAIL before ANY implementation**

### Contract Tests (Error Handling)

- [ ] **T006** [P] Contract test for `AppError` serialization in `btpc-desktop-app/src-tauri/tests/contract_error_state.rs`
  - **Files**: `btpc-desktop-app/src-tauri/tests/contract_error_state.rs` (NEW)
  - **Tests**: ErrorState → JSON matches `contracts/error_types.json` schema
  - **Expected**: FAIL (ErrorState not yet implemented)
  - **Dependencies**: T002 (error.rs structure)

- [ ] **T007** [P] Contract test for error sanitization in `btpc-desktop-app/src-tauri/tests/contract_error_sanitization.rs`
  - **Files**: `btpc-desktop-app/src-tauri/tests/contract_error_sanitization.rs` (NEW)
  - **Tests**: Private keys/passwords redacted from technical details
  - **Expected**: FAIL (sanitization not yet implemented)
  - **Dependencies**: T002 (error.rs structure)

### Contract Tests (Process Lifecycle)

- [ ] **T008** [P] Contract test for `ProcessHandle` lifecycle in `btpc-desktop-app/src-tauri/tests/contract_process_handle.rs`
  - **Files**: `btpc-desktop-app/src-tauri/tests/contract_process_handle.rs` (NEW)
  - **Tests**: Running → Crashed → Running transition matches `contracts/process_lifecycle.json`
  - **Expected**: FAIL (ProcessHandle not yet implemented)
  - **Dependencies**: T004 (process_health.rs structure)

- [ ] **T009** [P] Contract test for crash count reset in `btpc-desktop-app/src-tauri/tests/contract_crash_tracking.rs`
  - **Files**: `btpc-desktop-app/src-tauri/tests/contract_crash_tracking.rs` (NEW)
  - **Tests**: crash_count resets after 1 hour (3600s) of stable operation
  - **Expected**: FAIL (crash tracking not yet implemented)
  - **Dependencies**: T004 (process_health.rs structure)

### Contract Tests (Balance Calculation)

- [ ] **T010** [P] Unit test for address normalization in `btpc-desktop-app/src-tauri/tests/test_address_normalization.rs`
  - **Files**: `btpc-desktop-app/src-tauri/tests/test_address_normalization.rs` (NEW)
  - **Tests**: "BTPC..." and "btpc..." normalize to same value
  - **Expected**: FAIL (normalization not yet implemented)
  - **Dependencies**: None

- [ ] **T011** [P] Integration test for balance calculation in `btpc-desktop-app/src-tauri/tests/test_balance_calculation.rs`
  - **Files**: `btpc-desktop-app/src-tauri/tests/test_balance_calculation.rs` (NEW)
  - **Tests**: Wallet with 7 UTXOs (226.625 BTP) calculates correctly with mixed-case addresses
  - **Expected**: FAIL (utxo_manager.rs not yet fixed)
  - **Dependencies**: T010 (address normalization)

### Frontend Tests (Progressive Disclosure)

- [ ] **T012** [P] Frontend test for error display in `btpc-desktop-app/ui/tests/error_display.spec.js`
  - **Files**: `btpc-desktop-app/ui/tests/error_display.spec.js` (NEW)
  - **Tests**: Error displays user message with "Show Details" button
  - **Expected**: FAIL (UI not yet updated)
  - **Dependencies**: None

- [ ] **T013** [P] Frontend test for details expansion in `btpc-desktop-app/ui/tests/error_details_expansion.spec.js`
  - **Files**: `btpc-desktop-app/ui/tests/error_details_expansion.spec.js` (NEW)
  - **Tests**: Clicking "Show Details" reveals technical information
  - **Expected**: FAIL (details expansion not yet implemented)
  - **Dependencies**: None

### Integration Tests (Article XI)

- [ ] **T014** [P] Integration test for state synchronization in `btpc-desktop-app/tests/integration/state_consistency.spec.js`
  - **Files**: `btpc-desktop-app/tests/integration/state_consistency.spec.js` (NEW)
  - **Tests**: State updates propagate across pages in < 200ms (Article XI, Section 11.3)
  - **Expected**: FAIL (StateManager not yet implemented)
  - **Dependencies**: T003 (state_management.rs structure)

- [ ] **T015** [P] Integration test for event listener cleanup in `btpc-desktop-app/tests/integration/event_cleanup.spec.js`
  - **Files**: `btpc-desktop-app/tests/integration/event_cleanup.spec.js` (NEW)
  - **Tests**: Event listeners cleaned up on page unload (Article XI, Section 11.6)
  - **Expected**: FAIL (cleanup not yet implemented)
  - **Dependencies**: None

---

## Phase 3.3: Core Implementation (ONLY after tests are failing)

### Error Handling Implementation

- [ ] **T016** Implement `AppError` enum in `btpc-desktop-app/src-tauri/src/error.rs`
  - **Files**: `btpc-desktop-app/src-tauri/src/error.rs` (MODIFY)
  - **Implementation**: Add variants: ProcessCrash, ValidationFailure, DatabaseLock, RPCTimeout, MutexPoison
  - **Includes**: `impl From<AppError> for String` for Tauri compatibility
  - **Tests**: T006, T007 should now PASS
  - **Dependencies**: T006, T007 (tests must exist and fail first)

- [ ] **T017** Add error sanitization function in `btpc-desktop-app/src-tauri/src/error.rs`
  - **Files**: `btpc-desktop-app/src-tauri/src/error.rs` (MODIFY)
  - **Implementation**: Regex-based redaction of private keys, passwords in technical details
  - **Pattern**: `(priv_key|private_key|password|secret):\s*\S+` → `$1: [REDACTED]`
  - **Tests**: T007 should now PASS
  - **Dependencies**: T016 (AppError enum)

### State Management Implementation (Article XI)

- [ ] **T018** Implement `StateManager<T>` wrapper in `btpc-desktop-app/src-tauri/src/state_management.rs`
  - **Files**: `btpc-desktop-app/src-tauri/src/state_management.rs` (MODIFY)
  - **Implementation**: Generic wrapper with `update()` and `get()` methods auto-emitting Tauri events
  - **Article XI**: Section 11.3 (event emission), Section 11.1 (single source of truth)
  - **Tests**: T014 should now PASS
  - **Dependencies**: T014 (test must exist and fail first)

- [ ] **T019** Replace `Arc<Mutex<NodeStatus>>` with `StateManager<NodeStatus>` in `btpc-desktop-app/src-tauri/src/main.rs`
  - **Files**: `btpc-desktop-app/src-tauri/src/main.rs` (MODIFY)
  - **Implementation**: Update `AppState` struct, replace `.lock().unwrap()` calls with `.update()`
  - **Scope**: Lines ~50-100 (AppState definition), lines ~500-800 (node_status usage)
  - **Article XI**: Section 11.1 (backend authoritative state)
  - **Dependencies**: T018 (StateManager implementation)

- [ ] **T020** Replace `mining_processes: Arc<Mutex<...>>` with `StateManager<MiningProcesses>` in `btpc-desktop-app/src-tauri/src/main.rs`
  - **Files**: `btpc-desktop-app/src-tauri/src/main.rs` (MODIFY)
  - **Implementation**: Update mining-related Tauri commands to use `.update()` instead of `.lock().unwrap()`
  - **Scope**: Lines ~456, ~586, ~715, ~750, ~814, ~818, ~830, ~843 (mutex usage points from audit)
  - **Article XI**: Section 11.3 (automatic event emission)
  - **Dependencies**: T018 (StateManager implementation), T019 (pattern established)

### Process Management Implementation

- [ ] **T021** Implement `ProcessHandle` struct in `btpc-desktop-app/src-tauri/src/process_health.rs`
  - **Files**: `btpc-desktop-app/src-tauri/src/process_health.rs` (MODIFY)
  - **Implementation**: Fields from data-model.md, state transitions, health check loop
  - **Includes**: `shutdown_gracefully()` async method with SIGTERM + 10s timeout
  - **Tests**: T008 should now PASS
  - **Dependencies**: T008 (test must exist and fail first)

- [ ] **T022** Add crash tracking and auto-restart logic in `btpc-desktop-app/src-tauri/src/process_health.rs`
  - **Files**: `btpc-desktop-app/src-tauri/src/process_health.rs` (MODIFY)
  - **Implementation**:
    - First crash (crash_count == 0): auto-restart
    - Second crash (crash_count > 0): emit event for user notification
    - Reset crash_count after 1 hour stable
  - **Tests**: T009 should now PASS
  - **Dependencies**: T021 (ProcessHandle struct), T009 (test)

- [ ] **T023** Replace `std::mem::forget()` with tokio process reaping in `btpc-desktop-app/src-tauri/src/process_manager.rs`
  - **Files**: `btpc-desktop-app/src-tauri/src/process_manager.rs` (MODIFY)
  - **Implementation**: Replace `std::process::Command` with `tokio::process::Command`
  - **Scope**: Line 95 (`std::mem::forget(child)` → remove), use `tokio::process::Child` with proper Drop
  - **Tests**: Manual verification (no zombies after app close)
  - **Dependencies**: T021 (ProcessHandle structure)

- [ ] **T024** Implement graceful shutdown for all processes in `btpc-desktop-app/src-tauri/src/process_manager.rs`
  - **Files**: `btpc-desktop-app/src-tauri/src/process_manager.rs` (MODIFY)
  - **Implementation**:
    - Send SIGTERM
    - Wait 10 seconds using `tokio::time::timeout`
    - Log if timeout (process already reaped by Drop)
  - **Tests**: quickstart.md Scenario 3 (node shutdown < 10s)
  - **Dependencies**: T023 (tokio process integration)

### File Locking Implementation

- [ ] **T025** Implement `LockFile` wrapper in `btpc-desktop-app/src-tauri/src/lock_manager.rs`
  - **Files**: `btpc-desktop-app/src-tauri/src/lock_manager.rs` (MODIFY)
  - **Implementation**: Safe wrapper around `fs2::FileExt::try_lock_exclusive()`
  - **Includes**: `Drop` implementation for automatic unlock, stale lock detection
  - **Dependencies**: T005 (module structure)

- [ ] **T026** Replace `libc::flock()` usage in `btpc-desktop-app/src-tauri/src/main.rs`
  - **Files**: `btpc-desktop-app/src-tauri/src/main.rs` (MODIFY)
  - **Implementation**: Replace lines 2699-2700 with `LockFile::try_acquire()`
  - **Error Handling**: Return proper error instead of panicking on lock failure
  - **Dependencies**: T025 (LockFile implementation)

### Balance Calculation Fix

- [ ] **T027** Implement address normalization in `btpc-desktop-app/src-tauri/src/utxo_manager.rs`
  - **Files**: `btpc-desktop-app/src-tauri/src/utxo_manager.rs` (MODIFY)
  - **Implementation**: Add `normalize_address(addr: &str) -> String { addr.to_lowercase() }`
  - **Tests**: T010 should now PASS
  - **Dependencies**: T010 (test must exist and fail first)

- [ ] **T028** Add custom deserializer for case-insensitive UTXO map in `btpc-desktop-app/src-tauri/src/utxo_manager.rs`
  - **Files**: `btpc-desktop-app/src-tauri/src/utxo_manager.rs` (MODIFY)
  - **Implementation**: `deserialize_utxo_map<'de, D>` function from research.md
  - **Scope**: Lines ~258-271 (HashMap filter logic that's currently broken)
  - **Tests**: T011 should now PASS (226.625 BTP balance displays correctly)
  - **Dependencies**: T027 (normalize_address function)

### Mutex Unwrap Elimination (Main.rs Refactor)

- [ ] **T029** Phase 1: Replace mutex unwraps in node management commands (main.rs lines 456-494)
  - **Files**: `btpc-desktop-app/src-tauri/src/main.rs` (MODIFY)
  - **Implementation**: Replace `.lock().unwrap()` with `.map_err(|e| AppError::MutexPoison(...))?`
  - **Scope**: `start_node`, `stop_node`, `get_node_status` commands
  - **Count**: ~5 unwrap instances
  - **Dependencies**: T016 (AppError enum)

- [ ] **T030** Phase 2: Replace mutex unwraps in mining commands (main.rs lines 586-843)
  - **Files**: `btpc-desktop-app/src-tauri/src/main.rs` (MODIFY)
  - **Implementation**: Replace `.lock().unwrap()` with `.map_err(|e| AppError::MutexPoison(...))?`
  - **Scope**: `start_mining`, `stop_mining`, `get_mining_stats` commands
  - **Count**: ~15 unwrap instances
  - **Dependencies**: T029 (pattern established), T020 (StateManager for mining)

- [ ] **T031** Phase 3: Replace mutex unwraps in wallet commands (main.rs lines 910-1237)
  - **Files**: `btpc-desktop-app/src-tauri/src/main.rs` (MODIFY)
  - **Implementation**: Replace `.lock().unwrap()` with `.map_err(|e| AppError::MutexPoison(...))?`
  - **Scope**: `create_wallet`, `load_wallet`, `get_balance` commands
  - **Count**: ~20 unwrap instances
  - **Dependencies**: T030 (continuation of refactor)

- [ ] **T032** Phase 4: Replace remaining mutex unwraps (main.rs lines 1437-2746)
  - **Files**: `btpc-desktop-app/src-tauri/src/main.rs` (MODIFY)
  - **Implementation**: Replace `.lock().unwrap()` in health check, cleanup, shutdown handlers
  - **Scope**: Lines 1437, 1447, 1567, 1573, 1585, 1602, 1636, 1676, 1844, 1900, 1912, 1933, 1948, 2157, 2185, 2198
  - **Count**: ~15 unwrap instances
  - **Dependencies**: T031 (final phase of main.rs refactor)

---

## Phase 3.4: Frontend Integration (After Backend Complete)

### Progressive Disclosure Error UI

- [ ] **T033** Add error display HTML to `btpc-desktop-app/ui/password-modal.html`
  - **Files**: `btpc-desktop-app/ui/password-modal.html` (MODIFY)
  - **Implementation**: Add `<div class="error-message">` with `<details>` element per research.md
  - **Includes**: User message, collapsible technical details, copy button
  - **Tests**: T012 should now PASS
  - **Dependencies**: T012 (test exists)

- [ ] **T034** Add error display CSS to `btpc-desktop-app/ui/btpc-styles.css`
  - **Files**: `btpc-desktop-app/ui/btpc-styles.css` (MODIFY)
  - **Implementation**: Styles for `.error-message`, `.error-details`, `.error-technical-pre` from research.md
  - **Includes**: Monospace font for `<pre>` (NFR-020)
  - **Dependencies**: T033 (HTML structure)

- [ ] **T035** Create error handler module in `btpc-desktop-app/ui/btpc-error-handler.js`
  - **Files**: `btpc-desktop-app/ui/btpc-error-handler.js` (NEW)
  - **Implementation**:
    - `displayError(errorState)` function
    - Tauri `error_occurred` event listener
    - Clipboard copy using `window.__TAURI__.clipboard.writeText()`
  - **Tests**: T012, T013 should now PASS
  - **Dependencies**: T033 (HTML), T034 (CSS)

### Mining Resume Notification

- [ ] **T036** Add mining resume notification HTML to `btpc-desktop-app/ui/mining.html`
  - **Files**: `btpc-desktop-app/ui/mining.html` (MODIFY)
  - **Implementation**: Add notification banner with "Resume Mining" and "Don't Resume" buttons
  - **Spec**: FR-044, FR-045
  - **Dependencies**: None

- [ ] **T037** Implement mining resume detection in `btpc-desktop-app/src-tauri/src/wallet_manager.rs`
  - **Files**: `btpc-desktop-app/src-tauri/src/wallet_manager.rs` (MODIFY)
  - **Implementation**: Replace TODO macro (line 567) with actual `was_active_on_close` detection
  - **Saves**: `~/.btpc/mining_state.json` with `{ "was_active_on_close": true }`
  - **Spec**: FR-043
  - **Dependencies**: T036 (UI structure)

- [ ] **T038** Wire up mining resume buttons in `btpc-desktop-app/ui/mining.html`
  - **Files**: `btpc-desktop-app/ui/mining.html` (MODIFY)
  - **Implementation**: Button click handlers calling Tauri `start_mining()` or dismissing notification
  - **Article XI**: Backend-first (calls Tauri command, waits for response)
  - **Dependencies**: T037 (backend detection logic)

### Node Auto-Start Preference

- [ ] **T039** Add first-launch modal to `btpc-desktop-app/ui/settings.html`
  - **Files**: `btpc-desktop-app/ui/settings.html` (MODIFY)
  - **Implementation**: Modal asking "Auto-start node on launch?" with Yes/No buttons
  - **Spec**: FR-041
  - **Dependencies**: None

- [ ] **T040** Implement auto-start preference persistence in `btpc-desktop-app/src-tauri/src/main.rs`
  - **Files**: `btpc-desktop-app/src-tauri/src/main.rs` (MODIFY)
  - **Implementation**:
    - Check `~/.btpc/node_config.json` on startup
    - If missing, show modal (FR-041)
    - Save choice to file (FR-042)
  - **Spec**: FR-042
  - **Article XI**: Backend validates before localStorage (Section 11.2)
  - **Dependencies**: T039 (UI modal)

### Event Listener Cleanup

- [ ] **T041** Track active listeners in `btpc-desktop-app/ui/btpc-event-manager.js`
  - **Files**: `btpc-desktop-app/ui/btpc-event-manager.js` (MODIFY)
  - **Implementation**:
    - `registerListener(eventName, handler)` returns unlisten function
    - Store unlisten functions in array
    - `cleanupAllListeners()` calls all unlisten functions
  - **Article XI**: Section 11.6 (cleanup on unload)
  - **Dependencies**: None

- [ ] **T042** Add beforeunload cleanup to all pages
  - **Files**: `btpc-desktop-app/ui/index.html`, `mining.html`, `node.html`, `wallet-manager.html`, `settings.html` (MODIFY)
  - **Implementation**: Add `window.addEventListener('beforeunload', cleanupAllListeners)`
  - **Scope**: 5 HTML files
  - **Tests**: T015 should now PASS
  - **Dependencies**: T041 (event manager), T015 (test)

---

## Phase 3.5: Integration Tests & Validation

- [ ] **T043** [P] Integration test for balance display in `btpc-desktop-app/tests/integration/balance_display.spec.js`
  - **Files**: `btpc-desktop-app/tests/integration/balance_display.spec.js` (NEW)
  - **Tests**: quickstart.md Scenario 1 (226.625 BTP displays correctly)
  - **Automates**: Manual test procedure from quickstart.md
  - **Dependencies**: T028 (balance fix complete)

- [ ] **T044** [P] Integration test for mining operations in `btpc-desktop-app/tests/integration/mining_operations.spec.js`
  - **Files**: `btpc-desktop-app/tests/integration/mining_operations.spec.js` (NEW)
  - **Tests**: quickstart.md Scenario 2 (mining error handling, progressive disclosure)
  - **Automates**: Start mining → trigger error → verify UI response
  - **Dependencies**: T035 (error handler complete)

- [ ] **T045** [P] Integration test for error recovery in `btpc-desktop-app/tests/integration/error_recovery.spec.js`
  - **Files**: `btpc-desktop-app/tests/integration/error_recovery.spec.js` (NEW)
  - **Tests**: quickstart.md Scenario 5 (graceful failure, no cascade crash)
  - **Automates**: Kill node process → verify app remains responsive
  - **Dependencies**: T016 (AppError), T024 (graceful shutdown)

- [ ] **T046** [P] Integration test for crash recovery in `btpc-desktop-app/tests/integration/crash_recovery.spec.js`
  - **Files**: `btpc-desktop-app/tests/integration/crash_recovery.spec.js` (NEW)
  - **Tests**: quickstart.md Scenario 7 (auto-restart first crash, notification second crash)
  - **Automates**: Kill process twice → verify auto-restart then notification
  - **Dependencies**: T022 (crash tracking complete)

---

## Phase 3.6: Stress Testing & Performance

- [ ] **T047** Create 7-day memory leak test script in `btpc-desktop-app/tests/stress/seven_day_test.sh`
  - **Files**: `btpc-desktop-app/tests/stress/seven_day_test.sh` (NEW)
  - **Implementation**: Script from research.md (samples memory every 5 minutes)
  - **Spec**: NFR-017 (< 5% growth over 7 days)
  - **Dependencies**: All implementation complete (T001-T042)

- [ ] **T048** Create memory analysis script in `btpc-desktop-app/tests/stress/analyze_memory_log.py`
  - **Files**: `btpc-desktop-app/tests/stress/analyze_memory_log.py` (NEW)
  - **Implementation**: Python script from research.md (calculates growth percentage)
  - **Pass Criteria**: Growth < 5%
  - **Dependencies**: T047 (test script)

- [ ] **T049** [P] Performance benchmark for balance calculation in `btpc-desktop-app/benches/bench_balance_calc.rs`
  - **Files**: `btpc-desktop-app/benches/bench_balance_calc.rs` (NEW)
  - **Tests**: Balance calculation with 10,000 UTXOs completes in < 500ms (NFR-005)
  - **Uses**: criterion crate
  - **Dependencies**: T028 (balance calculation fix)

- [ ] **T050** [P] Performance benchmark for state sync in `btpc-desktop-app/benches/bench_state_sync.rs`
  - **Files**: `btpc-desktop-app/benches/bench_state_sync.rs` (NEW)
  - **Tests**: Backend state change → frontend event received in < 200ms (NFR-007)
  - **Article XI**: Section 11.3 compliance verification
  - **Dependencies**: T018 (StateManager)

---

## Phase 3.7: Polish & Documentation

### Code Quality

- [ ] **T051** Run `cargo clippy -- -D warnings` and fix all issues
  - **Command**: `cd btpc-desktop-app/src-tauri && cargo clippy -- -D warnings`
  - **Fixes**: All clippy warnings across modified files
  - **Expected**: Zero warnings
  - **Dependencies**: All implementation complete (T001-T042)

- [ ] **T052** [P] Add inline documentation (/// comments) to all new public APIs
  - **Files**: All NEW files from T002-T005, T025, T035, T041
  - **Scope**: `error.rs`, `state_management.rs`, `process_health.rs`, `lock_manager.rs`, `btpc-error-handler.js`
  - **Style**: Rust doc comments with examples
  - **Dependencies**: T051 (clippy clean)

- [ ] **T053** [P] Add unit tests for edge cases in `btpc-desktop-app/src-tauri/tests/unit/`
  - **Files**:
    - `tests/unit/test_error_sanitization_edge_cases.rs` (NEW)
    - `tests/unit/test_crash_counter_edge_cases.rs` (NEW)
    - `tests/unit/test_address_normalization_unicode.rs` (NEW)
  - **Coverage Target**: >90%
  - **Dependencies**: T051 (implementation complete)

### Article XI Compliance Verification

- [ ] **T054** Verify backend-first validation in all settings (Article XI, Section 11.2)
  - **Manual Check**: Review `settings.html`, `mining.html` for localStorage writes BEFORE Tauri invoke
  - **Expected**: Zero instances (all validate with backend first)
  - **References**: Spec lines 268-271 (Section 11.2)
  - **Dependencies**: T040, T042 (all frontend integration complete)

- [ ] **T055** Verify no duplicate toast notifications (Article XI, Section 11.7)
  - **Manual Check**: Test all user actions (start node, start mining) for duplicate toasts
  - **Expected**: Single toast per action
  - **References**: Spec line 287 (Section 11.7 prohibited patterns)
  - **Dependencies**: T038, T040 (notification UI complete)

- [ ] **T056** Cross-page state synchronization test (Article XI, Section 11.3)
  - **Manual Check**: Open Dashboard + Mining pages side-by-side, trigger state change
  - **Expected**: Both pages update within 200ms
  - **Tests**: T014 automated test should also PASS
  - **Dependencies**: T020 (StateManager in use), T014 (test exists)

### Security & Final Validation

- [ ] **T057** Verify all error messages sanitize sensitive data
  - **Manual Check**: Trigger errors with private keys/passwords in system state
  - **Expected**: Technical details show `[REDACTED]` not actual secrets
  - **Spec**: NFR-003, NFR-004
  - **Dependencies**: T017 (sanitization implemented)

- [ ] **T058** Verify no unsafe code without SAFETY comments
  - **Command**: `rg 'unsafe' btpc-desktop-app/src-tauri/src/ | grep -v '// SAFETY:'`
  - **Expected**: Zero results (all unsafe blocks documented)
  - **Spec**: Memory Safety Gate
  - **Dependencies**: T051 (all code complete)

- [ ] **T059** Run full test suite and verify >90% coverage
  - **Command**: `cd btpc-desktop-app/src-tauri && cargo test --all && cargo tarpaulin`
  - **Expected**: All tests pass, coverage > 90%
  - **Dependencies**: All tests complete (T006-T015, T043-T046, T049-T050, T053)

- [ ] **T060** Execute quickstart.md manual test procedure (all 8 scenarios)
  - **File**: `specs/003-fix-node-and/quickstart.md`
  - **Duration**: ~2 hours
  - **Validates**: All acceptance scenarios from spec.md
  - **Dependencies**: T059 (automated tests pass)

---

## Dependencies Graph

**Setup Phase** (can run in parallel):
```
T001 (deps) → T002 [P], T003 [P], T004 [P]
T001 (deps) → T005
```

**Test Phase** (must complete before implementation):
```
T002 → T006 [P], T007 [P]
T004 → T008 [P], T009 [P]
T003 → T014 [P]
T010 [P], T011 [P], T012 [P], T013 [P], T015 [P]
```

**Implementation Phase** (dependencies flow):
```
T006, T007 → T016 → T017
T008, T009 → T021 → T022 → T023 → T024
T014 → T018 → T019 → T020
T005 → T025 → T026
T010 → T027 → T028
T016 → T029 → T030 → T031 → T032
```

**Frontend Phase** (after backend):
```
T012 → T033 → T034 → T035
T036 → T037 → T038
T039 → T040
T015 → T041 → T042
```

**Integration & Validation** (parallel after implementation):
```
T028 → T043 [P]
T035 → T044 [P]
T024 → T045 [P]
T022 → T046 [P]
T028 → T049 [P]
T018 → T050 [P]
All complete → T047 → T048
```

**Polish** (sequential verification):
```
All implementation → T051 → T052 [P], T053 [P]
T042 → T054, T055, T056
T051 → T057, T058 → T059 → T060
```

---

## Parallel Execution Examples

### Setup Phase (T002-T005)
```bash
# Run in parallel - all create new files:
Task: "Create error module structure in btpc-desktop-app/src-tauri/src/error.rs"
Task: "Create state management module in btpc-desktop-app/src-tauri/src/state_management.rs"
Task: "Create process health monitor module in btpc-desktop-app/src-tauri/src/process_health.rs"
Task: "Create lock manager module in btpc-desktop-app/src-tauri/src/lock_manager.rs"
```

### Test Phase (T006-T015)
```bash
# Run in parallel - all create new test files:
Task: "Contract test for AppError serialization in btpc-desktop-app/src-tauri/tests/contract_error_state.rs"
Task: "Contract test for error sanitization in btpc-desktop-app/src-tauri/tests/contract_error_sanitization.rs"
Task: "Contract test for ProcessHandle lifecycle in btpc-desktop-app/src-tauri/tests/contract_process_handle.rs"
Task: "Contract test for crash count reset in btpc-desktop-app/src-tauri/tests/contract_crash_tracking.rs"
Task: "Unit test for address normalization in btpc-desktop-app/src-tauri/tests/test_address_normalization.rs"
Task: "Integration test for balance calculation in btpc-desktop-app/src-tauri/tests/test_balance_calculation.rs"
Task: "Frontend test for error display in btpc-desktop-app/ui/tests/error_display.spec.js"
Task: "Frontend test for details expansion in btpc-desktop-app/ui/tests/error_details_expansion.spec.js"
Task: "Integration test for state synchronization in btpc-desktop-app/tests/integration/state_consistency.spec.js"
Task: "Integration test for event listener cleanup in btpc-desktop-app/tests/integration/event_cleanup.spec.js"
```

### Integration Tests (T043-T046)
```bash
# Run in parallel - all different test files:
Task: "Integration test for balance display in btpc-desktop-app/tests/integration/balance_display.spec.js"
Task: "Integration test for mining operations in btpc-desktop-app/tests/integration/mining_operations.spec.js"
Task: "Integration test for error recovery in btpc-desktop-app/tests/integration/error_recovery.spec.js"
Task: "Integration test for crash recovery in btpc-desktop-app/tests/integration/crash_recovery.spec.js"
```

---

## Notes

- **[P]** tasks = different files, can run in parallel
- **TDD Mandatory**: All contract/integration tests (T006-T015) MUST fail before implementation (T016+)
- **Commit Strategy**: Commit after each task with descriptive message referencing task number
- **Article XI Compliance**: Tasks T018-T020, T033-T042, T054-T056 enforce constitutional patterns
- **Security**: Task T057 validates NFR-003, NFR-004 (no sensitive data exposure)
- **Performance**: Tasks T049-T050 validate NFR-005, NFR-007 benchmarks
- **7-Day Test**: Task T047-T048 validates NFR-017 (< 5% memory growth)

---

## Estimated Timeline

**Phase 3.1-3.2** (Setup + Tests): 2-3 days
**Phase 3.3** (Core Implementation): 4-5 days
**Phase 3.4** (Frontend Integration): 2 days
**Phase 3.5-3.6** (Integration + Stress Tests): 2 days (+ 7-day background test)
**Phase 3.7** (Polish): 1 day

**Total**: ~11-13 days + 7-day stress test running in background

---

**Template Version**: 1.1 (BTPC-specific)
**Last Updated**: 2025-10-25
**Feature**: 003-fix-node-and
**Constitution**: v1.0.1 - Article XI Compliance Mandatory
