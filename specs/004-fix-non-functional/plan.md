
# Implementation Plan: Fix Non-Functional Sub-Tabs

**Branch**: `004-fix-non-functional` | **Date**: 2025-10-25 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/home/bob/BTPC/BTPC/specs/004-fix-non-functional/spec.md`

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
**Primary Requirement**: Fix non-functional sub-tabs on Settings, Transactions, and Mining pages that do not respond to clicks. Implement proper tab switching functionality with visual feedback, content visibility toggling, and localStorage persistence of active tab state per page.

**Affected Pages**:
- Settings: Node/Application/Security tabs
- Transactions: Receive/History/Address Book tabs
- Mining: Configure/History tabs

**Technical Approach**: Pure frontend JavaScript implementation using event listeners for tab clicks, CSS class toggling for visual states, display property manipulation for content visibility, and localStorage API for state persistence. ARIA-compliant keyboard navigation with Tab, Enter, Space, and Arrow keys support.

## Technical Context
**Language/Version**: JavaScript ES6+ (vanilla JS, no framework required)
**Primary Dependencies**: btpc-common.js (existing utility module), btpc-storage.js (localStorage wrapper), browser localStorage API
**Storage**: Browser localStorage for tab state persistence (keys: btpc_active_tab_settings, btpc_active_tab_transactions, btpc_active_tab_mining)
**Testing**: Manual testing against 13 acceptance scenarios, potential Playwright automation for UI verification
**Target Platform**: Desktop (Tauri 2.0 webview on Linux, Windows, macOS)
**Project Type**: Desktop Wallet App (Tauri-based frontend UI feature)
**Performance Goals**: < 50ms tab switching response time (NFR-001), 60fps maintained during transitions (NFR-005), no perceptible localStorage delay (NFR-006)
**Constraints**: WCAG 2.1 AA accessibility compliance (NFR-002), ARIA roles for screen readers (NFR-007), Article XI constitutional compliance (localStorage exception for non-critical UI state)
**Scale/Scope**: 3 pages (Settings, Transactions, Mining), 8 total sub-tabs, isolated state per page, keyboard navigation support

## Constitution Check
*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

**Article XI - Desktop Application Development (Applicable)**:

**Section 11.1 - Single Source of Truth**: ✅ PASS
- localStorage used for non-critical UI state (tab preferences) only
- No authoritative blockchain/wallet state in frontend JavaScript
- Frontend displays current tab, backend not involved (pure UI navigation)

**Section 11.2 - Backend-First Validation**: ✅ N/A
- Tab switching is pure UI navigation with no data modification
- No backend validation required for UI preference changes
- No localStorage save on validation failure (no validation needed)

**Section 11.3 - Event-Driven Architecture**: ✅ N/A
- Tab state is local to each page, no cross-page synchronization needed
- No backend events required for UI-only tab switching
- State changes are immediate and local

**Section 11.6 - Event Listener Cleanup**: ✅ PASS
- Tab click handlers attached to DOM elements
- Automatic cleanup via DOM element removal on navigation
- No persistent listeners across page boundaries
- No memory leaks from tab event listeners

**Section 11.7 - Prohibited Patterns**: ✅ PASS
- ✅ NO localStorage before backend validation (no backend involved, localStorage for UI preference only)
- ✅ NO authoritative state in frontend (tab state is presentation-only)
- ✅ NO polling (event-driven click handlers)
- ✅ NO duplicate notifications (tab switching is silent UI operation)

**Article XI Exception Justification**:
Tab selection state is purely presentational UI preference. It does not affect blockchain consensus, wallet balance, transaction validity, or any critical data integrity. localStorage is used only for user convenience (remembering last active tab), which is explicitly permitted for non-critical UI state under Article XI.1.

**Performance Gates** (from NFRs):
- Tab switching: < 50ms response time (NFR-001)
- UI smoothness: 60fps during transitions (NFR-005)
- Storage operations: No perceptible delay (NFR-006)

**Accessibility Gates** (from NFRs):
- WCAG 2.1 AA color contrast for active/inactive states (NFR-002)
- ARIA roles: tablist, tab, tabpanel (NFR-007)
- Keyboard navigation: Tab, Enter, Space, Arrow keys (NFR-008)

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
btpc-desktop-app/
├── ui/                          # Frontend (Web technologies)
│   ├── settings.html           # Settings page with Node/Application/Security tabs
│   ├── transactions.html       # Transactions page with Receive/History/Address Book tabs
│   ├── mining.html            # Mining page with Configure/History tabs
│   ├── btpc-common.js         # Existing utility module (reused)
│   ├── btpc-storage.js        # localStorage wrapper (reused)
│   ├── btpc-tab-manager.js    # NEW: Tab switching logic module
│   └── btpc-styles.css        # Styles including tab active/inactive states
│
└── src-tauri/                  # Backend (Tauri/Rust) - NOT MODIFIED FOR THIS FEATURE
    └── src/
        └── main.rs            # Tauri commands (no changes needed)
```

