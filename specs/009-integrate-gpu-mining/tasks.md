# Tasks: GPU Mining Phase 3 - Stats & Monitoring

**Feature**: 009-integrate-gpu-mining
**Branch**: `009-integrate-gpu-mining`
**Plan**: [plan.md](plan.md)

## Task Execution Order

Tasks are numbered and must be executed in order unless marked with `[P]` for parallel execution.

### Phase 1: Setup & Dependencies
- T001: Verify warp dependency in Cargo.toml
- T002: Add stats_server module declaration

### Phase 2: Contract Tests (TDD - Parallel)
- T003 [P]: Write HTTP /stats endpoint contract test
- T004 [P]: Write HTTP /health endpoint contract test
- T005 [P]: Write GpuMiner.get_stats() unit test

### Phase 3: Core Implementation
- T006: Implement stats_server.rs HTTP module
- T007: Integrate stats server into main.rs
- T008: Verify GPU stats already implemented in gpu_miner.rs

### Phase 4: Desktop App Integration
- T009: Create Tauri GPU stats command (gpu_stats_commands.rs)
- T010: Register GPU stats command in Tauri main.rs
- T011: Add GPU stats UI to mining.html
- T012: Create mining-stats.js polling logic

### Phase 5: Integration & Validation
- T013: Build btpc_miner with GPU and stats features
- T014: Run contract tests (T003-T005)
- T015: Execute quickstart.md manual test scenario
- T016: Validate performance (stats overhead < 1%)

---

## Task Details

### T001: Verify warp dependency in Cargo.toml
**File**: `/home/bob/BTPC/BTPC/bins/btpc_miner/Cargo.toml`
**Type**: Setup
**Dependencies**: None

**Objective**: Ensure warp HTTP server dependency is present for stats endpoint.

**Steps**:
1. Read `bins/btpc_miner/Cargo.toml`
2. Check if `warp = "0.3"` exists in `[dependencies]` section
3. If missing, add: `warp = "0.3"` (but it should already be there from earlier work)
4. Verify `serde` and `serde_json` are present (required for JSON serialization)

**Success Criteria**:
- ✅ Cargo.toml contains `warp = "0.3"`
- ✅ No compilation errors when building

---

### T002: Add stats_server module declaration
**File**: `/home/bob/BTPC/BTPC/bins/btpc_miner/src/main.rs`
**Type**: Setup
**Dependencies**: None

**Objective**: Declare stats_server module in main.rs for HTTP endpoint implementation.

**Steps**:
1. Read `bins/btpc_miner/src/main.rs`
2. Check if `mod stats_server;` declaration exists
3. If missing, add `mod stats_server;` near other module declarations (e.g., after `mod gpu_miner;`)
4. Add conditional compilation if needed: `#[cfg(feature = "gpu")]`

**Success Criteria**:
- ✅ `mod stats_server;` declared in main.rs
- ✅ No compilation errors

---

### T003 [P]: Write HTTP /stats endpoint contract test
**File**: `/home/bob/BTPC/BTPC/tests/integration/test_gpu_stats_endpoint.rs`
**Type**: Contract Test (TDD)
**Dependencies**: None (parallel with T004, T005)

**Objective**: Write integration test verifying /stats endpoint returns valid GpuStats JSON.

**Test Requirements** (from contracts/gpu-stats-api.yaml):
1. Start stats server on port 8333
2. Send GET request to `http://127.0.0.1:18360/stats`
3. Verify HTTP 200 OK response
4. Verify response is valid JSON
5. Verify JSON matches GpuStats schema:
   - Required fields: device_name, vendor, compute_units, hashrate, total_hashes, uptime_seconds
   - Optional fields: temperature, power_usage
6. Verify error case: HTTP 500 when GPU not available

**Implementation Notes**:
- Use `reqwest` for HTTP client
- Mock GpuMiner or use real GPU if available
- Test both success and error responses

**Success Criteria**:
- ✅ Test file created at `tests/integration/test_gpu_stats_endpoint.rs`
- ✅ Test compiles (will fail until T006 implemented)
- ✅ Test covers both 200 OK and 500 error cases

