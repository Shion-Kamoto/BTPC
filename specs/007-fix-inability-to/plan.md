
# Implementation Plan: Fix Transaction Sending Between Wallets

**Branch**: `007-fix-inability-to` | **Date**: 2025-10-30 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/home/bob/BTPC/BTPC/specs/007-fix-inability-to/spec.md`

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
Fix critical transaction sending failures preventing users from transferring BTPC between their own wallets or to external addresses. The primary issue involves transaction creation, signing with ML-DSA, and broadcasting. Solution requires debugging UTXO selection, fixing signature generation with stored seeds, ensuring proper fee calculation, and implementing robust error handling with Article XI event-driven status updates.

## Technical Context
**Language/Version**: Rust 1.75+ (Tauri backend), JavaScript ES6+ (frontend)
**Primary Dependencies**: btpc-core (blockchain library), dilithium5 (ML-DSA crypto), rocksdb (UTXO storage), tauri 2.0
**Storage**: RocksDB for UTXO tracking, encrypted wallet files (.dat), transaction cache
**Testing**: cargo test (unit), integration tests for transaction flow, manual UI testing
**Target Platform**: Desktop application (Linux/Windows/macOS via Tauri)
**Project Type**: web (Tauri desktop app with HTML/JS frontend)
**Performance Goals**: Transaction creation <500ms, ML-DSA signing <100ms, UI responsive during processing
**Constraints**: Article XI backend-first validation, ML-DSA quantum-resistant signatures only, no localStorage for tx state
**Scale/Scope**: Fix existing functionality, support multiple wallets per user, handle concurrent transactions

### Clarification Assumptions (from spec NEEDS CLARIFICATION items):
1. **Error messages**: Assuming generic "Transaction failed" errors need specific messages for each failure type
2. **Network types**: Fix applies to all networks (Mainnet, Testnet, Regtest)
3. **Fee calculation**: Assuming fees are not calculated correctly, need dynamic fee based on tx size
4. **Log retention**: Default to 7 days for transaction debug logs

## Constitution Check
*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

**Security Gate**: ✅ ML-DSA signatures for all transactions (post-quantum)
**Testing Gate**: ⚠️ Will add comprehensive transaction flow tests (aim for >90% coverage)
**Performance Gate**: ✅ ML-DSA signing target <100ms (well within <2ms for signature ops)
**Memory Safety Gate**: ✅ All Rust backend code, no unsafe blocks planned
**Dependency Gate**: ✅ Using existing audited dependencies (btpc-core, dilithium5)
**Article XI Gate**: ✅ Backend-first validation, event-driven architecture, no frontend state

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
btpc-desktop-app/ (Tauri Desktop Application)
├── src-tauri/          # Rust backend
│   ├── src/
│   │   ├── wallet_commands.rs      # Transaction creation commands (FIX NEEDED)
│   │   ├── wallet_manager.rs       # UTXO management, balance tracking (FIX NEEDED)
│   │   ├── btpc_integration.rs     # Node RPC communication (VERIFY)
│   │   ├── tx_storage.rs           # Transaction cache/history
│   │   ├── utxo_manager.rs         # UTXO selection logic (FIX NEEDED)
│   │   └── error.rs                # Error handling improvements
│   └── tests/
│       ├── transaction_signing_integration.rs  # NEW: End-to-end tx tests
│       └── wallet_backup_integration.rs       # Existing tests to verify
│
├── ui/                 # JavaScript frontend
│   ├── transactions.html           # Transaction UI (UPDATE)
│   ├── btpc-common.js             # Common utilities
│   ├── btpc-event-listeners.js    # NEW: Transaction event handlers
│   └── btpc-error-handler.js      # NEW: Improved error display
│
└── btpc-core/         # Core blockchain library (external dependency)
    ├── src/
    │   ├── crypto/
    │   │   ├── keys.rs            # ML-DSA key handling (VERIFY SEED STORAGE)
    │   │   └── signatures.rs      # Signature generation (FIX NEEDED)
    │   └── blockchain/
    │       └── transaction.rs     # Transaction structure (VERIFY)
    └── tests/
```

**Structure Decision**: Desktop Wallet App (Tauri-based) - This is a bug fix for the existing Tauri desktop application. The focus is on fixing the transaction flow in the backend Rust code and improving error handling in the frontend JavaScript.

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
- [x] Phase 0: Research complete (/plan command)
- [x] Phase 1: Design complete (/plan command)
- [x] Phase 2: Task planning complete (/plan command - describe approach only)
- [ ] Phase 3: Tasks generated (/tasks command)
- [ ] Phase 4: Implementation complete
- [ ] Phase 5: Validation passed

**Gate Status**:
- [x] Initial Constitution Check: PASS
- [x] Post-Design Constitution Check: PASS
- [ ] All NEEDS CLARIFICATION resolved
- [x] Complexity deviations documented

---
*Based on Constitution v2.1.1 - See `/memory/constitution.md`*
## Phase 1: Design Decisions

### Transaction Flow Design
1. **UTXO Reservation System**:
   - ReservationToken pattern for locking UTXOs
   - Timeout mechanism (5 minutes default)
   - Automatic release on drop

2. **Event Flow Specification**:
   ```
   transaction:initiated → transaction:validated → transaction:signed
   → transaction:broadcast → transaction:confirmed/failed
   ```

3. **Error Categorization**:
   - Validation errors (address, amount)
   - Insufficient funds errors
   - Signing errors (key access)
   - Network errors (broadcast)
   - Timeout errors (stuck transactions)

4. **Fee Calculation Model**:
   - Dynamic: base_fee + (size_bytes * fee_rate)
   - RPC query for current rates
   - Fallback to conservative estimate

**Output**: data-model.md, contracts/, quickstart.md
