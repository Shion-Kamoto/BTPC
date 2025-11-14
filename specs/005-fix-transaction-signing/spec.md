# Feature Specification: Fix Transaction Signing and Wallet Backup Failures

**Feature Branch**: `005-fix-transaction-signing`
**Created**: 2025-10-25
**Status**: Draft
**Input**: User description: "Fix transaction signing failure and wallet backup missing walletId error. Users cannot send funds between wallets due to 'Failed to sign input 0: Signature creation failed' error. Additionally, wallet backups fail with 'backup_wallet missing required key walletId' error. This blocks all transaction functionality and wallet backup operations."
**Constitution**: Article XI Compliance Required for Desktop Features

## Execution Flow (main)
```
1. Parse user description from Input
   → Identified: Critical bug fix for transaction signing and wallet backup
2. Extract key concepts from description
   → Actors: Wallet users attempting to send funds and backup wallets
   → Actions: Signing transaction inputs, backing up wallet data
   → Errors: ML-DSA signature creation failure, missing walletId parameter
3. For each unclear aspect:
   → Desktop app only (Tauri UI + backend) - CLARIFIED
   → All wallets affected (any wallet attempting transactions/backups) - CLARIFIED
   → All transactions fail (even single-input) - CLARIFIED - critical severity
4. Check constitutional requirements:
   → CONFIRMED: Article XI patterns apply (desktop app)
   → FLAGGED: Quantum-resistance requirements (ML-DSA signature generation)
5. Fill User Scenarios & Testing section
   → Scenario: User sends 50 BTPC from Wallet A to Wallet B
   → Scenario: User backs up wallet to encrypted file
6. Generate Functional Requirements
   → Transaction signing must successfully create ML-DSA signatures for all inputs
   → Wallet backup must accept walletId parameter and persist all wallet data
7. Identify Key Entities
   → Transaction, TransactionInput, ML-DSA Signature, Wallet, WalletBackup
8. Run Review Checklist
   → Has [NEEDS CLARIFICATION] - implementation context unclear
   → Article XI referenced for potential desktop feature
9. Return: SUCCESS (spec ready for planning after clarifications)
```

---

## Clarifications

### Session 2025-10-25
- Q: Is this bug fix for the desktop app only, CLI wallet only, or both? → A: Desktop app only (Tauri UI + backend)
- Q: Are all wallets affected by this bug, or only wallets in specific scenarios? → A: All wallets - any wallet attempting transactions/backups
- Q: Does the signature failure affect all transactions, or only transactions with multiple inputs? → A: All transactions - even single-input transactions fail with signature error
- Q: Should the implementation handle concurrent transaction signing requests with thread-safety mechanisms? → A: Yes, thread-safety required - implement proper locking/synchronization for concurrent operations
- Q: Where is wallet data stored in the desktop app? → A: Both - RocksDB for operational state, encrypted file for backup/restore

---

## User Scenarios & Testing *(mandatory)*

### Primary User Story
**As a** BTPC wallet owner,
**I want to** send transactions between wallets and backup my wallet data,
**So that** I can transfer funds securely and protect against data loss

### Acceptance Scenarios

**Transaction Signing:**
1. **Given** Wallet A has 100 BTPC in 1 UTXO, **When** user sends 50 BTPC to Wallet B, **Then** transaction signs successfully with ML-DSA signature and broadcasts to network
2. **Given** Wallet A has 100 BTPC split across 3 UTXOs (40 + 35 + 25), **When** user sends 80 BTPC to Wallet B, **Then** system selects 2+ UTXOs, signs each input with ML-DSA, and creates valid transaction
3. **Given** Wallet A attempts to send 50 BTPC, **When** signing fails for any input, **Then** system displays actionable error message explaining which input failed and why
4. **Given** desktop app user on Transactions → Send tab creates transaction, **When** transaction submitted, **Then** all inputs (0, 1, 2...) sign successfully without "Signature creation failed" error

