# Tasks: [FEATURE NAME]

**Input**: Design documents from `/specs/[###-feature-name]/`
**Prerequisites**: plan.md (required), spec.md, research.md, data-model.md, contracts/
**Constitution**: Article XI patterns for desktop features

## Execution Flow (main)
```
1. Load plan.md from feature directory
   → If not found: ERROR "No implementation plan found"
   → Extract: tech stack (Rust/Tauri/React), libraries, structure
2. Check constitutional requirements:
   → If desktop feature: FLAG "Article XI patterns apply to tasks"
   → If blockchain feature: FLAG "Quantum-resistance required"
3. Load optional design documents:
   → data-model.md: Extract entities → model tasks (UTXO, Block, Wallet)
   → contracts/: Each file → contract test task
   → research.md: Extract decisions → setup tasks
4. Generate tasks by category:
   → Setup: Rust project, dependencies, clippy config
   → Tests: Contract tests (TDD), integration tests, unit tests
   → Core: Blockchain models, crypto services, RPC handlers
   → Desktop: Tauri commands, React components, event listeners
   → Integration: RocksDB, ML-DSA crypto, network P2P
   → Polish: Performance tests, docs, clippy fixes
5. Apply BTPC-specific task rules:
   → Different Rust modules = mark [P] for parallel
   → Same file = sequential (no [P])
   → Tests before implementation (TDD mandatory)
   → Desktop features = Article XI patterns in each task
6. Number tasks sequentially (T001, T002...)
7. Generate dependency graph
8. Create parallel execution examples
9. Validate task completeness:
   → All Tauri commands have tests?
   → All blockchain entities have validation?
   → All Article XI patterns covered?
10. Return: SUCCESS (tasks ready for execution)
```

## Format: `[ID] [P?] Description`
- **[P]**: Can run in parallel (different files, no dependencies)
- Include exact file paths in BTPC structure
- Reference Article XI sections for desktop tasks

## BTPC Path Conventions
```
BTPC/
├── btpc-core/src/               # Core blockchain library
│   ├── blockchain/              # Block, Transaction, UTXO
│   ├── crypto/                  # ML-DSA, SHA-512
│   ├── consensus/               # PoW, difficulty
│   ├── storage/                 # RocksDB
│   ├── network/                 # P2P protocol
│   └── rpc/                     # JSON-RPC API
├── bins/                        # Binaries
│   ├── btpc_node/src/          # Full node
│   ├── btpc_wallet/src/        # CLI wallet
│   └── btpc_miner/src/         # Mining app
├── btpc-desktop-app/            # Desktop wallet
│   ├── src-tauri/src/          # Rust backend (Tauri commands)
│   └── ui/                      # React frontend
└── tests/                       # Integration tests
    ├── integration/             # E2E blockchain tests
    └── contract/                # API contract tests
```

## Phase 3.1: Setup & Configuration
- [ ] T001 Create Rust module structure per implementation plan
- [ ] T002 Add dependencies to Cargo.toml (pqcrypto-dilithium, rocksdb, tokio, tauri)
- [ ] T003 [P] Configure clippy.toml and rustfmt.toml per BTPC standards
- [ ] T004 [P] Configure Tauri permissions and capabilities (if desktop feature)

## Phase 3.2: Tests First (TDD) ⚠️ MUST COMPLETE BEFORE 3.3
**CRITICAL: These tests MUST be written and MUST FAIL before ANY implementation**

**Blockchain/Core Tests:**
- [ ] T005 [P] Contract test for ML-DSA signature validation in tests/contract/test_crypto_validation.rs
- [ ] T006 [P] Contract test for UTXO state consistency in tests/contract/test_utxo_consistency.rs
- [ ] T007 [P] Contract test for block validation in tests/contract/test_block_validation.rs
- [ ] T008 [P] Integration test for transaction flow in tests/integration/test_transaction_flow.rs

**Desktop App Tests (if applicable):**
- [ ] T009 [P] Contract test for Tauri command `create_wallet` in tests/contract/test_wallet_commands.rs
- [ ] T010 [P] Contract test for backend-first validation (Article XI, Section 11.2)
- [ ] T011 [P] Integration test for event synchronization (Article XI, Section 11.3)
- [ ] T012 [P] Memory leak test for event listener cleanup (Article XI, Section 11.6)

## Phase 3.3: Core Implementation (ONLY after tests are failing)

**Blockchain/Crypto Implementation:**
- [ ] T013 [P] Block model with ML-DSA validation in btpc-core/src/blockchain/block.rs
- [ ] T014 [P] Transaction model with UTXO logic in btpc-core/src/blockchain/transaction.rs
- [ ] T015 [P] UTXO manager in btpc-core/src/blockchain/utxo_manager.rs
- [ ] T016 [P] ML-DSA signature service in btpc-core/src/crypto/dilithium.rs
- [ ] T017 [P] SHA-512 PoW consensus in btpc-core/src/consensus/pow.rs
- [ ] T018 BlockchainService connecting components in btpc-core/src/services/blockchain_service.rs

