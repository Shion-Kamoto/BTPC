# Quickstart: Node and Backend Stability Fixes Testing

**Feature**: 003-fix-node-and
**Date**: 2025-10-25
**Purpose**: Manual test procedure to validate all acceptance scenarios from spec.md

This guide walks through manual testing of the stability fixes to verify all functional requirements are met.

---

## Prerequisites

Before starting tests:
- [ ] btpc-desktop-app built in release mode (`cargo build --release`)
- [ ] btpc_node, btpc_miner, btpc_wallet binaries available in `~/.btpc/bin/`
- [ ] Test wallet with known balance (e.g., 226.625 BTP across 7 UTXOs)
- [ ] Clean state: `rm -rf ~/.btpc/*.lock ~/.btpc/*.json` (removes stale state)

---

## Test Scenario 1: Wallet Balance Display

**Spec Reference**: spec.md lines 63-67 (FR-001, FR-002, FR-003, FR-004, FR-005)

### Setup
```bash
# Create test wallet with known balance
cd btpc-core
./target/release/btpc_wallet create testingW2 --network regtest

# Send test transaction (7 UTXOs totaling 226.625 BTP)
# (Assumes mining script has created these UTXOs)
```

### Test Steps
1. Launch desktop app: `./btpc-desktop-app/src-tauri/target/release/btpc-desktop-app`
2. Navigate to **Wallet Manager** page
3. Load wallet `testingW2`

**Expected Results**:
- ✅ Balance displays **226.62500000 BTP** (FR-001)
- ✅ UTXO count shows **7 UTXOs** (FR-004)
- ✅ Each UTXO details clickable (FR-004)
- ✅ Zero-balance wallet shows **0.00000000 BTP** not blank (FR-005)

**Automated Test Location**: `tests/integration/balance_display.spec.js`

---

## Test Scenario 2: Mining Operations

**Spec Reference**: spec.md lines 69-74 (FR-011 through FR-015, FR-048, FR-049, FR-050)

### Test Steps
1. Launch desktop app
2. Navigate to **Node** page, click **Start Node**
3. Wait for node to sync (status shows "Running")
4. Navigate to **Mining** page, click **Start Mining**
5. **Trigger error**: Stop node while mining is active

**Expected Results**:
- ✅ Mining starts successfully (FR-011)
- ✅ Hashrate updates in real-time (FR-012)
- ✅ Error message appears: "Mining requires node to be running" (FR-013, FR-017)
- ✅ Error has **"Show Details"** button (FR-048)
- ✅ Clicking "Show Details" reveals technical stack trace (FR-049)
- ✅ **Copy to Clipboard** button works (FR-050)
- ✅ App remains responsive, does not crash (FR-016)
- ✅ Mining can be restarted after fixing error (FR-013)

**Automated Test Location**: `tests/integration/mining_operations.spec.js`

---

## Test Scenario 3: Node Management

**Spec Reference**: spec.md lines 76-81 (FR-006 through FR-010, FR-041, FR-042)

### Test Steps (First Launch)
1. Launch app **for the first time** (clean `~/.btpc/` directory)
2. **Expected**: Modal appears asking "Auto-start node on launch?" (FR-041)
3. Select **"Yes"** or **"No"**
4. **Expected**: Preference saved to `~/.btpc/node_config.json` (FR-042)

### Test Steps (Graceful Shutdown)
1. Start node via desktop app
2. Close application window
3. Wait 10 seconds
4. Check running processes: `ps aux | grep btpc_node`

**Expected Results**:
- ✅ Node starts/stops via desktop interface (FR-006)
- ✅ Node shutdown completes within 10 seconds (FR-007)
- ✅ No `btpc_node` process remains after app close (FR-010)
- ✅ No zombie processes: `ps aux | grep defunct` (FR-010)
- ✅ Database closes cleanly (check RocksDB logs for corruption warnings)

**Single-Instance Protection Test**:
1. Start desktop app (instance 1)
2. Try to start second instance: `./btpc-desktop-app`
3. **Expected**: Error message "Another instance is already running" (FR-008)
4. Lock file exists: `ls ~/.btpc/app.lock`

**Automated Test Location**: `tests/integration/node_management.spec.js`

