# Tasks: Fix Transaction Sending Between Wallets

**Input**: Design documents from `/home/bob/BTPC/BTPC/specs/007-fix-inability-to/`
**Prerequisites**: plan.md ✅, data-model.md ✅, contracts/ ✅, research.md ✅, quickstart.md ✅
**Constitution**: Article XI patterns for desktop features

## Summary

This is a **BUG FIX** for the existing BTPC desktop application. The transaction sending feature is broken due to three primary issues: (1) UTXO selection not tracking locked/reserved outputs, (2) ML-DSA signature generation failing due to missing seed storage, and (3) Fee calculation using hardcoded values. This tasks.md focuses on fixing these issues with minimal changes to the existing codebase.

## Format: `[ID] [P?] Description`
- **[P]**: Can run in parallel (different files, no dependencies)
- Include exact file paths in BTPC structure
- Reference Article XI sections for desktop tasks

## Phase 3.1: Setup & Configuration

- [x] **T001** Add missing dependencies to Cargo.toml
  - **File**: `btpc-desktop-app/src-tauri/Cargo.toml`
  - **Action**: Ensure `btpc-core` has seed storage support (already added in Feature 005)
  - **Verify**: `btpc-core = { version = "0.1.0", features = ["seed-storage"] }`
  - **Status**: Verify only (dependency already exists from Feature 005)

- [x] **T002** [P] Configure clippy for stricter transaction checks
  - **File**: `btpc-desktop-app/src-tauri/clippy.toml`
  - **Action**: Add `pedantic` lint level for transaction-related modules
  - **Add**: `warn = ["clippy::all", "clippy::pedantic"]`

## Phase 3.2: Tests First (TDD) ⚠️ MUST COMPLETE BEFORE 3.3
**CRITICAL: These tests MUST be written and MUST FAIL before ANY implementation**

### Contract Tests for Tauri Commands

- [x] **T003** [P] Contract test for `create_transaction` command
  - **File**: `btpc-desktop-app/src-tauri/tests/contract/test_create_transaction.rs`
  - **Test**: Request schema validation per `contracts/transaction-api.yaml` /create_transaction
  - **Assertions**:
    - Valid request with wallet_id, recipient, amount → 200 TransactionCreated
    - Invalid address → 400 INVALID_ADDRESS
    - Insufficient funds → 402 INSUFFICIENT_FUNDS
    - Locked UTXOs → 423 UTXO_LOCKED
  - **Expected**: MUST FAIL (no UTXO reservation logic yet)

- [x] **T004** [P] Contract test for `sign_transaction` command
  - **File**: `btpc-desktop-app/src-tauri/tests/contract/test_sign_transaction.rs`
  - **Test**: Signature generation with seed storage per `contracts/transaction-api.yaml` /sign_transaction
  - **Assertions**:
    - Valid transaction_id + password → 200 TransactionSigned
    - Invalid password → 401 INVALID_PASSWORD
    - Missing seed → 500 SIGNATURE_FAILED with missing_seed=true
    - Transaction not found → 404 TRANSACTION_NOT_FOUND
  - **Expected**: MUST FAIL (seed-based signing not implemented)

- [x] **T005** [P] Contract test for `broadcast_transaction` command with retry strategy
  - **File**: `btpc-desktop-app/src-tauri/tests/contract/test_broadcast_transaction.rs`
  - **Test**: Network broadcast per `contracts/transaction-api.yaml` /broadcast_transaction
  - **Assertions**:
    - Signed transaction → 200 TransactionBroadcast
    - Unsigned transaction → 400 INVALID_TRANSACTION
    - Node unavailable → 503 NETWORK_UNAVAILABLE with retry_after header
    - **Retry strategy validation**:
      * Automatic retry with exponential backoff (1s, 2s, 4s)
      * Maximum 3 retry attempts
      * Emit `transaction:retry` event with attempt count
      * Stop retrying on non-recoverable errors (400 series)
      * Final failure emits `transaction:failed` with all_retries_exhausted=true
  - **Expected**: MUST FAIL (broadcast logic incomplete, retry not implemented)

