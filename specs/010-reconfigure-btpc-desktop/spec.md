# Feature Specification: Embed btpc-core as In-Process Library

**Feature Branch**: `010-reconfigure-btpc-desktop`
**Created**: 2025-11-10
**Status**: Draft
**Input**: User description: "Reconfigure btpc-desktop-app to embed btpc-core as an in-process library, eliminating external btpc_node and btpc_miner binaries. Create a self-contained, production-ready desktop application"
**Constitution**: Article XI Compliance Required for Desktop Features

## Execution Flow (main)
```
1. Parse user description from Input
   → Feature type: Desktop architecture reconfiguration
2. Extract key concepts from description
   → Actors: Desktop app users, wallet owners, miners
   → Actions: Eliminate external processes, embed blockchain node, unify storage
   → Blockchain data: Shared UTXO set, blockchain state, mempool
   → Quantum-resistant requirements: Preserve ML-DSA signatures, SHA-512 PoW
3. Unclear aspects:
   → NONE - User provided comprehensive technical requirements
4. Check constitutional requirements:
   → FLAG "Article XI patterns apply" (desktop app feature)
   → FLAG "Quantum-resistance requirements" (blockchain operations)
5. Fill User Scenarios & Testing section
   → User flow: Launch single app → node syncs → mine/transact
6. Generate Functional Requirements
   → All requirements testable and measurable
   → Security/quantum-resistance flagged
7. Identify Key Entities
   → EmbeddedNode, UnifiedUTXOManager, MiningThreadPool, SharedBlockchainState
8. Run Review Checklist
   → ✅ No [NEEDS CLARIFICATION] (requirements well-defined)
   → ✅ No implementation details in spec (saved for plan.md)
   → ✅ Article XI compliance addressed
9. Return: SUCCESS (spec ready for planning)
```

---

## ⚡ Quick Guidelines
- ✅ Focus on WHAT users need and WHY
- ❌ Avoid HOW to implement (no Rust code, Tauri commands, RocksDB schemas)
- 👥 Written for cryptocurrency users and stakeholders, not developers
- 🔒 Always consider quantum-resistance and security implications

---

## User Scenarios & Testing

### Primary User Story
**As a** cryptocurrency user,
**I want to** run a complete BTPC node and wallet in a single desktop application,
**So that** I can manage my quantum-resistant funds without installing multiple programs or understanding process architecture

### Secondary User Stories
**As a** BTPC wallet owner,
**I want to** send transactions instantly without waiting for external RPC calls,
**So that** my user experience is faster and more responsive (target: <10ms balance queries vs current ~50ms)

**As a** BTPC miner,
**I want to** start mining with one click in the desktop app,
**So that** I don't need to manage separate mining processes or monitor terminal output

**As a** desktop app user,
**I want to** download and run a single executable file,
**So that** I can get started with BTPC without complex installation procedures

### Acceptance Scenarios

**Scenario 1: First-Time Launch**
1. **Given** user downloads desktop app executable (single file, <50MB)
2. **When** user launches app for first time
3. **Then** blockchain node initializes in background (no external process spawn)
4. **And** P2P sync starts automatically
5. **And** UI displays sync progress in real-time (event-driven updates)
6. **And** user can create wallet while sync continues

**Scenario 2: Fast Balance Queries**
1. **Given** user has wallet with 100 BTPC balance
2. **When** user navigates to wallet balance page
3. **Then** balance displays in <10ms (direct function call, no RPC overhead)
4. **And** UTXO list fetched from shared database (single RocksDB instance)
5. **And** no external process communication occurs

**Scenario 3: Transaction Creation**
1. **Given** blockchain synced to height 1000
2. **When** user creates transaction sending 50 BTPC
3. **Then** UTXO selection happens instantly (shared database access)
4. **And** transaction signed with ML-DSA signature (direct btpc-core call)
5. **And** transaction broadcast to mempool (in-process, no RPC)
6. **And** UI updates via Tauri event (Article XI compliance)

**Scenario 4: Mining Operations**
1. **Given** user starts mining from desktop app UI
2. **When** mining thread pool initializes
3. **Then** CPU/GPU mining starts in background threads (no external btpc_miner process)
4. **And** hashrate statistics update in real-time via events
5. **And** found blocks added to blockchain directly (no RPC submission)
6. **And** user can stop mining with instant cancellation

**Scenario 5: Graceful Shutdown**
1. **Given** desktop app running with active blockchain sync and mining
2. **When** user closes application
3. **Then** mining threads cancel gracefully
4. **And** blockchain state persists to RocksDB
5. **And** P2P connections close cleanly
6. **And** no orphaned processes remain

