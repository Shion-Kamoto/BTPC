# Session Handoff: Feature 010 Removal of Old Process Management
**Date**: 2025-11-10
**Status**: ⚠️ INCOMPLETE - Partial refactoring in progress, compilation broken
**Branch**: 009-integrate-gpu-mining
**Decision**: Proceeding with Option 1 (Full Refactoring)

## Problem Statement

Feature 010 (Embedded Node) was partially implemented, creating a **dual architecture conflict**:
- ✅ **New**: MiningThreadPool (in-process), embedded_node, unified_db
- ❌ **Old**: ProcessManager spawning external binaries
- **Result**: UI buttons control new system, but old system spawns uncontrolled processes

User reported:
> "Stopping both the node and the miner via their respective buttons do not stop either process. Even when killing the node via the terminal control + c doesnt stop the node, it just starts up again with the miner and continues to mining in the background and not in the app its self."

## Root Cause

Tasks T019-T022 (Code Removal Phase) from Feature 010 were **never completed**:
- T015: Remove ProcessManager from main.rs
- T019: Delete btpc_integration.rs (~377 lines)
- T020: Delete process_manager.rs (~459 lines)
- T021: Remove RpcClient usage (~424 lines)
- T022: Delete sync_service.rs (~410 lines)

## Decision Made: Full Refactoring (Option 1)

User chose to proceed with full refactoring rather than reverting. Reasoning:
- Work already done (Phases 1-2 partially complete)
- Detailed plan already exists (FEATURE_010_COMPLETION_PLAN.md)
- Problem is well-understood with all dependencies mapped
- Reverting would duplicate work (~4+ hours vs 2.5-3 hours)
- Phases are interdependent - must complete all or none
- Compiler will guide remaining work

## Work Completed This Session

### Phase 1: Remove Old Fields from AppState (✅ COMPLETED)
**File**: `btpc-desktop-app/src-tauri/src/main.rs` lines 412-440

**Removed fields:**
```rust
// REMOVED:
process_manager: Arc<process_manager::ProcessManager>,  // Line 417
mining_processes: Arc<Mutex<HashMap<String, Child>>>,   // Line 418
btpc: BtpcIntegration,                                   // Line 425
sync_service: Arc<Mutex<Option<BlockchainSyncService>>>, // Line 432
```

**Kept fields (Feature 010 new architecture):**
```rust
// KEPT:
unified_db: Arc<tokio::sync::RwLock<Option<...>>>,      // Line 437
embedded_node: Arc<tokio::sync::RwLock<Option<...>>>,   // Line 438
mining_pool: Arc<tokio::sync::RwLock<Option<...>>>,     // Line 439
```

### Phase 2: Remove Old Initializations (⚠️ PARTIALLY COMPLETED)
**File**: `btpc-desktop-app/src-tauri/src/main.rs` lines 442-459

**Changes made:**
```rust
// REMOVED btpc_integration initialization:
// let btpc = BtpcIntegration::new(config.btpc_home.clone());
// btpc.setup_directories(...);
// let installation_status = btpc.check_installation();

// REPLACED with direct fs operations:
fs::create_dir_all(&config.log_dir)?;
fs::create_dir_all(&config.data_dir)?;
fs::create_dir_all(&config.config_dir)?;

// CHANGED binaries_installed flag:
binaries_installed: false,  // Embedded node doesn't need external binaries
```

## ⚠️ CRITICAL ISSUE: Incomplete Refactoring

### Current State
**Compilation is BROKEN** - removed fields but code still references them extensively.

### Breaking References Still in Code

**1. AppState::new() still has orphaned initializations (lines 489-519):**
```rust
// Line 489-491: ERROR - these fields no longer exist in struct
process_manager: Arc::new(process_manager::ProcessManager::new(false)),
mining_processes: Arc::new(Mutex::new(HashMap::new())),

// Line 497: ERROR - btpc variable not initialized
btpc,

// Line 504: ERROR - sync_service field removed
sync_service: Arc::new(Mutex::new(None)),

// Lines 512-514: KEEP - Feature 010 new fields
unified_db: Arc::new(tokio::sync::RwLock::new(None)),
embedded_node: Arc::new(tokio::sync::RwLock::new(None)),
mining_pool: Arc::new(tokio::sync::RwLock::new(None)),
```

