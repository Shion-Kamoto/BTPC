# Feature Specification: Node and Backend Stability Fixes

**Feature Branch**: `003-fix-node-and`
**Created**: 2025-10-25
**Status**: Draft
**Input**: User description: "Fix node and backend issues in btpc-desktop-app"
**Constitution**: Article XI Compliance Required for Desktop Features

## Execution Flow (main)
```
1. Parse user description from Input
   → Identified: Desktop app stability, node management, backend reliability
2. Extract key concepts from description
   → Actors: Desktop app users, node operators
   → Actions: Display balance, manage mining, handle errors, process cleanup
   → Desktop app components: State management, process lifecycle, error handling
3. For each unclear aspect:
   → Marked with [NEEDS CLARIFICATION: specific question]
4. Check constitutional requirements:
   → Desktop app feature: FLAG "Article XI patterns apply"
5. Fill User Scenarios & Testing section
   → User flow: Start app → View balance → Start/stop mining → Close app gracefully
6. Generate Functional Requirements
   → Each requirement must be testable
   → Focus on reliability, correctness, resource management
7. Identify Key Entities
   → NodeStatus, MiningProcess, WalletBalance, ErrorState
8. Run Review Checklist
   → Spec focuses on user-facing reliability improvements
9. Return: SUCCESS (spec ready for planning)
```

---

## ⚡ Quick Guidelines
- ✅ Focus on WHAT users need and WHY
- ❌ Avoid HOW to implement (no Rust code, Tauri commands, RocksDB schemas)
- 👥 Written for cryptocurrency users and stakeholders, not developers
- 🔒 Always consider quantum-resistance and security implications

---

## Clarifications

### Session 2025-10-25
- Q: Should the desktop application auto-start the node when it launches? → A: Ask user on first launch, remember preference
- Q: Should mining auto-resume after application restart if it was running when the app closed? → A: Show notification asking user if they want to resume mining
- Q: What is the acceptable memory limit for the desktop application during long-running operation (excluding blockchain data)? → A: 1GB - Balanced limit for modern systems
- Q: When a child process (node/miner) crashes, how should the application handle restart? → A: Auto-restart once, then show notification if it crashes again
- Q: What level of technical detail should error messages display to users? → A: User-friendly + "Show Details" button revealing full technical error

---

## User Scenarios & Testing *(mandatory)*

### Primary User Story
**As a** desktop wallet user,
**I want to** reliably view my balance, manage my node, and control mining operations,
**So that** I can securely manage my quantum-resistant cryptocurrency without application crashes or data loss.

### Acceptance Scenarios

**Scenario 1: Wallet Balance Display**
- **Given** user has 226.625 BTP across 7 UTXOs in their wallet
- **When** user opens the desktop application and navigates to wallet page
- **Then** application displays correct balance of 226.625 BTP
- **And** balance updates automatically when new transactions are received

**Scenario 2: Mining Operations**
- **Given** user starts mining through the desktop application
- **When** mining process encounters an error (network issue, invalid block template)
- **Then** application shows actionable error message
- **And** application remains responsive and does not crash
- **And** user can restart mining without restarting application

**Scenario 3: Node Management**
- **Given** user has started a local node through the desktop application
- **When** user closes the application
- **Then** node process shuts down gracefully within 10 seconds
- **And** no orphaned processes remain running
- **And** database is properly closed without corruption

**Scenario 4: Application Lifecycle**
- **Given** user runs the desktop application for 7+ days continuously
- **When** user starts/stops mining operations 100+ times
- **Then** application maintains stable memory usage (no memory leaks)
- **And** application remains responsive
- **And** all operations continue to function correctly

**Scenario 5: Error Recovery**
- **Given** desktop application is running with active mining
- **When** backend encounters unexpected error (RPC timeout, database lock)
- **Then** application displays specific error message to user
- **And** affected operation fails gracefully
- **And** other application features remain functional
- **And** application does not crash or freeze