- [x] **T006** [P] Contract test for `estimate_fee` command
  - **File**: `btpc-desktop-app/src-tauri/tests/contract/test_estimate_fee.rs`
  - **Test**: Dynamic fee calculation per `contracts/transaction-api.yaml` /estimate_fee
  - **Assertions**:
    - Valid parameters → fee = base_fee + (size * fee_rate)
    - Fee varies with transaction size
    - RPC failure → fallback to conservative estimate
  - **Expected**: MUST FAIL (dynamic fee calculation missing)

- [x] **T007** [P] Contract test for `cancel_transaction` command
  - **File**: `btpc-desktop-app/src-tauri/tests/contract/test_cancel_transaction.rs`
  - **Test**: UTXO release on cancellation per `contracts/transaction-api.yaml` /cancel_transaction
  - **Assertions**:
    - Pending transaction → 200 with utxos_released count
    - Broadcast transaction → 400 (cannot cancel)
    - Transaction not found → 404
  - **Expected**: MUST FAIL (cancellation logic missing)

### Event Contract Tests (Article XI)

- [x] **T008** [P] Contract test for transaction event sequence
  - **File**: `btpc-desktop-app/src-tauri/tests/contract/test_transaction_events.rs`
  - **Test**: Event flow per `contracts/events.json` successful_transaction
  - **Assertions**:
    - Events emitted in correct order (initiated → validated → signed → broadcast → confirmed)
    - Each event has correct payload structure
    - Events originate from backend only (Article XI, Section 11.1)
    - No duplicate events (Article XI, Section 11.6)
  - **Expected**: MUST FAIL (event emission not implemented)

- [x] **T009** [P] Contract test for transaction failure events
  - **File**: `btpc-desktop-app/src-tauri/tests/contract/test_transaction_error_events.rs`
  - **Test**: Error events per `contracts/events.json` failed_transaction_*
  - **Assertions**:
    - `transaction:failed` emitted on any stage failure
    - Error payload includes stage, error_type, error_message, recoverable, suggested_action
    - UTXOs released on failure
  - **Expected**: MUST FAIL (error event emission missing)

### Integration Tests

- [x] **T010** [P] Integration test for full transaction flow
  - **File**: `btpc-desktop-app/src-tauri/tests/integration/test_transaction_flow_integration.rs`
  - **Test**: End-to-end transaction per `quickstart.md` Scenario 1
  - **Flow**:
    1. Create transaction with 2 wallets
    2. Sign with ML-DSA (seed-based)
    3. Broadcast to regtest node
    4. Verify balance updates
    5. Verify events emitted
  - **Expected**: MUST FAIL (missing UTXO reservation, seed-based signing)

- [x] **T011** [P] Integration test for concurrent transactions
  - **File**: `btpc-desktop-app/src-tauri/tests/integration/test_concurrent_transactions.rs`
  - **Test**: UTXO locking prevents double-spending per `quickstart.md` Scenario 4
  - **Flow**:
    1. Start two transactions simultaneously
    2. First locks UTXOs
    3. Second gets UTXO_LOCKED error
    4. First completes
    5. Second succeeds on retry
  - **Expected**: MUST FAIL (UTXO reservation not implemented)

- [x] **T012** [P] Integration test for error handling and UTXO release
  - **File**: `btpc-desktop-app/src-tauri/tests/integration/test_transaction_errors.rs`
  - **Test**: Error scenarios per `quickstart.md` Scenario 3
  - **Tests**:
    - Insufficient funds → clear error message
    - Invalid address → format validation error
    - Network disconnection → retry suggestion
    - **UTXO release verification**: After any transaction failure, verify:
      * UTXOs are unlocked and available
      * Can create new transaction with same UTXOs
      * Reservation count decrements correctly
  - **Expected**: MUST FAIL (specific error messages missing, UTXO release not implemented)

## Phase 3.3: Core Implementation (ONLY after tests are failing)

### UTXO Reservation System

- [x] **T013** [P] Implement ReservationToken in wallet_manager
  - **File**: `btpc-desktop-app/src-tauri/src/wallet_manager.rs`
  - **Implementation**: Add `ReservationToken` struct per `data-model.md`
  - **Fields**: `id: Uuid`, `transaction_id: Option<String>`, `utxos: Vec<(String, u32)>`, `created_at`, `expires_at`
  - **Methods**: `new()`, `extend_expiry()`, `is_expired()`, `release()`
  - **Storage**: `Arc<Mutex<HashMap<Uuid, ReservationToken>>>`
  - **Depends on**: None (new struct)
  - **Makes pass**: T003, T007, T010, T011

