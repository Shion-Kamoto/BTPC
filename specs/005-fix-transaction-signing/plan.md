# Implementation Plan: Fix Transaction Signing and Wallet Backup Failures

**Branch**: `005-fix-transaction-signing` | **Date**: 2025-10-25 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/home/bob/BTPC/BTPC/specs/005-fix-transaction-signing/spec.md`

## Execution Flow (/plan command scope)
```
1. Load feature spec from Input path ✅
   → Loaded successfully from /home/bob/BTPC/BTPC/specs/005-fix-transaction-signing/spec.md
2. Fill Technical Context (scan for NEEDS CLARIFICATION) ✅
   → Detected Project Type: Desktop Wallet App (Tauri-based)
   → Set Structure Decision: Tauri backend + Web frontend (btpc-desktop-app/)
3. Fill the Constitution Check section ✅
   → Article VI.3 TDD methodology applies
   → Article XI Desktop patterns apply
   → Article VIII ML-DSA signatures required
4. Evaluate Constitution Check section ✅
   → No violations detected
   → Update Progress Tracking: Initial Constitution Check ✅
5. Execute Phase 0 → research.md ⏳
6. Execute Phase 1 → contracts, data-model.md, quickstart.md, CLAUDE.md ⏳
7. Re-evaluate Constitution Check section ⏳
8. Plan Phase 2 → Describe task generation approach ⏳
9. STOP - Ready for /tasks command ⏳
```

**IMPORTANT**: The /plan command STOPS at step 9. Phases 2-4 are executed by other commands:
- Phase 2: /tasks command creates tasks.md
- Phase 3-4: Implementation execution (manual or via tools)

## Summary
**Primary Requirement**: Fix critical bug preventing all transaction signing and wallet backups in BTPC desktop application

**Technical Approach**:
1. **Transaction Signing**: Fix ML-DSA (Dilithium5) signature creation for all transaction inputs (currently failing with "Failed to sign input 0: Signature creation failed")
2. **Wallet Backup**: Add missing walletId parameter to backup_wallet Tauri command (currently failing with "backup_wallet missing required key walletId")
3. **Thread Safety**: Implement Arc<RwLock<Wallet>> or Mutex synchronization for concurrent transaction/backup operations
4. **Storage Integration**: Fix dual storage model (RocksDB for operational state + encrypted files for backup/restore)
5. **Article XI Compliance**: Implement backend-first validation and event-driven architecture for desktop app

## Technical Context
**Language/Version**: Rust 1.75+ (required for all core blockchain components) + JavaScript ES6+ (frontend)

**Primary Dependencies**:
- tokio (async runtime)
- serde (serialization)
- RocksDB (operational wallet storage)
- pqcrypto-dilithium (ML-DSA signatures)
- AES-256-GCM (wallet encryption)
- Argon2id (key derivation)
- Tauri 2.0 (desktop framework)

**Storage**:
- RocksDB column family for operational wallet state
- Encrypted files (.btpc) for backup/restore operations
- localStorage for frontend UI state (non-authoritative)

**Testing**:
- cargo test (unit tests for signature functions, backup serialization)
- Integration tests (transaction signing, wallet backup/restore end-to-end)
- Tauri command tests (send_transaction, backup_wallet)
- Frontend event tests (Article XI compliance)

**Target Platform**: Linux/Windows/macOS desktop (Tauri app)

**Project Type**: Desktop Wallet App (Tauri backend + Web frontend)

**Performance Goals**:
- Single-input transaction signing: <50ms
- Multi-input (10 inputs) signing: <500ms
- Wallet backup operation: <2 seconds for <1000 UTXOs
- ML-DSA signature generation: <2ms per signature

**Constraints**:
- MUST use ML-DSA (Dilithium5) for all signatures (Article VIII)
- MUST implement TDD methodology (Article VI.3)
- MUST follow Article XI desktop patterns (backend-first validation, event-driven)
- MUST use constant-time operations for all cryptographic code
- MUST NOT expose private keys in errors or logs
- MUST maintain thread-safety for concurrent operations

**Scale/Scope**:
- Desktop app: 1 user, 1 active wallet at a time
- Wallet size: <1000 UTXOs typical, <10k maximum
- Transaction size: 1-20 inputs typical
- Concurrent operations: 2-5 simultaneous Tauri commands possible

## Constitution Check
*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Initial Check (Before Phase 0)

**Article VI.3 - TDD Methodology (MANDATORY)**:
- ✅ **Compliant**: Plan includes TDD workflow (RED-GREEN-REFACTOR)
- ✅ **Test Coverage**: >90% coverage target specified
- ✅ **Integration Tests**: Transaction signing and backup tests planned

**Article VIII - ML-DSA Signature Requirements**:
- ✅ **Compliant**: Fix targets ML-DSA signature generation bugs
- ✅ **Constant-Time**: NFR-001 specifies constant-time operations
- ✅ **No Degradation**: Fix improves signature reliability, no fallback to weak crypto

**Article XI - Desktop Application Patterns**:
- ✅ **Single Source of Truth**: Wallet state in backend (Rust), not frontend
- ✅ **Backend-First Validation**: FR-007, FR-014 specify backend validation before UI updates
- ✅ **Event-Driven Architecture**: NFR-009 requires event updates for transaction progress
- ✅ **No localStorage Before Backend**: Compliant - signatures/backups never in localStorage

**Security Gate**:
- ✅ ML-DSA (Dilithium5) signatures maintained
- ✅ AES-256-GCM encryption for backup files
- ✅ Argon2id key derivation specified
- ✅ No private key exposure in errors (NFR-002, FR-018)

**Performance Gate**:
- ✅ Single-input signing <50ms (NFR-005)
- ✅ Multi-input (10) signing <500ms (NFR-006)
- ✅ Backup operation <2s for <1000 UTXOs (NFR-007)

**Memory Safety Gate**:
- ✅ Rust implementation (memory-safe by default)
- ✅ Thread-safety required (Arc<RwLock<>> specified)
- ✅ No unsafe blocks planned (will use pqcrypto safe wrappers)

**Result**: ✅ PASS - No constitutional violations detected

## Project Structure

### Documentation (this feature)
```
specs/005-fix-transaction-signing/
├── spec.md              # Feature specification (INPUT)
├── plan.md              # This file (/plan command output)
├── research.md          # Phase 0 output (ML-DSA API, RocksDB wallet patterns)
├── data-model.md        # Phase 1 output (Wallet, Transaction entities)
├── quickstart.md        # Phase 1 output (Test transaction signing + backup)
├── contracts/           # Phase 1 output (Tauri command contracts)
│   ├── send_transaction.yaml
│   └── backup_wallet.yaml
└── tasks.md             # Phase 2 output (/tasks command - NOT created by /plan)
```

### Source Code (repository root)
```
btpc-desktop-app/
├── src-tauri/                    # Rust backend
│   ├── src/
│   │   ├── commands/             # Tauri command handlers
│   │   │   ├── wallet.rs         # FIX: send_transaction command
│   │   │   └── backup.rs         # FIX: backup_wallet command (add walletId)
│   │   ├── wallet/               # Wallet management
│   │   │   ├── manager.rs        # FIX: Arc<RwLock<Wallet>> thread-safety
│   │   │   ├── signing.rs        # FIX: ML-DSA signature creation
│   │   │   └── backup.rs         # FIX: walletId serialization
│   │   └── state/                # Application state
│   │       └── wallet_state.rs   # FIX: Shared wallet state with RwLock
│   └── tests/
│       ├── transaction_signing.rs  # NEW: Integration tests for signing
│       └── wallet_backup.rs        # NEW: Integration tests for backup
│
├── ui/                           # Web frontend
│   ├── transactions.html         # VERIFY: Article XI compliance
│   ├── wallet-manager.html       # VERIFY: Article XI compliance
│   ├── btpc-event-manager.js     # USE: Event listener management
│   └── btpc-backend-first.js     # USE: Backend-first validation pattern
│
btpc-core/                        # Core blockchain library (if fixes needed)
├── src/
│   ├── crypto/
│   │   └── signature.rs          # INVESTIGATE: ML-DSA signature API
│   └── wallet/
│       ├── mod.rs                # INVESTIGATE: Wallet struct definition
│       └── transaction.rs        # INVESTIGATE: Transaction signing logic
│
tests/
└── integration/
    └── wallet_transaction_signing_e2e.rs  # NEW: End-to-end test