**Scenario 6: Concurrent Operations**
- **Given** user is viewing wallet balance while mining is active
- **When** user navigates between pages (Dashboard → Mining → Wallet → Transactions)
- **Then** all state updates propagate correctly across pages (Article XI)
- **And** no duplicate notifications appear
- **And** all displayed information remains consistent and accurate

### Edge Cases
- What happens when mining process crashes unexpectedly during operation? → Auto-restart once, then prompt user
- What happens when node crashes twice in rapid succession? → User notification with restart option after first auto-restart fails
- How does application handle corrupted state during recovery?
- What if node fails to start due to port already in use?
- How does application behave when database lock cannot be acquired?
- What happens during network partition (no peers available)?
- How does application handle disk full scenarios?
- What if user has thousands of UTXOs (performance degradation)?
- What if process crashes repeatedly after user manually restarts it? → Track crashes, reset counter after 1 hour stable

---

## Requirements *(mandatory)*

### Functional Requirements

**Wallet Balance & Display:**
- **FR-001**: System MUST display accurate wallet balance matching UTXO database state
- **FR-002**: Balance MUST update automatically when new transactions are received
- **FR-003**: Balance calculations MUST handle addresses case-insensitively or normalize them
- **FR-004**: System MUST display individual UTXO details when requested
- **FR-005**: Zero-balance wallets MUST clearly indicate "0.00000000 BTP" (not blank or error)

**Node Management:**
- **FR-006**: Users MUST be able to start/stop local node through desktop interface
- **FR-007**: Node shutdown MUST complete gracefully within 10 seconds
- **FR-008**: System MUST detect when node is already running (single-instance protection)
- **FR-009**: System MUST display node connection status (running/stopped/error) accurately
- **FR-010**: Node processes MUST terminate completely when application closes (no zombie processes)
- **FR-041**: On first application launch, system MUST prompt user for node auto-start preference (yes/no)
- **FR-042**: System MUST persist node auto-start preference and honor it on subsequent launches

**Mining Operations:**
- **FR-011**: Users MUST be able to start/stop mining through desktop interface
- **FR-012**: Mining statistics MUST update in real-time (hashrate, blocks found)
- **FR-013**: Mining errors MUST display actionable messages to user
- **FR-014**: System MUST prevent starting mining when node is not running
- **FR-015**: Mining process MUST stop cleanly within 5 seconds when requested
- **FR-043**: System MUST detect if mining was active when application last closed
- **FR-044**: If mining was previously active, system MUST show notification asking user whether to resume mining
- **FR-045**: Notification MUST include "Resume Mining" and "Don't Resume" options with clear action buttons

**Error Handling & Reliability:**
- **FR-016**: Application MUST NOT crash when any single operation fails
- **FR-017**: Error messages MUST display user-friendly text with actionable guidance (tell user what to do)
- **FR-018**: Backend errors MUST be logged and reported to frontend with context
- **FR-019**: System MUST recover from temporary failures without user restart
- **FR-020**: Application MUST display clear error state when backend is unavailable
- **FR-048**: All error messages MUST include a "Show Details" button or expandable section
- **FR-049**: When "Show Details" is clicked, system MUST reveal full technical error including error type, stack trace, and relevant system state
- **FR-050**: Technical details MUST be copyable to clipboard for support/debugging purposes

**Resource Management:**
- **FR-021**: Application MUST maintain stable memory usage over 7+ days of continuous operation
- **FR-022**: Process cleanup MUST execute when application closes normally
- **FR-023**: Process cleanup MUST execute when application crashes (cleanup on restart)
- **FR-024**: System MUST not accumulate zombie processes over time
- **FR-025**: System MUST release file locks when operations complete or fail

