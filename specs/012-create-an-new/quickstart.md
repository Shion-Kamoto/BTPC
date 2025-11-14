# Quickstart: GPU Mining Dashboard Manual Testing

**Feature**: 012-create-an-new
**Date**: 2025-11-11
**Purpose**: Manual validation of GPU Mining Dashboard functionality

## Prerequisites

**Hardware**:
- At least 1 OpenCL-capable GPU (NVIDIA, AMD, or Intel)
- GPU drivers properly installed
- NVML library (NVIDIA) or ADL library (AMD) for full health monitoring

**Software**:
- BTPC Desktop App built with Feature 012 changes
- Node running in regtest mode (for mining functionality)

**Environment Setup**:
```bash
# Ensure GPU drivers are loaded
nvidia-smi  # For NVIDIA GPUs
clinfo      # For OpenCL verification

# Start desktop app
cd btpc-desktop-app
npm run tauri:dev
```

---

## Test Scenarios

### Scenario 1: GPU Enumeration

**User Story**: As a miner, I want to see all GPUs detected on my system

**Steps**:
1. Launch BTPC Desktop App
2. Navigate to Mining page
3. Click "GPU Mining" sub-tab

**Expected Results**:
- [ ] All system GPUs are listed (verify count matches `nvidia-smi` or `clinfo`)
- [ ] Each GPU card shows:
  - Device index (GPU 0, GPU 1, ...)
  - Model name (e.g., "NVIDIA RTX 3080")
  - Vendor (NVIDIA/AMD/Intel)
  - Status: "Idle" (mining not started yet)
- [ ] UI renders in < 200ms (use Chrome DevTools Performance tab)

**Validation**: FR-001, FR-002, FR-004, NFR-001, NFR-003

---

### Scenario 2: Start GPU Mining & View Stats

**User Story**: As a miner, I want to monitor individual GPU mining performance

**Steps**:
1. With GPU dashboard open, click "Start GPU Mining" button
2. Select 1-2 GPUs to mine with
3. Wait 10 seconds for mining to stabilize
4. Observe GPU cards updating every 5 seconds

**Expected Results**:
- [ ] Selected GPUs show "Active" status
- [ ] Each active GPU displays:
  - Current hashrate (H/s, KH/s, or MH/s with appropriate unit)
  - Lifetime blocks found (starts at 0 for new GPU)
  - Mining uptime (format: "00:00:15" for 15 seconds)
  - Energy efficiency (e.g., "250 KH/W") or "N/A" if power sensor unavailable
  - Thermal efficiency (e.g., "15 KH/°C") or "N/A" if temperature sensor unavailable
- [ ] Stats update every 5 seconds (observe timestamp changes)
- [ ] No noticeable mining hashrate drop (<1% impact)

**Validation**: FR-005, FR-006, FR-007, FR-008, FR-008a, FR-008b, FR-010, FR-017, NFR-002

---

### Scenario 3: GPU Health Monitoring

**User Story**: As a miner, I want to monitor GPU temperature and health metrics

**Steps**:
1. With GPU mining active, observe health metrics section on each GPU card
2. Run GPU stress test in parallel (e.g., `nvidia-smi dmon` or mining benchmark)
3. Watch temperature increase over 30 seconds

**Expected Results**:
- [ ] Each GPU card displays:
  - Temperature (°C) - updates every 5 seconds
  - Fan speed (RPM) or "N/A"
  - Power consumption (W) or "N/A"
  - Memory usage (e.g., "4096 MB / 8192 MB") or "N/A"
  - Core clock speed (MHz) or "N/A"
- [ ] Temperature value matches external tool (±2°C tolerance)
- [ ] Health metrics update every 5 seconds
- [ ] Missing sensors show "N/A" gracefully (no crashes)

**Validation**: FR-009, FR-010, FR-012, FR-013, NFR-004

---

### Scenario 4: Temperature Warning Threshold

**User Story**: As a miner, I want to configure temperature warning thresholds