- [x] **T014** Implement UTXO lock/unlock methods
  - **File**: `btpc-desktop-app/src-tauri/src/wallet_manager.rs`
  - **Implementation**: Add methods to `WalletManager`
  - **Methods**:
    - `reserve_utxos(&mut self, utxos: Vec<(String, u32)>, tx_id: Option<String>) -> Result<ReservationToken>`
    - `release_reservation(&mut self, token: &ReservationToken) -> Result<()>`
    - `cleanup_expired_reservations(&mut self) -> Result<usize>` (called periodically)
  - **Depends on**: T013
  - **Makes pass**: T003, T007, T010, T011

- [x] **T014.1** Implement periodic cleanup of expired UTXO reservations
  - **File**: `btpc-desktop-app/src-tauri/src/wallet_manager.rs`
  - **Implementation**: Add background task for reservation cleanup
  - **Method**:
    ```rust
    pub fn start_cleanup_task(&self) -> tokio::task::JoinHandle<()> {
        let manager = Arc::clone(&self);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                if let Ok(count) = manager.cleanup_expired_reservations().await {
                    if count > 0 {
                        tracing::info!("Cleaned up {} expired UTXO reservations", count);
                    }
                }
            }
        })
    }
    ```
  - **Initialize**: Call in `WalletManager::new()`
  - **Depends on**: T014
  - **Makes pass**: Prevents memory leaks from abandoned reservations

### ML-DSA Signature with Seed Storage

- [x] **T015** [P] Update wallet creation to store seeds
  - **File**: `btpc-desktop-app/src-tauri/src/wallet_commands.rs`
  - **Implementation**: Modify `create_wallet` command
  - **Changes**:
    - Use `PrivateKey::from_seed(seed)` instead of direct key generation
    - Store seed in `KeyEntry::seed` field (encrypted)
    - Ensure backward compatibility (optional seed field)
  - **Reference**: Feature 005 implementation in `btpc-core/src/crypto/keys.rs`
  - **Depends on**: None (uses existing btpc-core feature)
  - **Makes pass**: T004, T010

- [x] **T015.1** Add wallet file integrity checks before signing
  - **File**: `btpc-desktop-app/src-tauri/src/wallet_commands.rs`
  - **Implementation**: Validate wallet file integrity before key access
  - **Checks**:
    - File size matches expected format (not truncated)
    - Checksum verification (SHA-256 of encrypted content)
    - JSON structure validation after decryption
    - Required fields present (wallet_id, keys, version)
  - **Error Handling**:
    - Return `WalletError::Corrupted` with specific failure reason
    - Suggest recovery action (restore from backup, re-create wallet)
    - Log corruption details for debugging
  - **Code pattern**:
    ```rust
    fn validate_wallet_integrity(wallet_path: &Path) -> Result<(), WalletError> {
        let file_data = fs::read(wallet_path)?;

        // Check file size
        if file_data.len() < MIN_WALLET_SIZE {
            return Err(WalletError::Corrupted("File truncated".into()));
        }

        // Verify checksum (last 32 bytes)
        let (content, checksum) = file_data.split_at(file_data.len() - 32);
        let computed = sha2::Sha256::digest(content);
        if computed.as_slice() != checksum {
            return Err(WalletError::Corrupted("Checksum mismatch".into()));
        }

        Ok(())
    }
    ```
  - **Depends on**: T015
  - **Makes pass**: Edge case handling for wallet corruption (spec L127)

- [x] **T016** Fix signature generation to use seeds
  - **File**: `btpc-desktop-app/src-tauri/src/wallet_commands.rs`
  - **Implementation**: Modify `sign_transaction` command
  - **Changes**:
    - Check if `key_entry.seed.is_some()`
    - Use `PrivateKey::from_key_pair_bytes_with_seed()` for reconstruction
    - Call `sign_with_seed_regeneration()` helper
    - Return `SIGNATURE_FAILED` with `missing_seed=true` if seed absent
  - **Depends on**: T015, T015.1
  - **Makes pass**: T004, T010

### Dynamic Fee Calculation