```

**Structure Decision**: Desktop Wallet App (Tauri-based) with dual storage model:
- **Backend (Tauri/Rust)**: Manages wallet state in RocksDB + creates encrypted backup files
- **Frontend (JavaScript/HTML)**: Displays wallet state, sends Tauri commands for actions
- **Core Library (btpc-core)**: Provides ML-DSA signature functions and wallet structures
- **Integration**: Tauri commands bridge frontend → backend → btpc-core

This fix primarily targets `btpc-desktop-app/src-tauri/src/commands/` and `btpc-desktop-app/src-tauri/src/wallet/` with minimal changes to btpc-core if ML-DSA API is correct.

## Phase 0: Outline & Research

### Research Tasks

1. **ML-DSA Signature API Investigation**
   - **Unknown**: Current ML-DSA signature creation API in btpc-core/src/crypto/signature.rs
   - **Research Task**: "Investigate ML-DSA signature creation function signature and error handling in btpc-core"
   - **Questions**:
     - What is the function signature for creating ML-DSA signatures?
     - What error types are returned on signature failure?
     - Is the private key format compatible with wallet storage?
     - Are there any known bugs in the current implementation?
   - **Expected Output**: Document exact API, identify root cause of "Signature creation failed" error

2. **RocksDB Wallet Storage Patterns**
   - **Unknown**: How wallet data is stored/retrieved from RocksDB in current implementation
   - **Research Task**: "Review RocksDB wallet column family schema and access patterns"
   - **Questions**:
     - What column family stores wallet data?
     - How is walletId currently stored (or is it missing)?
     - How are private keys serialized/deserialized?
     - Is there existing thread-safe access to wallet storage?
   - **Expected Output**: Document current storage schema, identify walletId missing from serialization

3. **Tauri Command Thread-Safety Patterns**
   - **Unknown**: Current wallet state management in Tauri backend
   - **Research Task**: "Analyze current Tauri state management for wallet operations"
   - **Questions**:
     - Is wallet state currently stored in Tauri managed state?
     - Are there any Arc<Mutex<>> or Arc<RwLock<>> wrappers?
     - How are concurrent send_transaction commands handled?
     - What is the state initialization pattern?
   - **Expected Output**: Document current concurrency model, plan Arc<RwLock<Wallet>> integration

4. **Wallet Backup File Format**
   - **Unknown**: Current backup file serialization format and encryption
   - **Research Task**: "Examine existing wallet backup serialization and encryption implementation"
   - **Questions**:
     - What serde format is used (JSON, bincode, custom)?
     - Is AES-256-GCM encryption already implemented?
     - What fields are currently serialized in backup?
     - Where is the backup_wallet Tauri command implementation?
   - **Expected Output**: Document current backup format, identify walletId field absence

5. **Article XI Event-Driven Patterns**
   - **Unknown**: Existing Tauri event emission patterns in btpc-desktop-app
   - **Research Task**: "Review existing Tauri event patterns for state change notifications"
   - **Questions**:
     - Are there existing event patterns for node/miner state changes?
     - What is the event naming convention?
     - How is btpc-event-manager.js used in current pages?
     - What is the cleanup pattern for event listeners?
   - **Expected Output**: Document event patterns, define transaction_broadcast and backup_completed events

### Research Execution Plan
1. Read btpc-core/src/crypto/signature.rs to understand ML-DSA API
2. Search for RocksDB wallet storage implementation (likely btpc-core/src/wallet/ or btpc-desktop-app/src-tauri/src/wallet/)
3. Read btpc-desktop-app/src-tauri/src/commands/ to find send_transaction and backup_wallet implementations
4. Examine btpc-desktop-app/src-tauri/src/main.rs for Tauri state initialization
5. Search for existing event emission patterns (grep for "emit" in src-tauri/)
6. Review btpc-desktop-app/ui/btpc-event-manager.js for event listener patterns

**Output**: `research.md` with all findings consolidated and [NEEDS CLARIFICATION] items resolved

## Phase 1: Design & Contracts
*Prerequisites: research.md complete*

### 1. Data Model (`data-model.md`)

**Entities from Specification**:

**Wallet** (Primary Entity - needs fixes):
- **Fields**:
  - `wallet_id: String` (REQUIRED - currently missing from serialization)
  - `encrypted_private_keys: Vec<u8>` (AES-256-GCM encrypted)
  - `public_keys: Vec<PublicKey>` (ML-DSA public keys)
  - `addresses: Vec<String>` (BTPC addresses)
  - `metadata: WalletMetadata` (name, creation_date, last_sync_height)
- **Storage**:
  - RocksDB column family "wallets" (operational state)
  - Encrypted file `~/.btpc/wallets/{wallet_id}.btpc` (backup)
- **State Transitions**:
  - Created → Unlocked (password provided) → Locked (timeout/explicit)
  - Backup: Unlocked → BackupInProgress → Backup Created
- **Validation Rules**:
  - wallet_id MUST be non-empty UUID
  - private_keys MUST be encrypted when at rest
  - public_keys MUST match private_keys (verify on unlock)

**Transaction** (Entity requiring signing fix):
- **Fields**:
  - `txid: [u8; 64]` (SHA-512 hash)
  - `inputs: Vec<TransactionInput>` (UTXOs to spend)
  - `outputs: Vec<TransactionOutput>` (recipients)
  - `signatures: Vec<MlDsaSignature>` (one per input)
- **State Transitions**:
  - Unsigned → Signing Input 0 → ... → All Inputs Signed → Broadcast
  - Current bug: Stuck at "Signing Input 0" → Error
- **Validation Rules**:
  - inputs.len() == signatures.len() (all inputs must be signed)
  - Each signature MUST verify against corresponding input's public key
  - Total input value >= total output value + fee

**TransactionInput**:
- **Fields**:
  - `previous_txid: [u8; 64]`
  - `output_index: u32` (vout)
  - `signature: Option<MlDsaSignature>` (filled during signing)
  - `public_key: PublicKey` (from UTXO)
- **Signing Process**:
  - Wallet retrieves private_key for public_key
  - Generate ML-DSA signature over transaction data
  - Attach signature to input
- **Validation Rules**:
  - public_key MUST exist in wallet
  - signature MUST be valid ML-DSA signature

**MlDsaSignature** (Dilithium5):
- **Fields**:
  - `data: [u8; 4595]` (fixed-size Dilithium5 signature)
- **Creation**: Requires private key + message hash
- **Verification**: public_key.verify(message_hash, signature) → bool

**WalletBackup**:
- **Fields**:
  - `wallet_id: String` (REQUIRED - currently missing)
  - `encrypted_data: Vec<u8>` (AES-256-GCM encrypted wallet)
  - `encryption_params: EncryptionParams` (IV, salt for Argon2id)
  - `version: u32` (backup format version)
- **File Format**: Binary (serde bincode)
- **Encryption**: AES-256-GCM with Argon2id-derived key from user password
- **Restoration**: Decrypt → deserialize → verify wallet_id matches

### 2. API Contracts (`/contracts/`)

**Contract 1: `send_transaction.yaml`** (OpenAPI 3.0)
```yaml
/api/wallet/send_transaction:
  post:
    summary: Sign and broadcast transaction (FIX TARGET)
    requestBody:
      required: true
      content:
        application/json:
          schema:
            type: object
            properties:
              recipient_address:
                type: string
                example: "BTPC1qXXXXXXXXXXXXXX"
              amount:
                type: integer
                format: int64
                description: Amount in base units (satoshis)
              fee:
                type: integer
                format: int64
    responses:
      200:
        description: Transaction signed and broadcast successfully
        content:
          application/json:
            schema:
              type: object
              properties:
                txid:
                  type: string
                  format: hex
                  example: "abcd1234..."
                inputs_signed:
                  type: integer
                  description: Number of inputs successfully signed
      400:
        description: Validation error or signature failure
        content:
          application/json:
            schema:
              type: object
              properties:
                error:
                  type: string
                  examples:
                    - "Private key missing for UTXO abc123:0"
                    - "Failed to sign input 0: Invalid private key format"
                input_index:
                  type: integer
                  description: Index of failed input (if applicable)
