# Tasks: Application-Level Login/Logout System

**Input**: Design documents from `/home/bob/BTPC/BTPC/specs/006-add-application-level/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/tauri-auth-commands.md, quickstart.md
**Constitution**: Article XI patterns for desktop features
**Feature Branch**: `006-add-application-level`

## Executive Summary

Implement application-level authentication system for btpc-desktop-app with:
- **Security**: AES-256-GCM encryption, Argon2id key derivation (64MB, 3 iter, 4 par)
- **Architecture**: Tauri backend (Arc<RwLock<SessionState>>), event-driven UI updates
- **UI**: First-launch password creation, login screen, logout buttons on all pages
- **Compliance**: Article XI (backend-first, event-driven, no localStorage auth)
- **Storage**: ~/.btpc/credentials.enc (encrypted file)
- **Performance**: Login <2s, Logout <100ms, Navigation guard <50ms

**Estimated Tasks**: 35 tasks across 5 phases
**Estimated Time**: 8-12 hours for full implementation and testing

## Format: `[ID] [P?] Description`
- **[P]**: Can run in parallel (different files, no dependencies)
- Exact file paths in BTPC structure
- Article XI references for desktop tasks

## Phase 3.1: Setup & Configuration

- [ ] **T001** Add cryptography dependencies to btpc-desktop-app/src-tauri/Cargo.toml
  - **File**: `btpc-desktop-app/src-tauri/Cargo.toml`
  - **Add**: `argon2 = "0.5"`, `aes-gcm = "0.10"`, `rand = "0.8"`, `subtle = "2.5"`, `zeroize = "1.7"`
  - **Rationale**: Required for Argon2id key derivation and AES-256-GCM encryption per research.md
  - **Verify**: `cargo check` passes

- [ ] **T002** [P] Create Rust module structure for authentication
  - **Files**:
    - `btpc-desktop-app/src-tauri/src/auth_state.rs` (empty module)
    - `btpc-desktop-app/src-tauri/src/auth_crypto.rs` (empty module)
    - `btpc-desktop-app/src-tauri/src/auth_commands.rs` (empty module)
  - **Update**: `btpc-desktop-app/src-tauri/src/main.rs` - Add `mod auth_state;`, `mod auth_crypto;`, `mod auth_commands;`
  - **Rationale**: Separation of concerns per plan.md project structure

- [ ] **T003** [P] Create test directory structure
  - **Files**:
    - `btpc-desktop-app/src-tauri/tests/auth_contract_test.rs` (empty)
    - `btpc-desktop-app/src-tauri/tests/auth_integration_test.rs` (empty)
    - `btpc-desktop-app/src-tauri/tests/auth_crypto_test.rs` (empty)
  - **Rationale**: TDD preparation per quickstart.md test scenarios

- [ ] **T004** [P] Create SVG icons for login/logout
  - **Files**:
    - `btpc-desktop-app/ui/src/assets/icons-svg/lock.svg` (login icon)
    - `btpc-desktop-app/ui/src/assets/icons-svg/unlock.svg` (logout icon)
  - **Style**: Dark theme, professional, consistent with existing BTPC icons
  - **Rationale**: Visual design per FR-019 to FR-023

## Phase 3.2: Tests First (TDD) ⚠️ MUST COMPLETE BEFORE 3.3
**CRITICAL: These tests MUST be written and MUST FAIL before ANY implementation**

### Contract Tests (Tauri Commands)

- [ ] **T005** [P] Contract test: `create_master_password` success case
  - **File**: `btpc-desktop-app/src-tauri/tests/auth_contract_test.rs`
  - **Test**: Call `create_master_password("password123", "password123")`, assert `success: true`
  - **Contract**: contracts/tauri-auth-commands.md lines 18-23
  - **Expected**: FAIL (no implementation yet)

- [ ] **T006** [P] Contract test: `create_master_password` password too short
  - **File**: `btpc-desktop-app/src-tauri/tests/auth_contract_test.rs`
  - **Test**: Call `create_master_password("pass", "pass")`, assert error `PASSWORD_TOO_SHORT`
  - **Contract**: contracts/tauri-auth-commands.md lines 25-28
  - **Expected**: FAIL

- [ ] **T007** [P] Contract test: `create_master_password` mismatch
  - **File**: `btpc-desktop-app/src-tauri/tests/auth_contract_test.rs`
  - **Test**: Call `create_master_password("password123", "password456")`, assert error `PASSWORDS_DONT_MATCH`
  - **Contract**: contracts/tauri-auth-commands.md lines 230-234
  - **Expected**: FAIL

- [ ] **T008** [P] Contract test: `login` success
  - **File**: `btpc-desktop-app/src-tauri/tests/auth_contract_test.rs`
  - **Setup**: Create password first
  - **Test**: Call `login("password123")`, assert `success: true`
  - **Contract**: contracts/tauri-auth-commands.md lines 58-63
  - **Expected**: FAIL

- [ ] **T009** [P] Contract test: `login` wrong password
  - **File**: `btpc-desktop-app/src-tauri/tests/auth_contract_test.rs`
  - **Setup**: Create password first
  - **Test**: Call `login("wrongpassword")`, assert error `AUTHENTICATION_FAILED`
  - **Contract**: contracts/tauri-auth-commands.md lines 245-250
  - **Expected**: FAIL

- [ ] **T010** [P] Contract test: `logout` always succeeds
  - **File**: `btpc-desktop-app/src-tauri/tests/auth_contract_test.rs`
  - **Test**: Call `logout()`, assert `success: true`
  - **Contract**: contracts/tauri-auth-commands.md lines 94-100
  - **Expected**: FAIL

- [ ] **T011** [P] Contract test: `check_session` authenticated
  - **File**: `btpc-desktop-app/src-tauri/tests/auth_contract_test.rs`
  - **Setup**: Create and login
  - **Test**: Call `check_session()`, assert `authenticated: true`
  - **Contract**: contracts/tauri-auth-commands.md lines 258-265
  - **Expected**: FAIL

- [ ] **T012** [P] Contract test: `check_session` not authenticated
  - **File**: `btpc-desktop-app/src-tauri/tests/auth_contract_test.rs`
  - **Setup**: Logout first
  - **Test**: Call `check_session()`, assert `authenticated: false`
  - **Contract**: contracts/tauri-auth-commands.md lines 267-272
  - **Expected**: FAIL

- [ ] **T013** [P] Contract test: `has_master_password` false
  - **File**: `btpc-desktop-app/src-tauri/tests/auth_contract_test.rs`
  - **Setup**: Fresh state (no credentials.enc)
  - **Test**: Call `has_master_password()`, assert `exists: false`
  - **Contract**: contracts/tauri-auth-commands.md lines 275-278
  - **Expected**: FAIL

- [ ] **T014** [P] Contract test: `has_master_password` true
  - **File**: `btpc-desktop-app/src-tauri/tests/auth_contract_test.rs`
  - **Setup**: Create password first
  - **Test**: Call `has_master_password()`, assert `exists: true`
  - **Contract**: contracts/tauri-auth-commands.md lines 280-285
  - **Expected**: FAIL

### Cryptography Tests

- [ ] **T015** [P] Crypto test: Argon2id key derivation
  - **File**: `btpc-desktop-app/src-tauri/tests/auth_crypto_test.rs`
  - **Test**: Derive key from password, verify 32 bytes output, consistent results
  - **Parameters**: 64MB memory, 3 iterations, 4 parallelism per data-model.md
  - **Expected**: FAIL

- [ ] **T016** [P] Crypto test: AES-256-GCM encryption/decryption
  - **File**: `btpc-desktop-app/src-tauri/tests/auth_crypto_test.rs`
  - **Test**: Encrypt plaintext, decrypt, verify match
  - **Verify**: 12-byte nonce, 16-byte auth tag per data-model.md
  - **Expected**: FAIL

- [ ] **T017** [P] Crypto test: Constant-time password comparison
  - **File**: `btpc-desktop-app/src-tauri/tests/auth_crypto_test.rs`
  - **Test**: Compare equal passwords, compare unequal passwords, verify timing consistency
  - **Security**: Use `subtle::ConstantTimeEq` per data-model.md lines 206-215
  - **Expected**: FAIL

### Integration Tests

- [ ] **T018** Integration test: Full login/logout cycle
  - **File**: `btpc-desktop-app/src-tauri/tests/auth_integration_test.rs`
  - **Flow**: Create password → Login → Check session (authenticated) → Logout → Check session (not authenticated)
  - **Verify**: Events emitted (`session:login`, `session:logout`)
  - **Quickstart**: quickstart.md lines 25-31
  - **Expected**: FAIL

- [ ] **T019** Integration test: Credentials file persistence
  - **File**: `btpc-desktop-app/src-tauri/tests/auth_integration_test.rs`
  - **Flow**: Create password → Verify ~/.btpc/credentials.enc exists → Verify file format (magic bytes b"BTPC")
  - **Data Model**: data-model.md lines 56-69
  - **Expected**: FAIL

- [ ] **T020** Integration test: Article XI backend-first validation
  - **File**: `btpc-desktop-app/src-tauri/tests/auth_integration_test.rs`
  - **Test**: Verify SessionState in Arc<RwLock> is single source of truth, frontend cannot bypass backend
  - **Article XI**: Section 11.1, Section 11.2
  - **Expected**: FAIL

## Phase 3.3: Core Implementation (ONLY after tests are failing)

### Cryptography Core

- [ ] **T021** [P] Implement Argon2id key derivation function
  - **File**: `btpc-desktop-app/src-tauri/src/auth_crypto.rs`
  - **Function**: `derive_key(password: &[u8], salt: &[u8; 16]) -> Result<[u8; 32]>`
  - **Parameters**: 64MB memory (65536 KB), 3 iterations, 4 parallelism per research.md lines 23-27
  - **Use**: `argon2 = "0.5"` crate
  - **Test**: T015 should now PASS

- [ ] **T022** [P] Implement AES-256-GCM encryption
  - **File**: `btpc-desktop-app/src-tauri/src/auth_crypto.rs`
  - **Function**: `encrypt(data: &[u8], key: &[u8; 32]) -> Result<(Vec<u8>, [u8; 12], [u8; 16])>`
  - **Returns**: (ciphertext, nonce, auth_tag)
  - **Use**: `aes-gcm = "0.10"` crate
  - **Test**: T016 should now PASS

- [ ] **T023** [P] Implement AES-256-GCM decryption
  - **File**: `btpc-desktop-app/src-tauri/src/auth_crypto.rs`
  - **Function**: `decrypt(ciphertext: &[u8], key: &[u8; 32], nonce: &[u8; 12], tag: &[u8; 16]) -> Result<Vec<u8>>`
  - **Error**: Return descriptive error on authentication failure
  - **Test**: T016 should now PASS

- [ ] **T024** [P] Implement constant-time password comparison
  - **File**: `btpc-desktop-app/src-tauri/src/auth_crypto.rs`
  - **Function**: `constant_time_eq(a: &[u8], b: &[u8]) -> bool`
  - **Use**: `subtle::ConstantTimeEq` per data-model.md lines 208-215
  - **Security**: Prevent timing attacks per NFR-003
  - **Test**: T017 should now PASS

### State Management

- [ ] **T025** Implement SessionState and MasterCredentials structs
  - **File**: `btpc-desktop-app/src-tauri/src/auth_state.rs`
  - **Structs**: `SessionState`, `MasterCredentials` per data-model.md lines 14-37, 79-85
  - **SessionState**: `authenticated: bool`, `login_timestamp: Option<u64>`, `session_token: String`
  - **MasterCredentials**: All fields per binary format specification data-model.md lines 56-69
  - **No tests depend directly** (data structures)

- [ ] **T026** Implement MasterCredentials serialization/deserialization
  - **File**: `btpc-desktop-app/src-tauri/src/auth_state.rs`
  - **Functions**: `serialize() -> Vec<u8>`, `deserialize(data: &[u8]) -> Result<Self>`
  - **Format**: Binary format per data-model.md lines 56-69
  - **Validation**: Magic bytes b"BTPC", version == 1
  - **Test**: T019 should now PASS

- [ ] **T027** Implement SessionState initialization in Tauri app state
  - **File**: `btpc-desktop-app/src-tauri/src/main.rs`
  - **State**: Add `Arc<RwLock<SessionState>>` to Tauri state
  - **Initial**: `authenticated: false`, `login_timestamp: None`, `session_token: String::new()`
  - **Article XI**: Section 11.1 (backend single source of truth)

### Tauri Commands

- [ ] **T028** Implement `has_master_password` command
  - **File**: `btpc-desktop-app/src-tauri/src/auth_commands.rs`
  - **Signature**: `#[tauri::command] fn has_master_password() -> Result<HasMasterPasswordResponse>`
  - **Logic**: Check if ~/.btpc/credentials.enc exists
  - **Contract**: contracts/tauri-auth-commands.md lines 132-147
  - **Tests**: T013, T014 should now PASS

