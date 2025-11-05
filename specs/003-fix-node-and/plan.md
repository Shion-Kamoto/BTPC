# Implementation Plan: Node and Backend Stability Fixes

**Branch**: `003-fix-node-and` | **Date**: 2025-10-25 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/home/bob/BTPC/BTPC/specs/003-fix-node-and/spec.md`

## Execution Flow (/plan command scope)
```
1. Load feature spec from Input path → COMPLETE
2. Fill Technical Context → COMPLETE
3. Fill Constitution Check section → COMPLETE
4. Evaluate Constitution Check section → COMPLETE
   → No constitutional violations found
   → Update Progress Tracking: Initial Constitution Check
5. Execute Phase 0 → research.md → COMPLETE
6. Execute Phase 1 → contracts, data-model.md, quickstart.md, CLAUDE.md → COMPLETE
7. Re-evaluate Constitution Check section → COMPLETE (no violations)
8. Plan Phase 2 → Describe task generation approach → COMPLETE
9. STOP - Ready for /tasks command → COMPLETE
```

## Summary

This feature addresses critical stability and reliability issues in the btpc-desktop-app affecting production readiness. The primary problems are:

1. **Mutex Poison Vulnerabilities**: 50+ instances of `.unwrap()` on mutex locks causing cascading crashes
2. **UTXO Balance Calculation Bug**: App displays 0.00 BTP despite database showing 226.625 BTP across 7 UTXOs
3. **Memory Leaks**: Zombie process accumulation from `std::mem::forget()` usage and missing cleanup
4. **Process Management**: Unsafe child process handling, lack of graceful shutdown
5. **Error Handling**: Panics instead of graceful degradation, poor user-facing error messages

**Technical Approach**: Systematic hardening of backend Rust code (src-tauri/) focusing on error propagation, resource cleanup, and Article XI compliance for state management. Fix balance calculation logic, implement proper process lifecycle management, add progressive disclosure error UI.

## Technical Context

**Language/Version**: Rust 1.75+ (backend), JavaScript ES6+ (frontend)
**Primary Dependencies**:
- Tauri 2.0 (IPC framework)
- tokio 1.35+ (async runtime)
- serde 1.0+ (serialization)
- RocksDB (blockchain state via btpc_node RPC)
- thiserror 1.0+ (error handling)
**Storage**:
- Backend: RocksDB via JSON-RPC to btpc_node (blockchain data)
- Frontend: localStorage (user preferences, UI state)
- File locks: ~/.btpc/*.lock files for single-instance protection
**Testing**:
- Unit tests: cargo test (Rust backend)
- Integration tests: Tauri test harness
- Manual QA: 7-day stress test (FR-021, NFR-009)
**Target Platform**: Linux (primary), Windows/macOS (secondary) - Desktop application
**Project Type**: Desktop (Tauri hybrid - Rust backend + HTML/CSS/JS frontend)
**Performance Goals**:
- Memory: < 1GB RAM usage excluding blockchain data (NFR-008)
- Memory stability: < 5% growth over 7 days (NFR-017)
- Balance calculation: < 500ms for 10k UTXOs (NFR-005)
- State sync: < 200ms backend-to-frontend (NFR-007, Article XI)
- Node shutdown: < 10 seconds graceful (FR-007)
- Mining shutdown: < 5 seconds (FR-015)
**Constraints**:
- Article XI compliance: Backend-first validation, event-driven architecture
- No breaking changes to existing Tauri commands
- Maintain backward compatibility with stored localStorage state
- No new external dependencies without security audit
**Scale/Scope**:
- Single-user desktop application
- Manages 1-3 child processes (node, miner, wallet CLI)
- Supports 10k+ UTXOs per wallet
- 7-day continuous operation requirement
- 1000+ mining start/stop cycles (NFR-012)

## Constitution Check
*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

**Security Gate**: ✅ PASS
- No cryptographic algorithm changes (ML-DSA, SHA-512 unchanged)
- Error messages sanitize sensitive data (NFR-003, NFR-004)
- File path validation prevents path traversal (NFR-001)
- Process argument sanitization prevents injection (NFR-002)
- Lock files use safe operations (NFR-018)

**Testing Gate**: ✅ PASS (with plan)
- Unit tests for all error handling paths
- Integration tests for process lifecycle
- Stress tests for memory leak detection (7-day test)
- Balance calculation regression tests
- >90% coverage target maintained

**Performance Gate**: ✅ PASS
- Balance calculation < 500ms (NFR-005) - meets blockchain requirement
- State sync < 200ms (NFR-007) - meets Article XI requirement
- Memory < 1GB (NFR-008) - within desktop app norms
- No signature/block validation changes (Constitution Article II maintained)

**Memory Safety Gate**: ✅ PASS
- All code in Rust (memory-safe by default)
- Removes unsafe `std::mem::forget()` usage (process_manager.rs:95)
- Removes unsafe `libc::flock()` usage (main.rs:2699-2700)
- Replaces `.unwrap()` with proper Result<T, E> error propagation
- All new unsafe blocks must have SAFETY comments

**Dependency Gate**: ✅ PASS
- No new external dependencies
- Uses existing audited dependencies (Tauri, tokio, serde)
- cargo-audit passes on all existing dependencies

**Desktop App Gate**: ✅ PASS (Article XI)
- Backend remains single source of truth (Section 11.1)
- Backend-first validation before localStorage (Section 11.2)
- Event-driven state updates (Section 11.3)
- Event listener cleanup (Section 11.6)
- No prohibited patterns (Section 11.7)

## Project Structure

### Documentation (this feature)
```
specs/003-fix-node-and/
├── plan.md              # This file (/plan command output)
├── spec.md              # Feature specification (input)
├── research.md          # Phase 0 output (/plan command)
├── data-model.md        # Phase 1 output (/plan command)
├── quickstart.md        # Phase 1 output (/plan command)
├── contracts/           # Phase 1 output (/plan command)
│   ├── error_types.json      # Error state JSON schema
│   ├── process_lifecycle.json # ProcessHandle lifecycle schema
│   └── tauri_events.json     # Event payloads schema
└── tasks.md             # Phase 2 output (/tasks command - NOT created by /plan)
```

### Source Code (repository root)
```
btpc-desktop-app/
├── src-tauri/ (Rust backend)
│   ├── src/
│   │   ├── main.rs                    # MODIFY: Remove 50+ mutex .unwrap() calls
│   │   ├── process_manager.rs         # MODIFY: Fix zombie process leaks
│   │   ├── utxo_manager.rs            # MODIFY: Fix balance calculation bug
│   │   ├── error.rs                   # MODIFY: Add progressive disclosure errors
│   │   ├── btpc_integration.rs        # MODIFY: Add timeout, path validation
│   │   ├── wallet_manager.rs          # MODIFY: Implement TODO macros (FR-043/044)
│   │   ├── state_management.rs        # NEW: Centralize Arc<Mutex<T>> patterns
│   │   ├── process_health.rs          # NEW: Health monitoring, crash tracking
│   │   └── lock_manager.rs            # NEW: Safe file locking wrapper
│   ├── tests/
│   │   ├── mutex_error_handling.rs    # NEW: Test error propagation
│   │   ├── balance_calculation.rs     # NEW: Test UTXO aggregation
│   │   ├── process_lifecycle.rs       # NEW: Test graceful shutdown
│   │   └── stress_tests.rs            # NEW: 7-day memory leak tests
│   └── Cargo.toml                     # MODIFY: Add dev dependencies for testing
│
├── ui/ (Frontend)
│   ├── password-modal.html            # MODIFY: Error progressive disclosure UI
│   ├── password-modal.js              # MODIFY: "Show Details" button handler
│   ├── mining.html                    # MODIFY: Mining resume notification
│   ├── settings.html                  # MODIFY: Node auto-start preference
│   ├── btpc-error-handler.js          # NEW: Centralized error display logic
│   └── btpc-event-manager.js          # MODIFY: Event listener cleanup tracking
│
└── tests/
    ├── integration/
    │   ├── balance_display.spec.js    # NEW: Test FR-001 scenario
    │   ├── mining_operations.spec.js  # NEW: Test FR-011-015 scenarios
    │   └── error_recovery.spec.js     # NEW: Test FR-016-020 scenarios
    └── stress/
        └── seven_day_test.sh          # NEW: Memory leak detection script
