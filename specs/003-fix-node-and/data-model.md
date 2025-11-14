# Data Model: Node and Backend Stability Fixes

**Feature**: 003-fix-node-and
**Date**: 2025-10-25
**Status**: Design Complete

This document defines the data structures and their relationships for the stability fixes.

---

## Entity Diagram

```
┌─────────────────┐
│  ErrorState     │
├─────────────────┤
│ error_type      │
│ user_message    │
│ technical_det.. │
│ timestamp       │
│ affected_comp.. │
│ crash_count     │
└─────────────────┘
        │
        │ emitted via
        ▼
┌─────────────────┐      manages     ┌─────────────────┐
│ StateManager<T> │◄─────────────────│   AppState      │
├─────────────────┤                  ├─────────────────┤
│ inner: Arc<..>  │                  │ node_status     │
│ app_handle      │                  │ mining_status   │
│ event_name      │                  │ wallet_balance  │
└─────────────────┘                  └─────────────────┘
        │
        │ auto-emits
        ▼
┌─────────────────────────────────────┐
│         Tauri Events                │
├─────────────────────────────────────┤
│ • error_occurred                    │
│ • process_status_changed            │
│ • balance_updated                   │
│ • node_status_changed               │
│ • mining_status_changed             │
└─────────────────────────────────────┘
        │
        │ listened by
        ▼
┌─────────────────┐
│   Frontend UI   │
├─────────────────┤
│ password-modal  │
│ mining.html     │
│ settings.html   │
│ wallet-manager  │
└─────────────────┘

┌─────────────────┐
│ ProcessHandle   │
├─────────────────┤
│ process_id      │
│ process_type    │──┐
│ status          │  │ enum: Node, Miner, WalletCLI
│ crash_count     │  │
│ start_time      │  │
│ last_health_..  │  │
│ command_line    │  │
│ stdout_buffer   │  │
│ stderr_buffer   │  │
└─────────────────┘  │
        │            │
        │ monitors   │
        ▼            │
┌─────────────────┐  │
│ Process Health  │  │
│   Monitor       │  │
├─────────────────┤  │
│ health_check()  │  │
│ track_crashes() │  │
│ auto_restart()  │  │
└─────────────────┘  │
                     │
┌────────────────────┘
│
▼
┌─────────────────┐
│   Child Process │
├─────────────────┤
│ btpc_node       │
│ btpc_miner      │
│ btpc_wallet     │
└─────────────────┘
```

---

## Entity Definitions

### 1. ErrorState

**Purpose**: Represents an application error with progressive disclosure for UI display.

**Schema Reference**: `contracts/error_types.json`

**Rust Definition**:
```rust
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorState {
    pub error_type: ErrorType,
    pub user_message: String,
    pub technical_details: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub affected_component: Component,
    pub crash_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ErrorType {
    ProcessCrash,
    ValidationFailure,
    DatabaseLock,
    RPCTimeout,
    InsufficientFunds,
    MutexPoison,
    NetworkError,
    FileSystemError,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Component {
    Node,
    Miner,
    Wallet,
    BalanceCalc,
    RpcClient,
    ProcessManager,
}
```

**Validation Rules**:
- `user_message`: 1-200 characters, no sensitive data (regex filter for private keys/passwords)
- `technical_details`: Optional, sanitized (redact patterns like `priv_key:`, `password:`)
- `crash_count`: 0-100 (reasonable upper bound)

**State Transitions**:
1. **Created** → Error occurs in backend
2. **Emitted** → Backend emits `error_occurred` event
3. **Displayed** → Frontend shows user message
4. **Expanded** (optional) → User clicks "Show Details"
5. **Dismissed** → User closes error message

**Relationships**:
- Created by any backend module experiencing an error
- Emitted via Tauri event system to all frontend pages
- Consumed by `btpc-error-handler.js` for UI display

---

### 2. ProcessHandle

**Purpose**: Manages lifecycle of child processes (node, miner, wallet CLI).