**Steps**:
1. Stop GPU mining (if active)
2. Navigate to Settings page
3. Find "GPU Temperature Threshold" setting
4. Change threshold to 70°C (down from default 80°C)
5. Save settings
6. Return to Mining → GPU Mining tab
7. Start GPU mining and wait for temperature to exceed 70°C

**Expected Results**:
- [ ] Settings page allows threshold input (range: 60°C - 95°C)
- [ ] Invalid values (e.g., 50°C or 100°C) show validation error
- [ ] Valid threshold saves successfully
- [ ] Settings persist after app restart (reload app, check threshold still 70°C)
- [ ] When GPU temp exceeds 70°C:
  - GPU card displays visual warning (e.g., orange/red border or icon)
  - Temperature value highlighted with warning color

**Validation**: FR-011, FR-011a, FR-011b, NFR-006

---

### Scenario 5: Thermal Throttling

**User Story**: As a miner, I want automatic thermal protection to prevent GPU damage

**Steps**:
1. Set temperature threshold to 75°C (Settings page)
2. Start GPU mining with all GPUs
3. Block GPU airflow (carefully cover fan intake) or increase mining intensity
4. Wait for temperature to exceed 75°C
5. Observe throttling behavior
6. Restore airflow/reduce load
7. Wait for temperature to drop below 70°C (threshold - 5°C hysteresis)

**Expected Results**:
- [ ] When temp exceeds 75°C:
  - GPU status changes to "Throttled"
  - Mining intensity reduced incrementally (e.g., 100% → 90% → 80%)
  - Throttle percentage displayed on GPU card
  - Log message emitted: "GPU 0 throttled to 90% due to 76°C temperature"
- [ ] Hashrate decreases proportionally to throttle percentage
- [ ] When temp drops below 70°C:
  - GPU status returns to "Active"
  - Mining intensity restored to 100%
  - Log message: "GPU 0 restored to 100% after temperature normalized"
- [ ] Throttling prevents temperature from exceeding 85°C (safety validation)

**Validation**: FR-013a, FR-013b, FR-013c, FR-013d

---

### Scenario 6: Persistent Stats

**User Story**: As a miner, I want lifetime blocks found to persist across app restarts

**Steps**:
1. Start GPU mining on GPU 0
2. Mine until at least 1 block found (or manually increment in testing)
3. Note the "Lifetime blocks found" value (e.g., "3 blocks")
4. Stop mining
5. Close desktop app completely
6. Relaunch desktop app
7. Navigate to Mining → GPU Mining tab

**Expected Results**:
- [ ] Lifetime blocks found for GPU 0 matches value before restart
- [ ] File exists: `~/.btpc/data/mining_stats_per_gpu.json`
- [ ] File content shows correct blocks found:
  ```json
  {
    "gpu_0": {
      "blocks_found": 3,
      "last_updated": "2025-11-11T10:00:00Z"
    }
  }
  ```
- [ ] Stats persist even after crash (kill -9 process, verify on restart)

**Validation**: FR-019, NFR-012

---

### Scenario 7: Multi-GPU Dashboard (10+ GPUs)

**User Story**: As a server miner, I want the dashboard to handle 10+ GPUs without lag

**Prerequisites**: System with 10+ GPUs (or simulate with mock data)

**Steps**:
1. Launch app with 10+ GPUs enumerated
2. Navigate to GPU Mining tab
3. Measure UI render time (Chrome DevTools Performance tab)
4. Start mining on all GPUs
5. Observe real-time updates with all GPUs active

**Expected Results**:
- [ ] All 10+ GPUs render without UI slowdown
- [ ] Initial render completes in < 200ms
- [ ] Stats updates every 5 seconds without frame drops
- [ ] Scrolling remains smooth (60 FPS)
- [ ] GPU cards displayed in grid or list layout (no horizontal overflow)

**Validation**: FR-016, NFR-003

---

### Scenario 8: GPU Hot-Plug (Advanced)