---

### T004 [P]: Write HTTP /health endpoint contract test
**File**: `/home/bob/BTPC/BTPC/tests/integration/test_gpu_stats_endpoint.rs`
**Type**: Contract Test (TDD)
**Dependencies**: None (parallel with T003, T005)

**Objective**: Write test verifying /health endpoint returns service status.

**Test Requirements** (from contracts/gpu-stats-api.yaml):
1. Send GET request to `http://127.0.0.1:18360/health`
2. Verify HTTP 200 OK response
3. Verify JSON structure: `{"status": "ok", "service": "btpc_miner_stats"}`

**Success Criteria**:
- ✅ Health endpoint test added to test_gpu_stats_endpoint.rs
- ✅ Test verifies correct JSON response

---

### T005 [P]: Write GpuMiner.get_stats() unit test
**File**: `/home/bob/BTPC/BTPC/bins/btpc_miner/src/gpu_miner.rs`
**Type**: Unit Test (TDD)
**Dependencies**: None (parallel with T003, T004)

**Objective**: Write unit test for GpuMiner.get_stats() method.

**Test Requirements** (from data-model.md):
1. Create mock GpuMiner instance (or use test GPU)
2. Call `get_stats()` method
3. Verify returned GpuStats contains:
   - Non-empty device_name
   - Non-empty vendor
   - compute_units > 0
   - hashrate calculation: `total_hashes / uptime_seconds / 1_000_000.0`
4. Verify optional fields are None (unless vendor extensions available)

**Success Criteria**:
- ✅ Unit test added to gpu_miner.rs `#[cfg(test)] mod tests`
- ✅ Test verifies all required GpuStats fields
- ✅ Test compiles (may fail if get_stats() not implemented yet)

---

### T006: Implement stats_server.rs HTTP module
**File**: `/home/bob/BTPC/BTPC/bins/btpc_miner/src/stats_server.rs`
**Type**: Core Implementation
**Dependencies**: T001, T002, T003, T004 (tests written first)

**Objective**: Create HTTP server module with /stats and /health endpoints.

**Implementation Requirements** (from plan.md and research.md):
1. Create `start_stats_server()` async function:
   - Parameters: `gpu_miner: Option<Arc<Mutex<GpuMiner>>>`, `port: u16`
   - Returns: `Result<String, String>` (server URL or error)

2. Implement `/stats` endpoint:
   - Use warp::path("stats").and(warp::get())
   - Lock GpuMiner mutex and call get_stats()
   - Return JSON response with GpuStats
   - Handle errors: GPU not enabled, lock failed, stats query failed

3. Implement `/health` endpoint:
   - Return `{"status": "ok", "service": "btpc_miner_stats"}`

4. Spawn server on background tokio task:
   - Listen on 127.0.0.1:port
   - Return server URL for logging

**Code Structure** (from existing stats_server.rs):
```rust
use std::sync::{Arc, Mutex};
use warp::Filter;
use crate::gpu_miner::{GpuMiner, GpuStats};

pub async fn start_stats_server(
    gpu_miner: Option<Arc<Mutex<GpuMiner>>>,
    port: u16,
) -> Result<String, String> {
    // Implementation here
}
```

**Success Criteria**:
- ✅ stats_server.rs created with start_stats_server() function
- ✅ Both /stats and /health endpoints implemented
- ✅ Proper error handling for all failure cases
- ✅ T003 and T004 contract tests pass

---

### T007: Integrate stats server into main.rs
**File**: `/home/bob/BTPC/BTPC/bins/btpc_miner/src/main.rs`
**Type**: Core Implementation
**Dependencies**: T006 (stats_server.rs must exist)

**Objective**: Start stats HTTP server when GPU mining is enabled.

**Implementation Requirements**:
1. Find the GPU mining initialization section in main.rs
2. After GpuMiner is created (Arc<Mutex<GpuMiner>>), start stats server:
   ```rust
   #[cfg(feature = "gpu")]
   if let Some(ref gpu_miner) = gpu_miner {
       match stats_server::start_stats_server(Some(gpu_miner.clone()), 8333).await {
           Ok(url) => {
               println!("📊 GPU stats server listening on {}", url);
           }
           Err(e) => {
               eprintln!("⚠️  Failed to start stats server: {}", e);
           }
       }
   }
   ```
