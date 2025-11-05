# TD-001 Continuation Guide - Completing the Refactoring

**Date**: 2025-11-04
**Status**: Partial POC Complete (2/6 functions extracted)
**Outcome**: Architectural constraints block full extraction
**Files Created**:
- `src/transaction_commands_core.rs` - Core business logic module (279 lines) ✅
- `MD/TD001_REFACTORING_DESIGN.md` - Design document ✅
- `MD/TD001_POC_STATUS.md` - Status and constraints ✅
- `MD/SESSION_HANDOFF_2025-11-04_TD001_CONTINUATION.md` - Handoff doc ✅
- This guide - Continuation steps ✅

---

## ⚠️ ARCHITECTURAL CONSTRAINTS DISCOVERED

**Module Visibility Issues** (lib.rs vs main.rs):

**Functions CAN Be Extracted** (No main.rs dependencies):
- ✅ **create_transaction_core()** - Uses TransactionBuilder + Address (both in lib.rs)
- ✅ **estimate_fee_core()** - Uses TransactionBuilder.summary() (in lib.rs)

**Functions CANNOT Be Extracted** (Require main.rs modules):
- ❌ **broadcast_transaction** - Needs `RpcClient` (E0433: module only in main.rs)
- ❌ **sign_transaction** - API mismatches (KeyEntry methods don't match docs)
- ❌ **get_transaction_status** - Needs `TransactionStateManager` (E0432: only in main.rs)
- ❌ **cancel_transaction** - Needs `TransactionStateManager` + UTXOManager state

**Root Cause**:
- `rpc_client` module declared in main.rs, not exported in lib.rs
- `TransactionStateManager` defined in transaction_commands.rs (part of main.rs binary)
- Infrastructure modules cannot be imported by lib.rs modules

**Options Going Forward**:
1. **Accept Partial POC** - Keep 2 extracted functions, defer rest (0 hours)
2. **Full Refactoring** - Move infrastructure to lib.rs (3-4 hours)
3. **Alternative Approach** - Test Tauri commands with integration tests (see T032 analysis)

---

## What's Been Completed ✅

### 1. Design Document
**File**: `MD/TD001_REFACTORING_DESIGN.md`
- Architecture pattern defined
- Error types designed
- API structure documented
- Timeline estimated (4-5 hours total)

### 2. Core Module Created (Partial)
**File**: `btpc-desktop-app/src-tauri/src/transaction_commands_core.rs` (279 lines)

**Implemented**:
- ✅ `TransactionError` - comprehensive error type (9 variants)
- ✅ `create_transaction_core()` - Transaction building with UTXO selection
- ✅ `estimate_fee_core()` - Fee estimation with TransactionBuilder
- ✅ Result types: `TransactionCreationResult`, `EstimateFeeResult`
- ✅ Test module stub (TODO comment for future tests)

**NOT Implemented** (Blocked by architectural constraints):
- ❌ `sign_transaction_core()` - API mismatches
- ❌ `broadcast_transaction_core()` - Needs RpcClient (main.rs)
- ❌ `get_transaction_status_core()` - Needs TransactionStateManager (main.rs)
- ❌ `cancel_transaction_core()` - Needs TransactionStateManager (main.rs)

**Status**: ✅ Compiles successfully, added to lib.rs, 0 errors

### 3. Pattern Established
The core functions demonstrate the refactoring pattern:
- **No Tauri dependencies** (no State, no AppHandle)
- **Pure business logic** - testable independently
- **Structured results** - custom types, not JSON responses
- **Comprehensive errors** - detailed error variants

---

## Remaining Work (2-3 hours)

### Phase 1: Complete Core Module (1-1.5 hours)

**4 Functions to Implement**:

#### A. `broadcast_transaction_core()`
**Complexity**: Medium (RPC communication + error handling)
**Estimated**: 30 min

```rust
pub async fn broadcast_transaction_core(
    rpc_client: &RpcClient,
    transaction: &Transaction,
) -> Result<BroadcastResult, TransactionError> {
    // 1. Verify transaction is fully signed
    // 2. Serialize to hex
    // 3. Call RPC send_raw_transaction
    // 4. Return transaction ID
}
```

**Extract from**: `transaction_commands.rs` lines 499-817

#### B. `get_transaction_status_core()`
**Complexity**: Trivial (state lookup)
**Estimated**: 10 min

```rust
pub fn get_transaction_status_core(
    tx_state_manager: &TransactionStateManager,
    transaction_id: &str,
) -> Result<TransactionState, TransactionError> {
    // Simple lookup, already isolated
}
```

**Extract from**: `transaction_commands.rs` lines 818-827

#### C. `cancel_transaction_core()`
**Complexity**: Low (state update + UTXO release)
**Estimated**: 20 min

```rust
pub fn cancel_transaction_core(
    tx_state_manager: &TransactionStateManager,
    utxo_manager: &UTXOManager,
    transaction_id: &str,
) -> Result<CancelResult, TransactionError> {
    // 1. Check if cancellable (not broadcast)
    // 2. Get reservation token
    // 3. Release reservation
    // 4. Return result
}
```

**Extract from**: `transaction_commands.rs` lines 830-887

#### D. `estimate_fee_core()`
**Complexity**: Medium (UTXO selection + fee calculation)
**Estimated**: 30 min

```rust
pub async fn estimate_fee_core(
    utxo_manager: &UTXOManager,
    fee_estimator: &FeeEstimator,
    from_address: &str,
    to_address: &str,
    amount: u64,
    custom_fee_rate: Option<u64>,
) -> Result<EstimateFeeResult, TransactionError> {
    // 1. Select UTXOs for amount
    // 2. Calculate transaction size
    // 3. Estimate fee with FeeEstimator
    // 4. Return breakdown
}
```

**Extract from**: `transaction_commands.rs` lines 909-1008

---

### Phase 2: Refactor Tauri Commands (30-60 min)

**Pattern for Each Command**:

```rust
// BEFORE (example: create_transaction)
#[tauri::command]
pub async fn create_transaction(
    state: State<'_, AppState>,
    request: CreateTransactionRequest,
    app: AppHandle,
) -> Result<CreateTransactionResponse, String> {
    // 170 lines of business logic mixed with events
}

// AFTER
#[tauri::command]
pub async fn create_transaction(
    state: State<'_, AppState>,
    request: CreateTransactionRequest,
    app: AppHandle,
) -> Result<CreateTransactionResponse, String> {
    use crate::transaction_commands_core::*;

    // 1. Emit initiated event
    app.emit("transaction:initiated", ...)?;

    // 2. Extract dependencies from State
    let utxo_manager = state.utxo_manager.lock()?;
    let rpc_port = *state.active_rpc_port.read().await;
    let fee_estimator = FeeEstimator::new(rpc_port);

    // 3. Select UTXOs (still done here - needs utxo_manager lock)
    let utxos = utxo_manager.select_utxos_for_amount(...)?;

    // 4. Call core function (TESTABLE!)
    let result = create_transaction_core(
        utxos,
        &request.from_address,
        &request.to_address,
        request.amount,
        calculated_fee,
    )?;

    // 5. Reserve UTXOs (needs utxo_manager lock)
    let reservation = utxo_manager.reserve_utxos(
        result.utxo_keys.clone(),
        Some(result.summary.transaction_id.clone()),
    )?;

    // 6. Update state
    state.tx_state_manager.set_transaction(...);

    // 7. Emit success events
    app.emit("transaction:validated", ...)?;
    app.emit("utxo:reserved", ...)?;

    // 8. Return response (convert core result to Response type)
    Ok(CreateTransactionResponse {
        transaction_id: result.summary.transaction_id,
        fee: result.fee,
        ...
    })
}
```

**Commands to Refactor**:
1. `create_transaction` (line 201) - 30 min
2. `sign_transaction` (line 371) - 20 min
3. `broadcast_transaction` (line 499) - 20 min
4. `get_transaction_status` (line 818) - 5 min
5. `cancel_transaction` (line 830) - 10 min
6. `estimate_fee` (line 909) - 15 min

---

### Phase 3: Update Test Files (1 hour)

**Test Files Needing Updates** (11 total):

**Priority 1** (core functionality tests):
1. `test_create_transaction.rs` - Use `create_transaction_core()`
2. `test_sign_transaction.rs` - Use `sign_transaction_core()`
3. `test_broadcast_transaction.rs` - Use `broadcast_transaction_core()`

**Priority 2** (helper/integration tests):
4. `test_estimate_fee.rs`
5. `test_cancel_transaction.rs`
6. `test_transaction_flow_integration.rs` - May still test Tauri commands
7. `test_concurrent_transactions.rs` - May still test Tauri commands

**Priority 3** (event/error tests):
8. `test_transaction_errors.rs` - Use core functions
9. `test_transaction_error_events.rs` - May still test Tauri commands
10. `test_transaction_events.rs` - May still test Tauri commands
11. `test_other_commands.rs`

**Example Test Update**:

```rust
// BEFORE (test stub - cannot run without Tauri)
#[test]
#[ignore] // Cannot test without Tauri infrastructure
fn test_create_transaction_success() {
    // Would need to mock AppHandle + State
}

// AFTER (can run!)
#[tokio::test]
async fn test_create_transaction_core_success() {
    use crate::transaction_commands_core::*;

    // Use existing test infrastructure
    let env = TestEnvironment::new().unwrap();
    let wallet = env.create_wallet_with_balance("test", 100_000_000).unwrap();

    // Get UTXOs (already available from test infrastructure)
    let utxos = wallet.utxos.clone();

    // Call core function (NO TAURI NEEDED!)
    let result = create_transaction_core(
        utxos,
        &wallet.address,
        "btpc1q...", // Test recipient
        50_000_000,  // Amount
        1_000,       // Fee
    ).unwrap();

    // Assertions
    assert_eq!(result.fee, 1_000);
    assert!(result.transaction.inputs.len() > 0);
    assert_eq!(result.transaction.outputs.len(), 2); // Recipient + change
}
```

**Process for Each Test File**:
1. Remove `#[ignore]` attribute
2. Replace Tauri command calls with core function calls
3. Use `TestEnvironment`, `TestWallet`, `MockRpcClient` from test infrastructure
4. Run test: `cargo test test_create_transaction`

---

## Quick Start Commands

### 1. Implement Remaining Core Functions
```bash
# Edit core module
code btpc-desktop-app/src-tauri/src/transaction_commands_core.rs

# Add broadcast_transaction_core() at line ~460
# Add get_transaction_status_core() at line ~490
# Add cancel_transaction_core() at line ~500
# Add estimate_fee_core() at line ~530
```

### 2. Refactor Tauri Commands
```bash
# Edit transaction_commands.rs
code btpc-desktop-app/src-tauri/src/transaction_commands.rs

# For each command (lines 201, 371, 499, 818, 830, 909):
# 1. Import: use crate::transaction_commands_core::*;
# 2. Extract State dependencies
# 3. Call core function
# 4. Emit events
# 5. Return response
```

### 3. Update Tests
```bash
# Pick a test file
code btpc-desktop-app/src-tauri/tests/test_create_transaction.rs

# Remove #[ignore]
# Replace command calls with core calls
# Add test infrastructure imports

# Run test
cd btpc-desktop-app/src-tauri
cargo test test_create_transaction
```

### 4. Verify All Tests Pass
```bash
cd btpc-desktop-app/src-tauri
cargo test --lib
```

---

## Success Criteria

### Partial POC Complete ✅ (This Session)
- [x] Design document created
- [x] Core module structure created
- [x] 2 core functions implemented (create, estimate_fee)
- [x] Error types comprehensive
- [x] Module compiles successfully (0 errors)
- [x] Pattern documented
- [x] Architectural constraints documented

### TD-001 Full Completion (BLOCKED - Requires Architectural Refactoring):
- [ ] All 6 core functions implemented ❌ BLOCKED (4 functions need main.rs modules)
- [ ] All 6 Tauri commands refactored to thin wrappers
- [ ] At least 3-5 test files updated and passing
- [ ] No regressions in existing tests (410 tests)
- [ ] Code compiles with 0 errors
- [ ] Updated TECHNICAL_DEBT_BACKLOG.md ✅ DONE (partial POC status)
- [ ] Updated STATUS.md ⏳ IN PROGRESS (test automation status)

**Note**: Full completion blocked by module visibility (RpcClient, TransactionStateManager in main.rs only).
See "Architectural Constraints Discovered" section for details.

---

## Tips & Best Practices

### When Extracting Business Logic:
1. **Identify pure logic** - no State, no AppHandle, no events
2. **Pass dependencies explicitly** - utxo_manager, rpc_client as parameters
3. **Return structured data** - custom result types, not Response structs
4. **Use comprehensive errors** - TransactionError variants, not String

### When Refactoring Tauri Commands:
1. **Keep event emission** - Tauri commands still emit events (Article XI)
2. **Keep state management** - Tauri commands still update tx_state_manager
3. **Extract params from State** - just before calling core function
4. **Convert results** - core result → Response type for JSON serialization

### When Updating Tests:
1. **Use test infrastructure** - MockRpcClient, TestWallet, TestEnvironment (already built!)
2. **Test core functions directly** - no AppHandle/State needed
3. **Keep integration tests** - some tests may still test full Tauri commands
4. **Remove #[ignore]** - tests should run now!

---

## Expected Outcomes

**After Full Completion**:
- ✅ 6 Tauri commands = thin wrappers (~20-30 lines each)
- ✅ Core module = ~1000 lines of testable business logic
- ✅ ~70 integration tests running (currently ignored)
- ✅ Test coverage > 80% for transaction logic
- ✅ No Tauri mocking needed for unit tests
- ✅ Better separation of concerns
- ✅ Easier to maintain and refactor

**Updated Files**:
- `src/transaction_commands_core.rs` (NEW, ~1000 lines final)
- `src/transaction_commands.rs` (refactored, ~400 lines final vs 1008 current)
- `src/lib.rs` (1 line added)
- 11 test files (updated to use core functions)
- `MD/TECHNICAL_DEBT_BACKLOG.md` (mark TD-001 complete)
- `MD/STATUS.md` (update test automation status)

---

## Troubleshooting

### "Core function doesn't compile"
- Check imports: `use crate::utxo_manager::UTXO;`
- Check error type conversions: `.map_err(|e| TransactionError::...)?`
- Verify function signatures match design

### "Test fails after refactoring"
- Verify test infrastructure imports: `use crate::transaction_commands_core::*;`
- Check parameter order: core functions have explicit params
- Use `TestEnvironment::new()` for test setup

### "Tauri command refactor breaks app"
- Ensure event emission still happens (Article XI)
- Verify state management (tx_state_manager updates)
- Check error conversion: core Error → String for Tauri

---

**Next Session**: Continue with Phase 1 (implement 4 remaining core functions) or Phase 2 (refactor Tauri commands)

**Estimated Completion**: 2-3 hours remaining work