**Structure Decision**: Desktop Wallet App (Tauri-based) - Frontend-only feature

This feature modifies only the frontend UI layer in `btpc-desktop-app/ui/`. The backend (Rust/Tauri) is not involved because:
1. Tab switching is pure client-side UI navigation
2. No new Tauri commands needed
3. No backend state changes or validation required
4. localStorage operations are browser-native (no Tauri API needed)

**Files to Create**:
- `btpc-desktop-app/ui/btpc-tab-manager.js` - Core tab switching logic module

**Files to Modify**:
- `btpc-desktop-app/ui/settings.html` - Add tab functionality
- `btpc-desktop-app/ui/transactions.html` - Add tab functionality
- `btpc-desktop-app/ui/mining.html` - Add tab functionality
- `btpc-desktop-app/ui/btpc-styles.css` - Add tab visual states (if needed)

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
1. **From research.md**: Create btpc-tab-manager.js module
   - Task: Implement TabManager class with all methods
   - Task: Add ARIA attribute management
   - Task: Add keyboard navigation handlers
   - Task: Add localStorage persistence

2. **From data-model.md**: Update HTML files
   - Task: Add ARIA attributes to settings.html tab buttons
   - Task: Add ARIA attributes to transactions.html tab buttons
   - Task: Add ARIA attributes to mining.html tab buttons
   - Task: Replace inline switchTab() with TabManager initialization

3. **From quickstart.md**: Manual testing
   - Task: Execute 13 acceptance scenarios (1-13)
   - Task: Test edge cases (localStorage disabled, rapid clicks, missing content)
   - Task: Verify ARIA attributes and screen reader compatibility
   - Task: Validate performance (< 50ms, 60fps)

4. **Optional CSS enhancements**:
   - Task: Add focus state styles (.tab-btn:focus) if not already present

**Ordering Strategy**:
1. **Module First**: Create btpc-tab-manager.js (foundation)
2. **HTML Updates**: Modify settings.html, transactions.html, mining.html [P] (can be parallel)
3. **Testing**: Manual test execution (depends on implementation complete)

**Estimated Output**: 10-12 numbered, ordered tasks in tasks.md

**Dependencies**:
- Settings.html depends on btpc-tab-manager.js
- Transactions.html depends on btpc-tab-manager.js
- Mining.html depends on btpc-tab-manager.js
- Testing depends on all HTML files updated

**IMPORTANT**: This phase is executed by the /tasks command, NOT by /plan

## Phase 3+: Future Implementation
*These phases are beyond the scope of the /plan command*

**Phase 3**: Task execution (/tasks command creates tasks.md)  
**Phase 4**: Implementation (execute tasks.md following constitutional principles)  
**Phase 5**: Validation (run tests, execute quickstart.md, performance validation)

## Complexity Tracking
*Fill ONLY if Constitution Check has violations that must be justified*

**No Constitutional Violations**: All Article XI checks passed. No complexity deviations to document.

This feature is LOW COMPLEXITY:
- Pure frontend JavaScript (no backend changes)
- Uses existing DOM structure (minor HTML attribute additions only)
- Single shared module (btpc-tab-manager.js) eliminates code duplication
- No new dependencies (vanilla JS + browser localStorage API)
- Estimated implementation time: ~5 hours


## Progress Tracking
*This checklist is updated during execution flow*

**Phase Status**:
- [x] Phase 0: Research complete (/plan command) - research.md created
- [x] Phase 1: Design complete (/plan command) - data-model.md, quickstart.md created, CLAUDE.md updated
- [x] Phase 2: Task planning complete (/plan command - describe approach only) - approach documented above
- [x] Phase 3: Tasks generated (/tasks command) - tasks.md created with 14 implementation tasks
- [ ] Phase 4: Implementation complete - NEXT STEP (execute T001-T014)
- [ ] Phase 5: Validation passed

**Gate Status**:
- [x] Initial Constitution Check: PASS (Article XI compliant, localStorage exception justified)
- [x] Post-Design Constitution Check: PASS (no design changes violating constitution)
- [x] All NEEDS CLARIFICATION resolved (none in Technical Context)
- [x] Complexity deviations documented (none - feature is low complexity)

**Artifacts Generated**:
- [x] `/specs/004-fix-non-functional/plan.md` (this file)
- [x] `/specs/004-fix-non-functional/research.md` (patterns, decisions, root cause analysis)
- [x] `/specs/004-fix-non-functional/data-model.md` (entities: TabState, TabButton, TabContent, TabManager)
- [x] `/specs/004-fix-non-functional/quickstart.md` (manual test plan, 13 scenarios + edge cases)
- [x] `/specs/004-fix-non-functional/tasks.md` (14 implementation tasks: T001-T014)
- [x] `/CLAUDE.md` (updated with JavaScript, localStorage, Tauri context)

**Next Step**: Execute tasks T001-T014 to implement the feature

---
*Based on Constitution v2.1.1 - See `/memory/constitution.md`*