```

**Structure Decision**: Desktop Application (Tauri-based)
- Backend: Rust (src-tauri/src/) handles all state, process management, RPC communication
- Frontend: HTML/CSS/JS (ui/) displays state, handles user input, forwards to backend
- This is an existing application requiring stability fixes, not greenfield development
- Structure aligns with Article XI: backend authoritative, frontend presentational

## Phase 0: Outline & Research
*Output: research.md with all technical decisions documented*

### Research Topics

1. **Rust Error Handling Best Practices**
   - **Unknown**: How to convert 50+ `.unwrap()` calls to proper error propagation without breaking existing Tauri commands
   - **Research Task**: Investigate `thiserror` patterns for Tauri Result<T, String> returns
   - **Decision Needed**: Error type hierarchy (AppError enum vs multiple error types)
   - **Alternatives**: anyhow (too generic), custom error structs (too verbose)

2. **Process Management Patterns**
   - **Unknown**: How to properly reap zombie processes without `std::mem::forget()`
   - **Research Task**: Investigate `nix` crate for POSIX signal handling, tokio::process for async reaping
   - **Decision Needed**: Reaper thread vs async task vs signal handler
   - **Alternatives**: libc (unsafe), std::process (no reaping), third-party process supervisor

3. **UTXO Balance Calculation Fix**
   - **Unknown**: Root cause of HashMap filter logic failure (utxo_manager.rs:258-271)
   - **Research Task**: Analyze address normalization (case sensitivity, prefix handling)
   - **Decision Needed**: Address comparison strategy (normalize vs case-fold vs exact match)
   - **Known Issue**: Current code likely comparing different address formats

4. **File Locking Safety**
   - **Unknown**: Safe alternative to `libc::flock()` in Rust
   - **Research Task**: Investigate `fs2` crate for cross-platform locking, `nix` for POSIX fcntl
   - **Decision Needed**: fcntl vs flock semantics for single-instance lock
   - **Alternatives**: fs2 (cross-platform), nix (POSIX-only), manual libc (unsafe)

5. **Event-Driven State Updates (Article XI)**
   - **Unknown**: How to ensure all state changes emit events without forgetting any
   - **Research Task**: Design state change tracking (macro annotations, wrapper types)
   - **Decision Needed**: Event emission strategy (manual, trait-based, proc macro)
   - **Alternatives**: Manual emit calls (error-prone), StateManager wrapper (ergonomic), proc macros (complex)

6. **Memory Leak Detection Strategy**
   - **Unknown**: How to validate < 5% memory growth over 7 days (NFR-017)
   - **Research Task**: Investigate memory profiling tools (valgrind, heaptrack, jemalloc stats)
   - **Decision Needed**: Test automation approach for 7-day stress test
   - **Alternatives**: Manual testing (slow), CI-based (expensive), local automation (chosen)

7. **Progressive Disclosure Error UI**
   - **Unknown**: Frontend pattern for "Show Details" button with clipboard copy
   - **Research Task**: Review Tauri clipboard API, error serialization format
   - **Decision Needed**: Error detail format (JSON, plain text, formatted HTML)
   - **Alternatives**: JSON (machine-readable), plain text (copyable), formatted (readable)

**Output**: Create `research.md` consolidating findings from all 7 topics

## Phase 1: Design & Contracts
*Prerequisites: research.md complete*

### 1. Data Model (`data-model.md`)

Extract from specification Key Entities section:

**ErrorState** (FR-048, FR-049, FR-050, NFR-019, NFR-020)
- Fields:
  - `error_type: String` (e.g., "ProcessCrash", "ValidationFailure", "DatabaseLock")
  - `user_message: String` (plain language, actionable)
  - `technical_details: Option<String>` (stack trace, system state)
  - `timestamp: DateTime<Utc>`
  - `affected_component: String` (e.g., "node", "miner", "balance_calc")
  - `crash_count: u32` (for FR-046 tracking)
  - `details_expanded: bool` (UI state, not serialized)
- Validation:
  - `user_message` must not contain private keys, passwords (regex filter)
  - `technical_details` must be sanitized (redact sensitive patterns)
- State Transitions: created → displayed → (optionally) details_expanded → dismissed
- Serialization: Tauri event payload (JSON)

**ProcessHandle** (FR-036, FR-037, FR-038, FR-039, FR-040, FR-046, FR-047)
- Fields:
  - `process_id: u32` (OS PID)
  - `process_type: ProcessType` (enum: Node, Miner, WalletCLI)
  - `start_time: DateTime<Utc>`
  - `command_line: Vec<String>` (for restart)
  - `status: ProcessStatus` (enum: Running, Crashed, Stopped)
  - `crash_count: u32` (reset after 1 hour stable)
  - `last_health_check: DateTime<Utc>`
  - `stdout_buffer: BoundedVec<String>` (max 1000 lines)
  - `stderr_buffer: BoundedVec<String>` (max 1000 lines)
- Validation:
  - `crash_count` increments on crash, resets after 1 hour (3600s) of Running status
  - Health check every 5 seconds (FR-038)
- State Transitions:
  - Running → Crashed (auto-restart once if crash_count == 0)
  - Crashed → Running (after restart)
  - Crashed → Stopped (user cancels restart or crash_count > 0)
  - Running → Stopped (graceful shutdown)
- Cleanup: SIGTERM + 10s timeout → SIGKILL (FR-007)

**NodeStatus** (existing, Article XI)
- Modifications:
  - Add `auto_start_preference: Option<bool>` (FR-041, FR-042)
  - Add `last_crash_time: Option<DateTime<Utc>>` (for crash tracking)
- No structural changes to existing fields

**MiningStatus** (existing, Article XI)
- Modifications:
  - Add `was_active_on_close: bool` (FR-043)
  - Add `last_stop_time: Option<DateTime<Utc>>` (for resume notification)
- No structural changes to existing hashrate, blocks_found fields

**WalletBalance** (existing, needs fix)
- Root Cause Analysis Required:
  - Address normalization issue (case sensitivity)
  - HashMap filter logic (utxo_manager.rs:258-271)
  - JSON deserialization from btpc_wallet RPC
- Fix Approach: Document in research.md after root cause analysis

### 2. API Contracts (`contracts/`)

**Error Types Schema** (`contracts/error_types.json`)
```json
{
  "ErrorState": {
    "type": "object",
    "properties": {
      "error_type": {"type": "string", "enum": ["ProcessCrash", "ValidationFailure", "DatabaseLock", "RPCTimeout", "InsufficientFunds"]},
      "user_message": {"type": "string", "minLength": 1, "maxLength": 200},
      "technical_details": {"type": "string", "nullable": true},
      "timestamp": {"type": "string", "format": "date-time"},
      "affected_component": {"type": "string"},
      "crash_count": {"type": "integer", "minimum": 0}
    },
    "required": ["error_type", "user_message", "timestamp", "affected_component", "crash_count"]
  }
}
```

**Tauri Events Schema** (`contracts/tauri_events.json`)
```json
{
  "events": {
    "error_occurred": {
      "payload": {"$ref": "#/ErrorState"}
    },
    "process_status_changed": {
      "payload": {
        "process_type": {"type": "string", "enum": ["node", "miner", "wallet"]},
        "status": {"type": "string", "enum": ["running", "stopped", "crashed"]},
        "crash_count": {"type": "integer"}
      }
    },
    "balance_updated": {
      "payload": {
        "balance": {"type": "string", "pattern": "^[0-9]+\\.[0-9]{8}$"},
        "utxo_count": {"type": "integer", "minimum": 0}
      }
    }
  }
}
```

**Process Lifecycle Schema** (`contracts/process_lifecycle.json`)
```json
{
  "ProcessHandle": {
    "type": "object",
    "properties": {
      "process_id": {"type": "integer", "minimum": 1},
      "process_type": {"type": "string", "enum": ["Node", "Miner", "WalletCLI"]},
      "status": {"type": "string", "enum": ["Running", "Crashed", "Stopped"]},
      "crash_count": {"type": "integer", "minimum": 0, "maximum": 100},
      "start_time": {"type": "string", "format": "date-time"}
    },
    "required": ["process_id", "process_type", "status", "crash_count", "start_time"]
  }
}
```

### 3. Contract Tests (TDD - must fail initially)

**Test File**: `src-tauri/tests/contract_error_state.rs`
```rust
#[test]
fn test_error_state_serialization() {
    // Test ErrorState → JSON matches schema
    // EXPECTED: FAIL (ErrorState not yet implemented)
}