**Schema Reference**: `contracts/process_lifecycle.json`

**Rust Definition**:
```rust
use tokio::process::Child;
use chrono::{DateTime, Utc};

pub struct ProcessHandle {
    pub child: Option<Child>,
    pub process_id: u32,
    pub process_type: ProcessType,
    pub status: ProcessStatus,
    pub crash_count: u32,
    pub start_time: DateTime<Utc>,
    pub last_health_check: DateTime<Utc>,
    pub command_line: Vec<String>,
    pub stdout_buffer: BoundedVec<String>,  // max 1000 lines
    pub stderr_buffer: BoundedVec<String>,  // max 1000 lines
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessType {
    Node,
    Miner,
    WalletCLI,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessStatus {
    Running,
    Crashed,
    Stopped,
}

// Bounded vector for log buffering (prevents memory leaks)
pub struct BoundedVec<T> {
    inner: Vec<T>,
    capacity: usize,
}

impl<T> BoundedVec<T> {
    pub fn push(&mut self, value: T) {
        if self.inner.len() >= self.capacity {
            self.inner.remove(0);  // Remove oldest
        }
        self.inner.push(value);
    }
}
```

**Validation Rules**:
- `crash_count`: Increments on crash, resets after 1 hour (3600s) of Running status
- `last_health_check`: Updated every 5 seconds (FR-038)
- `stdout_buffer`/`stderr_buffer`: Max 1000 lines (prevents unbounded memory growth)

**State Transitions**:
```
┌─────────┐  crash (count=0)   ┌─────────┐
│ Running │──────────────────►  │ Crashed │
└─────────┘                     └─────────┘
     ▲                               │
     │                               │ auto-restart
     │                               │ (first crash only)
     │                               ▼
     │                          ┌─────────┐
     │                          │ Running │
     │                          └─────────┘
     │                               │
     │                               │ crash (count>0)
     │                               ▼
     │                          ┌─────────┐
     │                          │ Crashed │
     │                          └─────────┘
     │                               │
     │                               │ user cancels
     │                               ▼
     │  user starts             ┌─────────┐
     └──────────────────────────┤ Stopped │
                                └─────────┘
```

**Cleanup Sequence**:
1. Send SIGTERM to process
2. Wait up to 10 seconds for graceful shutdown
3. If timeout, process already dropped → tokio runtime reaps zombie
4. No manual `waitpid()` required (handled by `tokio::process::Child`)

**Relationships**:
- Managed by `ProcessManager` (process_manager.rs)
- Health monitored by `ProcessHealthMonitor` (process_health.rs)
- Status changes emit `process_status_changed` events

---

### 3. StateManager<T>

**Purpose**: Generic wrapper providing automatic event emission on state changes (Article XI compliance).

**Rust Definition**:
```rust
use std::sync::{Arc, Mutex};
use tauri::Manager;
use serde::Serialize;

pub struct StateManager<T> {
    inner: Arc<Mutex<T>>,
    app_handle: tauri::AppHandle,
    event_name: &'static str,
}

impl<T: Clone + Serialize> StateManager<T> {
    pub fn new(value: T, app_handle: tauri::AppHandle, event_name: &'static str) -> Self {
        Self {
            inner: Arc::new(Mutex::new(value)),
            app_handle,
            event_name,
        }
    }

    /// Update state and automatically emit event
    pub fn update<F, R>(&self, f: F) -> Result<R, AppError>
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut guard = self.inner.lock()
            .map_err(|_| AppError::MutexPoison(self.event_name.to_string()))?;

        let result = f(&mut *guard);

        // Snapshot state before releasing lock
        let state_snapshot = guard.clone();
        drop(guard);

        // Emit event (Article XI Section 11.3)
        self.app_handle.emit_all(self.event_name, state_snapshot)
            .map_err(|e| AppError::EventEmission(e.to_string()))?;

        Ok(result)
    }

    /// Read-only access to state
    pub fn get(&self) -> Result<T, AppError> {
        self.inner.lock()
            .map(|guard| guard.clone())
            .map_err(|_| AppError::MutexPoison(self.event_name.to_string()))
    }
}
```