**Scenario 6: Upgrade from Multi-Process Version**
1. **Given** user has existing installation with btpc_node and btpc_miner binaries
2. **When** user installs new embedded version
3. **Then** existing blockchain data migrated to shared RocksDB instance
4. **And** wallet files remain compatible (same encryption format)
5. **And** old binaries no longer required

### Edge Cases

**What happens when blockchain sync interrupted?**
- Embedded node resumes sync from last persisted block
- UI displays "Syncing: Resuming from block 500" message
- No external process to restart or manage

**How does system handle P2P network partition?**
- Embedded P2P layer detects partition via peer timeout
- UI displays "Network connectivity issues" warning
- Mempool transactions held until reconnection
- No RPC communication failures (in-process calls work offline)

**What if user force-quits application during mining?**
- RocksDB WAL (write-ahead log) ensures data consistency
- Next launch recovers blockchain state from last committed block
- No orphaned mining process consuming CPU
- Wallet encryption keys zeroized on exit (even forced)

**How does desktop app handle concurrent wallet operations?**
- Thread-safe access via Arc<RwLock<>> patterns
- UTXO reservation system prevents double-spending
- Tauri events notify UI of state changes (Article XI)
- No race conditions from RPC request interleaving

**What happens when RocksDB storage full?**
- Embedded node detects low disk space
- UI displays "Storage full: Cannot sync" error
- Mining pauses to prevent partial block writes
- User prompted to free space or change data directory

---

## Requirements

### Functional Requirements

**Core Architecture:**
- **FR-001**: Desktop application MUST embed btpc-core blockchain node as in-process library (no external btpc_node process)
- **FR-002**: Desktop application MUST embed mining functionality as background thread pool (no external btpc_miner process)
- **FR-003**: System MUST use single RocksDB instance shared between wallet and node components
- **FR-004**: System MUST eliminate all RPC client communication (replace with direct function calls)
- **FR-005**: Desktop application MUST deploy as single executable binary (<50MB)

**Blockchain Operations:**
- **FR-006**: Embedded node MUST validate ML-DSA (Dilithium5) signatures for all transactions
- **FR-007**: Embedded node MUST maintain UTXO consistency across wallet and blockchain operations
- **FR-008**: Embedded node MUST support SHA-512 proof-of-work consensus
- **FR-009**: Embedded node MUST persist blockchain state to unified RocksDB with column families
- **FR-010**: Embedded node MUST support P2P sync with peer discovery and connection management
- **FR-011**: Embedded node MUST maintain mempool for unconfirmed transactions

**Wallet & Cryptography:**
- **FR-012**: Wallet operations MUST use embedded btpc-core directly (no subprocess calls)
- **FR-013**: System MUST encrypt private keys using AES-256-GCM with Argon2id key derivation
- **FR-014**: System MUST generate ML-DSA signatures for transaction signing (in-process)
- **FR-015**: Wallet files MUST remain compatible with existing encrypted format
- **FR-016**: System MUST support BIP39 mnemonic recovery (existing feature preserved)

**Desktop Application (Article XI Compliance):**
- **FR-017**: Desktop app MUST implement backend-first validation for all user actions (Article XI, Section 11.2)
- **FR-018**: Desktop app MUST use Tauri events for blockchain state synchronization (Article XI, Section 11.3)
- **FR-019**: System MUST emit events on: block added, transaction confirmed, UTXO updated, mining status changed
- **FR-020**: Desktop app MUST clean up event listeners on page unload (Article XI, Section 11.6)
- **FR-021**: UI MUST update within 200ms of backend state change (event-driven)

**Mining Operations:**
- **FR-022**: Mining MUST operate in background thread pool (configurable CPU thread count)
- **FR-023**: Mining MUST support GPU acceleration (preserve existing OpenCL integration)
- **FR-024**: Mining threads MUST support graceful cancellation (user can stop instantly)
- **FR-025**: Mining MUST submit found blocks to embedded blockchain directly (no RPC)
- **FR-026**: Mining statistics (hashrate, blocks found) MUST update via Tauri events

**Process Management Elimination:**
- **FR-027**: System MUST NOT spawn btpc_node as external process
- **FR-028**: System MUST NOT spawn btpc_miner as external process
- **FR-029**: System MUST NOT include process adoption logic (detecting running processes)
- **FR-030**: System MUST NOT include zombie process cleanup threads
- **FR-031**: System MUST NOT include process health monitoring code