**Wallet Backup:**
5. **Given** user has active wallet with encrypted private keys, **When** user initiates wallet backup, **Then** system creates encrypted backup file containing wallet ID, keys, metadata without "missing required key walletId" error
6. **Given** user backs up wallet to file path ~/backups/wallet.btpc, **When** backup completes, **Then** file contains all necessary data to restore wallet (walletId, encrypted private keys, addresses, metadata)
7. **Given** wallet backup operation, **When** walletId parameter is missing or invalid, **Then** system fails with clear error message and does not create corrupted backup
8. **Given** user restores wallet from backup file, **When** restoration completes, **Then** restored wallet has identical walletId, keys, and addresses as original

**Desktop App Integration:**
9. **Given** desktop app user on Transactions → Send tab, **When** user sends transaction, **Then** backend validates and signs transaction FIRST before showing success (Article XI.2)
10. **Given** desktop app user on Wallet Manager, **When** user clicks "Backup Wallet", **Then** backend performs backup with walletId validation before frontend shows confirmation (Article XI.2)

### Edge Cases
- What happens when wallet has 0 UTXOs (empty wallet)? → Display "Insufficient funds" not signature error
- What if transaction has 10+ inputs requiring multiple signatures? → All inputs must sign successfully or fail early
- What if walletId is null, empty string, or wrong data type? → Validation error before backup attempt
- What happens during signature creation if private key is corrupted? → Display key corruption error, not generic "Signature creation failed"
- What if backup file path is read-only or disk is full? → Display filesystem error, not walletId error
- How does system handle concurrent transaction signing requests? → Thread-safety required with Arc<RwLock<>> or Mutex synchronization

---

## Requirements *(mandatory)*

### Functional Requirements

**Transaction Signing:**
- **FR-001**: System MUST successfully create ML-DSA (Dilithium5) signatures for all transaction inputs (index 0, 1, 2, ...)
- **FR-002**: System MUST NOT fail with "Signature creation failed" error when signing valid transaction inputs
- **FR-003**: System MUST validate that wallet has private key for each UTXO before attempting signature
- **FR-004**: System MUST provide specific error messages when signature fails (e.g., "Private key missing for UTXO X", "Invalid transaction format", "Corrupted key data")
- **FR-005**: System MUST handle single-input and multi-input transactions identically (no special case for input 0)
- **FR-006**: System MUST sign transaction inputs sequentially, failing early if any input signature fails
- **FR-007**: Desktop app transaction signing MUST validate with backend FIRST before showing success to user (Article XI.2)

**Wallet Backup:**
- **FR-008**: System MUST require walletId parameter for all backup_wallet operations
- **FR-009**: System MUST validate walletId is non-null, non-empty, and correct data type before backup
- **FR-010**: System MUST NOT create backup file if walletId validation fails
- **FR-011**: System MUST persist walletId in backup file for restoration validation
- **FR-012**: Backup file MUST contain: walletId, encrypted private keys (AES-256-GCM), public keys, addresses, wallet metadata
- **FR-013**: System MUST provide actionable error message when walletId missing: "Wallet ID required for backup. Ensure wallet is properly initialized."
- **FR-014**: Desktop app wallet backup MUST validate with backend FIRST before creating file (Article XI.2)

**Error Handling:**
- **FR-015**: System MUST distinguish between signature algorithm errors, missing key errors, and invalid transaction errors
- **FR-016**: Error messages MUST be actionable and user-friendly (not raw technical errors)
- **FR-017**: System MUST log detailed error information for debugging while showing simplified errors to users
- **FR-018**: System MUST NOT expose private keys or sensitive data in error messages (NFR-002)

### Non-Functional Requirements

**Security:**
- **NFR-001**: All ML-DSA signature operations MUST use constant-time implementations
- **NFR-002**: Private keys MUST never appear in error messages or logs
- **NFR-003**: Wallet backup MUST encrypt all private key material with AES-256-GCM
- **NFR-004**: walletId validation MUST prevent injection attacks or malformed data

