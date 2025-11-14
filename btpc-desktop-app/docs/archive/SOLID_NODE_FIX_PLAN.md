# Desktop App Node Persistence - SOLID PERMANENT FIX

**Goal**: Make the desktop app node ACTUALLY work correctly and persist across page navigation
**Type**: Permanent production-ready solution
**Time Estimate**: 2-3 hours
**Date**: 2025-10-06

---

## Problem Statement

The desktop app needs to run an embedded `btpc_node` process that:
1. ✅ Starts when user clicks "Start Node"
2. ✅ Persists across ALL page navigation
3. ✅ Can be stopped cleanly
4. ✅ Doesn't conflict with other nodes
5. ✅ Restarts if crashed
6. ✅ Cleans up on app exit

**Current State**: ❌ Node stops when navigating pages (CRITICAL BUG)

---

## Root Cause Analysis

### Issue #1: Port Conflicts
**File**: `src-tauri/src/main.rs:166`
```rust
rpc: RpcConfig {
    port: 18350,  // ❌ CONFLICT: Same as testnet node!
}
```

**Impact**: Desktop node can't bind to port 18350 (testnet owns it)

### Issue #2: Process Handle Dropped
**File**: `src-tauri/src/main.rs:462-464`
```rust
let mut processes = state.processes.lock().unwrap();
processes.insert("node".to_string(), child);  // ❌ Child handle stored
```

**Impact**: When `Child` is dropped (garbage collection), process may terminate

### Issue #3: No Process Detachment
**Current**: Process is child of Tauri app
**Problem**: If parent (Tauri) loses reference, child may die

### Issue #4: No Health Monitoring
**Current**: No way to detect if node crashed
**Problem**: UI shows "Running" even if node is dead

---

## Permanent Fix Architecture

### Component 1: Background Process Manager

Create a dedicated background service that manages long-running processes independently of Tauri's lifecycle.

**New File**: `src-tauri/src/process_manager.rs`