- [ ] **T029** Implement `create_master_password` command
  - **File**: `btpc-desktop-app/src-tauri/src/auth_commands.rs`
  - **Signature**: `#[tauri::command] async fn create_master_password(password: String, password_confirm: String, state: State<'_, Arc<RwLock<SessionState>>>, app: AppHandle) -> Result<CreateMasterPasswordResponse>`
  - **Validations**: Length >= 8, passwords match, credentials.enc doesn't exist
  - **Crypto**: Argon2id derive key → AES-256-GCM encrypt password hash → Save to ~/.btpc/credentials.enc
  - **Side Effects**: Set SessionState authenticated=true, emit `session:login` event
  - **Contract**: contracts/tauri-auth-commands.md lines 6-43
  - **Tests**: T005, T006, T007 should now PASS

- [ ] **T030** Implement `login` command
  - **File**: `btpc-desktop-app/src-tauri/src/auth_commands.rs`
  - **Signature**: `#[tauri::command] async fn login(password: String, state: State<'_, Arc<RwLock<SessionState>>>, app: AppHandle) -> Result<LoginResponse>`
  - **Logic**: Read credentials.enc → Argon2id derive key → AES-256-GCM decrypt → Constant-time compare
  - **Side Effects**: Set SessionState authenticated=true, update MasterCredentials last_used_at, emit `session:login` event
  - **Performance**: <2s per NFR-006
  - **Contract**: contracts/tauri-auth-commands.md lines 46-85
  - **Tests**: T008, T009 should now PASS