**Key Features**:
- **Automatic Events**: Every `.update()` call emits corresponding Tauri event
- **Error Propagation**: Mutex poison converts to `AppError` instead of panicking
- **Type-Safe**: Compiler ensures `T` is `Clone + Serialize`
- **Article XI Compliant**: Backend-first updates, event-driven frontend sync

**Usage Example**:
```rust
// In AppState initialization
let node_status = StateManager::new(
    NodeStatus::default(),
    app_handle.clone(),
    "node_status_changed"
);

// In Tauri command
#[tauri::command]
async fn start_node(state: State<'_, AppState>) -> Result<(), String> {
    state.node_status.update(|status| {
        status.running = true;
        status.start_time = Some(Utc::now());
    }).map_err(|e| e.to_string())?;

    // "node_status_changed" event automatically emitted
    Ok(())
}
```

---

### 4. NodeStatus (Modified)

**Purpose**: Represents current state of blockchain node (existing entity with additions).

**Modifications**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStatus {
    // Existing fields
    pub running: bool,
    pub blockchain_height: u64,
    pub peer_count: u32,
    pub sync_progress: f64,  // 0.0 to 1.0
    pub network_type: NetworkType,

    // NEW: Auto-start preference (FR-041, FR-042)
    pub auto_start_preference: Option<bool>,

    // NEW: Crash tracking (FR-046)
    pub last_crash_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkType {
    Mainnet,
    Testnet,
    Regtest,
}
```

**Validation Rules**:
- `sync_progress`: 0.0 ≤ value ≤ 1.0
- `auto_start_preference`: `None` until user makes first-launch choice
- `last_crash_time`: Updated when `ProcessStatus::Crashed` occurs

**Persistence**:
- `auto_start_preference` saved to `~/.btpc/node_config.json`
- Loaded on app startup, applied to node auto-start logic

---

### 5. MiningStatus (Modified)

**Purpose**: Represents current state of mining operation (existing entity with additions).

**Modifications**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningStatus {
    // Existing fields
    pub active: bool,
    pub hashrate: f64,
    pub blocks_found: u64,
    pub thread_count: u32,

    // NEW: Resume detection (FR-043, FR-044)
    pub was_active_on_close: bool,

    // NEW: Last stop time (for resume notification)
    pub last_stop_time: Option<DateTime<Utc>>,
}
```

**Validation Rules**:
- `hashrate`: >= 0.0 (H/s)
- `thread_count`: >= 1
- `was_active_on_close`: Set to `true` when app closes with mining active

**Persistence**:
- `was_active_on_close` saved to `~/.btpc/mining_state.json`
- Loaded on app startup, triggers resume notification if `true`

---

### 6. WalletBalance (Existing, Bug Fix)

**Purpose**: Represents user's spendable funds calculated from UTXOs.

**Current Bug**: Address case sensitivity in HashMap lookup causes balance to show 0.00

**Fix Strategy** (from research.md):
```rust
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;

fn normalize_address(addr: &str) -> String {
    addr.to_lowercase()
}

#[derive(Deserialize)]
pub struct WalletUtxos {
    #[serde(deserialize_with = "deserialize_utxo_map")]
    pub utxos: HashMap<String, UtxoDetails>,
}

fn deserialize_utxo_map<'de, D>(deserializer: D) -> Result<HashMap<String, UtxoDetails>, D::Error>
where
    D: Deserializer<'de>,
{
    let map: HashMap<String, UtxoDetails> = HashMap::deserialize(deserializer)?;
    Ok(map.into_iter()
        .map(|(k, v)| (normalize_address(&k), v))
        .collect())
}

#[derive(Deserialize)]
pub struct UtxoDetails {
    pub amount: f64,
    pub confirmations: u32,
}
```

**Validation Rules**:
- Balance must match sum of UTXOs (no floating point drift)
- Address normalization applied consistently at JSON boundary

**Testing**:
- Regression test: Wallet with 7 UTXOs (226.625 BTP) displays correct balance
- Edge case: Mixed-case addresses in RPC response normalize correctly

---

## Data Flow Diagrams

### Error Handling Flow
```
Backend Error
     │
     ▼
AppError::variant(...)
     │
     ▼
ErrorState { user_msg, tech_details, ... }
     │
     ▼
emit_all("error_occurred", ErrorState)
     │
     ▼
Frontend: btpc-error-handler.js
     │
     ▼
Display: <details> progressive disclosure
     │
     ├─► User Message (always visible)
     └─► Technical Details (click to show)
```

### State Update Flow (Article XI)
```
User Action (e.g., click "Start Node")
     │
     ▼
Frontend: Tauri invoke("start_node")
     │
     ▼
Backend: start_node() command
     │
     ├─► StateManager<NodeStatus>.update(|status| ...)
     │        │
     │        ├─► Mutex lock
     │        ├─► Modify state
     │        ├─► Clone state snapshot
     │        ├─► Unlock mutex
     │        └─► emit_all("node_status_changed", snapshot)
     │
     ▼
Frontend: listen("node_status_changed")
     │
     ├─► Update index.html (dashboard)
     ├─► Update node.html
     └─► Update settings.html
```

### Process Lifecycle Flow
```
Start Process
     │
     ▼
ProcessHandle::new(ProcessType::Node)
     │
     ├─► tokio::process::Command::spawn()
     ├─► Store PID, start_time
     └─► Status = Running
     │
     ▼
Health Monitor (every 5s)
     │
     ├─► Process still alive? YES ──┐
     │                                │
     └─► Process died? YES            │
              │                       │
              ▼                       │
         Status = Crashed             │
              │                       │
              ├─► crash_count == 0?   │
              │        │              │
              │        YES             │
              │        │              │
              │        ▼              │
              │   auto_restart()     │
              │        │              │
              │        ▼              │
              │   Status = Running ◄─┘
              │
              └─► crash_count > 0?
                       │
                       YES
                       │
                       ▼
                  emit("process_status_changed")
                       │
                       ▼
                  Frontend shows "Restart?" notification
```

---

## Persistence Strategy

| Entity | Storage Location | Format | When Saved | When Loaded |
|--------|-----------------|--------|------------|-------------|
| `NodeStatus.auto_start_preference` | `~/.btpc/node_config.json` | JSON | On first-launch choice | App startup |
| `MiningStatus.was_active_on_close` | `~/.btpc/mining_state.json` | JSON | App close | App startup |
| `ProcessHandle.crash_count` | In-memory only (resets on app restart) | N/A | N/A | N/A |
| `ErrorState` | Not persisted (ephemeral) | N/A | N/A | N/A |

**Rationale**:
- Preferences persist across app restarts (user expectations)
- Crash counts reset on app restart (fresh start principle)
- Errors are ephemeral (not logged to disk, only displayed)

---

## Testing Requirements

### Unit Tests
1. `ErrorState` serialization matches `error_types.json` schema
2. `ErrorState` sanitization redacts sensitive patterns
3. `ProcessHandle` state transitions follow lifecycle rules
4. `StateManager<T>` emits events on every `.update()` call
5. `WalletBalance` address normalization handles mixed case

### Integration Tests
1. Balance calculation displays 226.625 BTP for 7-UTXO wallet
2. Node crash triggers auto-restart (first crash only)
3. Mining resume notification appears when `was_active_on_close == true`
4. Error progressive disclosure shows user message + details
5. Event listeners clean up on page unload (no memory leaks)

### Contract Tests
- JSON schemas validate against Rust type serialization
- Tauri event payloads match schema definitions

---

**Next Phase**: Phase 2 - Task Planning (tasks.md generation via /tasks command)