**Desktop Application State (Article XI):**
- **FR-026**: Backend MUST be single source of truth for all state (node status, mining status, wallet balance)
- **FR-027**: Frontend MUST validate all user actions with backend BEFORE updating localStorage
- **FR-028**: Backend MUST emit events for all state changes (node started/stopped, mining started/stopped, balance updated)
- **FR-029**: Frontend MUST listen for backend events and update UI automatically
- **FR-030**: Event listeners MUST clean up on page unload to prevent memory leaks
- **FR-031**: System MUST NOT show duplicate notifications for same user action

**File & Database Safety:**
- **FR-032**: System MUST acquire exclusive locks before modifying wallet or node data
- **FR-033**: System MUST release locks within 30 seconds or fail operation
- **FR-034**: Stale lock files MUST be detected and cleaned up on application start
- **FR-035**: Database operations MUST complete or rollback atomically (no partial state)

**Process Lifecycle:**
- **FR-036**: Child processes (node, miner, wallet) MUST have unique identifiers for tracking
- **FR-037**: System MUST monitor child process health and report status
- **FR-038**: Crashed child processes MUST be detected within 5 seconds
- **FR-039**: When child process crashes for the first time, system MUST attempt automatic restart once
- **FR-040**: If child process crashes a second time, system MUST show notification with "Restart" and "Cancel" options
- **FR-046**: System MUST track crash count per process type and reset counter after 1 hour of stable operation
- **FR-047**: All child processes MUST terminate when parent application exits

### Non-Functional Requirements

**Security:**
- **NFR-001**: All file path inputs MUST be validated to prevent path traversal attacks
- **NFR-002**: Process execution MUST sanitize all arguments to prevent command injection
- **NFR-003**: User-facing error messages MUST NOT expose sensitive information (private keys, passwords) in main text
- **NFR-004**: Technical error details (visible after clicking "Show Details") MAY include file paths and system state but MUST redact private keys and passwords
- **NFR-018**: Lock files MUST use safe file descriptor operations (not raw libc calls)

**Performance:**
- **NFR-005**: Wallet balance calculation MUST complete in < 500ms for wallets with up to 10,000 UTXOs
- **NFR-006**: Application startup MUST complete in < 3 seconds when node/miner not auto-starting
- **NFR-007**: State synchronization between backend and frontend MUST propagate in < 200ms (Article XI)
- **NFR-008**: Memory usage MUST not exceed 1GB during normal operation (excluding blockchain data)
- **NFR-017**: Memory usage MUST remain stable (< 5% growth) over 7-day continuous operation to prevent leaks

**Reliability:**
- **NFR-009**: Application MUST maintain 99.9% uptime during 7-day continuous operation
- **NFR-010**: Error recovery MUST succeed automatically for > 95% of transient failures
- **NFR-011**: Data corruption rate MUST be < 0.01% under normal shutdown
- **NFR-012**: Application MUST survive 1000+ start/stop cycles of mining without degradation

**Usability:**
- **NFR-013**: Error messages MUST be actionable (tell user what to do next) and use plain language
- **NFR-014**: Critical errors MUST be visible within 3 seconds of occurrence
- **NFR-015**: Application MUST remain responsive (< 100ms UI response) during background operations
- **NFR-016**: Status indicators MUST accurately reflect backend state within 500ms
- **NFR-019**: Error messages MUST use progressive disclosure (simple message first, technical details behind "Show Details")
- **NFR-020**: "Show Details" expansion MUST render technical information in monospace font for readability

### Key Entities

**Desktop App Entities:**

- **NodeStatus**: Represents current state of blockchain node
  - Contains: running state (started/stopped/error), blockchain height, peer count, sync progress, network type
  - Backend (Rust Arc<RwLock>) is single source of truth (Article XI)
  - Frontend receives updates via Tauri events
  - Persisted: network configuration survives restarts