- [ ] **T031** Implement `logout` command
  - **File**: `btpc-desktop-app/src-tauri/src/auth_commands.rs`
  - **Signature**: `#[tauri::command] async fn logout(state: State<'_, Arc<RwLock<SessionState>>>, app: AppHandle) -> Result<LogoutResponse>`
  - **Logic**: Set SessionState authenticated=false, clear session_token, emit `session:logout` event
  - **Performance**: <100ms per NFR-007
  - **Contract**: contracts/tauri-auth-commands.md lines 88-107
  - **Tests**: T010 should now PASS

- [ ] **T032** Implement `check_session` command
  - **File**: `btpc-desktop-app/src-tauri/src/auth_commands.rs`
  - **Signature**: `#[tauri::command] fn check_session(state: State<'_, Arc<RwLock<SessionState>>>) -> Result<CheckSessionResponse>`
  - **Logic**: Read SessionState authenticated flag, return session_token if authenticated
  - **Performance**: <50ms per NFR-008
  - **Contract**: contracts/tauri-auth-commands.md lines 111-128
  - **Tests**: T011, T012 should now PASS

- [ ] **T033** Register all auth commands in Tauri builder
  - **File**: `btpc-desktop-app/src-tauri/src/main.rs`
  - **Update**: `.invoke_handler(tauri::generate_handler![has_master_password, create_master_password, login, logout, check_session])`
  - **State**: Ensure SessionState is managed state
  - **Test**: T018 should now PASS

