# TD-001 POC Status Update - 2025-11-04

## Session Progress

### Work Completed
- Analyzed transaction_commands.rs (1008 lines, 6 commands)
- Created design document (TD001_REFACTORING_DESIGN.md)
- Created continuation guide (TD001_CONTINUATION_GUIDE.md)
- Attempted full core module implementation (transaction_commands_core.rs)

### Issues Encountered

#### Architectural Constraints
1. **RpcClient not accessible**: `rpc_client` module is in main.rs, not lib.rs
   - Cannot import for broadcast_transaction_core()
   - Broadcast operation tightly coupled to RPC infrastructure

2. **TransactionStateManager not accessible**: Defined in transaction_commands.rs (main.rs)
   - Cannot import for get_transaction_status_core() or cancel_transaction_core()
   - State management tightly coupled to Tauri State

3. **API Mismatches**: Original implementation uses:
   - `serialize_for_signature()` + single signature for all inputs (not per-input signing)
   - `Script::unlock_p2pkh()` for signature scripts (not manual construction)
   - `private_key.public_key()` (not `key_entry.public_key()`)

### Revised Scope

**Functions Suitable for Core Module**:
1. ✅ `create_transaction_core()` - Can be extracted (uses TransactionBuilder, Address validation)
2. ✅ `sign_transaction_core()` - Can be extracted (wallet loading, ML-DSA signing)
3. ✅ `estimate_fee_core()` - Can be extracted (uses TransactionBuilder.summary())

**Functions NOT Suitable for Core Module** (remain as Tauri commands):
4. ❌ `broadcast_transaction` - Requires RpcClient (only in main.rs)
5. ❌ `get_transaction_status` - Requires TransactionStateManager (only in main.rs)
6. ❌ `cancel_transaction` - Requires TransactionStateManager + UTXO manager

### Recommendation

**Complete POC with 3 functions only**:
- Demonstrates pattern successfully
- Extract the most complex business logic (create, sign, estimate_fee)
- Leave infrastructure-dependent operations in Tauri layer
- Update continuation guide to reflect this scope

**Estimated Time to Complete**: 30-60 min
- Fix API mismatches in create/sign/estimate_fee
- Remove broadcast/status/cancel from core module
- Verify compilation
- Update documentation

**Value**: Still achieves 70% of TD-001 goal (testable business logic extraction)

## Next Steps

1. Simplify core module to 3 functions (create, sign, estimate_fee)
2. Fix API calls to match actual implementation
3. Verify compilation
4. Update continuation guide
5. Document lessons learned for future refactoring

## Lessons Learned

- Module visibility matters: lib.rs vs main.rs distinction is critical
- Check actual APIs before implementing (use grep/read liberally)
- Infrastructure dependencies (RPC, State) are hard to extract
- Pure business logic (validation, calculation, transformation) is ideal for core modules