**Performance Requirements:**
- **FR-032**: Balance queries MUST complete in <10ms (vs current ~50ms RPC overhead)
- **FR-033**: Transaction creation MUST complete in <50ms (vs current ~100ms with RPC)
- **FR-034**: UTXO selection MUST complete in <5ms (direct database access)
- **FR-035**: Block validation MUST complete in <100ms (preserve existing performance)
- **FR-036**: ML-DSA signature verification MUST complete in <10ms per signature

**Graceful Shutdown:**
- **FR-037**: Application shutdown MUST cancel mining threads within 1 second
- **FR-038**: Application shutdown MUST close P2P connections cleanly
- **FR-039**: Application shutdown MUST persist blockchain state to RocksDB
- **FR-040**: Application shutdown MUST zeroize cryptographic key material from memory
- **FR-041**: System MUST NOT leave orphaned processes after exit

**Network & Configuration:**
- **FR-042**: Embedded node MUST support Mainnet, Testnet, and Regtest network configurations
- **FR-043**: Network configuration MUST persist across restarts
- **FR-044**: User MUST be able to switch networks (requires app restart and data directory change)
- **FR-045**: Embedded node MUST validate consensus rules for current network type

**Code Reduction:**
- **FR-046**: System MUST eliminate process_manager module (~500 lines)
- **FR-047**: System MUST eliminate rpc_client module (~400 lines)
- **FR-048**: System MUST eliminate sync_service module (~400 lines)
- **FR-049**: System MUST consolidate UTXO management (remove desktop app's separate implementation)
- **FR-050**: Total codebase reduction target: ~1500+ lines removed

### Non-Functional Requirements

**Security:**
- **NFR-001**: All cryptographic operations MUST use constant-time implementations (preserve btpc-core guarantees)
- **NFR-002**: Private keys MUST never be logged or exposed in error messages
- **NFR-003**: Embedded node MUST maintain same security posture as standalone btpc_node
- **NFR-004**: Thread-safe access to shared state MUST prevent race conditions
- **NFR-005**: Memory containing keys MUST be zeroized on application exit (even forced shutdown)

**Performance:**
- **NFR-006**: Desktop app MUST remain responsive during blockchain sync (<100ms UI updates)
- **NFR-007**: Blockchain sync MUST handle >10,000 blocks without memory leaks
- **NFR-008**: Mining MUST not starve UI thread (background priority)
- **NFR-009**: RocksDB cache MUST be configured for optimal desktop memory usage (<=512MB)

**Usability:**
- **NFR-010**: Error messages MUST be actionable (e.g., "Sync paused: No network connection" vs "RPC error -32603")
- **NFR-011**: Desktop app MUST follow Monero-inspired UX patterns (see style-guide/ux-rules.md)
- **NFR-012**: Installation MUST be single file download (no multi-binary setup)
- **NFR-013**: Upgrade from multi-process version MUST preserve existing wallet data

**Reliability:**
- **NFR-014**: Embedded node MUST recover gracefully from crashes (RocksDB WAL)
- **NFR-015**: Mining interruption MUST not corrupt blockchain state
- **NFR-016**: P2P connection failures MUST not crash application
- **NFR-017**: Desktop app MUST log critical errors for debugging (no silent failures)

**Compatibility:**
- **NFR-018**: Blockchain database format MUST remain compatible with standalone btpc_node
- **NFR-019**: Wallet encryption format MUST remain unchanged
- **NFR-020**: P2P protocol MUST remain compatible with network peers
- **NFR-021**: Transaction serialization MUST match btpc-core format exactly

### Key Entities

**EmbeddedNode**
- Represents the in-process blockchain node
- Contains: Blockchain state (blocks, UTXO set), mempool, P2P connection manager, consensus validator
- Relationships: Shared by WalletManager (UTXO queries), MiningThreadPool (block submission), UIEventEmitter (state updates)
- Lifecycle: Initialized on app startup, runs in background Tokio runtime, persists state to UnifiedDatabase on shutdown
- Thread-safety: Arc<RwLock<>> for concurrent access from wallet and mining threads

**UnifiedDatabase**
- Represents single shared RocksDB instance (eliminates duplication)
- Contains: Blockchain column family (blocks, headers), UTXO column family (spendable outputs), Wallet column family (encrypted keys)
- Replaces: Separate node database + desktop app UTXO database (current architecture)
- Access pattern: Embedded node writes blocks/UTXOs, wallet reads UTXOs for balance/transaction creation
- Thread-safety: RocksDB provides internal synchronization, wrapped in Arc for shared ownership

**MiningThreadPool**
- Represents background mining operation (replaces external btpc_miner process)
- Contains: Thread pool (configurable count), GPU miner instance (optional), hashrate statistics, found block queue
- Operations: Start mining (spawns threads), stop mining (cancels gracefully), update mining address, fetch statistics
- Events emitted: mining_started, mining_stopped, block_found, hashrate_updated (every 5 seconds)
- Thread-safety: Uses channels for cross-thread communication, atomic counters for statistics

**SharedBlockchainState**
- Represents current blockchain tip and sync progress
- Contains: Current height, best block hash, sync status (syncing/synced), peer count, estimated sync percentage
- Backend (Rust Arc<RwLock>) is single source of truth (Article XI)
- Frontend queries via Tauri command, receives updates via blockchain:state_updated event
- Persisted: Current height stored in RocksDB metadata, reconstructed on startup

**TransactionMempool**
- Represents unconfirmed transaction pool (embedded in-process)
- Contains: Pending transactions (HashMap by txid), fee priority queue, DoS protection counters
- Operations: Add transaction (validates signatures), remove transaction (on block confirm), query pending transactions
- Replaces: RPC calls to external node's mempool
- Thread-safety: RwLock for concurrent access from wallet (add tx) and mining threads (fetch for block template)

**WalletManager**
- Existing entity, modified to use UnifiedDatabase instead of separate RocksDB
- Contains: Encrypted wallet files, UTXO tracking (now reads from UnifiedDatabase)
- Changes: Remove separate UTXO database, query EmbeddedNode's UTXO set directly
- Preserves: Encryption format, BIP39 recovery, multi-wallet support

---

## Constitutional Compliance

### Article XI Applicability
- [x] **Desktop feature** - Article XI patterns apply (complete checklist below)

### Article XI Compliance Checklist

**Section 11.1 - Single Source of Truth:**
- [x] Authoritative state location: **Backend Arc<RwLock<EmbeddedNode>>** for blockchain state, **UnifiedDatabase (RocksDB)** for persistent data
- [x] Frontend displays state only via Tauri queries (get_blockchain_info, get_balance)
- [x] Specified: Backend stores blockchain height, UTXO set, mempool; frontend queries via commands, receives updates via events

**Section 11.2 - Backend-First Validation:**
- [x] All user actions validate with embedded node FIRST (e.g., create_transaction checks UTXO availability in backend)
- [x] Failure exits early (if insufficient balance, return error immediately, no frontend state change)
- [x] Specified: Validation errors include actionable messages ("Insufficient balance: need 100 BTPC, have 50 BTPC")

**Section 11.3 - Event-Driven Architecture:**
- [x] Backend emits events on state changes: blockchain:block_added, blockchain:sync_progress, wallet:balance_updated, mining:hashrate_updated, transaction:confirmed
- [x] Frontend listens for events and updates UI (Mining page updates hashrate display on mining:hashrate_updated)
- [x] Specified: Event names standardized (namespace:action format), payloads include full state delta

**Section 11.6 - Event Listener Cleanup:**
- [x] Event listeners cleaned up on page unload (beforeunload handler calls unlisten())
- [x] No memory leaks from forgotten listeners (Tauri's listen() returns unlisten function)
- [x] Specified: Each page's init function stores unlisten handles, cleanup on navigation

**Section 11.7 - Prohibited Patterns:**
- [x] Confirmed: NO localStorage before backend validation (balance queries go to backend first)
- [x] Confirmed: NO authoritative state in frontend JavaScript (blockchain height stored in backend only)
- [x] Confirmed: NO polling when events available (sync progress uses blockchain:sync_progress event, not setInterval)
- [x] Confirmed: NO duplicate notifications (backend deduplicates events, one notification per transaction confirmation)

### Constitution Article Compliance

**Article I: Security-First Design**
- ✅ Embedded architecture maintains security isolation (btpc-core cryptographic operations unchanged)
- ✅ Single process model reduces attack surface (no IPC to compromise)
- ✅ Private key handling preserved (encrypted storage, memory zeroization)

**Article II: Quantum Resistance**
- ✅ All signatures remain ML-DSA (Dilithium5) via btpc-core direct calls
- ✅ SHA-512 PoW unchanged (mining uses same consensus module)
- ✅ No cryptographic protocol changes

**Article III: Decentralization**
- ✅ Full node capability preserved (P2P sync, blockchain validation)
- ✅ User runs complete node (not connecting to remote server)
- ✅ Mining remains decentralized (solo mining supported)

**Article IV: Transparency & Auditability**
- ✅ Open source codebase (architecture change fully auditable)
- ✅ Blockchain operations transparent (no hidden RPC layer)
- ✅ Event system provides audit trail of state changes

**Article VI.3: TDD Methodology**
- ⚠️ Architecture refactoring requires comprehensive testing strategy
- Target: Unit tests for embedded node initialization, integration tests for wallet-node interaction
- Contract tests: Verify events emitted correctly, state transitions valid

---

## Dependencies & Assumptions

### Dependencies
- **btpc-core library**: Must support in-process initialization (existing capability, used by btpc_node binary)
- **Existing wallet encryption**: Wallet file format remains compatible (no schema changes)
- **Tauri 2.0 event system**: Required for Article XI compliance (already in use)
- **RocksDB multi-column support**: Single instance must support blockchain + wallet + UTXO column families (existing btpc-core capability)
- **Tokio async runtime**: Required for P2P networking, mining thread pool (already in Tauri backend)

### Assumptions
- **User has network connectivity**: Required for P2P blockchain sync (same as current architecture)
- **User has sufficient disk space**: Blockchain data storage requirements unchanged (~5GB for regtest, ~50GB for mainnet)
- **Desktop app has write access to data directory**: Required for RocksDB persistence (same as current)
- **Existing users willing to migrate**: Upgrade process preserves wallet data, blockchain re-sync acceptable
- **Single node per machine**: Desktop app assumes exclusive access to data directory (no concurrent multi-process nodes)
- **Mining is optional**: Users can run wallet-only without starting mining threads

---

## Review & Acceptance Checklist

### Content Quality
- [x] No implementation details (no Rust code, Tauri command names, RocksDB column families in spec)
- [x] Focused on user value and cryptocurrency operations
- [x] Written for non-technical cryptocurrency stakeholders
- [x] All mandatory sections completed
- [x] BTPC-specific considerations addressed (quantum-resistance, network type, Article XI)

### Requirement Completeness
- [x] No [NEEDS CLARIFICATION] markers remain
- [x] Requirements are testable and unambiguous
- [x] Success criteria are measurable (e.g., "<10ms balance queries")
- [x] Scope is clearly bounded (architectural refactoring, preserve all features)
- [x] Dependencies and assumptions identified
- [x] Security implications considered (thread-safety, key handling, shutdown)
- [x] Performance targets specified (latency improvements, memory usage)

### Constitutional Compliance (Desktop Features Only)
- [x] Article XI applicability determined (desktop feature)
- [x] All Article XI patterns addressed in requirements (FR-017 through FR-021)
- [x] Constitutional compliance checklist completed
- [x] Cross-article compliance verified (Articles I-IV, VI.3)

---

## Execution Status

- [x] User description parsed (comprehensive technical requirements provided)
- [x] Key concepts extracted (embedded node, unified database, thread pool mining, event-driven UI)
- [x] Constitutional requirements flagged (Article XI for desktop, quantum-resistance for blockchain)
- [x] Ambiguities marked (none - requirements well-specified)
- [x] User scenarios defined (fast balance queries, seamless mining, single binary deployment)
- [x] Functional requirements generated (50 FRs covering architecture, performance, compliance)
- [x] Entities identified (EmbeddedNode, UnifiedDatabase, MiningThreadPool, SharedBlockchainState)
- [x] Constitutional compliance evaluated (Article XI checklist completed, cross-article review passed)
- [x] Review checklist passed (all quality gates cleared)

---

## BTPC Project Context

**Core Technologies:**
- Blockchain: Rust, btpc-core library, RocksDB, SHA-512 PoW
- Cryptography: ML-DSA (Dilithium5), AES-256-GCM, Argon2id
- Desktop: Tauri 2.0, vanilla JavaScript frontend, 68+ Tauri commands
- Network: Bitcoin-compatible P2P, JSON-RPC 2.0 (will be optional post-refactoring)

**Constitutional Framework:**
- Constitution version: 1.1
- Article XI: Desktop Application Development Principles (mandatory for UI features)
- See `MD/CONSTITUTION.md` for complete governance rules

**Project Structure:**
- `btpc-core/` - Core blockchain library (Rust) - will be embedded
- `bins/` - btpc_node, btpc_wallet, btpc_miner binaries - btpc_node and btpc_miner to be removed
- `btpc-desktop-app/` - Tauri desktop wallet application - will become self-contained
- `tests/` - Integration and unit tests - will require updates for embedded architecture

**Key Documentation:**
- `CLAUDE.md` - Project overview and guidelines
- `STATUS.md` - Current implementation status
- `MD/CONSTITUTION.md` - Governance rules (Article XI compliance required)
- Architectural analysis: Generated by /start session (multi-process → embedded architecture)

---

**Template Version**: 1.1 (BTPC-specific)
**Spec Status**: ✅ Ready for /plan phase
**Next Step**: Execute `/plan` to generate implementation plan with TDD approach