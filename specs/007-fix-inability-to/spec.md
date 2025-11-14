# Feature Specification: Fix Transaction Sending Between Wallets

**Feature Branch**: `007-fix-inability-to`
**Created**: 2025-10-30
**Status**: Draft
**Input**: User description: "Fix inability to send transactions or transfer BTPC between internal wallets. Users cannot send funds between wallets due to transaction creation or signing failures. The send transaction functionality should work properly, allowing users to transfer BTPC between their own wallets and to external addresses. Need to identify and fix the root cause preventing successful transaction broadcasts."
**Constitution**: Article XI Compliance Required for Desktop Features

## Clarifications

### Session 2025-10-31
- Q: What specific error message(s) do users see when transaction sending fails? → A: "Failed to sign input 0: Signature creation failed"
- Q: Which network types are affected by this transaction sending failure? → A: Testnet and Regtest (Mainnet not yet tested)
- Q: Are transaction fees being calculated correctly before the signing failure occurs? → A: No, fee calculation also fails or produces wrong values
- Q: What should happen when a user tries to send a transaction using UTXOs that are already locked/reserved by another pending transaction? → A: Automatically select different UTXOs if available
- Q: How long should transaction attempt logs be retained for debugging purposes? → A: Until manually cleared (indefinite)

## Execution Flow (main)
```
1. Parse user description from Input
   → Feature: Fix transaction sending between wallets
2. Extract key concepts from description
   → Actors: Desktop app users with multiple wallets
   → Actions: Send transactions, transfer funds, broadcast transactions
   → Issues: Transaction creation failures, signing failures, broadcast failures
3. Identify unclear aspects:
   → Error messages shown? "Failed to sign input 0: Signature creation failed"
   → Network type affected? Testnet and Regtest (Mainnet not yet tested)
   → Fee calculation working? No, fee calculation also fails or produces wrong values
4. Check constitutional requirements:
   → FLAG "Article XI patterns apply" - Desktop app transaction feature
   → Backend-first validation required for transaction creation
   → Event-driven updates for transaction status
5. Fill User Scenarios & Testing section
   → Primary: User sends BTPC between own wallets
   → Secondary: User sends BTPC to external address
   → Tertiary: User handles transaction errors gracefully
6. Generate Functional Requirements
   → All requirements testable and unambiguous
   → Security requirements flagged (ML-DSA signing)
7. Identify Key Entities
   → Transaction: Inputs, outputs, signatures
   → UTXO: Available funds for spending
   → WalletState: Balance, UTXOs, keys
8. Run Review Checklist
   → Desktop feature with Article XI reference
   → All requirements testable
   → Security considerations addressed
9. Return: SUCCESS (spec ready for planning)
```

---

## ⚡ Quick Guidelines
- ✅ Focus on WHAT users need and WHY
- ❌ Avoid HOW to implement (no Rust code, Tauri commands, RocksDB schemas)
- 👥 Written for cryptocurrency users and stakeholders, not developers
- 🔒 Always consider quantum-resistance and security implications

### Section Requirements
- **Mandatory sections**: Must be completed for every feature
- **Optional sections**: Include only when relevant to the feature
- When a section doesn't apply, remove it entirely (don't leave as "N/A")

### For AI Generation
When creating this spec from a user prompt:
1. **Mark all ambiguities**: Use [NEEDS CLARIFICATION: specific question] for any assumption you'd need to make
2. **Don't guess**: If the prompt doesn't specify something (e.g., "wallet feature" without specifying CLI vs GUI), mark it
3. **Think like a tester**: Every vague requirement should fail the "testable and unambiguous" checklist item
4. **Common underspecified areas in BTPC**:
   - **Blockchain layer**: Mainnet vs Testnet vs Regtest behavior
   - **Crypto operations**: Which signature scheme (ML-DSA/Dilithium5)
   - **Desktop app state**: Backend-first validation requirements (Article XI)
   - **Wallet operations**: Encrypted persistence requirements
   - **Network behavior**: P2P protocol requirements
   - **Performance targets**: Block validation time, signature verification time
   - **Security/compliance**: Quantum-resistance level, key management
   - **Data retention**: UTXO cleanup, transaction history limits

### BTPC-Specific Considerations
- **Quantum Resistance**: All crypto features MUST use ML-DSA (Dilithium5) or approved post-quantum algorithms
- **Desktop App Features**: MUST reference Article XI constitutional patterns
- **Blockchain Features**: Specify network type (Mainnet/Testnet/Regtest)
- **Wallet Features**: Specify encryption requirements (AES-256-GCM, Argon2id)

---

## User Scenarios & Testing *(mandatory)*

### Primary User Story
**As a** BTPC wallet owner with multiple wallets,
**I want to** send transactions between my wallets and to external addresses,
**So that** I can manage my funds and make payments without transaction failures.

### Acceptance Scenarios