- [x] **T017** [P] Implement fee estimation service
  - **File**: `btpc-desktop-app/src-tauri/src/fee_estimator.rs` (NEW)
  - **Implementation**: Create `FeeEstimator` struct
  - **Methods**:
    - `estimate_transaction_size(inputs: usize, outputs: usize) -> usize` (bytes)
    - `get_current_fee_rate() -> Result<u64>` (query RPC)
    - `calculate_fee(size: usize, rate: u64) -> u64` (satoshis)
    - `estimate_fee_for_transaction(wallet_id, recipient, amount) -> Result<FeeEstimate>`
  - **Fallback**: Conservative estimate if RPC unavailable
  - **Depends on**: None (new module)
  - **Makes pass**: T006

- [x] **T018** Integrate fee estimation in transaction creation
  - **File**: `btpc-desktop-app/src-tauri/src/wallet_commands.rs`
  - **Implementation**: Modify `create_transaction` command
  - **Changes**:
    - Call `fee_estimator.estimate_fee_for_transaction()` before UTXO selection
    - Select UTXOs to cover `amount + estimated_fee`
    - Recalculate actual fee based on selected UTXOs
    - Add change output if needed
  - **Depends on**: T017
  - **Makes pass**: T003, T006, T010

### Event System Integration (Article XI)

- [x] **T019** [P] Implement transaction event emitter
  - **File**: `btpc-desktop-app/src-tauri/src/events.rs` (NEW)
  - **Implementation**: Create `TransactionEventEmitter` struct
  - **Methods** (per `contracts/events.json`):
    - `emit_initiated(app_handle, wallet_id, recipient, amount)`
    - `emit_validated(app_handle, tx_id, inputs_count, outputs_count, fee)`
    - `emit_signing_started(app_handle, tx_id, inputs_to_sign)`
    - `emit_input_signed(app_handle, tx_id, input_index)`
    - `emit_signed(app_handle, tx_id, signatures_count)`
    - `emit_broadcast(app_handle, tx_id, broadcast_to_peers)`
    - `emit_confirmed(app_handle, tx_id, block_height, confirmations)`
    - `emit_failed(app_handle, tx_id, stage, error_type, error_message, recoverable, suggested_action)`
  - **Article XI Compliance**: Backend-only emission (Section 11.1)
  - **Depends on**: None (new module)
  - **Makes pass**: T008, T009, T010

- [x] **T020** Emit events in create_transaction command
  - **File**: `btpc-desktop-app/src-tauri/src/wallet_commands.rs`
  - **Implementation**: Add event calls in `create_transaction`
  - **Events**:
    - Start: `transaction:initiated`
    - After validation: `transaction:validated`
    - On UTXO lock: `utxo:reserved`
    - On error: `transaction:failed`
  - **Depends on**: T019
  - **Makes pass**: T008, T009

- [x] **T021** Emit events in sign_transaction command
  - **File**: `btpc-desktop-app/src-tauri/src/wallet_commands.rs`
  - **Implementation**: Add event calls in `sign_transaction`
  - **Events**:
    - Start: `transaction:signing_started`
    - Per input: `transaction:input_signed`
    - Complete: `transaction:signed`
    - On error: `transaction:failed`
  - **Depends on**: T019
  - **Makes pass**: T008, T009

- [x] **T022** Emit events in broadcast_transaction command
  - **File**: `btpc-desktop-app/src-tauri/src/wallet_commands.rs`
  - **Implementation**: Add event calls in `broadcast_transaction`
  - **Events**:
    - Start: `transaction:broadcast`
    - After mempool: `transaction:mempool_accepted`
    - On confirmation: `transaction:confirmed`
    - On error: `transaction:failed`
  - **Depends on**: T019
  - **Makes pass**: T008, T009

- [x] **T022.1** Implement broadcast retry mechanism
  - **File**: `btpc-desktop-app/src-tauri/src/wallet_commands.rs`
  - **Implementation**: Add retry logic to `broadcast_transaction`
  - **Strategy**:
    - Retry on network errors (503, timeout, connection refused)
    - Don't retry on client errors (400, 401, 404)
    - Exponential backoff: 1s, 2s, 4s (max 3 attempts)
    - Emit `transaction:retry` event before each retry
    - Track attempt count in transaction metadata
  - **Code pattern**:
    ```rust
    for attempt in 1..=3 {
        match btpc_client.broadcast(&signed_tx).await {
            Ok(response) => return Ok(response),
            Err(e) if e.is_recoverable() => {
                emit_retry_event(app_handle, tx_id, attempt);
                tokio::time::sleep(Duration::from_secs(2_u64.pow(attempt - 1))).await;
            }
            Err(e) => return Err(e), // Non-recoverable
        }
    }
    ```
  - **Depends on**: T019, T022
  - **Makes pass**: T005 (retry assertions)