**2. Commands still using removed fields:**

`state.btpc` usage (~50+ occurrences):
- Line 648: `state.btpc.check_installation()` in get_system_status()
- Line 915: `state.btpc.bin_dir.display()` in create_wallet()
- Line 917: `state.btpc.create_wallet()` in create_wallet()
- Line 935: `state.btpc.get_wallet_address()` in get_wallet_balance()
- Line 940: `state.btpc.get_wallet_balance()` in get_wallet_balance()
- Line 981: `state.btpc.get_wallet_address()` in get_wallet_address()
- Line 1025: `state.btpc.get_wallet_address()` in send_btpc()
- Line 1034: `state.btpc.get_wallet_address()` in send_btpc()
- Line 1146: `state.btpc.get_wallet_address()` in get_wallet_balance_with_mined()
- Line 1181: `state.btpc.install_binaries_from_build()` in setup_btpc()
- Line 1183: `state.btpc.check_installation()` in setup_btpc()
- Line 1296: `state.btpc.get_wallet_address()` in get_wallet_utxos()
- Line 1314: `state.btpc.get_wallet_address()` in get_spendable_utxos()
- Line 1346: `state.btpc.get_wallet_address()` in sync_wallet_utxos()
- Line 1680: `state.btpc.get_wallet_address()` in create_transaction_preview()

`state.process_manager` usage (~15+ occurrences):
- Line 624: `state.process_manager.is_running("node")` in get_system_status()
- Line 626-627: `state.process_manager.get_info("node")` in get_system_status()
- Line 686: `state.process_manager.is_running("node")` in start_node()
- Line 724: `state.process_manager.start_detached()` in start_node()
- Line 796: `state.process_manager.kill("node")` in stop_node()
- Line 845: `state.process_manager.is_running("node")` in get_node_status()
- Line 848: `state.process_manager.get_info("node")` in get_node_status()
- Line 2078: `state.process_manager.is_running("node")` in save_network_config()
- Line 2526: `app_state.process_manager.clone()` in main()
- Lines 2539-2542: `process_manager.scan_and_adopt()` in main() setup()
- Lines 2554-2560: `process_manager.clone()` in window close handler
- Lines 2564-2570: `pm_health.health_check()` in health monitor thread

`state.mining_processes` usage (~5+ occurrences):
- Lines 636-645: Lock and check mining_processes in get_system_status()
- Lines 865-868: Lock and check mining_processes in get_mining_status()

`state.sync_service` usage (~3+ occurrences):
- Lines 764-789: Lock and manipulate sync_service in start_node()
- Lines 800-807: Lock and stop sync_service in stop_node()
- Lines 2004-2012: Lock and query sync_service in get_sync_stats()

## Remaining Work (Phases 3-9)

### **IMMEDIATE NEXT STEP: Complete Phase 2**

**Fix AppState::new() by removing orphaned initializations:**

Location: `btpc-desktop-app/src-tauri/src/main.rs` lines 484-515

Remove these lines:
```rust
// Line 489-491 - REMOVE (fields don't exist):
process_manager: Arc::new(process_manager::ProcessManager::new(false)),
mining_processes: Arc::new(Mutex::new(HashMap::new())),

// Line 497 - REMOVE (btpc not initialized):
btpc,

// Line 504 - REMOVE (field doesn't exist):
sync_service: Arc::new(Mutex::new(None)),
```

Keep these lines (Feature 010):
```rust
// Lines 512-514 - KEEP:
unified_db: Arc::new(tokio::sync::RwLock::new(None)),
embedded_node: Arc::new(tokio::sync::RwLock::new(None)),
mining_pool: Arc::new(tokio::sync::RwLock::new(None)),
```

### Phase 3: Delete Old Node Commands

**Lines to DELETE entirely:**
- Lines 684-792: `start_node()` function (109 lines)
- Lines 795-841: `stop_node()` function (47 lines)
- Lines 844-860: `get_node_status()` function (17 lines)
- Line 863-878: `get_mining_status()` function (16 lines) - replaced by mining_commands::get_mining_stats

**Total: ~189 lines to delete**

**Replacement**: Feature 010 embedded_node commands (already registered):
- `btpc_desktop_app::commands::embedded_node::init_embedded_node`
- `btpc_desktop_app::commands::embedded_node::shutdown_embedded_node`
- `mining_commands::start_mining`
- `mining_commands::stop_mining`
- `mining_commands::get_mining_stats`