---

## Test Scenario 4: Application Lifecycle (7-Day Stress Test)

**Spec Reference**: spec.md lines 83-88 (FR-021, NFR-008, NFR-009, NFR-017, NFR-012)

### Automated Test Execution
```bash
# Run 7-day memory leak test
cd tests/stress
./seven_day_test.sh

# Monitor output in real-time
tail -f memory_leak_test_*.log
```

### Manual Monitoring
Every 24 hours during the test:
1. Check app responsiveness: Navigate between pages (Dashboard, Mining, Wallet)
2. Check memory usage: `ps aux | grep btpc-desktop-app | awk '{print $6/1024 " MB"}'`
3. Log memory reading in separate file

**Expected Results**:
- ✅ Memory usage stays below **1GB** (NFR-008)
- ✅ Memory growth < 5% over 7 days (NFR-017)
- ✅ App remains responsive (< 100ms page navigation) (NFR-015)
- ✅ 1000+ mining start/stop cycles complete without degradation (NFR-012)
- ✅ No crashes during 7-day period (NFR-009: 99.9% uptime)

**Analysis**:
```bash
# After 7 days, analyze results
python3 ../MD/analyze_memory_log.py tests/stress/memory_leak_test_*.log
```

Expected output:
```
Initial memory: 450MB
Final memory: 465MB
Growth: 15MB (3.33%)
PASS: Memory growth within 5% threshold
```

---

## Test Scenario 5: Error Recovery

**Spec Reference**: spec.md lines 90-96 (FR-016 through FR-020)

### Test Steps (RPC Timeout)
1. Start desktop app with node running
2. Start mining
3. **Kill node process manually**: `killall btpc_node`
4. Observe error handling

**Expected Results**:
- ✅ Error message displays: "Node connection lost. Mining stopped." (FR-017, FR-020)
- ✅ Mining stops gracefully (FR-019)
- ✅ Wallet page still functional (FR-016: no cascade crash)
- ✅ Dashboard page still updates (Article XI: event-driven updates)
- ✅ App does not crash or freeze (FR-016)

### Test Steps (Database Lock)
1. Start desktop app
2. Manually lock database: `flock ~/.btpc/blockchain.db -c 'sleep 60' &`
3. Try to start node

**Expected Results**:
- ✅ Error message: "Database locked. Wait for other processes to release lock." (FR-017)
- ✅ Node start fails gracefully (FR-020)
- ✅ Error shows **"Show Details"** button (FR-048)
- ✅ App remains responsive (FR-016)

**Automated Test Location**: `tests/integration/error_recovery.spec.js`

---

## Test Scenario 6: Concurrent Operations

**Spec Reference**: spec.md lines 98-103 (FR-026 through FR-031, Article XI)

### Test Steps
1. Start desktop app, start node, start mining
2. Open **Dashboard** page → observe mining stats updating
3. Navigate to **Mining** page → observe same stats
4. Navigate to **Wallet** page → check balance
5. **While on Wallet page**, stop mining from Dashboard (open Dashboard in new browser context)
6. Return to Mining page

**Expected Results**:
- ✅ All pages show identical state at all times (FR-026: backend single source of truth)
- ✅ State updates propagate across pages in < 200ms (NFR-007)
- ✅ No duplicate toast notifications when stopping mining (FR-031)
- ✅ Balance updates automatically when transaction received (FR-002, FR-028)
- ✅ Event listeners clean up on page unload: Check browser console for warnings (FR-030)

### Event Listener Cleanup Test
1. Open browser DevTools → Console
2. Navigate between pages 50 times (Dashboard → Mining → Wallet → Settings → repeat)
3. Check for memory leaks: DevTools → Memory → Take heap snapshot
4. Look for leaked listeners in snapshot

**Expected Results**:
- ✅ No warnings about unremoved event listeners
- ✅ Heap snapshot shows constant listener count (not growing)

**Automated Test Location**: `tests/integration/state_consistency.spec.js`

---

## Test Scenario 7: Process Crash Recovery

**Spec Reference**: Edge cases (FR-039, FR-040, FR-046)

### Test Steps (First Crash)
1. Start desktop app, start mining
2. **Kill miner process**: `killall btpc_miner`
3. Wait 5 seconds