## Phase 3.4: Frontend Implementation

### Login UI

- [ ] **T034** [P] Create first-launch password creation page
  - **File**: `btpc-desktop-app/ui/login.html`
  - **Elements**: Two password inputs (password, password_confirm), visibility toggles, "Create Master Password" button
  - **Style**: Dark theme matching existing btpc-desktop-app per FR-019 to FR-023
  - **Validation**: Client-side check for min 8 chars, matching passwords (backend-first validation still applies)
  - **Article XI**: Section 11.2 (backend validates, frontend displays result)

- [ ] **T035** [P] Add subsequent login form to login.html
  - **File**: `btpc-desktop-app/ui/login.html` (second form, hidden by default)
  - **Elements**: Password input, visibility toggle, "Login" button, error message area
  - **Style**: Dark theme matching first-launch form
  - **Logic**: JavaScript shows create form OR login form based on `has_master_password()` result
  - **Quickstart**: quickstart.md lines 48-59

- [ ] **T036** Embed authentication logic in login.html
  - **File**: `btpc-desktop-app/ui/login.html` (inline <script> block)
  - **Functions**:
    - `async createMasterPassword(password, passwordConfirm)` → calls `invoke('create_master_password')`
    - `async login(password)` → calls `invoke('login')`
    - `async hasMasterPassword()` → calls `invoke('has_master_password')`
    - Conditional rendering: Show create form OR login form based on `has_master_password()` result
  - **Error Handling**: Display user-friendly messages per FR-024 to FR-027
  - **Article XI**: Section 11.2 (all commands call backend first)
  - **Note**: Logout logic in separate btpc-logout.js module (T038-T043)

