# Tasks: GPU Mining Dashboard with Individual GPU Statistics

**Input**: Design documents from `/home/bob/BTPC/BTPC/specs/012-create-an-new/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md
**Constitution**: Article XI patterns apply (desktop feature)
**Feature Branch**: `012-create-an-new`

## Execution Summary

**Total Tasks**: 36
**Estimated Time**: 2-3 days
**Parallelizable Tasks**: 18 (marked with [P])
**Critical Path**: Setup → Tests → Core → Integration → Polish

---

## Phase 3.1: Setup & Configuration

- [ ] **T001** Create GPU monitoring Rust modules in btpc-desktop-app/src-tauri/src/
  - Files: `gpu_health_monitor.rs`, `thermal_throttle.rs`, `gpu_stats_commands.rs`
  - Create `commands/gpu_stats.rs` module
  - Add module declarations to `lib.rs`

- [ ] **T002** Add dependencies to btpc-desktop-app/src-tauri/Cargo.toml
  - Add: `nvml-wrapper = "0.9"` (NVIDIA GPU monitoring)
  - Add: `sysinfo = "0.30"` (cross-platform system info)
  - Verify: `opencl3 = "0.9"` already exists (Feature 010)
  - Run: `cargo audit` to verify security

- [ ] **T003** [P] Create persistent storage directory structure
  - Ensure: `~/.btpc/data/` directory exists
  - Create: Initial `mining_stats_per_gpu.json` with empty object `{}`
  - Test: File permissions allow read/write (atomic operations)

- [ ] **T004** [P] Create GPU Mining sub-tab HTML structure in btpc-desktop-app/ui/mining.html
  - Add: Second sub-tab button "GPU Mining" (ID: `gpu-mining-tab`)
  - Add: Tab content div (ID: `gpu-mining-content`)
  - Add: Temperature threshold setting input (Settings page)
  - Reuse: `btpc-tab-manager.js` from Feature 004

---

## Phase 3.2: Tests First (TDD) ⚠️ MUST COMPLETE BEFORE 3.3

**CRITICAL: These tests MUST be written and MUST FAIL before ANY implementation**

**Backend Contract Tests:**

- [ ] **T005** [P] Contract test for `enumerate_gpus` command
  - File: `btpc-desktop-app/src-tauri/tests/contract/test_gpu_enumeration.rs`
  - Test: Returns array of GPU devices with required fields
  - Test: Handles "No GPUs detected" error gracefully
  - Test: Completes in <500ms (NFR-001)
  - Assert: Device index, model name, vendor, opencl_capable fields present
  - **Expected**: Test FAILS (command not implemented yet)

- [ ] **T006** [P] Contract test for `get_gpu_mining_stats` command
  - File: `btpc-desktop-app/src-tauri/tests/contract/test_gpu_stats.rs`
  - Test: Returns stats for all GPUs when no parameter provided
  - Test: Returns stats for specific GPU when device_index provided
  - Test: Includes hashrate, blocks found, uptime, efficiency metrics
  - Test: Handles missing GPU device error
  - Assert: All fields match `data-model.md` schema
  - **Expected**: Test FAILS (command not implemented yet)

- [ ] **T007** [P] Contract test for `get_gpu_health_metrics` command
  - File: `btpc-desktop-app/src-tauri/tests/contract/test_gpu_health.rs`
  - Test: Returns health metrics for all GPUs
  - Test: Handles unavailable sensors gracefully (shows None/null)
  - Test: Includes temperature, fan speed, power, memory, clock speed
  - Assert: Nullable fields work correctly (graceful degradation)
  - **Expected**: Test FAILS (command not implemented yet)

- [ ] **T008** [P] Contract test for `set_temperature_threshold` command
  - File: `btpc-desktop-app/src-tauri/tests/contract/test_temperature_config.rs`
  - Test: Validates threshold range (60°C - 95°C)
  - Test: Rejects invalid thresholds (<60°C or >95°C)
  - Test: Backend validates FIRST before emitting event (Article XI, Section 11.2)
  - Test: Emits event after successful save
  - Assert: Error messages are actionable
  - **Expected**: Test FAILS (command not implemented yet)

- [ ] **T009** [P] Integration test for thermal throttling algorithm
  - File: `btpc-desktop-app/src-tauri/tests/integration/test_thermal_throttle.rs`
  - Test: Reduces mining intensity when temp exceeds threshold
  - Test: Incremental reduction (10% every 10 seconds)
  - Test: Restores intensity when temp drops below (threshold - 5°C)
  - Test: Emits `gpu-thermal-throttle` event with log message
  - Assert: Hysteresis prevents oscillation
  - **Expected**: Test FAILS (throttling not implemented yet)

- [ ] **T010** [P] Integration test for GPU stats persistence
  - File: `btpc-desktop-app/src-tauri/tests/integration/test_gpu_persistence.rs`
  - Test: Lifetime blocks found persists across app restarts
  - Test: Atomic file writes prevent corruption
  - Test: File format matches `data-model.md` schema
  - Assert: `~/.btpc/data/mining_stats_per_gpu.json` exists and valid
  - **Expected**: Test FAILS (persistence not implemented yet)

**Frontend Integration Tests:**

- [ ] **T011** [P] Integration test for GPU dashboard event-driven updates (Article XI, Section 11.3)
  - File: `btpc-desktop-app/ui/tests/test_gpu_dashboard_events.js` (or E2E test)
  - Test: Dashboard subscribes to `gpu-stats-updated` event
  - Test: UI updates every 5 seconds when event emitted
  - Test: NO frontend polling occurs (use Chrome DevTools Network tab to verify)
  - Assert: Event payload matches `data-model.md` GpuDashboardData schema
  - **Expected**: Test FAILS (event emission not implemented yet)

- [ ] **T012** [P] Memory leak test for event listener cleanup (Article XI, Section 11.6)
  - File: `btpc-desktop-app/ui/tests/test_event_cleanup.js` (or manual Chrome DevTools test)
  - Test: Event listeners removed on page navigation (beforeunload)
  - Test: No memory leaks after 100+ tab switches
  - Test: `unlisten()` called for all Tauri event listeners
  - Assert: Chrome DevTools Memory Profiler shows no listener accumulation
  - **Expected**: Test FAILS (cleanup not implemented yet)

---

## Phase 3.3: Core Implementation (ONLY after tests are failing)

**Backend - GPU Enumeration:**

- [ ] **T013** [P] Implement GPU enumeration service in gpu_health_monitor.rs
  - File: `btpc-desktop-app/src-tauri/src/gpu_health_monitor.rs`
  - Implement: `enumerate_gpus()` function using OpenCL device queries
  - Implement: `GpuDevice` struct from `data-model.md`
  - Implement: Hot-plug detection (refresh on hardware changes)
  - Handle: "No GPUs detected" error gracefully
  - **Verify**: T005 contract test now PASSES

- [ ] **T014** Implement `enumerate_gpus` Tauri command in gpu_stats_commands.rs
  - File: `btpc-desktop-app/src-tauri/src/gpu_stats_commands.rs`
  - Expose: `enumerate_gpus` as Tauri command
  - Call: `gpu_health_monitor::enumerate_gpus()`
  - Return: Serialized JSON matching `contracts/tauri-commands.yaml`
  - Performance: Target <500ms (NFR-001)
  - **Verify**: T005 contract test now PASSES completely

**Backend - GPU Health Monitoring:**

- [ ] **T015** [P] Implement NVML/ADL sensor polling in gpu_health_monitor.rs
  - File: `btpc-desktop-app/src-tauri/src/gpu_health_monitor.rs`
  - Implement: `GpuHealthMetrics` struct from `data-model.md`
  - Implement: `poll_gpu_health(device_index)` using NVML (NVIDIA)
  - Implement: Fallback to sysinfo crate for AMD/Intel (basic metrics)
  - Handle: Unavailable sensors gracefully (return `None`, display "N/A")
  - Poll frequency: Every 5 seconds (configurable)
  - **Verify**: T007 contract test now PASSES

- [ ] **T016** Extend MiningThreadPool with GPU stats tracking in mining_thread_pool.rs
  - File: `btpc-desktop-app/src-tauri/src/mining_thread_pool.rs`
  - Add: `GpuMiningStats` struct from `data-model.md`
  - Add: Per-GPU hashrate tracking (separate from aggregate hashrate)
  - Add: Lifetime blocks found counter per GPU
  - Add: Mining uptime tracking per GPU
  - Add: Efficiency metrics calculation (hashrate/watt, hashrate/temp)
  - Handle: Division by zero (return `None` if power or temp unavailable)
  - **Verify**: T006 contract test now PASSES

- [ ] **T017** Implement `get_gpu_mining_stats` Tauri command in gpu_stats_commands.rs
  - File: `btpc-desktop-app/src-tauri/src/gpu_stats_commands.rs`
  - Access: `Arc<tokio::sync::RwLock<MiningThreadPool>>` from AppState
  - Query: Per-GPU stats from MiningThreadPool
  - Parameter: Optional `gpu_device_index` for single GPU query
  - Return: Serialized stats matching `contracts/tauri-commands.yaml`
  - **Verify**: T006 contract test now PASSES

- [ ] **T018** Implement `get_gpu_health_metrics` Tauri command in gpu_stats_commands.rs
  - File: `btpc-desktop-app/src-tauri/src/gpu_stats_commands.rs`
  - Call: `gpu_health_monitor::poll_gpu_health()` for each GPU
  - Parameter: Optional `gpu_device_index` for single GPU query
  - Return: Serialized metrics matching `contracts/tauri-commands.yaml`
  - Handle: Sensor unavailability (graceful degradation)
  - **Verify**: T007 contract test now PASSES

**Backend - Temperature Configuration:**

- [ ] **T019** Implement temperature threshold configuration in gpu_stats_commands.rs
  - File: `btpc-desktop-app/src-tauri/src/gpu_stats_commands.rs`
  - Implement: `set_temperature_threshold(threshold: f32)` command
  - Validate: Range 60°C - 95°C (reject outside bounds)
  - Store: In AppState `Arc<tokio::sync::RwLock<f32>>` (backend is source of truth)
  - Emit: `gpu-config-updated` event AFTER successful save (Article XI, Section 11.2)
  - Implement: `get_temperature_threshold()` command (read from AppState)
  - **Verify**: T008 contract test now PASSES

**Backend - Thermal Throttling:**

- [ ] **T020** [P] Implement thermal throttling algorithm in thermal_throttle.rs
  - File: `btpc-desktop-app/src-tauri/src/thermal_throttle.rs`
  - Implement: `ThermalThrottle` service per `research.md` algorithm
  - Algorithm: If `temp > threshold`, reduce intensity by 10% every 10 seconds
  - Algorithm: If `temp < (threshold - 5°C)`, restore to 100% incrementally
  - Integrate: With MiningThreadPool to adjust mining intensity per GPU
  - Emit: `gpu-thermal-throttle` event with log message (FR-013d)
  - **Verify**: T009 integration test now PASSES

**Backend - Event Emission (Article XI, Section 11.3):**

- [ ] **T021** Implement 5-second event emission loop in main.rs
  - File: `btpc-desktop-app/src-tauri/src/main.rs`
  - Create: Async task that runs every 5 seconds
  - Call: `get_gpu_dashboard_data()` helper (combines devices + stats + health)
  - Emit: `gpu-stats-updated` event to all frontend listeners
  - Payload: Full `GpuDashboardData` from `data-model.md`
  - Article XI: Backend emits events, frontend listens (no frontend polling)
  - **Verify**: T011 integration test now PASSES

**Backend - Persistent Storage:**

- [ ] **T022** Implement GPU stats persistence in mining_thread_pool.rs
  - File: `btpc-desktop-app/src-tauri/src/mining_thread_pool.rs`
  - Implement: `save_gpu_stats()` function (atomic write to JSON)
  - Path: `~/.btpc/data/mining_stats_per_gpu.json`
  - Schema: Match `data-model.md` PersistedGpuStats
  - Atomic: Write to `.tmp`, fsync, rename (prevents corruption)
  - Load: Read stats on app startup, initialize lifetime blocks counters
  - **Verify**: T010 integration test now PASSES

**Frontend - GPU Dashboard UI:**

- [ ] **T023** Create GPU dashboard JavaScript in mining-gpu-dashboard.js
  - File: `btpc-desktop-app/ui/mining-gpu-dashboard.js`
  - Implement: `initGpuDashboard()` function (called on page load)
  - Implement: `renderGpuCards(devices, stats, health)` function
  - Implement: Tauri event listener for `gpu-stats-updated` (Article XI)
  - Display: All metrics from `data-model.md` per GPU (hashrate, temp, fan, power, efficiency)
  - Display: "N/A" for unavailable sensors (graceful degradation)
  - Subscribe: To `gpu-thermal-throttle` event for log messages
  - **Verify**: T011 integration test now PASSES

- [ ] **T024** Implement event listener cleanup in mining-gpu-dashboard.js (Article XI, Section 11.6)
  - File: `btpc-desktop-app/ui/mining-gpu-dashboard.js`
  - Store: `unlisten` function returned by `listen()` in global variable
  - Implement: `beforeunload` event handler calling `unlisten()`
  - Cleanup: All Tauri event listeners on page navigation
  - **Verify**: T012 memory leak test now PASSES

- [ ] **T025** Add GPU card CSS styles in btpc-styles.css
  - File: `btpc-desktop-app/ui/btpc-styles.css`
  - Add: `.gpu-card` class (card layout, border, padding)
  - Add: `.gpu-temp-warning` class (orange/red highlighting when temp > threshold)
  - Add: `.gpu-throttled` class (visual indicator for throttled status)
  - Add: Grid/list layout for multiple GPUs (responsive design)
  - Ensure: Accessible color contrast (WCAG 2.1 AA compliance)

**Frontend - Settings Integration:**

- [ ] **T026** Add temperature threshold setting in settings.html (Article XI, Section 11.2)
  - File: `btpc-desktop-app/ui/settings.html`
  - Add: Input field for temperature threshold (range: 60-95°C)
  - Add: "Save" button invoking `set_temperature_threshold()` command
  - Implement: Backend-first validation (call command, wait for response)
  - On success: Save to localStorage (`btpc_gpu_temp_threshold`), show success message
  - On error: Show error message, DO NOT save to localStorage (Article XI violation prevention)
  - Load: Read from localStorage on page load, fallback to 80°C default

---

## Phase 3.4: Integration & Polish

**Integration:**

- [ ] **T027** Register all GPU commands in lib.rs
  - File: `btpc-desktop-app/src-tauri/src/lib.rs`
  - Register: `enumerate_gpus`, `get_gpu_mining_stats`, `get_gpu_health_metrics`
  - Register: `set_temperature_threshold`, `get_temperature_threshold`
  - Export: All GPU-related modules publicly
  - Verify: Tauri build succeeds with new commands

- [ ] **T028** [P] Add GPU Mining sub-tab to mining.html navigation
  - File: `btpc-desktop-app/ui/mining.html`
  - Integrate: `btpc-tab-manager.js` for sub-tab switching
  - Add: Script tag for `mining-gpu-dashboard.js`
  - Wire: Tab click events to show/hide GPU dashboard content
  - Persist: Active sub-tab in localStorage (`btpc_active_tab_mining`)

**Code Quality:**

- [ ] **T029** [P] Unit tests for thermal throttling edge cases
  - File: `btpc-desktop-app/src-tauri/tests/unit/test_thermal_edge_cases.rs`
  - Test: Temperature sensor returns invalid data (NaN, negative values)
  - Test: GPU temperature drops suddenly (prevent over-throttling)
  - Test: Concurrent throttling on multiple GPUs
  - Test: Throttling with missing power sensor (efficiency calculation)

- [ ] **T030** [P] Performance test: GPU enumeration <500ms
  - File: `btpc-desktop-app/src-tauri/tests/benchmarks/bench_gpu_enumeration.rs`
  - Benchmark: `enumerate_gpus()` execution time
  - Assert: Mean time <500ms (NFR-001)
  - Test: With 1, 4, 8, 16 GPU configurations (if available)

- [ ] **T031** [P] Performance test: Stats update overhead <1%
  - File: `btpc-desktop-app/src-tauri/tests/benchmarks/bench_stats_overhead.rs`
  - Benchmark: Mining hashrate before/after stats polling enabled
  - Assert: Hashrate drop <1% (NFR-002)
  - Test: With 5-second polling interval active

- [ ] **T032** Run cargo clippy and fix all warnings
  - Command: `cargo clippy -- -D warnings`
  - Fix: All clippy warnings in new GPU modules
  - Verify: Zero warnings in `gpu_health_monitor.rs`, `thermal_throttle.rs`, `gpu_stats_commands.rs`

**Article XI Compliance Verification:**

- [ ] **T033** Verify backend-first validation for temperature threshold
  - Manual test: Attempt to set invalid threshold (50°C)
  - Expected: Backend rejects, frontend shows error, localStorage NOT modified
  - Verify: Article XI, Section 11.2 compliance

- [ ] **T034** Verify event-driven architecture (no frontend polling)
  - Manual test: Open Chrome DevTools Network tab
  - Verify: NO HTTP polling requests to backend for GPU stats
  - Verify: Only Tauri events (`gpu-stats-updated`) every 5 seconds
  - Article XI: Section 11.3 compliance

- [ ] **T035** Verify event listener cleanup (no memory leaks)
  - Manual test: Navigate between Mining page and other pages 100+ times
  - Tool: Chrome DevTools Memory Profiler
  - Verify: Event listener count does not increase
  - Article XI: Section 11.6 compliance

**Documentation:**

- [ ] **T036** [P] Update CLAUDE.md with Feature 012 summary
  - File: `/home/bob/BTPC/BTPC/CLAUDE.md`
  - Add: Feature 012 summary to Recent Changes section
  - Document: GPU monitoring dependencies (nvml-wrapper, opencl3)
  - Document: Thermal throttling algorithm
  - Keep: Under 150 lines total (remove oldest feature if needed)

---

## Dependencies

**Test Dependencies (TDD):**
- Tests T005-T012 MUST complete before implementation T013-T028
- All tests MUST FAIL initially (verify implementation not yet present)

**Backend Dependencies:**
- T013 (GPU enumeration) blocks T014, T017, T018, T021
- T015 (Health monitoring) blocks T018, T021, T020 (throttling)
- T016 (Stats tracking) blocks T017, T021, T022
- T019 (Temperature config) blocks T020 (throttling), T026 (Settings UI)
- T020 (Thermal throttling) blocks T021 (event emission)
- T021 (Event emission) blocks T023 (Frontend event listener)

**Frontend Dependencies:**
- T023 (Dashboard JS) blocks T024 (Event cleanup), T028 (Tab integration)
- T025 (CSS styles) blocks T028 (Tab integration)
- T026 (Settings integration) requires T019 (Backend command) completed first

**Integration Dependencies:**
- T027 (Command registration) requires all commands (T014, T017, T018, T019) completed
- T028 (HTML integration) requires T023, T024, T025 completed

**Polish Dependencies:**
- All implementation (T013-T028) before polish (T029-T036)

---

## Parallel Execution Examples

**Test Phase (Launch together - different test files):**
```bash
# All contract tests can run in parallel:
Task: T005 - Contract test for enumerate_gpus command
Task: T006 - Contract test for get_gpu_mining_stats command
Task: T007 - Contract test for get_gpu_health_metrics command
Task: T008 - Contract test for set_temperature_threshold command
Task: T009 - Integration test for thermal throttling algorithm
Task: T010 - Integration test for GPU stats persistence
Task: T011 - Integration test for GPU dashboard events
Task: T012 - Memory leak test for event cleanup
```

**Backend Core (Launch together - independent modules):**
```bash
# These can run in parallel (different files):
Task: T013 - Implement GPU enumeration service
Task: T015 - Implement NVML/ADL sensor polling
Task: T020 - Implement thermal throttling algorithm
Task: T022 - Implement GPU stats persistence
```

**Frontend & Polish (Launch together - different files):**
```bash
# These can run in parallel:
Task: T023 - Create GPU dashboard JavaScript
Task: T025 - Add GPU card CSS styles
Task: T029 - Unit tests for thermal edge cases
Task: T030 - Performance test: GPU enumeration
Task: T031 - Performance test: Stats overhead
Task: T036 - Update CLAUDE.md
```

---

## Manual Testing (After Implementation)

**Execute Quickstart Scenarios:**
1. Run all 9 scenarios from `quickstart.md`
2. Check acceptance criteria table (all must pass)
3. Verify performance benchmarks (NFR-001 through NFR-004)
4. Test edge cases (no GPUs, driver crash, hot-plug)

**Article XI Compliance Checklist:**
- [ ] T033: Backend-first validation works
- [ ] T034: No frontend polling (event-driven only)
- [ ] T035: Event listeners cleaned up (no memory leaks)

---

## Notes

- **[P] tasks** = Different files, no dependencies, can run in parallel
- **TDD mandatory**: Tests must FAIL before implementing (verify with `cargo test`)
- **Commit after each task**: Use descriptive commit messages referencing task ID
- **Article XI compliance**: Desktop feature - all patterns MUST be followed
- **Performance targets**: GPU enum <500ms, stats overhead <1%, UI render <200ms
- **Graceful degradation**: Missing sensors show "N/A", no crashes
- **Run cargo clippy frequently**: Catch issues early (T032 before final commit)

---

**Feature 012 Tasks Complete**: Ready for implementation phase
**Next Step**: Execute tasks T001-T036 in order, respecting dependencies
**Validation**: Run `quickstart.md` scenarios after T036 completes