### Error Handling Improvements

- [x] **T023** [P] Implement specific error types
  - **File**: `btpc-desktop-app/src-tauri/src/error.rs`
  - **Implementation**: Add error variants per `data-model.md` Error Taxonomy
  - **Errors**:
    - `ValidationError(InvalidAddress | InvalidAmount | InsufficientFunds | UtxoLocked)`
    - `SigningError(KeyNotFound | SeedMissing | SignatureFailed | WalletLocked | WalletCorrupted)`
    - `NetworkError(NodeUnavailable | BroadcastFailed | MempoolFull | FeeTooLow)`
    - `SystemError(StorageError | TimeoutError | CorruptionError)`
  - **WalletCorrupted variant**:
    ```rust
    WalletCorrupted {
        reason: String,
        suggested_action: String, // "Restore from backup" or "Re-create wallet"
    }
    ```
  - **Each**: Include user-friendly message, suggested_action field
  - **Depends on**: None (error.rs enhancement)
  - **Makes pass**: T012

- [x] **T024** Improve error messages in wallet commands
  - **File**: `btpc-desktop-app/src-tauri/src/wallet_commands.rs`
  - **Implementation**: Replace generic error strings with specific error types
  - **Changes**:
    - `create_transaction`: Return `INSUFFICIENT_FUNDS` with available/required
    - `sign_transaction`: Return `SIGNATURE_FAILED` with missing_seed flag
    - `broadcast_transaction`: Return `NETWORK_UNAVAILABLE` with retry_after
  - **Depends on**: T023
  - **Makes pass**: T003, T004, T005, T012

## Phase 3.4: Frontend Updates (Article XI)

### Event Listeners (No localStorage)

- [x] **T25** [P] Add transaction event listeners
  - **File**: `btpc-desktop-app/ui/transactions.html`
  - **Implementation**: Add Tauri event listeners in `<script>` section
  - **Listeners**:
    - `transaction:validated` → Show fee in UI
    - `transaction:signing_started` → Show "Signing..." status
    - `transaction:signed` → Show "Ready to broadcast"
    - `transaction:broadcast` → Show "Broadcasting..."
    - `transaction:confirmed` → Show success message
    - `transaction:failed` → Show error with suggested_action
  - **Article XI Compliance**: No state in localStorage (Section 11.4)
  - **Depends on**: T019 (event emitter exists)
  - **Makes pass**: T008, T010

- [x] **T26** Add event cleanup on page unload
  - **File**: `btpc-desktop-app/ui/transactions.html`
  - **Implementation**: Add `beforeunload` event handler
  - **Code**:
    ```javascript
    window.addEventListener('beforeunload', () => {
      if (window.unlistenTransactionEvents) {
        window.unlistenTransactionEvents.forEach(unlisten => unlisten());
      }
    });
    ```
  - **Article XI Compliance**: Prevent memory leaks (Section 11.6)
  - **Depends on**: T025
  - **Makes pass**: T008, Article XI compliance

### Balance Updates

- [x] **T27** [P] Add wallet balance update listener
  - **File**: `btpc-desktop-app/ui/wallet-manager.html`
  - **Implementation**: Listen for `wallet:balance_updated` event
  - **Updates**:
    - Refresh wallet list
    - Update displayed balance
    - Show balance breakdown (confirmed, pending, reserved)
  - **Article XI Compliance**: Backend pushes updates (Section 11.3)
  - **Depends on**: T019 (event emitter exists)
  - **Makes pass**: T010 (balance verification)

## Phase 3.5: Polish & Documentation

### Performance Validation

- [ ] **T028** [P] Benchmark transaction creation speed
  - **File**: `btpc-desktop-app/src-tauri/benches/bench_transaction_creation.rs`
  - **Target**: < 500ms for UTXO selection + fee calculation
  - **Test**: 100 transactions with varying UTXO sets
  - **Report**: Average, p50, p95, p99 latencies

