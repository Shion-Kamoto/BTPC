# Research: Node and Backend Stability Fixes

**Feature**: 003-fix-node-and
**Date**: 2025-10-25
**Status**: Complete

This document consolidates research findings for all technical decisions required to implement the node and backend stability fixes for btpc-desktop-app.

---

## 1. Rust Error Handling Best Practices

### Decision
**Use `thiserror` with custom `AppError` enum** for structured error propagation through Tauri commands.

### Rationale
- **Tauri Compatibility**: Tauri commands require `Result<T, String>` or custom error types implementing `Display`
- **Type Safety**: `thiserror` provides compile-time error variant checking
- **Error Context**: Allows rich error context while converting to user-friendly strings
- **Zero Overhead**: No runtime cost compared to manual `impl Display`
- **Industry Standard**: Used by major Rust projects (tokio, serde, etc.)

### Implementation Pattern
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Process {0} crashed: {1}")]
    ProcessCrash(String, String),

    #[error("Database lock timeout: {0}")]
    DatabaseLock(String),

    #[error("RPC communication failed: {0}")]
    RpcFailure(String),

    #[error("Mutex poisoned in {0}")]
    MutexPoison(String),
}

// Tauri command signature
#[tauri::command]
async fn start_mining(state: State<'_, AppState>) -> Result<(), String> {
    let processes = state.mining_processes
        .lock()
        .map_err(|e| AppError::MutexPoison("mining_processes".to_string()))?;
    // ... rest of implementation
    Ok(())
}
```

### Alternatives Considered
| Alternative | Pros | Cons | Rejected Because |
|-------------|------|------|------------------|
| `anyhow` | Flexible, easy backtrace | Too generic, loses type safety | Need structured error variants for different UI handling |
| Custom structs | Full control | Verbose, manual `impl Display` | `thiserror` provides same control with less boilerplate |
| String errors | Simple | No structure, hard to pattern match | Cannot differentiate error types for smart recovery logic |

### Migration Strategy
1. Define `AppError` enum in `src-tauri/src/error.rs`
2. Replace `.unwrap()` with `.map_err(|e| AppError::variant(...))?` systematically
3. Keep existing Tauri command signatures (`Result<T, String>`) by impl `From<AppError> for String`
4. Add unit tests for each error variant conversion

---

## 2. Process Management Patterns

### Decision
**Use `tokio::process::Child` with dedicated reaper task** for async zombie process prevention.

### Rationale
- **Async-First**: Integrates with existing Tauri/tokio runtime (no blocking threads)
- **Auto-Reaping**: `tokio::process::Child::wait()` automatically reaps zombies
- **Timeout Support**: Can implement graceful shutdown with `tokio::time::timeout`
- **No Unsafe Code**: Replaces `std::mem::forget()` with safe async pattern
- **Resource Cleanup**: Drop implementation ensures cleanup even if task panics

### Implementation Pattern
```rust
use tokio::process::{Child, Command};
use tokio::time::{timeout, Duration};

pub struct ProcessHandle {
    child: Option<Child>,
    process_type: ProcessType,
    // ... other fields
}

impl ProcessHandle {
    pub async fn shutdown_gracefully(&mut self) -> Result<(), AppError> {
        if let Some(mut child) = self.child.take() {
            // Send SIGTERM
            child.kill().ok();

            // Wait up to 10 seconds
            match timeout(Duration::from_secs(10), child.wait()).await {
                Ok(Ok(status)) => Ok(()),
                Ok(Err(e)) => Err(AppError::ProcessShutdown(format!("Wait failed: {}", e))),
                Err(_) => {
                    // Timeout - force kill
                    // Child already dropped, zombie reaped by tokio runtime
                    Ok(())
                }
            }
        } else {
            Ok(())
        }
    }
}