3. Handle the case when GPU mining is disabled (pass None to stats_server)

**Success Criteria**:
- ✅ Stats server starts when `--gpu` flag is used
- ✅ Console output shows "📊 GPU stats server listening on http://127.0.0.1:18360"
- ✅ Server responds to curl requests while miner runs

---

### T008: Verify GPU stats already implemented in gpu_miner.rs
**File**: `/home/bob/BTPC/BTPC/bins/btpc_miner/src/gpu_miner.rs`
**Type**: Verification
**Dependencies**: None

**Objective**: Confirm GpuStats struct and get_stats() method are implemented.

**Verification Steps**:
1. Read `bins/btpc_miner/src/gpu_miner.rs`
2. Verify `GpuStats` struct exists with all required fields (from data-model.md):
   - device_name, vendor, compute_units, max_work_group_size
   - global_mem_size, local_mem_size, max_clock_frequency
   - hashrate, total_hashes, uptime_seconds
   - temperature (Option), power_usage (Option)
3. Verify `get_stats()` method exists on GpuMiner impl
4. Verify `#[derive(Serialize, Deserialize)]` on GpuStats for JSON serialization

**If Missing**: Implement according to data-model.md specification.

**Success Criteria**:
- ✅ GpuStats struct complete with all 12 fields
- ✅ get_stats() method implemented
- ✅ Serde derives present for JSON serialization
- ✅ T005 unit test passes

---

### T009: Create Tauri GPU stats command (gpu_stats_commands.rs)
**File**: `/home/bob/BTPC/BTPC/btpc-desktop-app/src-tauri/src/gpu_stats_commands.rs`
**Type**: Desktop App Integration
**Dependencies**: T006, T007 (stats server must be running)

**Objective**: Create Tauri command to fetch GPU stats from miner HTTP endpoint.

**Implementation Requirements** (from plan.md):
1. Create new file: `btpc-desktop-app/src-tauri/src/gpu_stats_commands.rs`
2. Implement `get_gpu_stats()` Tauri command:
   ```rust
   use serde::{Deserialize, Serialize};

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct GpuStats {
       pub device_name: String,
       pub vendor: String,
       pub compute_units: u32,
       pub max_work_group_size: usize,
       pub global_mem_size: u64,
       pub local_mem_size: u64,
       pub max_clock_frequency: u32,
       pub hashrate: f64,
       pub total_hashes: u64,
       pub uptime_seconds: u64,
       pub temperature: Option<f32>,
       pub power_usage: Option<f32>,
   }

   #[tauri::command]
   pub async fn get_gpu_stats() -> Result<GpuStats, String> {
       let response = reqwest::get("http://127.0.0.1:18360/stats").await
           .map_err(|e| format!("Failed to fetch GPU stats: {}", e))?;

       if !response.status().is_success() {
           return Err(format!("Stats server returned error: {}", response.status()));
       }

       let stats: GpuStats = response.json().await
           .map_err(|e| format!("Failed to parse GPU stats: {}", e))?;

       Ok(stats)
   }
   ```

3. Add `reqwest` dependency to `btpc-desktop-app/src-tauri/Cargo.toml` if missing

**Success Criteria**:
- ✅ gpu_stats_commands.rs created with get_gpu_stats() command
- ✅ Command uses reqwest to fetch from http://127.0.0.1:18360/stats
- ✅ Proper error handling for network failures and JSON parsing

---

### T010: Register GPU stats command in Tauri main.rs
**File**: `/home/bob/BTPC/BTPC/btpc-desktop-app/src-tauri/src/main.rs`
**Type**: Desktop App Integration
**Dependencies**: T009 (gpu_stats_commands.rs must exist)

**Objective**: Register get_gpu_stats command in Tauri app builder.