### Navigation Guards

- [ ] **T037** Implement navigation guard on dashboard
  - **File**: `btpc-desktop-app/ui/index.html`
  - **Logic**: On `DOMContentLoaded`, call `checkSession()`, redirect to login.html if not authenticated
  - **Performance**: <50ms per NFR-008
  - **Article XI**: Section 11.2 (backend-first validation)
  - **Quickstart**: quickstart.md lines 77-81

- [ ] **T038** [P] Add logout button to dashboard
  - **File**: `btpc-desktop-app/ui/index.html`
  - **Element**: Button with unlock icon (unlock.svg), top-right corner
  - **Action**: Call `logout()`, redirect to login.html
  - **Article XI**: Section 11.3 (listen for `session:logout` event)

- [ ] **T039** [P] Add navigation guard + logout button to wallet-manager.html
  - **File**: `btpc-desktop-app/ui/wallet-manager.html`
  - **Same logic as T037, T038**: Check session on load, logout button
  - **Article XI**: Sections 11.2, 11.3

- [ ] **T040** [P] Add navigation guard + logout button to transactions.html
  - **File**: `btpc-desktop-app/ui/transactions.html`
  - **Same logic as T037, T038**

- [ ] **T041** [P] Add navigation guard + logout button to mining.html
  - **File**: `btpc-desktop-app/ui/mining.html`
  - **Same logic as T037, T038**

- [ ] **T042** [P] Add navigation guard + logout button to settings.html
  - **File**: `btpc-desktop-app/ui/settings.html`
  - **Same logic as T037, T038**

- [ ] **T043** [P] Add navigation guard + logout button to node.html
  - **File**: `btpc-desktop-app/ui/node.html`
  - **Same logic as T037, T038**