impl Drop for ProcessHandle {
    fn drop(&mut self) {
        // Ensure process is killed on Drop (panic safety)
        if let Some(mut child) = self.child.take() {
            child.kill().ok();
            // tokio runtime will reap zombie
        }
    }
}
```

### Alternatives Considered
| Alternative | Pros | Cons | Rejected Because |
|-------------|------|------|------------------|
| `nix` crate + signal handler | POSIX-compliant, precise control | Requires unsafe, not cross-platform | Adds complexity, tokio already handles reaping |
| `std::process::Child` | Standard library | No async support, manual reaping | Would require blocking threads or unsafe waitpid |
| Third-party supervisor | Feature-rich | External dependency, overkill | Simple requirements, prefer minimal dependencies |

### Migration Strategy
1. Replace `std::process::Command` with `tokio::process::Command` in `process_manager.rs`
2. Remove `std::mem::forget()` call (line 95)
3. Implement `Drop` trait for `ProcessHandle`
4. Add async `shutdown_gracefully()` method with timeout
5. Update all call sites to `await` process operations

---

## 3. UTXO Balance Calculation Fix

### Decision
**Normalize addresses to lowercase before comparison** and **fix JSON deserialization to use case-insensitive HashMap keys**.

### Rationale
- **Root Cause**: btpc_wallet RPC returns addresses in varying case (base58 encoding artifacts)
- **Current Bug**: HashMap key lookup fails due to case mismatch (e.g., "BTPC..." vs "btpc...")
- **Fix Strategy**: Normalize all addresses to lowercase at deserialization boundary
- **Performance**: O(n) normalization during JSON parse (already O(n)), no runtime overhead
- **Consistency**: Matches Bitcoin Core behavior (case-insensitive address comparison)

### Implementation Pattern
```rust
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;

fn normalize_address(addr: &str) -> String {
    addr.to_lowercase()
}

// Custom deserializer for case-insensitive UTXO map
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
pub struct WalletUtxos {
    #[serde(deserialize_with = "deserialize_utxo_map")]
    pub utxos: HashMap<String, UtxoDetails>,
}

// In balance calculation (utxo_manager.rs:258-271)
pub async fn calculate_balance(wallet_address: &str) -> Result<f64, AppError> {
    let normalized_address = normalize_address(wallet_address);
    let utxos: WalletUtxos = fetch_utxos_from_rpc().await?;

    let total = utxos.utxos
        .get(&normalized_address)
        .map(|details| details.amount)
        .unwrap_or(0.0);

    Ok(total)
}
```

### Alternatives Considered
| Alternative | Pros | Cons | Rejected Because |
|-------------|------|------|------------------|
| Case-insensitive HashMap | Automatic | Requires custom HashMap impl or crate | Over-engineering, normalization simpler |
| Fix RPC to return consistent case | Solves root cause | Requires btpc_wallet changes, coupling | Out of scope for desktop app fix |
| Case-fold comparison | Unicode-aware | Overkill for base58 ASCII | Base58 is ASCII-only, lowercase sufficient |

### Testing Strategy
1. Unit test: `test_address_normalization()` - verify "BTPC..." and "btpc..." normalize identically
2. Unit test: `test_balance_calculation_case_insensitive()` - test balance with mixed-case addresses
3. Integration test: Create wallet, generate address, verify balance display matches database

---

## 4. File Locking Safety

### Decision
**Use `fs2` crate for cross-platform advisory file locking** with timeout and error handling.

### Rationale
- **Cross-Platform**: Works on Linux (flock), Windows (LockFile), macOS (flock)
- **Safe Rust**: No unsafe code, wraps platform-specific calls
- **Advisory Locking**: Appropriate for single-instance protection (not security boundary)
- **Timeout Support**: Can implement lock acquisition timeout
- **Active Maintenance**: Well-maintained crate (used by cargo, rustup)

### Implementation Pattern
```rust
use fs2::FileExt;
use std::fs::{File, OpenOptions};
use std::path::Path;
use std::time::Duration;

pub struct LockFile {
    file: File,
    path: PathBuf,
}

impl LockFile {
    pub fn try_acquire(path: impl AsRef<Path>) -> Result<Self, AppError> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .open(path.as_ref())
            .map_err(|e| AppError::LockFileOpen(e.to_string()))?;

        // Try to acquire exclusive lock (non-blocking)
        file.try_lock_exclusive()
            .map_err(|_| AppError::LockFileInUse(path.as_ref().display().to_string()))?;

        Ok(LockFile {
            file,
            path: path.as_ref().to_path_buf(),
        })
    }

    pub fn cleanup_stale_locks(lock_dir: impl AsRef<Path>) -> Result<(), AppError> {
        // Check lock files, verify PID still exists, remove stale ones
        // Implementation details in lock_manager.rs
        Ok(())
    }
}