**Desktop App Implementation (if applicable):**
- [ ] T019 [P] Tauri command for wallet creation in src-tauri/src/wallet_commands.rs
- [ ] T020 [P] Backend state management (Arc<RwLock>) in src-tauri/src/state.rs (Article XI, Section 11.1)
- [ ] T021 Event emission on state changes in src-tauri/src/main.rs (Article XI, Section 11.3)
- [ ] T022 React component with event listeners in ui/src/components/WalletManager.tsx
- [ ] T023 Event cleanup in beforeunload in ui/btpc-common.js (Article XI, Section 11.6)
- [ ] T024 Backend-first validation in settings page in ui/settings.html (Article XI, Section 11.2)

**RPC/API Implementation:**
- [ ] T025 JSON-RPC handler for getblock in btpc-core/src/rpc/handlers/blockchain.rs
- [ ] T026 JSON-RPC handler for sendrawtransaction in btpc-core/src/rpc/handlers/wallet.rs
- [ ] T027 Input validation and error handling in btpc-core/src/rpc/validation.rs

## Phase 3.4: Integration & Storage

**Database Integration:**
- [ ] T028 RocksDB column families setup in btpc-core/src/storage/db_init.rs
- [ ] T029 UTXO persistence to RocksDB in btpc-core/src/storage/utxo_storage.rs
- [ ] T030 Block persistence to RocksDB in btpc-core/src/storage/block_storage.rs

**Cryptography Integration:**
- [ ] T031 Integrate ML-DSA library with constant-time operations
- [ ] T032 AES-256-GCM wallet encryption with Argon2id key derivation
- [ ] T033 Secure key storage and zeroization on drop

**Network Integration (if applicable):**
- [ ] T034 P2P protocol message handlers in btpc-core/src/network/handlers.rs
- [ ] T035 Bitcoin-compatible block propagation
- [ ] T036 Peer connection management

## Phase 3.5: Polish & Documentation

**Code Quality:**
- [ ] T037 [P] Unit tests for edge cases in tests/unit/test_consensus_edge_cases.rs
- [ ] T038 [P] Performance test: Block validation < 100ms in tests/benchmarks/bench_validation.rs
- [ ] T039 [P] Performance test: ML-DSA signature < 10ms in tests/benchmarks/bench_crypto.rs
- [ ] T040 Run cargo clippy -- -D warnings and fix all issues
- [ ] T041 Remove code duplication (DRY principle)

**Article XI Compliance (Desktop Features):**
- [ ] T042 Verify backend-first validation in all settings (Article XI, Section 11.2)
- [ ] T043 Verify event listeners cleaned up on all pages (Article XI, Section 11.6)
- [ ] T044 Verify no duplicate toast notifications (Article XI, Section 11.6)
- [ ] T045 Cross-page state synchronization test (Article XI, Section 11.3)