- [ ] **T029** [P] Benchmark ML-DSA signing speed
  - **File**: `btpc-desktop-app/src-tauri/benches/bench_signature.rs`
  - **Target**: < 100ms per signature
  - **Test**: 1000 signatures with seed regeneration
  - **Report**: Average, p50, p95, p99 latencies

### Article XI Compliance Verification

- [ ] **T030** Verify backend-first validation
  - **File**: Manual test in `quickstart.md`
  - **Test**: Modify frontend to send invalid data
  - **Expected**: Backend rejects with clear error (not frontend validation)
  - **Article XI**: Section 11.2 (Backend authority)

- [ ] **T031** Verify no localStorage usage for transaction state
  - **File**: Manual test in `quickstart.md`
  - **Test**: Inspect browser storage during transaction
  - **Expected**: No transaction data in localStorage
  - **Article XI**: Section 11.4 (No localStorage-first)

- [ ] **T032** Verify event listener cleanup
  - **File**: Manual test in `quickstart.md`
  - **Test**: Navigate between pages, check memory
  - **Expected**: No memory leaks, listeners removed
  - **Article XI**: Section 11.6 (Cleanup required)

### Code Quality

- [x] **T033** Run cargo clippy and fix all warnings
  - **Command**: `cargo clippy --all-targets --all-features -- -D warnings`
  - **Fix**: All warnings in transaction-related modules
  - **Focus**: `wallet_commands.rs`, `wallet_manager.rs`, `fee_estimator.rs`, `events.rs`

- [x] **T034** [P] Add inline documentation
  - **Files**: All modified Rust files
  - **Action**: Add `///` doc comments to all public functions
  - **Include**: Examples, error cases, Article XI compliance notes

- [x] **T035** [P] Update CLAUDE.md with bug fix details
  - **File**: `/home/bob/BTPC/BTPC/CLAUDE.md`
  - **Add**: Feature 007 summary in Recent Changes section
  - **Include**: UTXO reservation, seed-based signing, dynamic fees

### Security Validation

- [ ] **T036** Verify no private key logging
  - **Tool**: `rg -i "private.?key|seed" btpc-desktop-app/src-tauri/src/ --type rust`
  - **Expected**: No logs/prints containing keys or seeds
  - **Fix**: Use `SecureString` or `Zeroizing` wrappers

- [ ] **T037** Verify constant-time operations
  - **File**: Manual review of `wallet_commands.rs`
  - **Check**: All cryptographic comparisons use constant-time functions
  - **Fix**: Replace `==` with `constant_time_eq` for sensitive data

### Final Testing

- [ ] **T038** Run full test suite
  - **Command**: `cargo test --workspace --all-features`
  - **Expected**: All tests pass (including new T003-T012)
  - **Report**: Test count, coverage percentage

- [ ] **T039** Execute quickstart manual tests
  - **File**: `specs/007-fix-inability-to/quickstart.md`
  - **Scenarios**: 1-5 (all must pass)
  - **Document**: Results in `MD/QUICKSTART_RESULTS_007.md`

- [ ] **T040** Performance validation
  - **Test**: Transaction creation < 500ms (T028)
  - **Test**: ML-DSA signing < 100ms (T029)
  - **Test**: UI responsive during processing
  - **Report**: All targets met

## Dependencies

### Test Dependencies
- **Tests (T003-T012) MUST complete before implementation (T013-T027)**

### Implementation Dependencies
- T014 (UTXO lock/unlock) depends on T013 (ReservationToken)
- T014.1 (UTXO cleanup task) depends on T014 (lock/unlock methods)
- T015.1 (wallet integrity checks) depends on T015 (wallet creation with seeds)
- T016 (signature with seeds) depends on T015, T015.1 (wallet creation with seeds, integrity checks)
- T018 (fee integration) depends on T017 (fee estimator)
- T020, T021, T022 (event emission) depend on T019 (event emitter)
- T022.1 (broadcast retry) depends on T019, T022 (event emitter, broadcast events)
- T024 (error messages) depends on T023 (error types)
- T026 (event cleanup) depends on T025 (event listeners)

### Frontend Dependencies
- T025 (event listeners) depends on T019 (backend events exist)
- T027 (balance updates) depends on T019 (backend events exist)

### Polish before validation
- T033-T037 (code quality) before T038-T040 (final testing)

## Parallel Execution Examples