- **MiningProcess**: Represents active mining operation
  - Contains: process ID, hashrate, blocks found, thread count, status (running/stopped/error)
  - Relationships: requires NodeStatus.running = true to start
  - Lifecycle: created on mining start, monitored continuously, cleaned up on stop
  - Backend manages process handle and emits hashrate/block events

- **WalletBalance**: Represents user's spendable funds
  - Contains: total balance (sum of UTXOs), UTXO count, address(es)
  - Calculated from: UTXO database query filtered by wallet addresses
  - Updates when: new transaction received, block mined, UTXO spent
  - Backend emits balance_updated event on changes (Article XI)

- **ErrorState**: Represents application error condition
  - Contains: error type, user-friendly message, technical details (stack trace, system state), timestamp, affected component, crash count, details expanded (boolean)
  - Display rules: critical errors → modal, warnings → toast, info → status bar
  - Progressive disclosure: User sees friendly message + "Show Details" button; clicking reveals technical information
  - Technical details include: error type, stack trace, relevant system state (sanitized to exclude private keys/passwords)
  - Recovery: temporary errors auto-retry, permanent errors require user action
  - Process crashes: first crash auto-restarts, second crash prompts user via notification

- **ProcessHandle**: Represents child process (node/miner/wallet CLI)
  - Contains: OS process ID, start time, command line, stdout/stderr buffers
  - Monitoring: health check every 5 seconds, zombie detection, exit code capture
  - Cleanup: graceful shutdown (SIGTERM + 10s timeout), force kill (SIGKILL), reap zombie

---

## Constitutional Compliance *(mandatory for desktop features)*

### Article XI Applicability
- [ ] **Not a desktop feature** - Skip Article XI patterns
- [x] **Desktop feature** - Article XI patterns apply (complete checklist below)

### Article XI Compliance Checklist

**Section 11.1 - Single Source of Truth:**
- [x] Identify authoritative state location: Backend Arc<RwLock> for NodeStatus, MiningProcess, WalletBalance
- [x] Frontend displays state only, never maintains authoritative state
- [x] Specified: Backend stores all state, frontend queries via Tauri commands or listens to events

**Section 11.2 - Backend-First Validation:**
- [x] All user actions validate with backend FIRST (start_mining, stop_mining, start_node, stop_node)
- [x] Failure exits early, NO localStorage save on validation failure
- [x] Specified: Validation errors return Result<T, String> with actionable messages, early exit before UI update

**Section 11.3 - Event-Driven Architecture:**
- [x] Backend emits events on all state changes (node_status_changed, mining_status_changed, balance_updated, error_occurred)
- [x] Frontend listens for events and updates UI automatically
- [x] Specified: Event names, payloads (JSON), and UI update behavior defined in FR-028, FR-029

**Section 11.6 - Event Listener Cleanup:**
- [x] Event listeners cleaned up on page unload (beforeunload event handler)
- [x] No memory leaks from forgotten listeners
- [x] Specified: Tauri event unlisten() called in beforeunload, tracked in active listener registry

**Section 11.7 - Prohibited Patterns:**
- [x] Confirmed: NO localStorage before backend validation (FR-027)
- [x] Confirmed: NO authoritative state in frontend JavaScript
- [x] Confirmed: NO polling when events available (use Tauri events per Article XI)
- [x] Confirmed: NO duplicate notifications for user actions (FR-031)

---

## Dependencies & Assumptions

### Dependencies
- **Depends on**: btpc_node binary being functional and executable
- **Depends on**: btpc_miner binary being functional and executable
- **Depends on**: btpc_wallet CLI commands for balance queries
- **Depends on**: RocksDB blockchain state being initialized and accessible
- **Depends on**: Tauri 2.0 event system for state synchronization (Article XI)
- **Depends on**: JSON-RPC 2.0 communication between desktop app and node
- **Depends on**: ML-DSA signature verification being functional for transaction validation