**User Story**: As a miner, I want the dashboard to detect GPU changes without restart

**Prerequisites**: System with removable GPU (eGPU via Thunderbolt) or VM with hot-plug support

**Steps**:
1. Start with 2 GPUs enumerated
2. Note GPU count on dashboard
3. Remove 1 GPU (unplug eGPU or remove from VM)
4. Wait 10 seconds
5. Add GPU back
6. Wait 10 seconds

**Expected Results**:
- [ ] Dashboard updates GPU count when GPU removed (shows "1 GPU")
- [ ] Removed GPU marked as "Unavailable" or removed from list
- [ ] When GPU re-added, dashboard re-enumerates and shows "2 GPUs"
- [ ] No application crashes during hot-plug events
- [ ] Lifetime stats preserved for re-added GPU (if same device)

**Validation**: FR-003, NFR-011

---

### Scenario 9: Article XI Compliance

**User Story**: As a developer, I want to verify constitutional compliance

**Steps**:
1. Open Chrome DevTools Console (F12)
2. Navigate to Mining → GPU Mining tab
3. Monitor console for backend events
4. Change temperature threshold in Settings
5. Verify event-driven updates

**Expected Results**:
- [ ] Console shows `gpu-stats-updated` event every 5 seconds
- [ ] Event payload contains full dashboard data (devices, stats, health)
- [ ] NO JavaScript errors about authoritative state
- [ ] Temperature threshold: backend validates FIRST, then emits event
- [ ] Frontend saves to localStorage AFTER backend validation succeeds
- [ ] Event listeners cleaned up on page navigation (check with browser memory profiler)

**Validation**: Article XI Sections 11.1, 11.2, 11.3, 11.6, 11.7

---

## Acceptance Criteria Summary

**All scenarios must pass** for Feature 012 to be considered complete:

| Scenario | Status | Notes |
|----------|--------|-------|
| 1. GPU Enumeration | ⬜ Pending | |
| 2. Start GPU Mining & View Stats | ⬜ Pending | |
| 3. GPU Health Monitoring | ⬜ Pending | |
| 4. Temperature Warning Threshold | ⬜ Pending | |
| 5. Thermal Throttling | ⬜ Pending | |
| 6. Persistent Stats | ⬜ Pending | |
| 7. Multi-GPU Dashboard (10+ GPUs) | ⬜ Pending | |
| 8. GPU Hot-Plug (Advanced) | ⬜ Pending | |
| 9. Article XI Compliance | ⬜ Pending | |

---

## Performance Validation

**Benchmark Requirements**:
- [ ] GPU enumeration: < 500ms (NFR-001)
- [ ] Stats update overhead: < 1% hashrate impact (NFR-002)
- [ ] UI render time (10 GPUs): < 200ms (NFR-003)
- [ ] Sensor polling: No noticeable mining latency (NFR-004)

**Profiling Tools**:
- Chrome DevTools Performance tab (UI render time)
- `cargo flamegraph` (Rust backend profiling)
- `nvidia-smi dmon` (GPU utilization monitoring)

---

## Edge Case Testing

**Additional Scenarios** (optional, for comprehensive validation):

1. **No GPUs Detected**:
   - Remove all GPUs or run on system without OpenCL
   - Expected: Dashboard shows "No GPUs detected" message (no crash)

2. **GPU Driver Crash**:
   - Simulate driver crash during mining
   - Expected: GPU status shows "Error", user sees actionable message

3. **Invalid Temperature Threshold**:
   - Attempt to set threshold to 50°C or 100°C
   - Expected: Backend validation error, no localStorage save

4. **Concurrent Mining Operations**:
   - Start GPU mining, then start CPU mining
   - Expected: Both mining types run independently, no conflicts

5. **Long-Running Stability**:
   - Mine for 24+ hours continuously
   - Expected: No memory leaks, stats remain accurate, no crashes

---

**Quickstart Complete**: 2025-11-11
**Next Phase**: Run `/tasks` command to generate task implementation plan