#[test]
fn test_error_sanitization() {
    // Test private keys/passwords redacted
    // EXPECTED: FAIL (sanitization not yet implemented)
}
```

**Test File**: `src-tauri/tests/contract_process_handle.rs`
```rust
#[test]
fn test_process_handle_lifecycle() {
    // Test Running → Crashed → Running transition
    // EXPECTED: FAIL (ProcessHandle not yet implemented)
}

#[test]
fn test_crash_count_reset() {
    // Test crash_count resets after 1 hour stable
    // EXPECTED: FAIL (crash tracking not yet implemented)
}
```

**Test File**: `ui/tests/error_display.spec.js`
```javascript
test('error displays user message with Show Details button', async () => {
  // Test progressive disclosure UI
  // EXPECTED: FAIL (UI not yet updated)
});

test('Show Details reveals technical information', async () => {
  // Test details expansion
  // EXPECTED: FAIL (details expansion not yet implemented)
});
```

### 4. Integration Test Scenarios (from User Stories)

**Scenario 1: Wallet Balance Display** (spec.md lines 63-67)
```gherkin
Given user has 226.625 BTP across 7 UTXOs
When user opens desktop application and navigates to wallet page
Then application displays correct balance of 226.625 BTP
And balance updates automatically when new transactions are received
```
**Test File**: `tests/integration/balance_display.spec.js`

**Scenario 2: Mining Operations** (spec.md lines 69-74)
```gherkin
Given user starts mining through desktop application
When mining process encounters an error
Then application shows actionable error message
And application remains responsive and does not crash
And user can restart mining without restarting application
```
**Test File**: `tests/integration/mining_operations.spec.js`

**Scenario 5: Error Recovery** (spec.md lines 90-96)
```gherkin
Given desktop application running with active mining
When backend encounters unexpected error (RPC timeout, database lock)
Then application displays specific error message to user
And affected operation fails gracefully
And other application features remain functional
And application does not crash or freeze
```
**Test File**: `tests/integration/error_recovery.spec.js`

### 5. Quickstart Test (`quickstart.md`)

Create manual test procedure validating all 6 acceptance scenarios:
- Start app → Verify balance display
- Start/stop mining → Verify error handling
- Close app → Verify graceful shutdown
- Run 7-day stress test → Verify memory stability
- Trigger errors → Verify progressive disclosure
- Navigate pages → Verify state consistency

### 6. Update Agent Context

Execute:
```bash
.specify/scripts/bash/update-agent-context.sh claude
```

This will update `CLAUDE.md` with:
- NEW: Error handling patterns (Result<T, AppError>)
- NEW: Process lifecycle management patterns
- NEW: Article XI event emission requirements
- NEW: Memory leak testing procedures
- Keep existing: Project structure, constitution references, recent changes

## Phase 2: Task Planning Approach
*This section describes what the /tasks command will do - DO NOT execute during /plan*

**Task Generation Strategy**:
1. Load `.specify/templates/tasks-template.md` as base
2. Generate tasks from Phase 1 artifacts:
   - Each contract test → one task (TDD red phase)
   - Each data model entity → one implementation task (TDD green phase)
   - Each integration test scenario → one task (TDD integration)
   - Refactoring tasks for main.rs, process_manager.rs, utxo_manager.rs
3. Order tasks by dependency graph:
   - Data models before contract tests
   - Contract tests before implementation
   - Error handling before process lifecycle (dependency)
   - Backend before frontend (Article XI)
4. Mark [P] for parallelizable tasks (independent modules)

**Ordering Strategy**:
1. **Foundation** (sequential):
   - Research completion validation
   - Error type definition (ErrorState enum)
   - Process handle definition (ProcessHandle struct)

2. **Contract Tests** (parallel) [P]:
   - Error state serialization tests
   - Process lifecycle tests
   - Balance calculation tests

3. **Backend Implementation** (mixed):
   - main.rs mutex refactor (sequential, touches many files)
   - process_manager.rs zombie fix [P]
   - utxo_manager.rs balance fix [P]
   - wallet_manager.rs TODO implementation [P]
   - New modules (state_management.rs, process_health.rs, lock_manager.rs) [P]

4. **Frontend Integration** (after backend):
   - Error display UI (password-modal updates)
   - Mining resume notification
   - Node auto-start preference
   - Event listener cleanup

5. **Integration Tests** (after backend + frontend):
   - Balance display scenario
   - Mining operations scenario
   - Error recovery scenario

6. **Validation** (final):
   - 7-day stress test
   - Performance benchmarks
   - Constitution compliance review

**Estimated Output**: 35-40 numbered, ordered tasks in tasks.md
- 10 contract test tasks
- 15 implementation tasks
- 8 frontend tasks
- 5 integration test tasks
- 2 validation tasks

**IMPORTANT**: This phase is executed by the /tasks command, NOT by /plan

## Phase 3+: Future Implementation
*These phases are beyond the scope of the /plan command*

**Phase 3**: Task execution (/tasks command creates tasks.md)
**Phase 4**: Implementation (execute tasks.md following TDD and constitutional principles)
**Phase 5**: Validation (run tests, execute quickstart.md, 7-day stress test, performance validation)

## Complexity Tracking
*No constitutional violations requiring justification*

This feature does not introduce any constitutional deviations:
- Uses existing Rust/Tauri architecture
- No new cryptographic algorithms
- Follows Article XI desktop patterns
- Maintains >90% test coverage
- No new unsafe code (removes existing unsafe code)

## Progress Tracking
*This checklist is updated during execution flow*

**Phase Status**:
- [x] Phase 0: Research complete (/plan command) - research.md created
- [x] Phase 1: Design complete (/plan command) - data-model.md, contracts/, quickstart.md, CLAUDE.md created
- [x] Phase 2: Task planning complete (/plan command - describe approach only)
- [ ] Phase 3: Tasks generated (/tasks command) - NEXT STEP
- [ ] Phase 4: Implementation complete
- [ ] Phase 5: Validation passed

**Gate Status**:
- [x] Initial Constitution Check: PASS
- [x] Post-Design Constitution Check: PASS
- [x] All NEEDS CLARIFICATION resolved (research.md complete)
- [x] Complexity deviations documented (none)

**Artifacts Created**:
- [x] plan.md (this file)
- [x] research.md (7 technical decisions documented)
- [x] data-model.md (6 entities defined with relationships)
- [x] contracts/error_types.json (ErrorState schema)
- [x] contracts/tauri_events.json (5 event payload schemas)
- [x] contracts/process_lifecycle.json (ProcessHandle schema)
- [x] quickstart.md (8 test scenarios + benchmarks)
- [x] CLAUDE.md (agent context updated)

---
*Based on Constitution v1.0.1 - See `.specify/memory/constitution.md`*