### Phase 4: Replace RpcClient Usage

**5 commands need updating to use embedded_node instead of RpcClient:**

1. **get_address_balance_from_node()** (lines 2022-2034)
2. **get_blockchain_info()** (lines 2155-2199)
3. **get_recent_blocks()** (lines 2202-2249)
4. **get_recent_transactions()** (lines 2252-2278)
5. **search_blockchain()** (lines 2288-2328)

**Pattern to apply:**
```rust
// OLD:
use btpc_desktop_app::rpc_client::RpcClient;
let rpc_client = RpcClient::new(&state.config.rpc.host, state.config.rpc.port);
let result = rpc_client.some_method().await;

// NEW:
let node_guard = state.embedded_node.read().await;
let node = node_guard.as_ref().ok_or("Node not initialized")?;
let blockchain = node.read().unwrap().get_blockchain();
let result = blockchain.some_method();
```

### Phase 5: Remove Module Declarations

**Lines to DELETE:**
```rust
// Line 53:
mod btpc_integration;

// Line 60:
mod sync_service;

// Line 61:
mod process_manager;

// Line 90:
use btpc_integration::BtpcIntegration;

// Line 94:
use sync_service::{BlockchainSyncService, SyncConfig, SyncStats};
```

### Phase 6: Remove Process Adoption Logic

**Lines to DELETE in main() function:**
- Lines 2536-2571: Entire setup() implementation
  - Process scanning/adoption code (2539-2551)
  - Window close handler with process cleanup (2554-2561)
  - Process health monitoring thread (2564-2570)

**Keep:**
- Transaction monitor service (lines 2573-2578) ✅
- Tauri invoke_handler registration ✅

### Phase 7: Remove Old Command Registrations

**In invoke_handler (lines 2582-2718), remove:**
```rust
start_node,           // Line 2585
stop_node,            // Line 2586
get_node_status,      // Line 2587
get_mining_status,    // Line 2588
```

**Keep (Feature 010 commands already registered):**
```rust
mining_commands::start_mining,       // Line 2715
mining_commands::stop_mining,        // Line 2716
mining_commands::get_mining_stats,   // Line 2717
btpc_desktop_app::commands::embedded_node::*, // Lines 2709-2714
```

### Phase 8: Delete Old Module Files

**Files to delete:**
```bash
cd /home/bob/BTPC/BTPC/btpc-desktop-app/src-tauri
rm src/btpc_integration.rs    # T019 (~377 lines)
rm src/process_manager.rs     # T020 (~459 lines)
rm src/sync_service.rs         # T022 (~410 lines)
```

**Note**: `rpc_client.rs` is in lib.rs - handle separately if needed

### Phase 9: Update Frontend

**Check if frontend calls old commands:**
```bash
cd /home/bob/BTPC/BTPC/btpc-desktop-app
grep -r "start_node\|stop_node\|get_node_status\|get_mining_status" ui/
```

**Expected changes needed:**
- Remove "Start Node" / "Stop Node" buttons (node is embedded, always running)
- Change `get_mining_status` → `get_mining_stats` in any remaining files
- Keep "Start Mining" / "Stop Mining" buttons ✅

### Phase 10: Fix Remaining Compilation Errors

After mechanical deletions, fix remaining errors by:
1. Remove `state.btpc` references in wallet commands (replace with direct wallet_manager access)
2. Update `get_system_status()` to use embedded_node instead of process_manager
3. Update `setup_btpc()` to remove binary installation checks

## Execution Strategy

### Step-by-Step Approach

1. **Complete Phase 2** (~10 min)
   - Fix AppState::new() initialization
   - Run `cargo check` to see next errors

2. **Execute Phase 3** (~20 min)
   - Delete start_node(), stop_node(), get_node_status(), get_mining_status()
   - Run `cargo check`

3. **Execute Phase 5** (~5 min)
   - Remove module declarations (phases 4 and 5 can be swapped)
   - Run `cargo check`

4. **Execute Phase 6** (~10 min)
   - Remove process adoption logic from main()
   - Run `cargo check`

5. **Execute Phase 7** (~5 min)
   - Remove old command registrations
   - Run `cargo check`