**Scenario 1: Send Between Internal Wallets**
1. **Given** user has Wallet A with 100 BTPC and Wallet B with 0 BTPC,
   **When** user sends 50 BTPC from Wallet A to Wallet B,
   **Then** transaction is created, signed with ML-DSA, broadcast successfully, and balances update correctly.

2. **Given** user initiates a transaction between wallets,
   **When** the transaction is being processed,
   **Then** user sees real-time status updates (creating, signing, broadcasting, confirmed).

**Scenario 2: Send to External Address**
1. **Given** user has a wallet with sufficient balance,
   **When** user enters a valid BTPC address and amount,
   **Then** transaction is created with correct outputs and broadcast to the network.

2. **Given** user attempts to send more than available balance,
   **When** transaction validation occurs,
   **Then** user receives clear error message about insufficient funds before signing attempt.

**Scenario 3: Handle Transaction Errors**
1. **Given** transaction creation fails for any reason,
   **When** error occurs during the process,
   **Then** user sees specific, actionable error message and wallet state remains unchanged.

2. **Given** network is unavailable for broadcast,
   **When** user attempts to send transaction,
   **Then** system queues transaction for later broadcast or allows manual retry.

### Edge Cases
- **UTXO Conflict Resolution**: When UTXOs are locked/reserved by another pending transaction, system automatically selects different available UTXOs
- **Exact Balance**: When balance exactly equals send amount, system must account for fees and either reduce send amount or reject transaction with clear message
- **Wallet Corruption**: If wallet file is corrupted during transaction signing, system must detect corruption and prevent partial transaction broadcast
- **Partial Failure Recovery**: System must rollback all transaction state changes on any failure during creation/signing pipeline
- **Wallet Switching**: Switching wallets during transaction creation must cancel pending transaction and release locked UTXOs

---

## Requirements *(mandatory)*

### Functional Requirements

**Transaction Creation & Validation:**
- **FR-001**: System MUST correctly select available UTXOs for spending, automatically choosing different UTXOs when preferred ones are locked
- **FR-002**: System MUST correctly calculate transaction fees based on size and network requirements (current implementation produces wrong values and must be fixed)
- **FR-003**: System MUST validate recipient addresses before transaction creation
- **FR-004**: System MUST prevent double-spending by locking UTXOs during transaction creation
- **FR-005**: System MUST create change outputs when input exceeds output + fees

**Transaction Signing:**
- **FR-006**: System MUST sign transactions with ML-DSA (Dilithium5) using correct private keys (current error: "Failed to sign input 0: Signature creation failed" must be resolved)
- **FR-007**: System MUST validate signatures before broadcast attempt
- **FR-008**: System MUST securely access wallet private keys without exposing them
- **FR-009**: System MUST handle password-protected wallets during signing

**Transaction Broadcasting:**
- **FR-010**: System MUST broadcast signed transactions to connected nodes
- **FR-011**: System MUST verify transaction acceptance by the network
- **FR-012**: System MUST handle broadcast failures with retry mechanism
- **FR-013**: System MUST update wallet balance after successful broadcast

**Desktop Application (Article XI):**
- **FR-014**: Desktop app MUST validate transaction parameters with backend before signing (Article XI, Section 11.2)
- **FR-015**: Desktop app MUST emit events for transaction status changes (Article XI, Section 11.3)
- **FR-016**: Desktop app MUST display real-time transaction status without polling (Article XI, Section 11.7)
- **FR-017**: System MUST maintain transaction state in backend, not frontend (Article XI, Section 11.1)

**Error Handling:**
- **FR-018**: System MUST provide specific error messages for each failure type
- **FR-019**: System MUST rollback partial transactions on failure
- **FR-020**: System MUST release locked UTXOs on transaction failure
- **FR-021**: System MUST log transaction attempts for debugging (retained until manually cleared)

### Non-Functional Requirements

**Security:**
- **NFR-001**: Private keys MUST remain encrypted except during signing operation
- **NFR-002**: System MUST use constant-time ML-DSA signing to prevent timing attacks
- **NFR-003**: Transaction data MUST be validated to prevent injection attacks

**Performance:**
- **NFR-004**: Transaction creation MUST complete in < 500ms for typical transaction
- **NFR-005**: ML-DSA signing MUST complete in < 100ms
- **NFR-006**: UI MUST remain responsive during transaction processing

**Usability:**
- **NFR-007**: Error messages MUST explain what went wrong and how to fix it
- **NFR-008**: Transaction status MUST be clearly visible during all stages
- **NFR-009**: System MUST prevent accidental double-sends through UI locks

### Key Entities

**Transaction:**
- Contains: Transaction ID, inputs (UTXOs being spent), outputs (recipients), ML-DSA signatures, fees
- States: Creating, Signing, Broadcasting, Pending, Confirmed, Failed
- Relationships: Consumes UTXOs, creates new UTXOs, belongs to Wallet

**UTXO (Unspent Transaction Output):**
- Contains: Transaction ID + output index, amount, owner public key, lock status
- States: Available, Locked (reserved), Spent
- Relationships: Owned by Wallet, consumed by Transaction