**Implementation Steps**:
1. Add module declaration: `mod gpu_stats_commands;`
2. Add command to `.invoke_handler()`:
   ```rust
   .invoke_handler(tauri::generate_handler![
       // ... existing commands ...
       gpu_stats_commands::get_gpu_stats,
   ])
   ```

**Success Criteria**:
- ✅ Module declared in main.rs
- ✅ Command registered in invoke_handler
- ✅ Desktop app builds without errors
- ✅ Command callable from frontend: `await invoke('get_gpu_stats')`

---

### T011: Add GPU stats UI to mining.html
**File**: `/home/bob/BTPC/BTPC/btpc-desktop-app/ui/mining.html`
**Type**: Frontend UI
**Dependencies**: T010 (Tauri command must be registered)

**Objective**: Add GPU stats display panel to mining page.

**Implementation Requirements**:
1. Find the mining controls section in mining.html
2. Add GPU stats panel below mining controls:
   ```html
   <div id="gpu-stats-panel" class="stats-panel" style="display: none;">
       <h3>GPU Mining Statistics</h3>
       <div class="stats-grid">
           <div class="stat-item">
               <span class="stat-label">Device:</span>
               <span id="gpu-device-name" class="stat-value">-</span>
           </div>
           <div class="stat-item">
               <span class="stat-label">Vendor:</span>
               <span id="gpu-vendor" class="stat-value">-</span>
           </div>
           <div class="stat-item">
               <span class="stat-label">Hashrate:</span>
               <span id="gpu-hashrate" class="stat-value">-</span>
           </div>
           <div class="stat-item">
               <span class="stat-label">Compute Units:</span>
               <span id="gpu-compute-units" class="stat-value">-</span>
           </div>
           <div class="stat-item">
               <span class="stat-label">VRAM:</span>
               <span id="gpu-vram" class="stat-value">-</span>
           </div>
           <div class="stat-item">
               <span class="stat-label">Clock Speed:</span>
               <span id="gpu-clock" class="stat-value">-</span>
           </div>
           <div class="stat-item">
               <span class="stat-label">Uptime:</span>
               <span id="gpu-uptime" class="stat-value">-</span>
           </div>
           <div class="stat-item">
               <span class="stat-label">Total Hashes:</span>
               <span id="gpu-total-hashes" class="stat-value">-</span>
           </div>
       </div>
       <div id="gpu-stats-error" class="error-message" style="display: none;"></div>
   </div>
   ```

3. Add CSS styling in btpc-styles.css (or inline):
   ```css
   .stats-panel {
       border: 1px solid #ccc;
       padding: 15px;
       margin-top: 20px;
       border-radius: 5px;
   }
   .stats-grid {
       display: grid;
       grid-template-columns: repeat(2, 1fr);
       gap: 10px;
   }
   .stat-item {
       display: flex;
       justify-content: space-between;
   }
   .stat-label {
       font-weight: bold;
   }
   ```

**Success Criteria**:
- ✅ GPU stats panel added to mining.html
- ✅ All stat fields have unique IDs for JavaScript updates
- ✅ Panel hidden by default (shown when GPU mining active)
- ✅ Error message div for stats fetch failures

---

### T012: Create mining-stats.js polling logic
**File**: `/home/bob/BTPC/BTPC/btpc-desktop-app/ui/js/mining-stats.js`
**Type**: Frontend Logic
**Dependencies**: T011 (HTML elements must exist)

**Objective**: Implement JavaScript to poll GPU stats and update UI.