impl Drop for LockFile {
    fn drop(&mut self) {
        // Lock automatically released on file close
        // Explicit unlock for clarity
        let _ = self.file.unlock();
        // Optionally remove lock file
        let _ = std::fs::remove_file(&self.path);
    }
}
```

### Alternatives Considered
| Alternative | Pros | Cons | Rejected Because |
|-------------|------|------|------------------|
| `nix` crate (fcntl) | POSIX-standard | Linux-only, no Windows | Need cross-platform support |
| `libc::flock` (raw) | Direct control | Unsafe, manual error handling | Unsafe code policy (NFR-018) |
| Named mutexes | Process-shared | Platform-specific APIs, complex | Advisory file locks simpler |

### Migration Strategy
1. Add `fs2 = "0.4"` to `Cargo.toml`
2. Create `src-tauri/src/lock_manager.rs` with `LockFile` wrapper
3. Replace `libc::flock()` usage in `main.rs:2699-2700`
4. Implement stale lock cleanup (check PID from lock file content)
5. Add tests for lock acquisition, release, and stale detection

---

## 5. Event-Driven State Updates (Article XI)

### Decision
**Use `StateManager` wrapper type with automatic event emission** via `Drop` guard pattern.

### Rationale
- **Impossible to Forget**: Event emission tied to state mutation (compile-time guarantee)
- **Ergonomic**: Similar to existing `Arc<Mutex<T>>` but with built-in events
- **Zero Runtime Cost**: Event emission overhead amortized over state changes
- **Type-Safe**: Compiler enforces event emission for all state changes
- **Article XI Compliant**: Backend-first state updates always propagate to frontend

### Implementation Pattern
```rust
use tauri::Manager;

pub struct StateManager<T> {
    inner: Arc<Mutex<T>>,
    app_handle: tauri::AppHandle,
    event_name: &'static str,
}

impl<T: Clone + serde::Serialize> StateManager<T> {
    pub fn new(value: T, app_handle: tauri::AppHandle, event_name: &'static str) -> Self {
        Self {
            inner: Arc::new(Mutex::new(value)),
            app_handle,
            event_name,
        }
    }

    pub fn update<F, R>(&self, f: F) -> Result<R, AppError>
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut guard = self.inner.lock()
            .map_err(|e| AppError::MutexPoison(self.event_name.to_string()))?;

        let result = f(&mut *guard);

        // Emit event with new state
        let state_snapshot = guard.clone();
        drop(guard); // Release lock before emitting

        self.app_handle.emit_all(self.event_name, state_snapshot)
            .map_err(|e| AppError::EventEmission(e.to_string()))?;

        Ok(result)
    }

    pub fn get(&self) -> Result<T, AppError> {
        self.inner.lock()
            .map(|guard| guard.clone())
            .map_err(|e| AppError::MutexPoison(self.event_name.to_string()))
    }
}
```

**Usage Example**:
```rust
// In AppState
pub struct AppState {
    pub node_status: StateManager<NodeStatus>,
    pub mining_status: StateManager<MiningStatus>,
}

// In Tauri command
#[tauri::command]
async fn start_node(state: State<'_, AppState>) -> Result<(), String> {
    state.node_status.update(|status| {
        status.running = true;
        status.start_time = Some(Utc::now());
    }).map_err(|e| e.to_string())?;

    // Event "node_status_changed" automatically emitted
    Ok(())
}
```

### Alternatives Considered
| Alternative | Pros | Cons | Rejected Because |
|-------------|------|------|------------------|
| Manual `emit_all()` calls | Simple, direct | Error-prone, easy to forget | Article XI compliance requires reliability |
| Trait-based (custom `StateMut` trait) | Flexible | Complex trait bounds, steep learning curve | Over-engineering for this use case |
| Proc macros | Declarative, clean | Build complexity, harder to debug | Wrapper pattern achieves same goal simply |

### Migration Strategy
1. Create `src-tauri/src/state_management.rs` with `StateManager<T>` type
2. Replace `Arc<Mutex<NodeStatus>>` with `StateManager<NodeStatus>` in `AppState`
3. Replace `mining_processes: Arc<Mutex<...>>` with `StateManager<MiningProcesses>`
4. Update all `.lock().unwrap()` call sites to use `.update()` or `.get()`
5. Remove manual `emit_all()` calls (now automatic)
6. Add integration tests verifying events emitted on state changes

---

## 6. Memory Leak Detection Strategy

### Decision
**Use automated stress test script with `/usr/bin/time -v` for memory tracking** over 7-day period.

### Rationale
- **No Extra Dependencies**: Uses standard Linux `time` utility (GNU coreutils)
- **Automated**: Shell script runs continuously, logs memory samples
- **Accurate**: Tracks RSS (Resident Set Size) and peak memory usage
- **CI/CD Ready**: Can run in background, outputs to log file for analysis
- **Baseline Comparison**: Establishes memory growth rate (target: <5% over 7 days)

### Implementation Pattern
```bash
#!/bin/bash
# tests/stress/seven_day_test.sh

APP_PATH="./src-tauri/target/release/btpc-desktop-app"
LOG_FILE="./memory_leak_test_$(date +%Y%m%d_%H%M%S).log"
DURATION_DAYS=7
DURATION_SECONDS=$((DURATION_DAYS * 24 * 60 * 60))