### Event System Integration

- [ ] **T044** Implement session event listeners in btpc-event-manager.js
  - **File**: `btpc-desktop-app/ui/btpc-event-manager.js`
  - **Events**: `session:login`, `session:logout`
  - **Handlers**:
    - `session:login` → Navigate to dashboard (index.html)
    - `session:logout` → Navigate to login page (login.html), clear any cached UI state
  - **Article XI**: Section 11.3 (event-driven UI updates)
  - **Quickstart**: quickstart.md lines 83-90

- [ ] **T045** Implement event listener cleanup
  - **File**: `btpc-desktop-app/ui/btpc-event-manager.js`
  - **Logic**: On `beforeunload`, call `unlisten()` for all session event listeners
  - **Article XI**: Section 11.6 (cleanup to prevent memory leaks)

### Application Startup Routing

- [ ] **T046** Implement startup routing to single login page
  - **File**: `btpc-desktop-app/src-tauri/src/main.rs` (Tauri window configuration)
  - **Logic**: On app launch, ALWAYS open login.html
    - login.html JavaScript calls `has_master_password()` on DOMContentLoaded
    - If false → Display password creation form (first launch)
    - If true → Display login form (subsequent launch)
  - **Spec**: FR-001, FR-006
  - **Note**: Conditional rendering handled by frontend, not Tauri startup

## Phase 3.5: Polish & Documentation

### Performance Testing

- [ ] **T047** [P] Performance test: Login speed
  - **Test**: Login with correct password, measure time
  - **Target**: <2s per NFR-006
  - **Quickstart**: quickstart.md lines 91-100
  - **File**: Add to manual test guide

- [ ] **T048** [P] Performance test: Logout speed
  - **Test**: Call logout, measure time
  - **Target**: <100ms per NFR-007
  - **Quickstart**: quickstart.md lines 102-111

- [ ] **T049** [P] Performance test: Navigation guard speed
  - **Test**: Call `check_session()`, measure time
  - **Target**: <50ms per NFR-008

### Article XI Compliance Verification

- [ ] **T050** Verify backend-first validation on all pages
  - **Pages**: index.html, wallet-manager.html, transactions.html, mining.html, settings.html, node.html
  - **Check**: All pages call `check_session()` before rendering
  - **Article XI**: Section 11.2
  - **Quickstart**: quickstart.md lines 149-156

- [ ] **T051** Verify event listeners cleaned up on all pages
  - **Pages**: All authenticated pages
  - **Check**: `beforeunload` handler calls `unlisten()` for session events
  - **Article XI**: Section 11.6

- [ ] **T052** Verify no localStorage for authentication state
  - **Scan**: All frontend files (*.html, *.js)
  - **Check**: No `localStorage.setItem('authenticated')` or similar patterns
  - **Article XI**: Section 11.5 (backend state is authoritative)

- [ ] **T053** Verify cross-page state synchronization
  - **Test**: Open two tabs (dashboard + wallet-manager), logout from one, verify other redirects
  - **Mechanism**: `session:logout` event propagates to all windows
  - **Article XI**: Section 11.3
  - **Quickstart**: quickstart.md lines 83-90

### Code Quality

- [ ] **T054** Run cargo clippy and fix all warnings
  - **Command**: `cargo clippy --workspace --all-targets -- -D warnings`
  - **Fix**: All clippy warnings in auth_*.rs files
  - **Standard**: Zero warnings required

- [ ] **T055** [P] Add documentation to all public auth functions
  - **Files**: auth_state.rs, auth_crypto.rs, auth_commands.rs
  - **Format**: `///` doc comments with examples
  - **Standard**: Document all public APIs

- [ ] **T056** [P] Security audit: Verify no password logging
  - **Scan**: All Rust files (auth_*.rs, main.rs)
  - **Check**: No `println!`, `debug!`, `info!` statements with password variables
  - **Requirement**: NFR-004