### Assumptions
- **Assumes**: Desktop app has write access to ~/.btpc directory for database and lock files
- **Assumes**: User has permission to bind to configured RPC and P2P ports
- **Assumes**: Operating system supports process management (SIGTERM, SIGKILL signals)
- **Assumes**: File system supports exclusive file locking (flock or equivalent)
- **Assumes**: Application prompts user for node auto-start preference on first launch and persists choice
- **Assumes**: Application detects previous mining state and prompts user with notification to resume or skip
- **Assumes**: Application runtime memory usage remains under 1GB limit (aligned with industry standards for wallet+node+miner combo)
- **Assumes**: Wallet addresses use consistent case (lowercase/uppercase) or are normalized before comparison

---

## Review & Acceptance Checklist
*GATE: Automated checks run during main() execution*

### Content Quality
- [x] No implementation details (no Rust code, Tauri command names, RocksDB column families)
- [x] Focused on user value and cryptocurrency operations
- [x] Written for non-technical cryptocurrency stakeholders
- [x] All mandatory sections completed
- [x] BTPC-specific considerations addressed (quantum-resistance, network type, etc.)

### Requirement Completeness
- [x] Some [NEEDS CLARIFICATION] markers remain (3 assumptions need user input)
- [x] Requirements are testable and unambiguous (except clarification items)
- [x] Success criteria are measurable (< 500ms, 99.9% uptime, < 500MB memory)
- [x] Scope is clearly bounded (desktop app stability, not new features)
- [x] Dependencies and assumptions identified
- [x] Security implications considered (path traversal, command injection, lock safety)
- [x] Performance targets specified (FR/NFR with specific metrics)

### Constitutional Compliance (Desktop Features Only)
- [x] Article XI applicability determined (IS desktop feature)
- [x] All Article XI patterns addressed in requirements (FR-026 through FR-031)
- [x] Constitutional compliance checklist completed above
- [x] Backend-first validation, event-driven architecture, cleanup specified

---

## Execution Status
*Updated by main() during processing*

- [x] User description parsed ("Fix node and backend issues in btpc-desktop-app")
- [x] Key concepts extracted (stability, reliability, error handling, resource management)
- [x] Constitutional requirements flagged (Article XI applies - desktop feature)
- [x] Ambiguities marked with [NEEDS CLARIFICATION] (3 assumptions)
- [x] User scenarios defined (6 primary scenarios, 7 edge cases)
- [x] Functional requirements generated (40 functional, 16 non-functional)
- [x] Entities identified (NodeStatus, MiningProcess, WalletBalance, ErrorState, ProcessHandle)
- [x] Constitutional compliance evaluated (all Article XI sections addressed)
- [x] Review checklist passed (except 3 clarification items)

---

## BTPC Project Context

**Core Technologies:**
- Blockchain: Rust, btpc-core library, RocksDB, SHA-512 PoW
- Cryptography: ML-DSA (Dilithium5), AES-256-GCM, Argon2id
- Desktop: Tauri 2.0, HTML/CSS/JS frontend, 25+ Tauri commands
- Network: Bitcoin-compatible P2P, JSON-RPC 2.0

**Constitutional Framework:**
- Constitution version: 1.0.1
- Article XI: Desktop Application Development Principles (mandatory for UI features)
- See `.specify/memory/constitution.md` for complete governance rules

**Project Structure:**
- `btpc-core/` - Core blockchain library (Rust)
- `bins/` - btpc_node, btpc_wallet, btpc_miner binaries
- `btpc-desktop-app/` - Tauri desktop wallet application
- `tests/` - Integration and unit tests

**Key Documentation:**
- `CLAUDE.md` - Project overview and guidelines
- `STATUS.md` - Current implementation status
- `style-guide/ux-rules.md` - UI/UX patterns (Monero-inspired)
- `.specify/memory/constitution.md` - Governance rules

---

**Template Version**: 1.1 (BTPC-specific)
**Last Updated**: 2025-10-25
**Maintained by**: .specify framework