6. **Execute Phase 8** (~5 min)
   - Delete old module files
   - Run `cargo check`

7. **Fix remaining compilation errors** (~30-60 min)
   - Let compiler guide fixes
   - Update commands one by one
   - Most will be simple deletions or replacements

8. **Execute Phase 4** (~20-30 min)
   - Replace RpcClient with embedded_node in 5 commands
   - May need to understand embedded_node API

9. **Execute Phase 9** (~15 min)
   - Check and update frontend

10. **Testing** (~30 min)
    - Build succeeds
    - App launches
    - Mining start/stop works
    - No external processes spawn

**Total estimated: 2.5-3 hours**

## Testing Requirements

After completion:
- [ ] `cargo check` passes
- [ ] `cargo build --release` succeeds
- [ ] No `process_manager` references remain
- [ ] No `btpc_integration` references remain
- [ ] No `sync_service` references remain
- [ ] No external btpc_node/btpc_miner processes spawn
- [ ] Mining start/stop buttons work in UI
- [ ] Mining stats display correctly
- [ ] UI reflects mining state correctly
- [ ] `ps aux | grep btpc` shows only desktop app process
- [ ] Embedded node initializes on app start

## File Status

### Modified (In Progress)
1. **btpc-desktop-app/src-tauri/src/main.rs**
   - Lines 412-440: ✅ Removed old AppState fields
   - Lines 442-459: ✅ Removed btpc_integration initialization
   - Lines 484-515: ⚠️ Still has orphaned initializations (NEXT STEP)
   - **Status**: ⚠️ Compilation broken - Phase 2 incomplete

### To Be Modified
2. **btpc-desktop-app/src-tauri/src/main.rs** (remaining changes)
   - Lines 684-860: DELETE old node commands (Phase 3)
   - Lines 2022-2328: UPDATE RpcClient usage (Phase 4)
   - Lines 53, 60, 61, 90, 94: DELETE module declarations (Phase 5)
   - Lines 2536-2571: DELETE process adoption (Phase 6)
   - Lines 2585-2588: DELETE command registrations (Phase 7)

### To Be Deleted
3. **btpc-desktop-app/src-tauri/src/btpc_integration.rs** (Phase 8)
4. **btpc-desktop-app/src-tauri/src/process_manager.rs** (Phase 8)
5. **btpc-desktop-app/src-tauri/src/sync_service.rs** (Phase 8)

### To Be Checked
6. **btpc-desktop-app/ui/*.html** - Check for old command calls (Phase 9)

### Created This Session
7. **MD/FEATURE_010_COMPLETION_PLAN.md** ✅
8. **MD/SESSION_HANDOFF_2025-11-10_FEATURE_010_REFACTOR.md** ✅ (this file)

## Next Session Start Commands

```bash
cd /home/bob/BTPC/BTPC/btpc-desktop-app/src-tauri

# Verify current state
git status
git diff src/main.rs | head -100

# Continue with Phase 2 completion
# Read lines 484-515 to see orphaned initializations
# Then remove them systematically
```

## Key Reminders

1. **Don't commit** until all phases complete and tests pass
2. **Use compiler as guide** - fix errors one by one
3. **Work in phases** - complete each phase before moving to next
4. **Test incrementally** - run `cargo check` after each phase
5. **Trust the process** - mechanical deletions, not complex decisions
6. **Phases 3, 5, 6, 7, 8** are pure deletions (easy/safe)
7. **Phases 2, 4, 10** require code fixes (use compiler errors as guide)
8. **Phase 9** may require minimal frontend changes

## Why This Approach Will Work

- ✅ Detailed plan exists with line numbers
- ✅ All dependencies cataloged
- ✅ Most work is mechanical deletion
- ✅ Compiler will catch any missed references
- ✅ Phases can be done incrementally
- ✅ Can test after each phase
- ✅ Clear success criteria
- ✅ No architectural decisions needed

## References

- Feature 010 Plan: `/specs/010-reconfigure-btpc-desktop/plan.md`
- Feature 010 Tasks: `/specs/010-reconfigure-btpc-desktop/tasks.md`
- Completion Plan: `/MD/FEATURE_010_COMPLETION_PLAN.md`
- Mining Commands (NEW): `src/mining_commands.rs`
- Embedded Node (NEW): `src/lib.rs` + `src/commands/embedded_node.rs`