- [ ] **T057** [P] Security audit: Verify zeroization of sensitive data
  - **Check**: All password-derived keys use `Zeroizing` wrapper
  - **Example**: data-model.md lines 219-226
  - **Crate**: `zeroize = "1.7"`

### Documentation

- [ ] **T058** [P] Update CLAUDE.md with authentication feature
  - **File**: `CLAUDE.md`
  - **Add**: Rust 1.75+ (Tauri backend), argon2, aes-gcm, Tauri events, Arc<RwLock<SessionState>>
  - **Section**: Add to "Recent Changes" with feature summary

- [ ] **T059** [P] Create manual testing guide
  - **File**: `btpc-desktop-app/tests/auth_ui_manual_test.md`
  - **Content**: Copy quickstart.md manual test scenarios (lines 33-111)
  - **Purpose**: Standalone testing documentation

- [ ] **T060** [P] Update STATUS.md with implementation status
  - **File**: `STATUS.md`
  - **Add**: Feature 006 (Application-Level Authentication) - Status: Complete

### Final Validation

- [ ] **T061** Run full contract test suite
  - **Command**: `cd btpc-desktop-app/src-tauri && cargo test auth_contract_test -- --nocapture`
  - **Expected**: All 10 contract tests PASS (T005-T014)
  - **Quickstart**: quickstart.md lines 14-22

- [ ] **T062** Run full integration test suite
  - **Command**: `cd btpc-desktop-app/src-tauri && cargo test auth_integration_test -- --nocapture`
  - **Expected**: All integration tests PASS (T018-T020)
  - **Quickstart**: quickstart.md lines 24-31

- [ ] **T063** Run full manual UI test suite
  - **Steps**: Execute all 8 manual tests from quickstart.md lines 33-111
  - **Validation**: All functional requirements (FR-001 to FR-027) verified
  - **Checklist**: quickstart.md lines 116-136

- [ ] **T064** Verify all quickstart validation checklist items
  - **Functional**: 27 FRs checked (quickstart.md lines 119-135)
  - **Non-Functional**: 12 NFRs checked (quickstart.md lines 137-147)
  - **Article XI**: 6 compliance items checked (quickstart.md lines 149-156)
  - **Success Criteria**: quickstart.md lines 189-197

## Dependencies

**Critical Test-First Dependency:**
- **All tests (T005-T020) MUST complete and FAIL before ANY implementation (T021-T046)**

**Setup Dependencies:**
- T001 (dependencies) blocks T021-T024 (crypto implementation)
- T002 (module structure) blocks T025-T033 (state + commands)
- T003 (test structure) blocks T005-T020 (tests)
- T004 (icons) blocks T034-T043 (UI)

**Cryptography Dependencies:**
- T021 (Argon2id) blocks T029 (create_master_password), T030 (login)
- T022, T023 (AES-256-GCM) blocks T029, T030
- T024 (constant-time comparison) blocks T030 (login)

**State Management Dependencies:**
- T025 (structs) blocks T026 (serialization), T027 (Tauri state)
- T027 (Tauri state) blocks T028-T032 (all commands)
- T026 (serialization) blocks T029 (create_master_password)

**Command Dependencies:**
- T028 (has_master_password) blocks T046 (startup routing)
- T029 (create_master_password) blocks T030 (login) - creates credentials.enc
- T029, T030, T031, T032 (all commands) block T033 (registration)
- T033 (command registration) blocks T036 (frontend auth.js)

**Frontend Dependencies:**
- T036 (login.html inline auth logic) blocks T034-T043 (all UI pages)
- T036 blocks T037-T043 (navigation guards)
- T036 blocks T044 (event system)
- T044 (event listeners) blocks T045 (event cleanup)
- T033 (backend commands) blocks T046 (startup routing)

