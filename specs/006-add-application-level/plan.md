
# Implementation Plan: Application-Level Login/Logout System

**Branch**: `006-add-application-level` | **Date**: 2025-10-28 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/home/bob/BTPC/BTPC/specs/006-add-application-level/spec.md`

## Execution Flow (/plan command scope)
```
1. Load feature spec from Input path
   → If not found: ERROR "No feature spec at {path}"
2. Fill Technical Context (scan for NEEDS CLARIFICATION)
   → Detect Project Type from file system structure or context (web=frontend+backend, mobile=app+api)
   → Set Structure Decision based on project type
3. Fill the Constitution Check section based on the content of the constitution document.
4. Evaluate Constitution Check section below
   → If violations exist: Document in Complexity Tracking
   → If no justification possible: ERROR "Simplify approach first"
   → Update Progress Tracking: Initial Constitution Check
5. Execute Phase 0 → research.md
   → If NEEDS CLARIFICATION remain: ERROR "Resolve unknowns"
6. Execute Phase 1 → contracts, data-model.md, quickstart.md, agent-specific template file (e.g., `CLAUDE.md` for Claude Code, `.github/copilot-instructions.md` for GitHub Copilot, `GEMINI.md` for Gemini CLI, `QWEN.md` for Qwen Code or `AGENTS.md` for opencode).
7. Re-evaluate Constitution Check section
   → If new violations: Refactor design, return to Phase 1
   → Update Progress Tracking: Post-Design Constitution Check