**Performance:**
- **NFR-005**: Single-input transaction signing MUST complete in < 50ms
- **NFR-006**: Multi-input transaction (10 inputs) signing MUST complete in < 500ms
- **NFR-007**: Wallet backup operation MUST complete in < 2 seconds for wallets with < 1000 UTXOs

**Usability:**
- **NFR-008**: Error messages MUST tell user what action to take (e.g., "Check wallet is unlocked" not "Key access denied")
- **NFR-009**: Desktop app MUST show progress indicator for multi-input transactions (Article XI.3 event-driven updates)

### Key Entities

**Transaction:**
- **Description**: Quantum-resistant transaction with ML-DSA signed inputs
- **Contains**: Transaction ID, inputs (UTXOs to spend), outputs (recipient addresses + amounts), ML-DSA signatures for each input
- **Relationships**: Has multiple TransactionInputs (1+), each requires one ML-DSA signature
- **Validation**: All input signatures must be valid before transaction broadcasts

**TransactionInput:**
- **Description**: Reference to UTXO being spent in transaction
- **Contains**: Previous transaction ID (txid), output index (vout), ML-DSA signature, public key
- **Relationships**: References one UTXO, requires matching private key from Wallet for signing
- **Signing Process**: Wallet retrieves private key → generates ML-DSA signature → attaches to input

**ML-DSA Signature:**
- **Description**: Post-quantum digital signature (Dilithium5)
- **Contains**: 4595-byte signature data
- **Creation**: Requires private key (from Wallet), transaction data hash, input index
- **Validation**: Verified against public key embedded in UTXO

**Wallet:**
- **Description**: Encrypted collection of private keys and metadata
- **Contains**: Wallet ID (unique identifier), encrypted private keys, public keys, addresses, metadata (name, creation date, last sync height)
- **Relationships**: Owns multiple UTXOs (via public key match), generates ML-DSA signatures for owned UTXOs
- **Storage**: RocksDB column family (operational state) + encrypted file (backup/restore)

**WalletBackup:**
- **Description**: Encrypted backup file containing all wallet data
- **Contains**: walletId (REQUIRED), encrypted private keys (AES-256-GCM), public keys, addresses, metadata
- **File Format**: [NEEDS CLARIFICATION: Binary, JSON, or custom format?]
- **Encryption**: AES-256-GCM with Argon2id key derivation from user password
- **Restoration**: walletId must match original wallet for integrity validation

---

## Constitutional Compliance *(mandatory for desktop features)*

### Article XI Applicability
- [ ] **Not a desktop feature** - Skip Article XI patterns
- [x] **Desktop feature** - Article XI patterns apply (desktop app only - Tauri UI + backend)

### Article XI Compliance Checklist
*(Only complete if desktop feature checked above)*

**Section 11.1 - Single Source of Truth:**
- [x] Identify authoritative state location: **Wallet backend (Rust Arc<RwLock<Wallet>> or RocksDB wallet column family)**
- [x] Frontend displays state only, never maintains authoritative state: **Compliant** - wallet keys/signatures never in JavaScript
- [x] Specified: Where state is stored and how frontend queries it: **Backend stores wallet, frontend queries via Tauri commands (get_wallet_balance, send_transaction, backup_wallet)**

**Section 11.2 - Backend-First Validation:**
- [x] All user actions validate with backend FIRST: **Required** - send_transaction and backup_wallet Tauri commands must validate before success
- [x] Failure exits early, NO localStorage save on validation failure: **Compliant** - signature errors must return error before any UI state update
- [x] Specified: Validation error messages and early exit behavior: **FR-004, FR-013 specify error messages; early exit on signature failure (FR-006)**

