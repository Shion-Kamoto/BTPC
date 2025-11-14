# Feature 010 Completion Plan: Remove Old Process Management

**Status**: Critical - Dual architecture causing process control issues
**Created**: 2025-11-10
**Priority**: P0 (Blocking)

## Problem Statement

Feature 010 (Embedded Node) was partially implemented, creating a **dual architecture conflict**:

- ✅ **New**: MiningThreadPool (in-process), embedded_node, unified_db (lines 441-443)
- ❌ **Old**: ProcessManager spawning external binaries (line 417)
- **Result**: UI buttons control new system, but old system spawns uncontrolled processes

## Root Cause

Tasks T019-T022 (Code Removal Phase) were **never completed**:
- T019: Delete btpc_integration.rs
- T020: Delete process_manager.rs
- T021: Remove RpcClient usage
- T022: Delete sync_service.rs

## Execution Plan

### Phase 1: Remove from AppState (main.rs lines 412-519)

**Remove these fields:**
```rust
// Line 417 - Remove
process_manager: Arc<process_manager::ProcessManager>,

// Line 418 - Remove
mining_processes: Arc<Mutex<HashMap<String, Child>>>,

// Line 425 - Remove
btpc: BtpcIntegration,

// Line 432 - Remove (replaced by embedded_node)
sync_service: Arc<Mutex<Option<BlockchainSyncService>>>,
```

**Keep these fields (Feature 010 new architecture):**
```rust
// Lines 441-443 - KEEP
unified_db: Arc<tokio::sync::RwLock<Option<...>>>,
embedded_node: Arc<tokio::sync::RwLock<Option<...>>>,
mining_pool: Arc<tokio::sync::RwLock<Option<...>>>,
```

### Phase 2: Remove AppState::new() Initialization (lines 489-519)

**Remove these initializations:**
```rust
// Line 449 - Remove
let btpc = BtpcIntegration::new(config.btpc_home.clone());

// Lines 451-452 - Remove (no longer need btpc directories)
btpc.setup_directories(&config.log_dir, &config.data_dir, &config.config_dir)?;

// Line 454 - Remove
let installation_status = btpc.check_installation();

// Line 494 - Remove
process_manager: Arc::new(process_manager::ProcessManager::new(false)),

// Line 495 - Remove
mining_processes: Arc::new(Mutex::new(HashMap::new())),

// Line 502 - Remove
btpc,

// Line 509 - Remove
sync_service: Arc::new(Mutex::new(None)),
```

**Add these initializations:**
```rust
// Lines 515-517 - ADD (Feature 010 fields)
unified_db: Arc::new(tokio::sync::RwLock::new(None)),
embedded_node: Arc::new(tokio::sync::RwLock::new(None)),
mining_pool: Arc::new(tokio::sync::RwLock::new(None)),
```

### Phase 3: Remove Old Node Commands

**Commands to DELETE:**
```rust
// Lines 689-798 - DELETE start_node() function
// Lines 800-846 - DELETE stop_node() function
// Lines 849-917 - DELETE get_node_status() function
```

**Commands to KEEP (Feature 010):**
- `mining_commands::start_mining` ✅
- `mining_commands::stop_mining` ✅
- `mining_commands::get_mining_stats` ✅

### Phase 4: Remove RpcClient Usage

**Search and replace all occurrences:**
```bash
grep -n "rpc_client\|RpcClient" src/main.rs
```

**Lines to modify:**
- Line 2033-2036: get_address_balance() - Use embedded_node directly
- Line 2161-2170: get_blockchain_info() - Use embedded_node directly
- Line 2212-2233: get_recent_blocks() - Use embedded_node directly
- Line 2262-2266: get_recent_transactions() - Use embedded_node directly
- Line 2297-2320: search_blockchain() - Use embedded_node directly

**Pattern to replace:**
```rust
// OLD:
let rpc_client = RpcClient::new(&state.config.rpc.host, state.config.rpc.port);
let result = rpc_client.some_method().await;

// NEW:
let node_guard = state.embedded_node.read().await;
let node = node_guard.as_ref().ok_or("Node not initialized")?;
let result = node.read().unwrap().some_method();
```

### Phase 5: Remove Module Declarations (lines 53-61)

**Remove:**
```rust
// Line 53 - Remove
mod btpc_integration;

// Line 59 - Remove (comment says moved to lib.rs but still imported)
// mod rpc_client;

// Line 61 - Remove
mod process_manager;

// Line 90 - Remove
use btpc_integration::BtpcIntegration;
```

### Phase 6: Remove Process Adoption Logic (lines 2531-2569)

**Delete entire setup() hook:**
```rust
// Lines 2531-2569 - DELETE
// This spawns external processes and adopts them - NO LONGER NEEDED
```

### Phase 7: Remove invoke_handler Registrations (lines 2590-2592)

**Remove:**
```rust
// Lines 2590-2592 - Remove from generate_handler![]
start_node,
stop_node,
get_node_status,
```

### Phase 8: Delete Old Module Files (T019-T022)

**Delete these files:**
```bash
rm src/btpc_integration.rs    # T019 (~377 lines)
rm src/process_manager.rs     # T020 (~459 lines)
# Note: rpc_client.rs in lib.rs, needs separate handling
# Note: sync_service.rs removal needs wallet_manager.rs update first
```

### Phase 9: Update Frontend (if needed)

**Check if frontend calls old commands:**
```bash
grep -r "start_node\|stop_node\|get_node_status" ui/
```

**Replace with:**
- Remove "Start Node" / "Stop Node" buttons (node is always embedded)
- Keep only "Start Mining" / "Stop Mining" buttons

## Validation Checklist

After changes:
- [ ] `cargo check` passes
- [ ] No `process_manager` references remain
- [ ] No `btpc_integration` references remain
- [ ] No external btpc_node/btpc_miner processes spawn
- [ ] Mining start/stop buttons work
- [ ] UI reflects mining state correctly
- [ ] `ps aux | grep btpc` shows only desktop app process

## Rollback Plan

If issues occur:
```bash
git stash
git checkout HEAD -- src/main.rs
# Restart from Phase 1
```

## Estimated Time

- Phase 1-7: ~2 hours (careful refactoring)
- Phase 8: ~15 minutes (file deletion)
- Phase 9: ~30 minutes (frontend updates)
- Testing: ~30 minutes

**Total: ~3-4 hours**

## Next Steps

1. Review this plan
2. Execute phases sequentially
3. Test after each phase
4. Complete T033 (Constitutional Compliance) after all phases done