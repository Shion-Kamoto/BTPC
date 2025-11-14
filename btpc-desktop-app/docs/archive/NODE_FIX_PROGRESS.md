# Node Persistence Fix - Implementation Progress

**Date**: 2025-10-06
**Status**: ✅ Phase 1 Complete | ⏳ Phase 2 In Progress

---

## ✅ COMPLETED (3/12)

### 1. ✅ Created process_manager.rs module
**File**: `src-tauri/src/process_manager.rs`
- Detached process spawning using `setsid()` on Unix
- Process health checking via `kill -0`
- Graceful shutdown (SIGTERM) and force kill (SIGKILL)
- Process tracking with PID instead of Child handles

### 2. ✅ Updated port configuration (18360 RPC)
**File**: `src-tauri/src/main.rs:168`
```rust
port: 18360,  // Desktop node RPC (isolated from testnet on 18350)
```

### 3. ✅ Enabled RPC for desktop node
**File**: `src-tauri/src/main.rs:152`
```rust
enable_rpc: true,  // ✅ FIXED: Enable RPC for desktop node
```

### 4. ✅ Replaced AppState.processes field
**File**: `src-tauri/src/main.rs:247`
```rust
process_manager: Arc<process_manager::ProcessManager>,  // NEW
```

### 5. ✅ Updated AppState::new() initialization
**File**: `src-tauri/src/main.rs:292`
```rust
process_manager: Arc::new(process_manager::ProcessManager::new(false)),
```

---

## ⏳ REMAINING CHANGES (7/12)

All changes below need to be made in `src-tauri/src/main.rs`:

### 6. ⏳ Rewrite start_node() (Lines 418-516)

**Current**: Uses `Command::spawn()` and stores `Child` handle
**Need**: Use `ProcessManager.start_detached()`

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

    // Write initial log message
    let initial_log_message = format!(
        "Node started successfully at {} (PID: {})\nListening and synchronizing blockchain data...\nNetwork: {}\nData directory: {}\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        process_info.pid,
        state.config.network,
        data_dir.display()
    );
    let _ = fs::write(&log_file, initial_log_message);

    // Auto-start blockchain sync if RPC enabled
    if state.config.node.enable_rpc {
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        let mut sync_service_guard = state.sync_service.lock().unwrap();
        if sync_service_guard.is_none() {
            let sync_config = SyncConfig {
                rpc_host: state.config.rpc.host.clone(),
                rpc_port: state.config.rpc.port,
                poll_interval_secs: 10,
                max_blocks_per_sync: 100,
            };

            let service = BlockchainSyncService::new(state.utxo_manager.clone(), sync_config);
            match service.start() {
                Ok(_) => {
                    *sync_service_guard = Some(service);
                    println!("🔄 Blockchain sync service auto-started");
                }
                Err(e) => {
                    eprintln!("⚠️ Failed to auto-start blockchain sync: {}", e);
                }
            }
        }
    }

    Ok(format!("Node started successfully (PID: {})", process_info.pid))
}
```

### 7. ⏳ Rewrite stop_node() (Lines 518-563)

```rust
#[tauri::command]
async fn stop_node(state: State<'_, AppState>) -> Result<String, String> {
    state.process_manager.stop("node")?;

    // Stop blockchain sync service if running
    {
        let mut sync_service_guard = state.sync_service.lock().unwrap();
        if let Some(service) = sync_service_guard.as_ref() {
            service.stop();
            *sync_service_guard = None;
            println!("🛑 Blockchain sync service stopped");
        }
    }

    // Update status
    {
        let mut status = state.status.write().await;
        status.node_status = "Stopped".to_string();
        status.node_pid = None;
    }

    // Append stop message to log file
    let log_file = state.config.log_dir.join("node.log");
    let stop_message = format!("Node stopped at {} by user request\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"));
    let _ = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
        .and_then(|mut f| {
            use std::io::Write;
            f.write_all(stop_message.as_bytes())
        });

    Ok("Node stopped successfully".to_string())
}
```

### 8. ⏳ Rewrite get_node_status() (Lines 565-596)

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

### 9. ⏳ Update get_system_status() (Lines 356-390)

```rust
#[tauri::command]
async fn get_system_status(state: State<'_, AppState>) -> Result<SystemStatus, String> {
    let mut status = {
        let status_guard = state.status.read().await;
        status_guard.clone()
    };

    // Check node status using ProcessManager
    if state.process_manager.is_running("node") {
        status.node_status = "Running".to_string();
        if let Some(info) = state.process_manager.get_info("node") {
            status.node_pid = Some(info.pid);
        }
    } else {
        status.node_status = "Stopped".to_string();
        status.node_pid = None;
    }

    // Check mining status using ProcessManager
    if state.process_manager.is_running("mining") {
        if status.mining_status == "Stopped" {
            status.mining_status = "Running".to_string();
        }
    } else {
        status.mining_status = "Stopped".to_string();
    }

    // Update installation status
    let installation_status = state.btpc.check_installation();
    status.binaries_installed = installation_status.is_complete;

    // Update logs info
    status.logs_available = get_log_info(&state.config.log_dir);

    Ok(status)
}
```

### 10. ⏳ Update get_mining_status() (Lines 598-609)

```rust
#[tauri::command]
async fn get_mining_status(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let is_mining = state.process_manager.is_running("mining");

    Ok(serde_json::json!({
        "is_mining": is_mining,
        "hashrate": 0,
        "blocks_found": 0
    }))
}
```

### 11. ⏳ Rewrite start_mining() (Lines 905-1030)

**Note**: Mining uses piped stdout/stderr for live log streaming.
**Challenge**: ProcessManager uses null/file redirection, not pipes.
**Solution**: Keep current implementation OR refactor to write to file and tail it.

**For now**: Keep as-is since mining needs live output streaming to UI

### 12. ⏳ Rewrite stop_mining() (Lines 1163-1190)

```rust
#[tauri::command]
async fn stop_mining(state: State<'_, AppState>) -> Result<String, String> {
    state.process_manager.stop("mining")?;

    // Add log entry
    {
        let mut mining_logs = state.mining_logs.lock().unwrap();
        mining_logs.add_entry("INFO".to_string(), "Mining stopped by user".to_string());
    }

    // Update status
    {
        let mut status = state.status.write().await;
        status.mining_status = "Stopped".to_string();
    }

    Ok("Mining stopped successfully".to_string())
}
```

### 13. ⏳ Add cleanup handler in main() (Lines 1826-1908)

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
        .invoke_handler(tauri::generate_handler![/* ... */])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

---

## 📝 NOTES

### Mining Implementation Decision

Mining currently uses `tokio::spawn` with piped stdout/stderr for real-time log streaming to the UI. This is incompatible with ProcessManager's detached approach.

**Options**:
1. **Keep current**: Leave mining as-is (still uses `Child` handles)
2. **Refactor**: Write logs to file and tail them for UI updates
3. **Hybrid**: Use ProcessManager but add pipe support

**Recommendation**: Option 1 (keep current) - mining is short-lived and controlled by user.

### Testing Checklist

After implementation:
- [ ] Compile check: `cargo build --release`
- [ ] Start node → Navigate pages → Node persists
- [ ] Check `ps aux | grep btpc_node` shows PID
- [ ] Stop node → Process terminates
- [ ] Close app → Cleanup runs
- [ ] Start mining → Navigate → Check behavior

---

## Time Estimate

- Remaining changes: **45-60 minutes**
- Testing: **15-20 minutes**
- Total: **~1 hour**