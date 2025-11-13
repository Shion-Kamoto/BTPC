
# Implementation Plan: GPU Mining Dashboard with Individual GPU Statistics

**Branch**: `012-create-an-new` | **Date**: 2025-11-11 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/home/bob/BTPC/BTPC/specs/012-create-an-new/spec.md`

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
**Primary Requirement**: Create a GPU Mining Dashboard as a sub-tab on the Mining page that displays all system GPUs with individual mining statistics, health metrics (temperature, fan speed, power, memory, clock speed), efficiency metrics (hashrate/watt, hashrate/temperature), and automatic thermal protection via throttling.

**Technical Approach**: Extend existing MiningThreadPool (Feature 010) with GPU health monitoring (NVML for NVIDIA GPUs - required for production, sysinfo crate fallback for AMD/Intel basic metrics), implement Tauri event system for real-time stat updates (5-second polling), add GPU enumeration service, create mining.html sub-tab UI, persist per-GPU lifetime stats, and implement thermal throttling algorithm with configurable temperature thresholds (default: 80°C).

## Technical Context
**Language/Version**: Rust 1.75+ (Tauri backend), JavaScript ES6+ (frontend), OpenCL/CUDA for GPU mining
**Primary Dependencies**: Tauri 2.0, opencl3 (Rust OpenCL bindings), serde (serialization), tokio::sync (async locks), nvml-wrapper (NVIDIA GPU monitoring - required), sysinfo (cross-platform fallback for AMD/Intel)
**Storage**: JSON file for per-GPU persistent stats (~/.btpc/data/mining_stats_per_gpu.json), localStorage for UI state (temperature threshold setting)
**Testing**: cargo test (unit tests), Tauri integration tests, frontend E2E tests for dashboard
**Target Platform**: Linux desktop (primary), Windows/macOS (cross-platform Tauri)
**Project Type**: Desktop Wallet App (Tauri-based) - frontend + backend architecture
**Performance Goals**: GPU enumeration <500ms, stats updates <1% mining overhead, UI render <200ms for 10+ GPUs, 5-second sensor polling interval
**Constraints**: Article XI mandatory (backend-first validation, event-driven, single source of truth), temperature sensor availability varies by GPU/platform, graceful degradation for missing sensors
**Scale/Scope**: Support 1-16 GPUs, handle hot-plug events, persist lifetime stats across app restarts, no changes to CPU mining

## Constitution Check
*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

**Security Gate**: N/A (no cryptographic operations - display-only GPU dashboard)
**Testing Gate**: >90% test coverage required for GPU monitoring, stats calculation, thermal throttling
**Performance Gate**: GPU enumeration <500ms, stats updates <1% mining overhead, UI render <200ms
**Memory Safety Gate**: All Rust code, no unsafe blocks (GPU sensor access via safe libraries)
**Dependency Gate**: opencl3, NVML/ADL libraries must be audited with cargo-audit
**Article XI Gate**: Desktop feature MUST comply with Article XI:
  - Backend (MiningThreadPool) is single source of truth for GPU stats
  - Frontend displays only, never maintains authoritative GPU state
  - Backend emits "gpu-stats-updated" events every 5 seconds
  - Frontend listens for events and updates UI (no polling)
  - Event listeners cleaned up on page unload (beforeunload handler)
  - NO localStorage before backend validation (temperature threshold validated first)
  - NO duplicate notifications for user actions

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
btpc-desktop-app/src-tauri/src/  (Tauri Backend - Rust)
├── mining_thread_pool.rs      # EXTEND: Add GPU health monitoring methods
├── gpu_stats_commands.rs      # NEW: Tauri commands for GPU dashboard
├── commands/
│   └── gpu_stats.rs          # NEW: GPU enumeration, stats, health queries
├── gpu_health_monitor.rs      # NEW: NVML/ADL wrapper for sensor polling
├── thermal_throttle.rs        # NEW: Thermal protection algorithm
└── lib.rs                     # UPDATE: Register new GPU commands

btpc-desktop-app/src-tauri/tests/
├── gpu_stats_tests.rs         # NEW: Unit tests for GPU monitoring
├── thermal_throttle_tests.rs  # NEW: Unit tests for throttling logic
└── integration/
    └── test_gpu_dashboard.rs  # NEW: Integration test for full workflow

btpc-desktop-app/ui/  (Frontend - JavaScript/HTML)
├── mining.html                # UPDATE: Add GPU Mining sub-tab
├── mining-gpu-dashboard.js    # NEW: GPU dashboard logic
├── btpc-styles.css            # UPDATE: GPU card styles, temperature warnings
└── btpc-tab-manager.js        # REUSE: Existing tab management (Feature 004)

~/.btpc/data/  (Persistent Storage)
└── mining_stats_per_gpu.json  # NEW: Per-GPU lifetime blocks found
```

**Structure Decision**: **Desktop Wallet App (Tauri-based)**. This feature extends the existing btpc-desktop-app with GPU-specific monitoring modules. Backend uses Rust for GPU sensor access (NVML for NVIDIA, ADL for AMD via opencl3 crate). Frontend uses vanilla JavaScript with existing btpc-tab-manager.js for sub-tab navigation. Architecture follows Article XI: backend is single source of truth, emits "gpu-stats-updated" events, frontend listens and updates UI.

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
- [x] Phase 2: Task planning complete (/plan command - describe approach only)
- [ ] Phase 3: Tasks generated (/tasks command)
- [ ] Phase 4: Implementation complete
- [ ] Phase 5: Validation passed

**Gate Status**:
- [x] Initial Constitution Check: PASS (no constitutional violations)
- [x] Post-Design Constitution Check: PASS (no new violations after design)
- [x] All NEEDS CLARIFICATION resolved (5 clarifications answered via /clarify)
- [x] Complexity deviations documented (N/A - no violations)

---
*Based on Constitution v2.1.1 - See `/memory/constitution.md`*
