# Desktop App Node Persistence - Critical Fix Plan

**Issue**: Node stops when navigating between pages in the desktop app
**Severity**: 🔴 **CRITICAL BLOCKER** - Prevents any real-world testing or deployment
**Date**: 2025-10-06

---

## Problem Analysis

### Root Cause #1: Process Lifecycle Management
**Issue**: The Tauri backend spawns the node process but doesn't properly manage its lifecycle across page navigation.

**Current Behavior**:
1. User clicks "Start Node" on one page
2. Node process spawns (stored in `state.processes` HashMap)
3. User navigates to different page
4. **Problem**: Process may be getting killed or losing reference

**Evidence**:
- Node starts successfully (PID 3098109 currently running)
- Process is stored in `Arc<Mutex<HashMap<String, Child>>>`
- But node stops when pages change

### Root Cause #2: Port Configuration Conflict
**Issue**: Desktop node is configured to use port 18350 (same as testnet) for RPC

**Current Configuration** (main.rs:166):
```rust
rpc: RpcConfig {
    host: "127.0.0.1".to_string(),
    port: 18350,  // ❌ CONFLICT: Same as testnet node
    enable_cors: true,
},
```

**Actual Command** (line 445-447):
```rust
"--rpcport", &state.config.rpc.port.to_string(),  // Uses 18350
"--rpcbind", &state.config.rpc.host,
```

**Result**: Desktop node tries to bind to port 18350 but testnet node already owns it, causing conflict.

### Root Cause #3: Process Reference Loss
**Issue**: `Child` process stored in HashMap may be dropped when page changes

**Current Storage** (main.rs:462-464):
```rust
let mut processes = state.processes.lock().unwrap();
processes.insert("node".to_string(), child);
```

**Problem**: If the `Child` handle is dropped (e.g., during page navigation cleanup), the process becomes orphaned and may terminate.

### Root Cause #4: Frontend State Not Persisting
**Issue**: Frontend may be re-initializing state on page navigation, causing repeated start attempts

**Behavior**:
- Each page loads fresh JavaScript
- State may not persist across navigation
- Could trigger duplicate start commands

---

## Comprehensive Fix Strategy

### Fix #1: Change Desktop Node to Wallet-Only Mode ✅ RECOMMENDED

**Approach**: Don't run a local node at all - connect directly to testnet node via RPC

**Changes Required**:

#### A. Update Node Config (main.rs:146-150)
```rust
node: NodeConfig {
    sync_interval_secs: 5,
    max_peers: 50,
    listen_port: 18361,
    enable_rpc: false,  // ✅ Already disabled - good!
},
```

#### B. Disable "Start Node" Button in UI
```html
<!-- ui/node.html -->
<button id="start-node-btn" disabled title="Desktop app connects to external node">
    Start Node (External)
</button>
```

#### C. Remove start_node Command or Make it No-Op
```rust
#[tauri::command]
async fn start_node(state: State<'_, AppState>) -> Result<String, String> {
    // Desktop app doesn't run local node - connects to testnet
    Ok("Desktop app is configured as wallet-only. Please run a separate btpc_node instance.".to_string())
}
```

**Pros**:
- ✅ Simplest solution
- ✅ No process management issues
- ✅ No port conflicts
- ✅ Users run dedicated node separately
- ✅ Matches typical wallet app architecture (Electrum, Exodus, etc.)

**Cons**:
- ❌ Requires separate node process
- ❌ Less "all-in-one" user experience

---

### Fix #2: Proper Port Separation for Desktop Node (If keeping local node)

**Approach**: Fix port conflict and ensure proper process management

**Changes Required**:

#### A. Change RPC Port to 18360 (main.rs:164-168)
```rust
rpc: RpcConfig {
    host: "127.0.0.1".to_string(),
    port: 18360,  // ✅ Changed from 18350 to avoid conflict
    enable_cors: true,
},
```

#### B. Add Node Config for Desktop Node
```rust
node: NodeConfig {
    sync_interval_secs: 5,
    max_peers: 50,
    listen_port: 18361,   // P2P port
    enable_rpc: true,      // ✅ Enable RPC for desktop node
},
```

#### C. Update RPC Client to Use Correct Port
Currently the app connects to testnet (18350). If running local node, change to 18360:

```rust
// Add configuration option
pub struct AppConfig {
    // ...
    use_local_node: bool,  // Toggle between local (18360) and testnet (18350)
}
```

**Pros**:
- ✅ Desktop app fully self-contained
- ✅ No external dependencies

**Cons**:
- ❌ Complex process management
- ❌ Requires fixing process lifecycle issues
- ❌ Higher resource usage (2 nodes running)

---

### Fix #3: Process Lifecycle Management (Required if keeping local node)

**Problem**: `Child` process may be dropped, causing node termination

**Solution**: Use persistent background process with proper cleanup

#### A. Change Process Storage to Use PID Instead of Child
```rust
pub struct AppState {
    processes: Arc<Mutex<HashMap<String, u32>>>,  // Store PID, not Child
    // ...
}
```

#### B. Spawn Node as Detached Process
```rust
use std::process::Stdio;

let mut cmd = Command::new(&bin_path);
cmd.args([/* ... */])
   .stdin(Stdio::null())
   .stdout(stdout_file)
   .stderr(stderr_file)
   .spawn()
   .map_err(|e| format!("Failed to start node: {}", e))?;

// Detach the process (let it run independently)
let pid = child.id();
std::mem::forget(child);  // Don't wait for process to exit

// Store only the PID
processes.insert("node".to_string(), pid);
```