**Implementation Requirements**:
1. Create `btpc-desktop-app/ui/js/mining-stats.js`
2. Implement stats polling function:
   ```javascript
   const { invoke } = window.__TAURI__.tauri;

   let statsInterval = null;

   async function updateGpuStats() {
       try {
           const stats = await invoke('get_gpu_stats');

           // Update UI elements
           document.getElementById('gpu-device-name').textContent = stats.device_name;
           document.getElementById('gpu-vendor').textContent = stats.vendor;
           document.getElementById('gpu-hashrate').textContent = `${stats.hashrate.toFixed(2)} MH/s`;
           document.getElementById('gpu-compute-units').textContent = stats.compute_units;
           document.getElementById('gpu-vram').textContent = `${(stats.global_mem_size / 1073741824).toFixed(1)} GB`;
           document.getElementById('gpu-clock').textContent = `${stats.max_clock_frequency} MHz`;
           document.getElementById('gpu-uptime').textContent = formatUptime(stats.uptime_seconds);
           document.getElementById('gpu-total-hashes').textContent = stats.total_hashes.toLocaleString();

           // Show panel, hide error
           document.getElementById('gpu-stats-panel').style.display = 'block';
           document.getElementById('gpu-stats-error').style.display = 'none';
       } catch (error) {
           // Show error message
           document.getElementById('gpu-stats-error').textContent = `GPU stats unavailable: ${error}`;
           document.getElementById('gpu-stats-error').style.display = 'block';
       }
   }

   function formatUptime(seconds) {
       const hours = Math.floor(seconds / 3600);
       const mins = Math.floor((seconds % 3600) / 60);
       const secs = seconds % 60;
       return `${hours.toString().padStart(2, '0')}:${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
   }

   function startGpuStatsPolling() {
       updateGpuStats(); // Immediate update
       statsInterval = setInterval(updateGpuStats, 2000); // Poll every 2 seconds
   }

   function stopGpuStatsPolling() {
       if (statsInterval) {
           clearInterval(statsInterval);
           statsInterval = null;
       }
       document.getElementById('gpu-stats-panel').style.display = 'none';
   }

   // Export functions for mining page
   window.GpuStats = {
       start: startGpuStatsPolling,
       stop: stopGpuStatsPolling
   };
   ```

3. Include script in mining.html:
   ```html
   <script src="js/mining-stats.js"></script>
   ```

4. Call `window.GpuStats.start()` when GPU mining starts
5. Call `window.GpuStats.stop()` when mining stops

**Success Criteria**:
- ✅ mining-stats.js created with polling logic
- ✅ Stats update every 2 seconds when mining active
- ✅ Graceful error handling when stats unavailable
- ✅ Uptime formatted as HH:MM:SS
- ✅ VRAM displayed in GB, hashrate in MH/s

---

### T013: Build btpc_miner with GPU and stats features
**File**: N/A (build command)
**Type**: Integration
**Dependencies**: T001-T008 (all miner code complete)

**Objective**: Rebuild btpc_miner with GPU support and verify stats server compiles.

**Build Commands**:
```bash
# Clean build
cargo clean

# Build with GPU features
cargo build --release --bin btpc_miner --features gpu

# Deploy to ~/.btpc/bin/
cp target/release/btpc_miner /home/bob/.btpc/bin/btpc_miner

# Verify binary
/home/bob/.btpc/bin/btpc_miner --help | grep -E "gpu|stats"
```

**Success Criteria**:
- ✅ Build completes without errors
- ✅ Binary includes GPU support
- ✅ Stats server code compiled in
- ✅ Binary size ~2.8M (similar to previous GPU build)

---

### T014: Run contract tests (T003-T005)
**File**: N/A (test command)
**Type**: Integration Testing
**Dependencies**: T013 (miner built), T003-T005 (tests written)

**Objective**: Execute all contract tests and verify they pass.

**Test Commands**:
```bash
# Run integration tests
cargo test --test test_gpu_stats_endpoint -- --nocapture