echo "Starting 7-day memory leak test" | tee -a "$LOG_FILE"
echo "Duration: $DURATION_DAYS days ($DURATION_SECONDS seconds)" | tee -a "$LOG_FILE"

# Start app in background
$APP_PATH &
APP_PID=$!

# Sample memory every 5 minutes
INTERVAL=300
SAMPLES=$((DURATION_SECONDS / INTERVAL))

for i in $(seq 1 $SAMPLES); do
    sleep $INTERVAL

    # Get memory stats
    RSS=$(ps -o rss= -p $APP_PID 2>/dev/null)
    if [ -z "$RSS" ]; then
        echo "App crashed at sample $i" | tee -a "$LOG_FILE"
        exit 1
    fi

    RSS_MB=$((RSS / 1024))
    ELAPSED=$((i * INTERVAL))
    ELAPSED_HOURS=$((ELAPSED / 3600))

    echo "[$ELAPSED_HOURS hours] RSS: ${RSS_MB}MB" | tee -a "$LOG_FILE"

    # Trigger mining start/stop every hour
    if [ $((i % 12)) -eq 0 ]; then
        # Call Tauri command via IPC to start/stop mining
        echo "Triggering mining cycle" | tee -a "$LOG_FILE"
    fi
done

kill $APP_PID
echo "Test complete. Check $LOG_FILE for analysis." | tee -a "$LOG_FILE"
```

### Analysis Script
```python
# Analyze memory leak test results
import re
import sys

def analyze_memory_log(log_file):
    samples = []
    with open(log_file) as f:
        for line in f:
            match = re.search(r'\[(\d+) hours\] RSS: (\d+)MB', line)
            if match:
                hours, rss_mb = int(match.group(1)), int(match.group(2))
                samples.append((hours, rss_mb))

    if not samples:
        print("No samples found")
        return

    initial_mb = samples[0][1]
    final_mb = samples[-1][1]
    growth_mb = final_mb - initial_mb
    growth_pct = (growth_mb / initial_mb) * 100

    print(f"Initial memory: {initial_mb}MB")
    print(f"Final memory: {final_mb}MB")
    print(f"Growth: {growth_mb}MB ({growth_pct:.2f}%)")

    # NFR-017: < 5% growth over 7 days
    if growth_pct < 5.0:
        print("PASS: Memory growth within 5% threshold")
        sys.exit(0)
    else:
        print(f"FAIL: Memory growth {growth_pct:.2f}% exceeds 5% threshold")
        sys.exit(1)

if __name__ == "__main__":
    analyze_memory_log(sys.argv[1])
```

### Alternatives Considered
| Alternative | Pros | Cons | Rejected Because |
|-------------|------|------|------------------|
| Valgrind | Detailed leak detection | Massive slowdown (10-50x), not for 7-day test | Impractical runtime cost |
| heaptrack | Profiler-grade data | Requires GUI analysis, heavyweight | Need automated pass/fail |
| jemalloc stats | Rust-integrated | Requires rebuilding with allocator flag | Want zero code changes |

### Validation Criteria (NFR-017)
- Initial memory: Record at hour 0
- Final memory: Record at hour 168 (7 days)
- Growth calculation: `(final - initial) / initial * 100`
- **PASS**: Growth < 5%
- **FAIL**: Growth >= 5%

---

## 7. Progressive Disclosure Error UI

### Decision
**Use collapsible `<details>` HTML element** with Tauri clipboard API for copying technical details.

### Rationale
- **Native HTML**: No JavaScript framework required, accessible
- **Progressive Disclosure**: Collapsed by default, user clicks to expand (NFR-019)
- **Semantic HTML**: Screen reader friendly (`<summary>` announces interactivity)
- **Copy to Clipboard**: Tauri provides `writeText()` API for one-click copy
- **Monospace Rendering**: `<pre>` tag for technical details (NFR-020)

### Implementation Pattern

**HTML (password-modal.html, error display section)**:
```html
<div class="error-message" id="errorDisplay" style="display:none;">
    <div class="error-header">
        <span class="error-icon">⚠️</span>
        <span class="error-title" id="errorTitle"></span>
    </div>

    <p class="error-user-message" id="errorUserMessage"></p>

    <details class="error-details">
        <summary>Show Technical Details</summary>
        <pre id="errorTechnicalDetails" class="error-technical-pre"></pre>
        <button id="copyErrorButton" class="btn btn-secondary">Copy to Clipboard</button>
    </details>
</div>
```

**CSS (btpc-styles.css)**:
```css
.error-message {
    background: #fee;
    border: 1px solid #c33;
    border-radius: 4px;
    padding: 16px;
    margin: 16px 0;
}