### Test Phase (T003-T012 all parallel)
```bash
# Launch all contract tests together (different files):
Task: "Contract test for create_transaction command in btpc-desktop-app/src-tauri/tests/contract/test_create_transaction.rs"
Task: "Contract test for sign_transaction command in btpc-desktop-app/src-tauri/tests/contract/test_sign_transaction.rs"
Task: "Contract test for broadcast_transaction command in btpc-desktop-app/src-tauri/tests/contract/test_broadcast_transaction.rs"
Task: "Contract test for estimate_fee command in btpc-desktop-app/src-tauri/tests/contract/test_estimate_fee.rs"
Task: "Contract test for cancel_transaction command in btpc-desktop-app/src-tauri/tests/contract/test_cancel_transaction.rs"
Task: "Contract test for transaction event sequence in btpc-desktop-app/src-tauri/tests/contract/test_transaction_events.rs"
Task: "Contract test for transaction failure events in btpc-desktop-app/src-tauri/tests/contract/test_transaction_error_events.rs"
Task: "Integration test for full transaction flow in btpc-desktop-app/src-tauri/tests/integration/test_transaction_flow_integration.rs"
Task: "Integration test for concurrent transactions in btpc-desktop-app/src-tauri/tests/integration/test_concurrent_transactions.rs"
Task: "Integration test for error handling in btpc-desktop-app/src-tauri/tests/integration/test_transaction_errors.rs"
```

### Core Implementation (T013, T015, T017, T019, T023 parallel - different modules)
```bash
# Launch independent modules together:
Task: "Implement ReservationToken in wallet_manager in btpc-desktop-app/src-tauri/src/wallet_manager.rs"
Task: "Update wallet creation to store seeds in btpc-desktop-app/src-tauri/src/wallet_commands.rs"
Task: "Implement fee estimation service in btpc-desktop-app/src-tauri/src/fee_estimator.rs"
Task: "Implement transaction event emitter in btpc-desktop-app/src-tauri/src/events.rs"
Task: "Implement specific error types in btpc-desktop-app/src-tauri/src/error.rs"
```

### Frontend Updates (T025, T027 parallel - different files)
```bash
# Launch frontend listeners together:
Task: "Add transaction event listeners in btpc-desktop-app/ui/transactions.html"
Task: "Add wallet balance update listener in btpc-desktop-app/ui/wallet-manager.html"
```

### Polish (T028, T029, T034, T035 parallel - independent tasks)
```bash
# Launch documentation and benchmarks together:
Task: "Benchmark transaction creation speed in btpc-desktop-app/src-tauri/benches/bench_transaction_creation.rs"
Task: "Benchmark ML-DSA signing speed in btpc-desktop-app/src-tauri/benches/bench_signature.rs"
Task: "Add inline documentation in all modified Rust files"
Task: "Update CLAUDE.md with bug fix details in /home/bob/BTPC/BTPC/CLAUDE.md"
```

## Notes

- **[P] tasks** = different files, no dependencies, can run in parallel
- **TDD mandatory**: Write tests first (T003-T012), verify they FAIL, then implement
- **Article XI compliance**: All desktop tasks follow backend-first, event-driven patterns
- **Backward compatibility**: Seed field is optional (support existing wallets without seeds)
- **Testing**: Run `cargo test` after each task to ensure no regressions
- **Commit**: Commit after each task with descriptive message (e.g., "T013: Implement ReservationToken for UTXO locking")

## Success Criteria

✅ **All tasks complete when**:
1. All 43 tests pass (including 10 new contract/integration tests)
2. Can send between internal wallets (quickstart Scenario 1)
3. Can send to external addresses (quickstart Scenario 2)
4. Proper error messages displayed (quickstart Scenario 3)
5. No double-spending possible (quickstart Scenario 4)
6. Events fire in correct sequence (quickstart Scenario 5)
7. Performance targets met (< 500ms creation, < 100ms signing)
8. Article XI compliance verified (T030-T032)
9. Zero clippy warnings (T033)
10. No security issues (T036-T037)

---

**Tasks Generated**: 43 tasks (10 tests, 18 implementation, 15 polish/validation)
**Estimated Effort**: 2-3 days (TDD adds upfront work but reduces debugging; +3-4 hours for new tasks T014.1, T015.1, T022.1)
**Priority**: P0 (Critical bug blocking core functionality)
**Constitution Compliance**: Article XI enforced for all desktop tasks