**Documentation:**
- [ ] T046 [P] Update CLAUDE.md with new feature details
- [ ] T047 [P] Update STATUS.md with implementation status
- [ ] T048 [P] Add inline documentation (/// comments) to all public APIs
- [ ] T049 [P] Create user guide section in docs/ (if user-facing feature)

**Security & Validation:**
- [ ] T050 Run cargo audit and address vulnerabilities
- [ ] T051 Verify all crypto operations use constant-time implementations
- [ ] T052 Verify no private keys logged or exposed
- [ ] T053 Run full test suite (cargo test --workspace)

## Dependencies
**Test Dependencies:**
- Tests (T005-T012) MUST complete before implementation (T013-T027)

**Blockchain Dependencies:**
- T013 (Block model) blocks T015 (UTXO manager), T018 (BlockchainService)
- T014 (Transaction model) blocks T015 (UTXO manager), T018 (BlockchainService)
- T016 (ML-DSA signatures) blocks T013 (Block validation), T014 (Transaction signing)

**Desktop App Dependencies (Article XI):**
- T020 (Backend state) blocks T021 (Event emission), T024 (Backend-first validation)
- T021 (Event emission) blocks T022 (React event listeners)
- T022 (React listeners) blocks T023 (Event cleanup)

**Storage Dependencies:**
- T028 (RocksDB setup) blocks T029 (UTXO persistence), T030 (Block persistence)
- T015 (UTXO manager) blocks T029 (UTXO persistence)

**Implementation before polish:**
- All implementation (T013-T036) before polish (T037-T053)

## Parallel Execution Examples

**Test Phase (all parallel):**
```bash
# Launch T005-T008 together (different test files):
Task: "Contract test for ML-DSA signature validation in tests/contract/test_crypto_validation.rs"
Task: "Contract test for UTXO state consistency in tests/contract/test_utxo_consistency.rs"
Task: "Contract test for block validation in tests/contract/test_block_validation.rs"
Task: "Integration test for transaction flow in tests/integration/test_transaction_flow.rs"
```

**Core Implementation (parallel models):**
```bash
# Launch T013-T017 together (different modules):
Task: "Block model with ML-DSA validation in btpc-core/src/blockchain/block.rs"
Task: "Transaction model with UTXO logic in btpc-core/src/blockchain/transaction.rs"
Task: "UTXO manager in btpc-core/src/blockchain/utxo_manager.rs"
Task: "ML-DSA signature service in btpc-core/src/crypto/dilithium.rs"
Task: "SHA-512 PoW consensus in btpc-core/src/consensus/pow.rs"
```

**Desktop App (parallel components):**
```bash
# Launch T019, T022, T024 together (different files):
Task: "Tauri command for wallet creation in src-tauri/src/wallet_commands.rs"
Task: "React component with event listeners in ui/src/components/WalletManager.tsx"
Task: "Backend-first validation in settings page in ui/settings.html"
```

## Notes
- [P] tasks = different files, no dependencies
- Verify tests fail before implementing (TDD)
- Commit after each task with descriptive message
- Desktop features MUST follow Article XI patterns
- All crypto operations MUST be quantum-resistant (ML-DSA)
- Run cargo clippy frequently to catch issues early

## BTPC-Specific Task Generation Rules
*Applied during main() execution*

### 1. From Blockchain Spec
- Each block/transaction operation → validation test + implementation
- Each consensus rule → contract test + implementation
- Each crypto operation → constant-time verification test

### 2. From Desktop Spec (Article XI)
- Each Tauri command → contract test + implementation
- Each state change → event emission + listener task
- Each user action → backend-first validation task
- Each page navigation → event cleanup task

### 3. From Data Model
- Each blockchain entity (Block, UTXO, Transaction) → model + storage task [P]
- Each wallet entity → encryption + persistence task
- Relationships → service layer integration tasks

### 4. From User Stories
- Each blockchain flow → integration test [P]
- Each wallet operation → E2E test
- Quickstart scenarios → validation tasks

### 5. Ordering (BTPC-specific)
- Setup → Tests → Blockchain Models → Crypto Services → Desktop Commands → Storage → Network → Polish
- Dependencies block parallel execution
- Desktop features include Article XI compliance tasks

## Validation Checklist
*GATE: Checked by main() before returning*

**General:**
- [ ] All contract tests come before implementation
- [ ] Parallel tasks truly independent (different files)
- [ ] Each task specifies exact file path in BTPC structure
- [ ] No task modifies same file as another [P] task

**Blockchain-Specific:**
- [ ] All crypto operations have constant-time tests
- [ ] All blockchain entities have validation tests
- [ ] All consensus rules have contract tests
- [ ] Performance targets specified (< 100ms validation, < 10ms signatures)

**Desktop-Specific (if applicable):**
- [ ] Article XI compliance tasks included for each pattern
- [ ] Backend-first validation tasks present
- [ ] Event emission tasks present
- [ ] Event cleanup tasks present
- [ ] No localStorage-first patterns in tasks
- [ ] Cross-page state synchronization tested

**Constitutional Compliance:**
- [ ] Article XI applicability determined
- [ ] If applicable: All Article XI sections have corresponding tasks
- [ ] If applicable: Compliance verification tasks in Polish phase

---

## BTPC Project Context

**Tech Stack:**
- **Language**: Rust 1.75+ (stable)
- **Blockchain**: btpc-core library, RocksDB, SHA-512 PoW
- **Crypto**: pqcrypto-dilithium (ML-DSA), AES-256-GCM, Argon2id
- **Desktop**: Tauri 2.0, React, TypeScript
- **Network**: Tokio async runtime, Bitcoin-compatible P2P
- **Testing**: cargo test, criterion benchmarks, Miri for unsafe code

**Code Quality Standards:**
- Run `cargo clippy -- -D warnings` (zero warnings required)
- Run `cargo fmt` (enforce style)
- Use `#![deny(unsafe_code)]` unless cryptographically required
- Document all public APIs with `///` comments
- Use `anyhow::Result` for error handling

**Performance Targets:**
- Block validation: < 100ms
- ML-DSA signature verification: < 10ms
- Transaction validation: < 50ms
- Desktop app state updates: < 200ms (Article XI)
- RPC response time: < 100ms

**Constitutional Framework:**
- Constitution version: 1.0.1
- Article XI: Desktop Application Development Principles
- See `.specify/memory/constitution.md` for governance rules

**Key Documentation:**
- `CLAUDE.md` - Project guidelines and tech stack
- `STATUS.md` - Current implementation status
- `style-guide/ux-rules.md` - UI/UX patterns (Monero-inspired)
- `.specify/memory/constitution.md` - Constitutional governance
- `MANUAL_TESTING_GUIDE.md` - Testing procedures

---

**Template Version**: 1.1 (BTPC-specific)
**Last Updated**: 2025-10-11
**Maintained by**: .specify framework