.error-user-message {
    font-size: 14px;
    color: #333;
    margin: 8px 0;
}

.error-details {
    margin-top: 12px;
}

.error-details summary {
    cursor: pointer;
    color: #0066cc;
    user-select: none;
}

.error-details summary:hover {
    text-decoration: underline;
}

.error-technical-pre {
    font-family: 'Courier New', monospace;
    font-size: 12px;
    background: #f5f5f5;
    border: 1px solid #ddd;
    padding: 12px;
    overflow-x: auto;
    margin: 8px 0;
}

#copyErrorButton {
    margin-top: 8px;
}
```

**JavaScript (btpc-error-handler.js)**:
```javascript
// Display error with progressive disclosure
async function displayError(errorState) {
    const display = document.getElementById('errorDisplay');
    const title = document.getElementById('errorTitle');
    const userMsg = document.getElementById('errorUserMessage');
    const techDetails = document.getElementById('errorTechnicalDetails');

    title.textContent = errorState.error_type;
    userMsg.textContent = errorState.user_message;

    if (errorState.technical_details) {
        techDetails.textContent = errorState.technical_details;
    } else {
        // Hide details section if no technical info
        document.querySelector('.error-details').style.display = 'none';
    }

    display.style.display = 'block';
}

// Copy to clipboard using Tauri API
document.getElementById('copyErrorButton').addEventListener('click', async () => {
    const techDetails = document.getElementById('errorTechnicalDetails').textContent;

    try {
        await window.__TAURI__.clipboard.writeText(techDetails);
        // Show success feedback
        const btn = document.getElementById('copyErrorButton');
        const originalText = btn.textContent;
        btn.textContent = 'Copied!';
        setTimeout(() => {
            btn.textContent = originalText;
        }, 2000);
    } catch (err) {
        console.error('Failed to copy to clipboard:', err);
    }
});

// Listen for error events from backend
window.__TAURI__.event.listen('error_occurred', (event) => {
    displayError(event.payload);
});
```

**Backend Emission (Rust)**:
```rust
use tauri::Manager;

pub fn emit_error(app_handle: &tauri::AppHandle, error: AppError) {
    let error_state = ErrorState {
        error_type: error.error_type_str(),
        user_message: error.user_friendly_message(),
        technical_details: Some(format!("{:?}", error)), // Debug repr with stack trace
        timestamp: Utc::now(),
        affected_component: error.component(),
        crash_count: 0,
    };

    app_handle.emit_all("error_occurred", error_state)
        .unwrap_or_else(|e| eprintln!("Failed to emit error event: {}", e));
}
```

### Alternatives Considered
| Alternative | Pros | Cons | Rejected Because |
|-------------|------|------|------------------|
| Modal dialog | Focus-grabbing, clear | Interrupts workflow, annoying | Too intrusive for non-critical errors |
| JSON format | Machine-readable | Not user-friendly for copy/paste | Users need readable format for support tickets |
| Formatted HTML | Rich formatting | Hard to copy cleanly, not monospace | Plain text in `<pre>` more universally useful |

### Accessibility Notes
- `<summary>` element announces "button" role to screen readers
- Collapsed by default (progressive disclosure best practice)
- `<pre>` preserves formatting for technical details
- Copy button provides keyboard-accessible alternative to manual selection

---

## Summary of Decisions

| Research Topic | Decision | Key Rationale |
|----------------|----------|---------------|
| 1. Error Handling | `thiserror` with `AppError` enum | Type-safe, Tauri-compatible, industry standard |
| 2. Process Management | `tokio::process` with reaper task | Async-first, auto-reaping, no unsafe code |
| 3. Balance Calculation | Normalize addresses to lowercase | Fixes case-sensitivity bug at JSON boundary |
| 4. File Locking | `fs2` crate for cross-platform locks | Safe, cross-platform, well-maintained |
| 5. Event Emission | `StateManager<T>` wrapper type | Compile-time guarantee, Article XI compliant |
| 6. Memory Leak Detection | Automated script with `/usr/bin/time` | No dependencies, CI-ready, baseline tracking |
| 7. Error UI | `<details>` HTML with Tauri clipboard | Native, accessible, progressive disclosure |

All decisions prioritize:
- ✅ **Safety**: Remove unsafe code, proper error handling
- ✅ **Article XI Compliance**: Backend-first, event-driven
- ✅ **Maintainability**: Standard patterns, minimal dependencies
- ✅ **Testability**: TDD-friendly, automated validation

---

**Next Phase**: Phase 1 - Design & Contracts (data-model.md, contracts/, quickstart.md)