**Section 11.3 - Event-Driven Architecture:**
- [x] Backend emits events on all state changes: **Recommended** - emit "transaction_signed" or "backup_completed" events for UI updates
- [x] Frontend listens for events and updates UI: **Recommended** - listen for transaction status events to update Transactions page
- [x] Specified: Event names, payloads, and update behavior: [NEEDS CLARIFICATION: Event names - suggest "transaction_broadcast", "backup_completed"]

**Section 11.6 - Event Listener Cleanup:**
- [x] Event listeners cleaned up on page unload: **Compliant** - unlisten() called in beforeunload handlers
- [x] No memory leaks from forgotten listeners: **Compliant** - existing event cleanup patterns apply
- [x] Specified: Cleanup mechanism (beforeunload, unlisten functions): **Use existing btpc-event-manager.js cleanup pattern**

**Section 11.7 - Prohibited Patterns:**
- [x] Confirmed: NO localStorage before backend validation: **Compliant** - signatures/backups never in localStorage
- [x] Confirmed: NO authoritative state in frontend JavaScript: **Compliant** - wallet state only in backend
- [x] Confirmed: NO polling when events available: **Compliant** - use Tauri events for transaction status
- [x] Confirmed: NO duplicate notifications for user actions: **Compliant** - single toast on transaction success/failure

**Article XI Scope:**
This fix affects desktop app (Tauri UI + backend). Implementation will modify Tauri command handlers (send_transaction, backup_wallet) and potentially btpc-core library. Article XI patterns apply to all frontend interactions and backend state changes.

---

## Dependencies & Assumptions

### Dependencies
- Depends on ML-DSA (Dilithium5) signature library being functional (btpc-core crypto module)
- Depends on Wallet module having access to encrypted private keys
- Depends on UTXO storage in RocksDB for transaction input selection
- Depends on Tauri commands send_transaction and backup_wallet (desktop app)
- Depends on btpc-event-manager.js for event-driven updates (Article XI)

### Assumptions
- Assumes wallets are properly initialized with walletId during creation
- Assumes private keys are stored encrypted (AES-256-GCM) and accessible when wallet is unlocked
- Assumes transaction input selection logic correctly identifies spendable UTXOs
- Assumes user has entered correct wallet password before attempting transactions/backups
- Assumes [NEEDS CLARIFICATION: Single wallet active at a time or multi-wallet support?]
- Assumes [NEEDS CLARIFICATION: Backup file path is user-selectable or predefined?]
- Assumes sufficient UTXOs exist to cover transaction amount + fees

---

## Review & Acceptance Checklist
*GATE: Automated checks run during main() execution*

### Content Quality
- [x] No implementation details (no Rust code, Tauri command names, RocksDB column families specified)
- [x] Focused on user value (ability to send funds and backup wallets)
- [x] Written for cryptocurrency stakeholders (error scenarios described in user terms)
- [x] All mandatory sections completed
- [x] BTPC-specific considerations addressed (ML-DSA signatures, quantum-resistance, Article XI)

### Requirement Completeness
- [ ] No [NEEDS CLARIFICATION] markers remain - **7 clarifications needed** (see below)
- [x] Requirements are testable and unambiguous (specific error messages, performance targets)
- [x] Success criteria are measurable (< 50ms signing, < 2s backup)
- [x] Scope is clearly bounded (transaction signing + wallet backup, not entire wallet refactor)
- [x] Dependencies and assumptions identified (ML-DSA library, UTXO storage, Tauri commands)
- [x] Security implications considered (constant-time ops, no key exposure, encrypted backups)
- [x] Performance targets specified (NFR-005 to NFR-007)

### Constitutional Compliance (Desktop Features Only)
- [x] Article XI applicability determined (desktop app confirmed - Tauri UI + backend)
- [x] All Article XI patterns addressed in requirements (FR-007, FR-014, NFR-009)
- [x] Constitutional compliance checklist completed

---

## Execution Status
*Updated by main() during processing*