```

**Contract 2: `backup_wallet.yaml`** (OpenAPI 3.0)
```yaml
/api/wallet/backup:
  post:
    summary: Create encrypted wallet backup (FIX TARGET)
    requestBody:
      required: true
      content:
        application/json:
          schema:
            type: object
            required:
              - wallet_id
              - backup_path
              - password
            properties:
              wallet_id:
                type: string
                format: uuid
                description: REQUIRED - Wallet identifier (FIX: currently missing)
              backup_path:
                type: string
                example: "/home/user/backups/wallet.btpc"
              password:
                type: string
                format: password
                description: Password for backup encryption
    responses:
      200:
        description: Backup created successfully
        content:
          application/json:
            schema:
              type: object
              properties:
                backup_path:
                  type: string
                wallet_id:
                  type: string
                backup_size_bytes:
                  type: integer
      400:
        description: Validation error
        content:
          application/json:
            schema:
              type: object
              properties:
                error:
                  type: string
                  examples:
                    - "Wallet ID required for backup. Ensure wallet is properly initialized."
                    - "Invalid backup path: directory does not exist"
```

### 3. Contract Tests (Failing Tests)

**Test File: `btpc-desktop-app/src-tauri/tests/contract_send_transaction.rs`**
```rust
// RED Phase: This test will FAIL until implementation is fixed
#[tokio::test]
async fn test_send_transaction_contract() {
    // Given: Wallet with 1 UTXO (100 BTPC)
    let wallet = setup_test_wallet_with_utxo(100_0000_0000).await;

    // When: Send 50 BTPC to recipient
    let response = send_transaction(
        "BTPC1qRecipient",
        50_0000_0000,
        1000
    ).await;

    // Then: Should return txid and inputs_signed = 1
    assert!(response.is_ok());
    assert_eq!(response.unwrap().inputs_signed, 1);
}
```

**Test File: `btpc-desktop-app/src-tauri/tests/contract_backup_wallet.rs`**
```rust
// RED Phase: This test will FAIL until walletId is added
#[tokio::test]
async fn test_backup_wallet_contract() {
    // Given: Initialized wallet with walletId
    let wallet_id = "550e8400-e29b-41d4-a716-446655440000";
    let wallet = setup_test_wallet(wallet_id).await;

    // When: Create backup
    let response = backup_wallet(
        wallet_id,
        "/tmp/test_backup.btpc",
        "test_password"
    ).await;

    // Then: Should succeed and return walletId
    assert!(response.is_ok());
    assert_eq!(response.unwrap().wallet_id, wallet_id);
}
```

### 4. Integration Test Scenarios (`quickstart.md`)

```markdown
# Quickstart: Transaction Signing & Wallet Backup Fix Verification