```rust
use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub status: ProcessStatus,
    pub restart_count: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ProcessStatus {
    Starting,
    Running,
    Stopped,
    Crashed,
}

pub struct ProcessManager {
    processes: Arc<Mutex<HashMap<String, ProcessInfo>>>,
    auto_restart: bool,
}

impl ProcessManager {
    pub fn new(auto_restart: bool) -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
            auto_restart,
        }
    }

    /// Start a process as detached (survives parent lifecycle)
    pub fn start_detached(
        &self,
        name: String,
        command: String,
        args: Vec<String>,
        stdout_path: Option<PathBuf>,
        stderr_path: Option<PathBuf>,
    ) -> Result<ProcessInfo, String> {
        let mut cmd = Command::new(&command);
        cmd.args(&args);

        // Critical: Detach from parent process
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            unsafe {
                cmd.pre_exec(|| {
                    // Create new session (detach from terminal)
                    libc::setsid();
                    Ok(())
                });
            }
        }

        // Redirect output
        if let Some(stdout) = stdout_path {
            let file = std::fs::File::create(stdout)
                .map_err(|e| format!("Failed to create stdout file: {}", e))?;
            cmd.stdout(file);
        } else {
            cmd.stdout(Stdio::null());
        }

        if let Some(stderr) = stderr_path {
            let file = std::fs::File::create(stderr)
                .map_err(|e| format!("Failed to create stderr file: {}", e))?;
            cmd.stderr(file);
        } else {
            cmd.stderr(Stdio::null());
        }

        cmd.stdin(Stdio::null());

        // Spawn and immediately detach
        let child = cmd.spawn()
            .map_err(|e| format!("Failed to spawn process: {}", e))?;

        let pid = child.id();

        // CRITICAL: Forget the child handle so it's not waited on
        std::mem::forget(child);

        let info = ProcessInfo {
            pid,
            name: name.clone(),
            started_at: chrono::Utc::now(),
            status: ProcessStatus::Running,
            restart_count: 0,
        };

        // Store process info
        {
            let mut processes = self.processes.lock().unwrap();
            processes.insert(name.clone(), info.clone());
        }

        Ok(info)
    }

    /// Check if process is actually running (not just in our map)
    pub fn is_running(&self, name: &str) -> bool {
        let processes = self.processes.lock().unwrap();

        if let Some(info) = processes.get(name) {
            #[cfg(unix)]
            {
                // Use kill -0 to check if process exists (doesn't actually kill)
                std::process::Command::new("kill")
                    .args(["-0", &info.pid.to_string()])
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false)
            }

            #[cfg(windows)]
            {
                // Windows equivalent
                std::process::Command::new("tasklist")
                    .args(["/FI", &format!("PID eq {}", info.pid)])
                    .output()
                    .map(|o| String::from_utf8_lossy(&o.stdout).contains(&info.pid.to_string()))
                    .unwrap_or(false)
            }
        } else {
            false
        }
    }

    /// Stop a process gracefully (SIGTERM)
    pub fn stop(&self, name: &str) -> Result<(), String> {
        let processes = self.processes.lock().unwrap();

        if let Some(info) = processes.get(name) {
            #[cfg(unix)]
            {
                std::process::Command::new("kill")
                    .args(["-TERM", &info.pid.to_string()])
                    .status()
                    .map_err(|e| format!("Failed to stop process: {}", e))?;
            }

            #[cfg(windows)]
            {
                std::process::Command::new("taskkill")
                    .args(["/PID", &info.pid.to_string(), "/T"])
                    .status()
                    .map_err(|e| format!("Failed to stop process: {}", e))?;
            }

            Ok(())
        } else {
            Err(format!("Process '{}' not found", name))
        }
    }

    /// Get process info
    pub fn get_info(&self, name: &str) -> Option<ProcessInfo> {
        let processes = self.processes.lock().unwrap();
        processes.get(name).cloned()
    }

    /// Health check - update status of all processes
    pub fn health_check(&self) {
        let mut processes = self.processes.lock().unwrap();

        for (name, info) in processes.iter_mut() {
            let running = self.check_pid_running(info.pid);

            if !running && matches!(info.status, ProcessStatus::Running) {
                info.status = ProcessStatus::Crashed;
            }
        }
    }

    #[cfg(unix)]
    fn check_pid_running(&self, pid: u32) -> bool {
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    #[cfg(windows)]
    fn check_pid_running(&self, pid: u32) -> bool {
        std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid)])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains(&pid.to_string()))
            .unwrap_or(false)
    }
}
```

---

### Component 2: Updated AppState

**File**: `src-tauri/src/main.rs` (around line 80)

```rust
pub struct AppState {
    wallet_manager: Arc<Mutex<WalletManager>>,
    utxo_manager: Arc<Mutex<UTXOManager>>,
    rpc_client: Arc<RpcClient>,
    btpc_integration: Arc<BtpcIntegration>,
    config: AppConfig,
    status: Arc<RwLock<AppStatus>>,
    process_manager: Arc<ProcessManager>,  // ✅ NEW: Dedicated process manager
    sync_service: Arc<Mutex<BlockchainSyncService>>,
}
```

---

### Component 3: Fixed Port Configuration

**File**: `src-tauri/src/main.rs:164-168`

```rust
rpc: RpcConfig {
    host: "127.0.0.1".to_string(),
    port: 18360,  // ✅ CHANGED: Desktop node uses 18360 (testnet uses 18350)
    enable_cors: true,
},
```

**File**: `src-tauri/src/main.rs:146-150`

```rust
node: NodeConfig {
    sync_interval_secs: 5,
    max_peers: 50,
    listen_port: 18361,     // P2P port for desktop node
    enable_rpc: true,       // ✅ CHANGED: Enable RPC for desktop node
},
```

---

### Component 4: Updated start_node Command