8. Plan Phase 2 → Describe task generation approach (DO NOT create tasks.md)
9. STOP - Ready for /tasks command
```

**IMPORTANT**: The /plan command STOPS at step 7. Phases 2-4 are executed by other commands:
- Phase 2: /tasks command creates tasks.md
- Phase 3-4: Implementation execution (manual or via tools)

## Summary
Add application-level authentication system to btpc-desktop-app with AES-256-GCM encrypted master password storage. On first launch, users create a master password (separate from wallet passwords) that gates access to the application. Login screen with dark theme and professional icons displays on subsequent launches. Logout button available on all pages. Technical approach: Tauri backend stores SessionState in Arc<RwLock> as single source of truth, frontend receives session events (`session:login`, `session:logout`) for UI updates. MasterCredentials encrypted with Argon2id (64MB, 3 iterations, 4 parallelism) and stored in ~/.btpc/credentials.enc.

## Technical Context
**Language/Version**: Rust 1.75+ (Tauri backend), JavaScript ES6+ (frontend)
**Primary Dependencies**: Tauri 2.0 (already in use), argon2 (key derivation), aes-gcm (encryption), tauri events system
**Storage**: Encrypted file (~/.btpc/credentials.enc) for MasterCredentials, in-memory Arc<RwLock<SessionState>> for authentication state
**Testing**: cargo test (Rust backend), manual UI testing (Tauri app), integration tests for login/logout flow
**Target Platform**: Linux/Windows/macOS desktop (Tauri cross-platform)
**Project Type**: Desktop (Tauri-based: Rust backend + web frontend)
**Performance Goals**: Login validation <2s (Argon2id overhead acceptable), logout <100ms, navigation guard <50ms
**Constraints**: Article XI compliance (backend-first validation, event-driven, no localStorage before backend), constant-time password comparison, OWASP Argon2id parameters
**Scale/Scope**: Single-user desktop application, local authentication only (no multi-user or network auth in v1)

## Constitution Check
*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

**Security Gate (Article I - Security-First)**:
✅ PASS - Using OWASP-recommended Argon2id (64MB, 3 iter, 4 par) + AES-256-GCM
✅ PASS - Constant-time password comparison to prevent timing attacks
✅ PASS - No hardcoded secrets, master password never stored in plaintext
✅ PASS - Cryptographically secure RNG for salt generation (16+ bytes)
⚠️  N/A - Post-quantum requirement (master password is not blockchain crypto)

**Testing Gate (Article III - TDD)**:
✅ PASS - Contract tests for Tauri commands (create_master_password, login, logout, check_session)
✅ PASS - Integration tests for full login/logout cycle
✅ PASS - Unit tests for Argon2id + AES-256-GCM encryption/decryption
✅ PASS - Manual UI testing for first-launch password creation

**Desktop App Gate (Article XI)**:
✅ PASS - Backend Arc<RwLock<SessionState>> is single source of truth
✅ PASS - Frontend never maintains authoritative authentication state
✅ PASS - Backend-first validation: password check before session establishment
✅ PASS - Event-driven: `session:login` and `session:logout` events emitted
✅ PASS - Event listener cleanup on page unload specified
✅ PASS - No localStorage for auth state (spec explicitly prohibits)

**Memory Safety Gate**:
✅ PASS - All Rust backend code (Tauri), no unsafe blocks needed
✅ PASS - argon2 and aes-gcm crates are memory-safe

**Dependency Gate**:
✅ PASS - argon2 and aes-gcm are well-audited cryptography crates
✅ PASS - Tauri 2.0 already in use and audited by project

**Performance Gate**:
✅ PASS - Login < 2s (Argon2id overhead acceptable for security)
✅ PASS - Logout < 100ms (clear in-memory state)
✅ PASS - Navigation guard < 50ms (check Arc<RwLock> state)

## Project Structure

### Documentation (this feature)
```
specs/[###-feature]/
├── plan.md              # This file (/plan command output)
├── research.md          # Phase 0 output (/plan command)
├── data-model.md        # Phase 1 output (/plan command)
├── quickstart.md        # Phase 1 output (/plan command)
├── contracts/           # Phase 1 output (/plan command)
└── tasks.md             # Phase 2 output (/tasks command - NOT created by /plan)
```

### Source Code (repository root)
```
btpc-desktop-app/ (Tauri-based Desktop Wallet)
├── src-tauri/ (Rust backend)
│   ├── src/
│   │   ├── main.rs                    # Tauri entry point
│   │   ├── auth_commands.rs           # NEW: Login/logout Tauri commands
│   │   ├── auth_state.rs              # NEW: SessionState + MasterCredentials
│   │   ├── auth_crypto.rs             # NEW: Argon2id + AES-256-GCM functions
│   │   ├── wallet_commands.rs         # Existing wallet operations
│   │   ├── node_commands.rs           # Existing node management
│   │   └── state_manager.rs           # Existing unified state
│   ├── tests/
│   │   ├── auth_integration_test.rs   # NEW: Login/logout cycle tests
│   │   └── auth_crypto_test.rs        # NEW: Encryption/decryption tests
│   └── Cargo.toml                     # Add argon2, aes-gcm dependencies
│
├── ui/ (Web frontend)
│   ├── login.html                     # NEW: First-launch password creation + subsequent login (conditional rendering)
│   ├── index.html                     # Existing dashboard (add logout button)
│   ├── wallet-manager.html            # Existing (add logout button)
│   ├── transactions.html              # Existing (add logout button)
│   ├── mining.html                    # Existing (add logout button)
│   ├── settings.html                  # Existing (add logout button)
│   ├── node.html                      # Existing (add logout button)
│   ├── btpc-logout.js                 # NEW: Logout button logic (reusable module)
│   ├── btpc-navigation-guard.js       # NEW: Authentication checks on page load
│   ├── btpc-event-listeners.js        # NEW: Session event handlers
│   ├── btpc-common.js                 # Existing utility module
│   ├── btpc-event-manager.js          # Existing event system (for session events)
│   └── src/assets/icons-svg/
│       ├── lock.svg                   # NEW: Login icon
│       ├── unlock.svg                 # NEW: Logout icon
│       └── ...                        # Existing professional icons
│
└── tests/
    └── auth_ui_manual_test.md         # NEW: Manual UI testing guide

~/.btpc/ (Application data directory)
└── credentials.enc                    # NEW: Encrypted master credentials file
```

**Structure Decision**: Tauri desktop app (Option 2) selected. Feature adds authentication layer to existing btpc-desktop-app. Backend Rust code in `src-tauri/src/auth_*.rs` modules for separation of concerns. Frontend adds single `login.html` with conditional rendering (first-launch password creation OR subsequent login based on `has_master_password()` result). Logout, navigation guard, and event listeners extracted to separate modules (`btpc-logout.js`, `btpc-navigation-guard.js`, `btpc-event-listeners.js`). All existing pages (`index.html`, `wallet-manager.html`, etc.) updated with logout button and navigation guards. Encrypted credentials stored in standard app data directory (~/.btpc/) with `.enc` extension to indicate encrypted format.

## Phase 0: Outline & Research
1. **Extract unknowns from Technical Context** above:
   - For each NEEDS CLARIFICATION → research task
   - For each dependency → best practices task
   - For each integration → patterns task

2. **Generate and dispatch research agents**:
   ```
   For each unknown in Technical Context:
     Task: "Research {unknown} for {feature context}"
   For each technology choice:
     Task: "Find best practices for {tech} in {domain}"
   ```

3. **Consolidate findings** in `research.md` using format:
   - Decision: [what was chosen]
   - Rationale: [why chosen]
   - Alternatives considered: [what else evaluated]

**Output**: research.md with all NEEDS CLARIFICATION resolved

## Phase 1: Design & Contracts
*Prerequisites: research.md complete*

1. **Extract entities from feature spec** → `data-model.md`:
   - Entity name, fields, relationships
   - Validation rules from requirements
   - State transitions if applicable

2. **Generate API contracts** from functional requirements:
   - For each user action → endpoint
   - Use standard REST/GraphQL patterns
   - Output OpenAPI/GraphQL schema to `/contracts/`

3. **Generate contract tests** from contracts:
   - One test file per endpoint
   - Assert request/response schemas
   - Tests must fail (no implementation yet)

4. **Extract test scenarios** from user stories:
   - Each story → integration test scenario
   - Quickstart test = story validation steps

5. **Update agent file incrementally** (O(1) operation):
   - Run `.specify/scripts/bash/update-agent-context.sh claude`
     **IMPORTANT**: Execute it exactly as specified above. Do not add or remove any arguments.
   - If exists: Add only NEW tech from current plan
   - Preserve manual additions between markers
   - Update recent changes (keep last 3)
   - Keep under 150 lines for token efficiency
   - Output to repository root

**Output**: data-model.md, /contracts/*, failing tests, quickstart.md, agent-specific file

## Phase 2: Task Planning Approach
*This section describes what the /tasks command will do - DO NOT execute during /plan*

**Task Generation Strategy**:
- Load `.specify/templates/tasks-template.md` as base
- Generate tasks from Phase 1 design docs (contracts, data model, quickstart)
- Each contract → contract test task [P]
- Each entity → model creation task [P] 
- Each user story → integration test task
- Implementation tasks to make tests pass

**Ordering Strategy**:
- TDD order: Tests before implementation 
- Dependency order: Models before services before UI
- Mark [P] for parallel execution (independent files)

**Estimated Output**: 25-30 numbered, ordered tasks in tasks.md

**IMPORTANT**: This phase is executed by the /tasks command, NOT by /plan

## Phase 3+: Future Implementation
*These phases are beyond the scope of the /plan command*

**Phase 3**: Task execution (/tasks command creates tasks.md)  
**Phase 4**: Implementation (execute tasks.md following constitutional principles)  
**Phase 5**: Validation (run tests, execute quickstart.md, performance validation)

## Complexity Tracking
*Fill ONLY if Constitution Check has violations that must be justified*

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| [e.g., 4th project] | [current need] | [why 3 projects insufficient] |
| [e.g., Repository pattern] | [specific problem] | [why direct DB access insufficient] |


## Progress Tracking
*This checklist is updated during execution flow*

**Phase Status**:
- [x] Phase 0: Research complete (/plan command) - research.md created
- [x] Phase 1: Design complete (/plan command) - data-model.md, contracts/, quickstart.md, CLAUDE.md updated
- [x] Phase 2: Task planning complete (/plan command - described below)
- [x] Phase 3: Tasks generated (/tasks command) - tasks.md created with 64 numbered tasks
- [ ] Phase 4: Implementation complete
- [ ] Phase 5: Validation passed

**Gate Status**:
- [x] Initial Constitution Check: PASS (all gates passed, no violations)
- [x] Post-Design Constitution Check: PASS (re-evaluated, no new violations)
- [x] All NEEDS CLARIFICATION resolved (none found - all resolved in spec)
- [x] Complexity deviations documented (none - no constitutional violations)

**Artifacts Generated**:
- ✅ research.md (technology choices documented)
- ✅ data-model.md (3 entities: MasterCredentials, SessionState, LoginAttempt)
- ✅ contracts/tauri-auth-commands.md (5 commands + 2 events)
- ✅ quickstart.md (10 test scenarios + validation checklist)
- ✅ CLAUDE.md (updated with auth feature context)
- ✅ tasks.md (64 numbered tasks with dependencies and parallel execution guidance)

---
*Based on Constitution v2.1.1 - See `/memory/constitution.md`*