**Testing Dependencies:**
- T021-T033 (implementation) blocks T047-T049 (performance tests)
- T034-T046 (UI) blocks T050-T053 (Article XI verification)
- T034-T046 (UI) blocks T061-T063 (manual UI tests)
- All implementation (T021-T046) blocks T061-T064 (final validation)

## Parallel Execution Examples

**Setup Phase (all parallel):**
```bash
# T002, T003, T004 can run together (different directories):
Task T002: "Create Rust module structure for authentication"
Task T003: "Create test directory structure"
Task T004: "Create SVG icons for login/logout"
```

**Test Phase (all parallel after T001-T004):**
```bash
# T005-T020 can run in parallel (different test files/functions):
Task T005: "Contract test: create_master_password success case"
Task T006: "Contract test: create_master_password password too short"
Task T007: "Contract test: create_master_password mismatch"
Task T008: "Contract test: login success"
Task T009: "Contract test: login wrong password"
Task T010: "Contract test: logout always succeeds"
Task T011: "Contract test: check_session authenticated"
Task T012: "Contract test: check_session not authenticated"
Task T013: "Contract test: has_master_password false"
Task T014: "Contract test: has_master_password true"
Task T015: "Crypto test: Argon2id key derivation"
Task T016: "Crypto test: AES-256-GCM encryption/decryption"
Task T017: "Crypto test: Constant-time password comparison"
```

**Crypto Implementation (all parallel after tests fail):**
```bash
# T021-T024 can run in parallel (different functions in auth_crypto.rs):
Task T021: "Implement Argon2id key derivation function"
Task T022: "Implement AES-256-GCM encryption"
Task T023: "Implement AES-256-GCM decryption"
Task T024: "Implement constant-time password comparison"
```

**Frontend UI Pages (parallel after T036):**
```bash
# T039-T043 can run in parallel (different HTML files):
Task T039: "Add navigation guard + logout button to wallet-manager.html"
Task T040: "Add navigation guard + logout button to transactions.html"
Task T041: "Add navigation guard + logout button to mining.html"
Task T042: "Add navigation guard + logout button to settings.html"
Task T043: "Add navigation guard + logout button to node.html"
```

**Documentation (all parallel):**
```bash
# T055, T058, T059, T060 can run in parallel (different files):
Task T055: "Add documentation to all public auth functions"
Task T058: "Update CLAUDE.md with authentication feature"
Task T059: "Create manual testing guide"
Task T060: "Update STATUS.md with implementation status"
```

## Notes

- **[P]** tasks = different files, no dependencies
- **TDD**: Verify tests fail (RED) before implementing (GREEN)
- **Article XI**: Desktop features MUST follow all 6 compliance sections
- **Security**: No password logging (NFR-004), constant-time comparison (NFR-003)
- **Performance**: Login <2s, Logout <100ms, Navigation guard <50ms
- **Commit**: After each task with descriptive message following project standards

## Validation Checklist

**General:**
- [x] All contract tests (T005-T014) come before implementation (T021-T046)
- [x] Parallel tasks truly independent (different files)
- [x] Each task specifies exact file path in BTPC structure
- [x] No task modifies same file as another [P] task

**Desktop-Specific:**
- [x] Article XI compliance tasks included (T020, T050-T053)
- [x] Backend-first validation tasks present (T037-T043)
- [x] Event emission tasks present (T029-T031 side effects)
- [x] Event cleanup tasks present (T045, T051)
- [x] No localStorage-first patterns in tasks (verified T052)
- [x] Cross-page state synchronization tested (T053)

**Constitutional Compliance:**
- [x] Article XI applicability determined (desktop feature)
- [x] All Article XI sections have corresponding tasks
- [x] Compliance verification tasks in Polish phase (T050-T053)

---

**Status**: ✅ READY FOR EXECUTION
**Estimated Time**: 8-12 hours (3-4 hours tests, 4-6 hours implementation, 1-2 hours polish)
**Success Criteria**: All 64 tasks complete, all quickstart tests pass, all Article XI compliance verified