## Prerequisites
- BTPC desktop app running
- Test wallet created with 100 BTPC
- Second test wallet for recipient

## Test Scenario 1: Single-Input Transaction Signing
1. Open desktop app → Transactions → Send tab
2. Enter recipient address, amount 50 BTPC
3. Click "Send Transaction"
4. **Expected**: Transaction signs successfully, txid displayed
5. **Bug (before fix)**: "Failed to sign input 0: Signature creation failed"

## Test Scenario 2: Multi-Input Transaction Signing
1. Create wallet with 3 UTXOs (40 + 35 + 25 BTPC)
2. Send 80 BTPC (requires 2+ inputs)
3. **Expected**: All inputs sign successfully, transaction broadcasts
4. **Bug (before fix)**: Fails on first input

## Test Scenario 3: Wallet Backup with walletId
1. Open desktop app → Wallet Manager
2. Click "Backup Wallet"
3. Choose backup location
4. **Expected**: Backup file created with walletId in metadata
5. **Bug (before fix)**: "backup_wallet missing required key walletId"

## Test Scenario 4: Wallet Restoration Verification
1. Restore wallet from backup file created in Scenario 3
2. **Expected**: Restored wallet has same walletId, addresses, keys
3. Verify restoration integrity

## Test Scenario 5: Concurrent Transaction Signing (Thread-Safety)
1. Open 2 browser tabs with desktop app
2. Attempt to send transactions from both tabs simultaneously
3. **Expected**: Both transactions process correctly without race conditions
4. **Bug (before fix)**: Potential race condition or panic
```

### 5. Agent File Update

Execute: `.specify/scripts/bash/update-agent-context.sh claude`

**New Technologies to Add**:
- Arc<RwLock<Wallet>> for thread-safe wallet state
- Tauri command patterns for send_transaction, backup_wallet
- ML-DSA signature error handling
- Wallet backup serialization with walletId field
- Event-driven transaction status updates (Article XI)

**Preserve**: Manual additions in CLAUDE.md between `<!-- MANUAL ADDITIONS START -->` and `<!-- MANUAL ADDITIONS END -->`

**Output**: Updated CLAUDE.md at repository root with incremental context

## Phase 2: Task Planning Approach
*This section describes what the /tasks command will do - DO NOT execute during /plan*

**Task Generation Strategy**:

1. **Load Base Template**: `.specify/templates/tasks-template.md`

2. **Generate Contract Test Tasks** (RED Phase):
   - Task 1: [P] Write failing test for send_transaction contract (contract_send_transaction.rs)
   - Task 2: [P] Write failing test for backup_wallet contract (contract_backup_wallet.rs)
   - Task 3: [P] Write failing integration test for multi-input signing
   - Task 4: [P] Write failing integration test for wallet backup restoration

3. **Generate Model/Data Tasks** (RED → GREEN):
   - Task 5: [P] Add wallet_id field to Wallet struct serialization
   - Task 6: [P] Implement Arc<RwLock<Wallet>> wrapper for thread-safe access
   - Task 7: Fix ML-DSA signature creation error handling in signing.rs
   - Task 8: Add walletId parameter to backup_wallet Tauri command

4. **Generate Service/Logic Tasks** (GREEN Phase):
   - Task 9: Implement send_transaction command with proper error messages (depends on Task 7)
   - Task 10: Implement backup_wallet command with walletId serialization (depends on Task 5, 8)
   - Task 11: Add Arc<RwLock<>> synchronization to wallet state manager (depends on Task 6)
   - Task 12: Implement transaction_broadcast event emission (Article XI)
   - Task 13: Implement backup_completed event emission (Article XI)

5. **Generate Integration Tasks** (GREEN Phase):
   - Task 14: Make contract tests pass for send_transaction (depends on Task 9, 11)
   - Task 15: Make contract tests pass for backup_wallet (depends on Task 10, 11)
   - Task 16: Make multi-input signing integration test pass (depends on Task 9)
   - Task 17: Make wallet backup restoration test pass (depends on Task 10)

6. **Generate UI Verification Tasks** (Article XI Compliance):
   - Task 18: [P] Verify transactions.html follows Article XI backend-first pattern
   - Task 19: [P] Verify wallet-manager.html follows Article XI backend-first pattern
   - Task 20: [P] Add event listeners for transaction_broadcast in transactions.html
   - Task 21: [P] Add event listeners for backup_completed in wallet-manager.html

7. **Generate Quickstart Validation Task**:
   - Task 22: Execute quickstart.md test scenarios end-to-end

**Ordering Strategy**:
- **TDD Order**: Contract tests (Tasks 1-4) → Models (Tasks 5-8) → Services (Tasks 9-13) → Integration (Tasks 14-17) → UI (Tasks 18-21) → E2E (Task 22)
- **Dependency Order**: Tasks with [P] can run in parallel, others have explicit dependencies
- **Constitutional Order**: TDD cycle enforced (RED → GREEN → REFACTOR)

**Estimated Output**: 22 numbered, ordered tasks in tasks.md

**Parallel Execution Groups**:
- Group 1 [P]: Tasks 1, 2, 3, 4 (contract tests - independent)
- Group 2 [P]: Tasks 5, 6 (data model changes - independent files)
- Group 3 [P]: Tasks 18, 19 (UI verification - independent pages)
- Sequential: Tasks 7-17, 20-22 (have dependencies)

**IMPORTANT**: This phase is executed by the /tasks command, NOT by /plan

## Phase 3+: Future Implementation
*These phases are beyond the scope of the /plan command*

**Phase 3**: Task execution (/tasks command creates tasks.md)
**Phase 4**: Implementation (execute tasks.md following TDD RED-GREEN-REFACTOR)
**Phase 5**: Validation (run cargo test, execute quickstart.md, verify Article XI compliance)

## Complexity Tracking
*No constitutional violations detected - This section intentionally left empty*

## Progress Tracking
*This checklist is updated during execution flow*

**Phase Status**:
- [x] Phase 0: Research complete (/plan command) - ✅ research.md created
- [x] Phase 1: Design complete (/plan command) - ✅ Defined in plan.md
- [x] Phase 2: Task planning complete (/plan command - describe approach only) - ✅ DESCRIBED ABOVE
- [x] Phase 3: Tasks generated (/tasks command) - ✅ tasks.md created with 28 tasks
- [ ] Phase 4: Implementation complete
- [ ] Phase 5: Validation passed

**Gate Status**:
- [x] Initial Constitution Check: PASS ✅
- [ ] Post-Design Constitution Check: PASS (pending Phase 1)
- [ ] All NEEDS CLARIFICATION resolved (pending Phase 0)
- [x] Complexity deviations documented: N/A (no deviations)

---
*Based on Constitution v1.1 (2025-10-18) - See `/home/bob/BTPC/BTPC/.specify/memory/constitution.md`*
*Article VI.3 (TDD), Article VIII (ML-DSA), Article XI (Desktop Patterns) apply to this implementation*