# Run gpu_miner unit tests
cargo test --package btpc_miner --lib gpu_miner::tests -- --nocapture
```

**Success Criteria**:
- ✅ All contract tests pass (T003, T004)
- ✅ GpuMiner.get_stats() unit test passes (T005)
- ✅ No test failures or panics

---

### T015: Execute quickstart.md manual test scenario
**File**: `/home/bob/BTPC/BTPC/specs/009-integrate-gpu-mining/quickstart.md`
**Type**: Manual Testing
**Dependencies**: T013 (miner built), T009-T012 (desktop app complete)

**Objective**: Follow quickstart.md test scenario to validate end-to-end functionality.

**Test Procedure** (from quickstart.md):
1. **Terminal 1**: Start GPU miner
   ```bash
   /home/bob/.btpc/bin/btpc_miner --gpu --network regtest --address test_address
   ```
   Verify output: "📊 GPU stats server listening on http://127.0.0.1:18360"

2. **Terminal 2**: Query stats endpoint
   ```bash
   curl http://127.0.0.1:18360/stats | jq
   curl http://127.0.0.1:18360/health | jq
   ```
   Verify JSON response matches GpuStats schema

3. **Terminal 3**: Launch desktop app
   ```bash
   cd btpc-desktop-app && npm run tauri:dev
   ```
   - Navigate to Mining tab
   - Click "Start Mining" with GPU enabled
   - Verify GPU stats panel displays
   - Verify stats update every 2 seconds
   - Verify hashrate > 0 and increasing total_hashes

**Success Criteria**:
- ✅ Stats server starts without errors
- ✅ curl requests return valid JSON
- ✅ Desktop app displays GPU stats in real-time
- ✅ No crashes or UI errors

---

### T016: Validate performance (stats overhead < 1%)
**File**: N/A (performance testing)
**Type**: Performance Validation
**Dependencies**: T015 (manual test complete)

**Objective**: Verify stats querying has minimal impact on mining performance.

**Performance Test**:
1. **Baseline**: Run GPU mining for 60 seconds without stats queries
   - Record hashrate from miner logs
   - Average over 1-minute period

2. **With Stats**: Run GPU mining with stats server + desktop app polling (2s interval)
   - Record hashrate from stats endpoint
   - Average over 1-minute period

3. **Calculate Overhead**:
   ```
   Overhead = (Baseline - WithStats) / Baseline * 100%
   ```

4. **Query Latency**: Measure stats endpoint response time
   ```bash
   time curl -s http://127.0.0.1:18360/stats > /dev/null
   ```

**Success Criteria** (from plan.md):
- ✅ Mining overhead < 1% (hashrate degradation)
- ✅ Stats query latency < 10ms (local HTTP request)
- ✅ No memory leaks over 10-minute test
- ✅ CPU usage for stats server negligible

---

## Parallel Execution Examples

### Phase 2: Contract Tests (can run in parallel)
All test tasks in Phase 2 can be executed simultaneously as they create independent test files:

```bash
# Run all contract tests in parallel using Task tool
```

Tasks T003, T004, T005 are independent and can be done together.

### Phase 4: Desktop App Files (can run in parallel)
T009 and T011 can run in parallel (different files):
- T009 creates gpu_stats_commands.rs (Tauri backend)
- T011 modifies mining.html (frontend)

---

## Progress Tracking

**Phase 1: Setup**
- [x] T001: Verify warp dependency
- [x] T002: Add stats_server module declaration

**Phase 2: Contract Tests (TDD)**
- [ ] T003 [P]: Write /stats endpoint test
- [ ] T004 [P]: Write /health endpoint test
- [ ] T005 [P]: Write get_stats() unit test

**Phase 3: Core Implementation**
- [ ] T006: Implement stats_server.rs
- [ ] T007: Integrate stats server into main.rs
- [ ] T008: Verify GPU stats in gpu_miner.rs

**Phase 4: Desktop App**
- [ ] T009: Create gpu_stats_commands.rs
- [ ] T010: Register command in Tauri main.rs
- [ ] T011: Add GPU stats UI to mining.html
- [ ] T012: Create mining-stats.js polling

**Phase 5: Integration & Validation**
- [ ] T013: Build btpc_miner
- [ ] T014: Run contract tests
- [ ] T015: Execute quickstart scenario
- [ ] T016: Validate performance

---

## Completion Checklist

After all tasks are complete, verify:
- ✅ All contract tests pass (cargo test)
- ✅ Desktop app builds without errors (npm run tauri:build)
- ✅ Manual testing scenario works end-to-end
- ✅ Performance targets met (<1% overhead, <10ms latency)
- ✅ No compilation warnings or errors
- ✅ GPU stats display updates in real-time
- ✅ Graceful error handling when stats unavailable

---

**Next Command**: Execute tasks with `/implement` or manually in order T001 → T016.