#### C. Add Proper Process Monitoring
```rust
#[tauri::command]
async fn get_node_status(state: State<'_, AppState>) -> Result<NodeStatus, String> {
    let processes = state.processes.lock().unwrap();

    if let Some(&pid) = processes.get("node") {
        // Check if process is actually running
        #[cfg(unix)]
        let running = std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        Ok(NodeStatus {
            running,
            pid: Some(pid),
            // ...
        })
    } else {
        Ok(NodeStatus::default())
    }
}
```

#### D. Add Process Cleanup on App Exit
```rust
fn cleanup_processes(state: &AppState) {
    let processes = state.processes.lock().unwrap();

    for (name, &pid) in processes.iter() {
        println!("Stopping {} (PID {})", name, pid);

        #[cfg(unix)]
        {
            let _ = std::process::Command::new("kill")
                .args(["-TERM", &pid.to_string()])
                .status();
        }
    }
}
```

---

### Fix #4: Frontend State Persistence

**Problem**: Page navigation may reset state

**Solution**: Use Tauri's state management instead of frontend state

#### A. Add Backend State Query Command
```rust
#[tauri::command]
async fn get_app_state(state: State<'_, AppState>) -> Result<AppStateSnapshot, String> {
    Ok(AppStateSnapshot {
        node_running: state.processes.lock().unwrap().contains_key("node"),
        mining_running: state.processes.lock().unwrap().contains_key("miner"),
        wallet_count: state.wallet_manager.lock().await.wallet_count(),
        // ...
    })
}
```

#### B. Query State on Page Load (All HTML Pages)
```javascript
async function initializePage() {
    try {
        const appState = await window.__TAURI__.invoke('get_app_state');

        // Update UI based on actual backend state
        if (appState.node_running) {
            document.getElementById('node-status').textContent = 'Running';
            document.getElementById('start-node-btn').disabled = true;
            document.getElementById('stop-node-btn').disabled = false;
        }

        // ...
    } catch (error) {
        console.error('Failed to load app state:', error);
    }
}

// Call on every page load
window.addEventListener('DOMContentLoaded', initializePage);
```

---

## Recommended Implementation Plan

### Phase 1: Immediate Fix (Wallet-Only Mode) ⭐ RECOMMENDED

**Time**: 30 minutes
**Risk**: Low

1. ✅ Update UI to indicate "Wallet-Only Mode"
2. ✅ Disable "Start Node" button with tooltip
3. ✅ Make start_node() return informational message
4. ✅ Document requirement for external node
5. ✅ Test wallet operations with testnet node

**Result**: Desktop app functions as wallet-only, connecting to testnet RPC at 18350

---

### Phase 2: Proper Local Node Support (Future Enhancement)

**Time**: 2-3 hours
**Risk**: Medium

1. Change RPC port to 18360
2. Implement process detachment (Fix #3)
3. Add proper process monitoring
4. Add backend state persistence (Fix #4)
5. Comprehensive testing

**Result**: Desktop app can run its own node on port 18360

---

## Testing Checklist

### Wallet-Only Mode (Phase 1)
- [ ] Start desktop app
- [ ] Verify "Start Node" button is disabled/hidden
- [ ] Create wallet successfully
- [ ] Check wallet balance (via testnet RPC)
- [ ] Send transaction (via testnet RPC)
- [ ] Navigate between all pages
- [ ] Verify no node processes spawned by desktop app
- [ ] Verify testnet node (PID 53442) still running

### Local Node Mode (Phase 2)
- [ ] Start desktop app
- [ ] Click "Start Node"
- [ ] Verify node starts on port 18360
- [ ] Navigate between all pages
- [ ] Verify node process persists (check PID)
- [ ] Stop node via UI
- [ ] Verify process terminates cleanly
- [ ] Restart node
- [ ] Close app
- [ ] Verify node stops on app exit

---

## Files to Modify

### Phase 1 (Wallet-Only):
1. `ui/node.html` - Update UI to show wallet-only mode
2. `ui/index.html` - Update dashboard to clarify external node
3. `src-tauri/src/main.rs:417` - Make start_node() return info message
4. `btpc-desktop-app/QUICKSTART.md` - Document external node requirement

### Phase 2 (Local Node):
1. `src-tauri/src/main.rs:166` - Change RPC port to 18360
2. `src-tauri/src/main.rs:150` - Enable RPC for node config
3. `src-tauri/src/main.rs:417-490` - Implement process detachment
4. `src-tauri/src/main.rs` - Add get_app_state command
5. `ui/*.html` - Add state persistence on page load

---

## Decision Required

**Which approach should we implement?**

### Option A: Wallet-Only Mode (Recommended) ✅
- Quick fix (30 min)
- Low risk
- Matches industry standard (Electrum, Exodus)
- User runs separate node

### Option B: Embedded Node Mode
- Complex (2-3 hours)
- Medium risk
- Better UX (all-in-one)
- Requires process management fixes

**Recommendation**: **Start with Option A** (wallet-only) to unblock testing immediately, then implement Option B as enhancement later.

---

## Success Criteria

✅ Node process does NOT stop when navigating pages
✅ Wallet operations work across all pages
✅ Mining operations persist across navigation
✅ No port conflicts between desktop app and testnet
✅ Process cleanup works correctly on app exit
✅ User can test full workflow: create wallet → mine → send transaction

---

**Next Step**: Approve approach and begin implementation