**WalletState:**
- Contains: Balance, available UTXOs, pending transactions, sync height
- Backend state (Arc<RwLock>) is authoritative (Article XI)
- Relationships: Owns UTXOs, creates Transactions

**TransactionBuilder:**
- Contains: Selected inputs, outputs, change address, fee calculation
- Temporary entity during transaction creation
- Relationships: Reads from WalletState, produces Transaction

---

## Constitutional Compliance *(mandatory for desktop features)*

### Article XI Applicability
- [ ] **Not a desktop feature** - Skip Article XI patterns
- [x] **Desktop feature** - Article XI patterns apply (complete checklist below)

### Article XI Compliance Checklist

**Section 11.1 - Single Source of Truth:**
- [x] Identify authoritative state location: Backend WalletManager (Arc<RwLock>)
- [x] Frontend displays transaction state only, never maintains authoritative state
- [x] Specified: WalletState in backend, frontend queries via Tauri commands

**Section 11.2 - Backend-First Validation:**
- [x] All transaction parameters validate with backend FIRST
- [x] Failure exits early, NO localStorage save on validation failure
- [x] Specified: Validation includes address format, balance, UTXO availability

**Section 11.3 - Event-Driven Architecture:**
- [x] Backend emits events: transaction:created, transaction:signed, transaction:broadcast, transaction:confirmed, transaction:failed
- [x] Frontend listens for events and updates UI
- [x] Specified: Event payloads include transaction ID, status, error details

**Section 11.6 - Event Listener Cleanup:**
- [x] Event listeners cleaned up on page unload
- [x] No memory leaks from transaction status listeners
- [x] Specified: Cleanup via unlisten() in beforeunload handler

**Section 11.7 - Prohibited Patterns:**
- [x] Confirmed: NO localStorage for transaction state
- [x] Confirmed: NO authoritative balance in frontend
- [x] Confirmed: NO polling for transaction status (use events)
- [x] Confirmed: NO duplicate notifications for transaction events

---

## Dependencies & Assumptions

### Scope Clarification
- **Networks Affected**: This fix addresses transaction failures observed on **Testnet** and **Regtest** networks
- **Mainnet Status**: Mainnet has not yet been tested; transaction sending functionality on Mainnet is unknown
- **Primary Focus**: Fix must work on Testnet and Regtest; Mainnet compatibility to be verified separately

### Dependencies
- ML-DSA signature module must be functional for transaction signing
- RocksDB UTXO storage must accurately track spendable outputs
- RPC client must be connected to broadcast transactions
- Wallet encryption/decryption must work for accessing private keys
- Previous fix (Feature 005) for transaction signing must be complete
- Tauri events system must be operational (Article XI)

### Assumptions
- User has network connectivity for transaction broadcast
- Wallet contains valid ML-DSA keypairs
- Node is synchronized with blockchain for UTXO validation
- Desktop app has read/write access to wallet files
- Transaction fees are calculated based on network rules
- User knows wallet password for protected wallets

---

## Review & Acceptance Checklist
*GATE: Automated checks run during main() execution*

### Content Quality
- [x] No implementation details (no Rust code, Tauri command names, RocksDB column families)
- [x] Focused on user value and cryptocurrency operations
- [x] Written for non-technical cryptocurrency stakeholders
- [x] All mandatory sections completed
- [x] BTPC-specific considerations addressed (quantum-resistance, ML-DSA)

### Requirement Completeness
- [x] No [NEEDS CLARIFICATION] markers remain (all resolved via 2025-10-31 clarification session)
- [x] Requirements are testable and unambiguous
- [x] Success criteria are measurable
- [x] Scope is clearly bounded (transaction sending functionality)
- [x] Dependencies and assumptions identified
- [x] Security implications considered (key exposure, signing)
- [x] Performance targets specified

### Constitutional Compliance (Desktop Features Only)
- [x] Article XI applicability determined (desktop feature)
- [x] All Article XI patterns addressed in requirements
- [x] Constitutional compliance checklist completed
- [x] Backend-first validation specified

---

## Execution Status

- [x] User description parsed
- [x] Key concepts extracted (transactions, wallets, signing, broadcasting)
- [x] Constitutional requirements flagged (Article XI)
- [x] Ambiguities marked with [NEEDS CLARIFICATION] (5 items identified)
- [x] Clarifications completed (2025-10-31 session: 5 questions answered)
- [x] User scenarios defined (internal transfer, external send, error handling)
- [x] Functional requirements generated (21 requirements)
- [x] Entities identified (Transaction, UTXO, WalletState)
- [x] Constitutional compliance evaluated (Article XI)
- [x] Review checklist fully passed

---

## BTPC Project Context

**Core Technologies:**
- Blockchain: Rust, btpc-core library, RocksDB, SHA-512 PoW
- Cryptography: ML-DSA (Dilithium5), AES-256-GCM, Argon2id
- Desktop: Tauri 2.0, React frontend, 68 Tauri commands
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
**Last Updated**: 2025-10-11
**Maintained by**: .specify framework