**Expected Results**:
- ✅ Crash detected within 5 seconds (FR-038)
- ✅ Miner automatically restarts (FR-039: auto-restart on first crash)
- ✅ No user notification (silent auto-restart)
- ✅ Mining resumes with same settings

### Test Steps (Second Crash)
1. Miner is running (after auto-restart from first crash)
2. **Kill miner process again**: `killall btpc_miner`
3. Wait 5 seconds

**Expected Results**:
- ✅ Crash detected within 5 seconds (FR-038)
- ✅ **Notification appears**: "Miner crashed. Restart?" with buttons (FR-040)
- ✅ Buttons: **"Restart"** and **"Cancel"** (FR-040)
- ✅ Clicking **"Cancel"** leaves miner stopped
- ✅ Clicking **"Restart"** starts miner again

### Test Steps (Crash Counter Reset)
1. After second crash, **let miner run for 1 hour** without crashing
2. **Kill miner process** (third time)

**Expected Results**:
- ✅ Auto-restart occurs (FR-046: crash counter reset after 1 hour stable)
- ✅ No notification (treated as first crash again)

**Automated Test Location**: `tests/integration/crash_recovery.spec.js`

---

## Test Scenario 8: Mining Resume Notification

**Spec Reference**: Clarifications (FR-043, FR-044, FR-045)

### Test Steps
1. Start desktop app, start node, start mining
2. **Close app while mining is active**
3. Restart app

**Expected Results**:
- ✅ **Notification appears**: "Mining was active when app closed. Resume mining?" (FR-044)
- ✅ Notification has **"Resume Mining"** and **"Don't Resume"** buttons (FR-045)
- ✅ Clicking **"Resume Mining"** starts miner automatically
- ✅ Clicking **"Don't Resume"** leaves miner stopped

### Negative Test
1. Start app, start node, **do not** start mining
2. Close app
3. Restart app

**Expected Results**:
- ✅ **No notification** appears (only shown if mining was active)

---

## Performance Benchmarks

Run these benchmarks to validate NFR requirements:

### Balance Calculation Performance (NFR-005)
```bash
# Test with 10,000 UTXOs
time curl -s http://localhost:8332/wallet/testingW2/balance

# Expected: < 500ms
```

### Application Startup Time (NFR-006)
```bash
# Measure app startup (no auto-start)
time ./src-tauri/target/release/btpc-desktop-app --startup-benchmark

# Expected: < 3 seconds to first UI render
```

### State Synchronization Latency (NFR-007)
```javascript
// In browser DevTools console
performance.mark('state-change-start');
await window.__TAURI__.invoke('start_mining');
window.__TAURI__.event.listen('mining_status_changed', () => {
    performance.mark('state-change-end');
    performance.measure('sync-latency', 'state-change-start', 'state-change-end');
    console.log(performance.getEntriesByName('sync-latency')[0].duration);
});

// Expected: < 200ms
```

---

## Regression Test Checklist

After all fixes, verify no regressions:

### Existing Functionality
- [ ] Wallet creation still works
- [ ] Transaction sending still works
- [ ] Block mining still works
- [ ] Network sync still works
- [ ] All Tauri commands still respond
- [ ] UI navigation still smooth

### Security
- [ ] Error messages don't expose private keys (NFR-003)
- [ ] Technical details sanitize passwords (NFR-004)
- [ ] File path validation works (NFR-001)
- [ ] Process argument sanitization works (NFR-002)

### Constitution Compliance
- [ ] Article XI patterns followed (backend-first, events)
- [ ] No unsafe code added without SAFETY comments
- [ ] Test coverage > 90% (run `cargo tarpaulin`)

---

## Success Criteria

All scenarios PASS when:
- ✅ All manual test steps produce expected results
- ✅ All automated tests pass: `cargo test && npm test`
- ✅ 7-day stress test shows < 5% memory growth
- ✅ No crashes during entire testing period
- ✅ Performance benchmarks meet NFR thresholds

**Final Validation**: Run `cargo clippy` and `cargo audit` with zero errors

---

**Next Step**: After manual testing complete, proceed to automated task execution via `/implement` command
