# Node Persistence Fix - Implementation Status

**Date**: 2025-10-06
**Issue**: Node stops when navigating pages in desktop app
**Fix Type**: Real permanent solution (process manager architecture)

---

## What's Been Done ✅

### 1. Created Process Manager Module
**File**: `src-tauri/src/process_manager.rs` ✅ COMPLETE
- Detached process spawning using `setsid()` on Unix
- Process health checking via `kill -0`
- Graceful shutdown (SIGTERM) and force kill (SIGKILL)
- Process tracking with PID instead of Child handles
- Cross-platform support (Unix + Windows)

### 2. Updated Module Imports
**File**: `src-tauri/src/main.rs:63` ✅ COMPLETE
- Added `mod process_manager;` import

---

## What Still Needs to Be Done ⏳

### Critical Fixes Remaining (Required):

#### 1. Change RPC Port Configuration
**File**: `src-tauri/src/main.rs:167`
```rust
// CURRENT (WRONG - conflicts with testnet):
port: 18350,  // Connect to testnet node RPC

// CHANGE TO:
port: 18360,  // Desktop node RPC (isolated from testnet)
```

#### 2. Enable RPC for Desktop Node
**File**: `src-tauri/src/main.rs:152`
```rust
// CURRENT:
enable_rpc: false,  // Desktop app = wallet only

// CHANGE TO:
enable_rpc: true,  // Enable RPC for desktop node
```

#### 3. Replace AppState.processes with ProcessManager
**File**: `src-tauri/src/main.rs:245-247`
```rust
// CURRENT:
pub struct AppState {
    processes: Arc<Mutex<HashMap<String, Child>>>,  // OLD
    // ...
}

// CHANGE TO:
pub struct AppState {
    process_manager: Arc<ProcessManager>,  // NEW
    // ...
}
```

#### 4. Update AppState::new() to Initialize ProcessManager
**File**: `src-tauri/src/main.rs:290-300`
```rust
// ADD after line 300:
use process_manager::ProcessManager;

let app_state = Self {
    config,
    process_manager: Arc::new(ProcessManager::new(false)),  // ADD THIS
    status: Arc::new(RwLock::new(status)),
    // ... rest stays same
};
```

#### 5. Rewrite start_node() to Use ProcessManager
**File**: `src-tauri/src/main.rs:418-515`

Replace entire function with:
```rust
#[tauri::command]
async fn start_node(state: State<'_, AppState>) -> Result<String, String> {
    // Check if already running
    if state.process_manager.is_running("node") {
        return Ok("Node is already running".to_string());
    }

    let bin_path = state.config.btpc_home.join("bin").join("btpc_node");
    if !bin_path.exists() {
        return Err("Node binary not found. Please run setup first.".to_string());
    }

    let data_dir = state.config.data_dir.join("desktop-node");
    let log_file = state.config.log_dir.join("node.log");
    let err_file = state.config.log_dir.join("node.err");

    fs::create_dir_all(&data_dir).map_err(|e| format!("Failed to create data dir: {}", e))?;
    fs::create_dir_all(&state.config.log_dir).map_err(|e| format!("Failed to create log dir: {}", e))?;

    let listen_addr = format!("127.0.0.1:{}", state.config.node.listen_port);
    let args = vec![
        "--network".to_string(),
        state.config.network.to_string(),
        "--datadir".to_string(),
        data_dir.to_string_lossy().to_string(),
        "--rpcport".to_string(),
        "18360".to_string(),  // FIXED PORT
        "--rpcbind".to_string(),
        "127.0.0.1".to_string(),
        "--listen".to_string(),
        listen_addr,
    ];

    // Use ProcessManager for detached process
    let process_info = state.process_manager.start_detached(
        "node".to_string(),
        bin_path.to_string_lossy().to_string(),
        args,
        Some(log_file.clone()),
        Some(err_file),
    )?;

    // Update status
    {
        let mut status = state.status.write().await;
        status.node_status = "Running".to_string();
        status.node_pid = Some(process_info.pid);
    }

    Ok(format!("Node started successfully (PID: {})", process_info.pid))
}
```

#### 6. Rewrite stop_node() to Use ProcessManager
**File**: `src-tauri/src/main.rs:518-562`

Replace with:
```rust
#[tauri::command]
async fn stop_node(state: State<'_, AppState>) -> Result<String, String> {
    state.process_manager.stop("node")?;

    // Update status
    {
        let mut status = state.status.write().await;
        status.node_status = "Stopped".to_string();
        status.node_pid = None;
    }

    Ok("Node stopped successfully".to_string())
}
```

#### 7. Rewrite get_node_status() to Use ProcessManager
**File**: `src-tauri/src/main.rs:565-595`

Replace with:
```rust
#[tauri::command]
async fn get_node_status(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let running = state.process_manager.is_running("node");

    let (pid, status_str) = if running {
        let info = state.process_manager.get_info("node");
        (info.map(|i| i.pid), "running")
    } else {
        (None, "stopped")
    };

    Ok(serde_json::json!({
        "running": running,
        "status": status_str,
        "pid": pid
    }))
}
```

#### 8. Update get_system_status() to Use ProcessManager
**File**: `src-tauri/src/main.rs:362-370`

Replace with:
```rust
// Check node status
if state.process_manager.is_running("node") {
    status.node_status = "Running".to_string();
    if let Some(info) = state.process_manager.get_info("node") {
        status.node_pid = Some(info.pid);
    }
} else {
    status.node_status = "Stopped".to_string();
    status.node_pid = None;
}
```

#### 9. Apply Same Pattern to Mining (start_mining, stop_mining)
Use ProcessManager instead of Child handles

#### 10. Add Cleanup Handler in main()
**File**: `src-tauri/src/main.rs:1826`

Add after line 1830:
```rust
fn main() {
    let app_state = AppState::new().expect("Failed to initialize app state");
    let process_manager = app_state.process_manager.clone();

    tauri::Builder::default()
        .manage(app_state)
        .setup(|app| {
            // Cleanup on window close
            let window = app.get_window("main").unwrap();
            let pm = process_manager.clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { .. } = event {
                    pm.stop_all();
                }
            });
            Ok(())
        })
        .invoke_handler(/* ... */)
        // ...
}
```

---

## Testing Checklist (After Implementation)

- [ ] Build compiles: `cargo build --release`
- [ ] Start node → Navigate pages → Node still running
- [ ] Check `ps aux | grep btpc_node` shows desktop node on port 18360
- [ ] Stop node → Process terminates
- [ ] Close app → All processes terminate
- [ ] Start mining → Navigate → Mining persists

---

## Files Modified

1. ✅ `/home/bob/BTPC/BTPC/btpc-desktop-app/src-tauri/src/process_manager.rs` (NEW)
2. ⏳ `/home/bob/BTPC/BTPC/btpc-desktop-app/src-tauri/src/main.rs` (PARTIAL - needs completion)

---

## Estimated Time Remaining

**1.5 hours** to complete all remaining changes + testing

---

## Next Steps

1. Complete the 10 remaining changes listed above
2. Build and test
3. Verify node persists across page navigation
4. Document final status

**Note**: This is the REAL FIX - not a workaround. When complete, the desktop app will have production-ready process management that survives page navigation.