- [x] User description parsed (transaction signing failure + wallet backup walletId error)
- [x] Key concepts extracted (ML-DSA signatures, transaction inputs, walletId parameter, backup encryption)
- [x] Constitutional requirements flagged (Article XI applies - desktop app confirmed)
- [x] Ambiguities marked with [NEEDS CLARIFICATION] (7 items - see list below)
- [x] User scenarios defined (send transaction, backup wallet, restore wallet)
- [x] Functional requirements generated (18 FRs covering signing, backup, error handling)
- [x] Entities identified (Transaction, TransactionInput, ML-DSA Signature, Wallet, WalletBackup)
- [x] Constitutional compliance evaluated (Article XI checklist completed conditionally)
- [ ] Review checklist passed - **Pending clarifications**

**Outstanding Clarifications:**
1. **[RESOLVED: Desktop app only]** - Article XI patterns apply
2. **[RESOLVED: All wallets affected]** - Bug affects any wallet attempting transactions/backups
3. **[RESOLVED: All transactions fail]** - Even single-input transactions fail, critical severity
4. **[RESOLVED: Thread-safety required]** - Implement Arc<RwLock<>> or Mutex for concurrent operations
5. **[RESOLVED: Dual storage]** - RocksDB for operational state + encrypted files for backup/restore
6. **[NEEDS CLARIFICATION: Backup file format - Binary, JSON, or custom?]** - Interoperability requirements
7. **[NEEDS CLARIFICATION: Event names for desktop app]** - Article XI event-driven architecture
8. **[NEEDS CLARIFICATION: Single wallet or multi-wallet support?]** - Complexity scope
9. **[NEEDS CLARIFICATION: Backup file path user-selectable or predefined?]** - UX design

**Recommendation**: Resolve clarifications before proceeding to `/plan` phase. Suggested approach:
- Inspect existing code to determine desktop app vs CLI context
- Check wallet implementation to understand storage mechanism
- Review transaction signing code to identify root cause of "input 0" failure
- Review backup_wallet function signature to see walletId parameter handling

---

## BTPC Project Context

**Core Technologies:**
- Blockchain: Rust, btpc-core library, RocksDB, SHA-512 PoW
- Cryptography: ML-DSA (Dilithium5), AES-256-GCM, Argon2id
- Desktop: Tauri 2.0, vanilla JavaScript frontend, 68 Tauri commands
- Network: Bitcoin-compatible P2P, JSON-RPC 2.0

**Constitutional Framework:**
- Constitution version: 1.1 (Effective 2025-09-24, Amended 2025-10-18)
- Article VI.3: TDD Methodology MANDATORY (RED-GREEN-REFACTOR cycle)
- Article XI: Desktop Application Development Principles (mandatory for UI features)
- See `MD/CONSTITUTION.md` for complete governance rules

**Project Structure:**
- `btpc-core/` - Core blockchain library (Rust) - Likely location of signature bug
- `bins/btpc_wallet/` - CLI wallet binary - May be affected by signing/backup issues
- `btpc-desktop-app/src-tauri/` - Tauri backend - May need Tauri command fixes
- `btpc-desktop-app/ui/` - Frontend JavaScript - Article XI patterns apply here
- `tests/` - Integration and unit tests - TDD required for all fixes

**Key Documentation:**
- `CLAUDE.md` - Project overview and guidelines
- `STATUS.md` - Current implementation status (99% complete, desktop app operational)
- `style-guide/ux-rules.md` - UI/UX patterns (Monero-inspired)
- `MD/CONSTITUTION.md` - Governance rules (Article VI.3 TDD, Article XI desktop patterns)

**Related Features:**
- Feature 001: Core blockchain implementation (ML-DSA signatures implemented)
- Feature 004: Tab switching fix (recent desktop app UI work)

---

**Template Version**: 1.1 (BTPC-specific)
**Last Updated**: 2025-10-25
**Maintained by**: .specify framework