**File**: `src-tauri/src/main.rs:417-490`

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

    // Ensure directories exist
    fs::create_dir_all(&data_dir).map_err(|e| format!("Failed to create data dir: {}", e))?;
    fs::create_dir_all(&state.config.log_dir).map_err(|e| format!("Failed to create log dir: {}", e))?;

    // Prepare arguments
    let listen_addr = format!("127.0.0.1:{}", state.config.node.listen_port);
    let args = vec![
        "--network".to_string(),
        state.config.network.to_string(),
        "--datadir".to_string(),
        data_dir.to_string_lossy().to_string(),
        "--rpcport".to_string(),
        "18360".to_string(),  // ✅ Desktop node RPC port
        "--rpcbind".to_string(),
        "127.0.0.1".to_string(),
        "--listen".to_string(),
        listen_addr,
    ];

    // ✅ Start as detached process using ProcessManager
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
    let initial_log = format!(
        "Node started successfully at {} (PID: {})\nListening on port 18361 (P2P), 18360 (RPC)\nNetwork: {}\nData directory: {}\n",
        process_info.started_at.format("%Y-%m-%d %H:%M:%S UTC"),
        process_info.pid,
        state.config.network,
        data_dir.display()
    );
    let _ = fs::write(&log_file, initial_log);

    Ok(format!("Node started successfully (PID: {})", process_info.pid))
}
```

---

### Component 5: Health Monitoring Service

**New Background Task**: Runs every 5 seconds to check process health

```rust
async fn start_health_monitor(state: Arc<AppState>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(5));

        loop {
            interval.tick().await;

            // Check node process
            if let Some(info) = state.process_manager.get_info("node") {
                let running = state.process_manager.is_running("node");

                // Update status
                let mut status = state.status.write().await;
                if running {
                    status.node_status = "Running".to_string();
                    status.node_pid = Some(info.pid);
                } else {
                    status.node_status = "Stopped".to_string();
                    status.node_pid = None;
                }
            }

            // Check miner process
            if let Some(info) = state.process_manager.get_info("miner") {
                let running = state.process_manager.is_running("miner");

                let mut status = state.status.write().await;
                if running {
                    status.mining_status = "Running".to_string();
                } else {
                    status.mining_status = "Stopped".to_string();
                }
            }
        }
    });
}
```

**Call this in main():**

```rust
#[tokio::main]
async fn main() {
    // ... existing setup ...

    let state = Arc::new(AppState {
        // ...
        process_manager: Arc::new(ProcessManager::new(false)),
        // ...
    });

    // ✅ Start health monitoring
    start_health_monitor(state.clone()).await;

    // ... run Tauri app ...
}
```

---

### Component 6: Clean Shutdown

**File**: `src-tauri/src/main.rs` (add cleanup handler)

```rust
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // ... existing setup ...

            // ✅ Add cleanup on app exit
            let state = app.state::<AppState>();
            let process_manager = state.process_manager.clone();

            app.on_window_event(|event| {
                if let tauri::WindowEvent::CloseRequested { .. } = event.event() {
                    // Stop all managed processes
                    let _ = process_manager.stop("node");
                    let _ = process_manager.stop("miner");
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // ... existing commands ...
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

---

### Component 7: Frontend State Persistence

**All HTML pages** (index.html, wallet-manager.html, mining.html, node.html, etc.)

Add this to **every page**:

```javascript
// State restoration on page load
async function restoreAppState() {
    try {
        // Query actual backend state (not localStorage)
        const nodeStatus = await window.__TAURI__.invoke('get_node_status');
        const miningStatus = await window.__TAURI__.invoke('get_mining_status');

        // Update UI based on real state
        if (nodeStatus.running) {
            document.getElementById('node-status-text').textContent = '🟢 Running';
            document.getElementById('start-node-btn').disabled = true;
            document.getElementById('stop-node-btn').disabled = false;
            document.getElementById('node-pid').textContent = `PID: ${nodeStatus.pid}`;
        } else {
            document.getElementById('node-status-text').textContent = '🔴 Stopped';
            document.getElementById('start-node-btn').disabled = false;
            document.getElementById('stop-node-btn').disabled = true;
        }

        if (miningStatus.running) {
            document.getElementById('mining-status-text').textContent = '⛏️ Mining';
            document.getElementById('start-mining-btn').disabled = true;
            document.getElementById('stop-mining-btn').disabled = false;
        } else {
            document.getElementById('mining-status-text').textContent = '⏸️ Stopped';
            document.getElementById('start-mining-btn').disabled = false;
            document.getElementById('stop-mining-btn').disabled = true;
        }
    } catch (error) {
        console.error('Failed to restore app state:', error);
    }
}

// Run on every page load
window.addEventListener('DOMContentLoaded', restoreAppState);

// Optionally poll every 5 seconds for updates
setInterval(restoreAppState, 5000);
```

---

## Implementation Steps

### Step 1: Create Process Manager (30 min)
- [ ] Create `src-tauri/src/process_manager.rs`
- [ ] Implement detached process spawning
- [ ] Add health check methods
- [ ] Add graceful shutdown

### Step 2: Update Port Configuration (5 min)
- [ ] Change RPC port to 18360 (line 166)
- [ ] Enable RPC for node config (line 150)

### Step 3: Update AppState (10 min)
- [ ] Add ProcessManager to AppState
- [ ] Update initialization

### Step 4: Fix start_node Command (20 min)
- [ ] Use ProcessManager.start_detached()
- [ ] Update port numbers
- [ ] Remove old Child storage

### Step 5: Add Health Monitoring (15 min)
- [ ] Create start_health_monitor() background task
- [ ] Call in main()

### Step 6: Add Cleanup Handler (10 min)
- [ ] Add on_window_event handler
- [ ] Stop processes on app exit

### Step 7: Update Frontend (30 min)
- [ ] Add restoreAppState() to all HTML pages
- [ ] Query backend state on page load
- [ ] Add periodic polling

### Step 8: Apply Same Fix to Mining (20 min)
- [ ] Update start_mining to use ProcessManager
- [ ] Add mining health checks

### Step 9: Testing (30 min)
- [ ] Start node, navigate pages, verify persistence
- [ ] Stop node, verify clean shutdown
- [ ] Start mining, navigate, verify persistence
- [ ] Close app, verify processes stop

**Total Time**: ~2.5 hours

---

## Testing Checklist

### Process Persistence
- [ ] Start node → Navigate to all pages → Node still running
- [ ] Check `ps aux | grep btpc_node` shows desktop node (port 18360)
- [ ] UI shows correct status on all pages

### Port Isolation
- [ ] Desktop node uses port 18360 (RPC) and 18361 (P2P)
- [ ] Testnet node still on port 18350
- [ ] No port conflicts

### Process Lifecycle
- [ ] Start node → Stop node → Process terminates
- [ ] Start node → Close app → Process terminates
- [ ] Node crash → UI shows "Stopped" within 5 seconds

### Multi-Process
- [ ] Start node + mining simultaneously
- [ ] Both persist across navigation
- [ ] Both stop cleanly

---

## Success Criteria

✅ Node starts on port 18360 (not 18350)
✅ Node persists across ALL page navigation
✅ Process runs detached (doesn't die with parent)
✅ Health monitoring updates UI every 5 seconds
✅ Clean shutdown on app exit
✅ Mining works the same way
✅ No zombie processes

---

## This is the REAL PERMANENT FIX

Unlike the "wallet-only workaround", this actually:
- ✅ Fixes the root cause
- ✅ Makes embedded node work correctly
- ✅ Production-ready
- ✅ Professional process management
- ✅ Proper architecture

**Time**: 2.5 hours of focused work
**Result**: Fully working desktop app with embedded node

---

**Ready to implement?**