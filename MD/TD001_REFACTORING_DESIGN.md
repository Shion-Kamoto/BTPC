# TD-001: Tauri Commands Refactoring Design

**Date**: 2025-11-04
**Scope**: Extract business logic from 6 Tauri commands for testability
**Effort**: 4-5 hours (full), 2 hours (POC)

---

## Problem

Tauri commands mix UI framework code with business logic:
```rust
#[tauri::command]
pub async fn create_transaction(
    state: State<'_, AppState>,      // ❌ Hard to mock
    request: CreateTransactionRequest,
    app: AppHandle,                   // ❌ Hard to mock
) -> Result<CreateTransactionResponse, String> {
    // 170 lines of business logic + event emission
}
```

**Issues**:
- Cannot test without Tauri runtime
- Tight coupling to framework
- No unit test coverage

---

## Solution Design

### Architecture Pattern

**Before**:
```
Tauri Command (State + AppHandle) → Business Logic → Result
```

**After**:
```
Tauri Command (State + AppHandle)
  ├─> Extract params from State
  ├─> Call Core Function (pure business logic)
  ├─> Emit events via AppHandle
  └─> Return result
```

### Core Module API

**New file**: `src/transaction_commands_core.rs`

```rust
/// Core business logic - no Tauri dependencies
pub mod core {
    use crate::utxo_manager::UTXOManager;
    use crate::wallet_manager::WalletManager;
    use crate::fee_estimator::FeeEstimator;
    use crate::rpc_client::RpcClient;

    /// Result type for transaction creation
    pub struct TransactionCreationResult {
        pub transaction: Transaction,
        pub summary: TransactionSummary,
        pub reservation_token: String,
        pub utxo_keys: Vec<String>,
        pub fee: u64,
    }

    /// Create unsigned transaction (business logic only)
    pub async fn create_transaction_core(
        utxo_manager: &UTXOManager,
        fee_estimator: &FeeEstimator,
        from_address: &str,
        to_address: &str,
        amount: u64,
        custom_fee_rate: Option<u64>,
    ) -> Result<TransactionCreationResult, TransactionError> {
        // All business logic here
        // No State, no AppHandle, no events
    }

    /// Sign transaction with ML-DSA
    pub fn sign_transaction_core(
        transaction: &mut Transaction,
        wallet_path: &Path,
        password: &str,
    ) -> Result<SigningResult, TransactionError> {
        // All signing logic here
    }

    /// Broadcast transaction to network
    pub async fn broadcast_transaction_core(
        rpc_client: &RpcClient,
        transaction: &Transaction,
    ) -> Result<String, TransactionError> {
        // All broadcast logic here
    }

    // ... other core functions
}
```

### Refactored Tauri Command Pattern

```rust
#[tauri::command]
pub async fn create_transaction(
    state: State<'_, AppState>,
    request: CreateTransactionRequest,
    app: AppHandle,
) -> Result<CreateTransactionResponse, String> {
    // 1. Emit initiated event
    app.emit("transaction:initiated", ...);

    // 2. Extract dependencies from State
    let utxo_manager = state.utxo_manager.lock()?;
    let fee_estimator = FeeEstimator::new(rpc_port);

    // 3. Call core logic (testable!)
    let result = core::create_transaction_core(
        &utxo_manager,
        &fee_estimator,
        &request.from_address,
        &request.to_address,
        request.amount,
        request.fee_rate,
    ).await?;

    // 4. Update state
    state.tx_state_manager.set_transaction(...);

    // 5. Emit success events
    app.emit("transaction:validated", ...);
    app.emit("utxo:reserved", ...);

    // 6. Return response
    Ok(CreateTransactionResponse { ... })
}
```

---

## Commands to Refactor

### Phase 1: POC (2 hours) ✅ This Session
1. ✅ **create_transaction** (~170 lines) - Most complex, highest value
2. ✅ **sign_transaction** (~130 lines) - Critical for testing

### Phase 2: Remaining (2-3 hours) - Future Session
3. **broadcast_transaction** (~320 lines but mostly error handling)
4. **get_transaction_status** (~10 lines - trivial)
5. **cancel_transaction** (~60 lines)
6. **estimate_fee** (~100 lines)

---

## Test Files to Update

### Phase 1: POC (demonstrate pattern)
1. `test_create_transaction.rs` - Update to use core function
2. `test_sign_transaction.rs` - Update to use core function

### Phase 2: Remaining
3. `test_broadcast_transaction.rs`
4. `test_cancel_transaction.rs`
5. `test_estimate_fee.rs`
6. `test_transaction_flow_integration.rs`
7. `test_concurrent_transactions.rs`
8. `test_transaction_errors.rs`
9. `test_transaction_error_events.rs`
10. `test_transaction_events.rs`
11. `test_other_commands.rs`

---

## Benefits

**Before Refactoring**:
- ❌ Cannot unit test business logic
- ❌ Must mock entire Tauri runtime
- ❌ Tight coupling to framework
- ❌ 2497 lines of ignored test stubs

**After Refactoring**:
- ✅ Pure business logic testable
- ✅ Only need to mock dependencies (UTXO, RPC)
- ✅ Loose coupling via interfaces
- ✅ ~70 integration tests can run

---

## Success Criteria

### POC Complete When:
- [x] `transaction_commands_core.rs` created
- [x] `create_transaction` refactored to thin wrapper
- [x] `sign_transaction` refactored to thin wrapper
- [x] 2 test files updated and passing
- [x] No regressions in existing tests
- [x] Pattern documented for remaining work

### Full TD-001 Complete When:
- [ ] All 6 commands refactored
- [ ] All 11 test files updated
- [ ] All ~70 tests passing
- [ ] Code coverage > 80%
- [ ] Documentation updated

---

## Estimated Timeline

**POC (This Session)**: 2 hours
- Core module creation: 30 min
- create_transaction refactor: 45 min
- sign_transaction refactor: 30 min
- Test updates: 15 min

**Remaining Work**: 2-3 hours
- 4 commands refactor: 1.5-2 hours
- 9 test files update: 1-1.5 hours

**Total**: 4-5 hours (matches backlog estimate)

---

## Implementation Notes

**Key Decisions**:
1. Keep event emission in Tauri commands (Article XI compliance)
2. Core functions return structured results (not Response types)
3. Error types in core module (TransactionError)
4. Core functions async where needed (RPC calls)
5. Use references to avoid unnecessary clones

**Testing Strategy**:
- Core functions: Unit tests with mocked dependencies
- Tauri commands: Integration tests (if needed)
- Use existing test infrastructure (MockRpcClient, TestWallet, TestEnvironment)

---

**Next**: Implement POC with `create_transaction` and `sign_transaction`