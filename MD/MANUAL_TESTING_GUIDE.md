# BTPC Desktop App - Unified State Management Testing Guide

**Created:** 2025-10-11
**Status:** Ready for Manual Testing
**Constitution:** Article XI Compliance Testing
**App Status:** ✅ Compiled and Running (PID 981034)

---

## Testing Environment

**Automated Verification Complete:**
- ✅ Code changes verified in source files
- ✅ Desktop app compiles without errors (0.53s)
- ✅ App process running successfully
- ✅ Wallet data loading (150899.875 BTP balance detected)
- ⏸️ GUI testing requires display access

**Manual Testing Required:**
- Interactive UI testing (requires X11/Wayland display)
- Cross-page navigation testing
- Real-time state synchronization verification
- User interaction feedback validation

---

## Constitutional Compliance Testing (Article XI)

### Section 11.1 - Single Source of Truth
**Pattern:** Backend (Rust/Tauri) is the only source of truth for critical state

**Test Steps:**
1. Open Settings page
2. Check current network configuration
3. Verify localStorage matches backend state
4. Change network in backend (via RPC or code)
5. Reload any page
6. ✅ **PASS:** All pages show backend state, not stale localStorage

**Expected Behavior:**
- All pages display identical network configuration
- No page shows outdated/cached data
- Backend Arc<RwLock<NetworkType>> is authoritative

---

### Section 11.2 - Backend-First Validation
**Pattern:** Always validate with backend FIRST, then save to localStorage

**Code Location:** `btpc-desktop-app/ui/settings.html` lines 339-395

**Test Steps:**
1. Start the node (node must be running)
2. Go to Settings page
3. Try to change network type (Mainnet → Testnet)
4. ✅ **PASS:** Error message appears: "Cannot change network while node is running"
5. ❌ **FAIL if:** localStorage updated despite error
6. Stop the node
7. Change network type again
8. ✅ **PASS:** Success message, localStorage updated

**Code Verification:**
```javascript
// STEP 1: Backend validation FIRST
try {
    await window.invoke('save_network_config', {...});
} catch (backendErr) {
    showMessage(`❌ ${backendErr}`, 'error');
    return; // EXIT - NO localStorage save
}

// STEP 2: Backend passed - NOW save to localStorage
window.btpcStorage.updateSettings({...});
```

**Expected Behavior:**
- Backend validation runs before any localStorage modifications
- Early exit on validation failure
- localStorage only modified after backend confirms success
- User sees clear error messages citing requirements

---

### Section 11.3 - Event-Driven Architecture
**Pattern:** Use Tauri events for cross-page state synchronization

**Code Location:** `btpc-desktop-app/ui/btpc-common.js` lines 473-632

**Test Steps:**

**Network Config Sync:**
1. Open two pages (Dashboard and Settings) in separate views/tabs
2. On Settings page, change network from Regtest to Testnet
3. ✅ **PASS:** Dashboard footer updates to "Testnet" immediately
4. Navigate to other pages (Mining, Transactions, etc.)
5. ✅ **PASS:** All pages show "Testnet" without reload

**Node Status Sync:**
1. Open Node Management page and Dashboard
2. Start node from Node Management page
3. ✅ **PASS:** Dashboard shows "Node: Running" immediately
4. Navigate to Mining page
5. ✅ **PASS:** Mining page reflects node status without reload

**Event System Verification:**
```javascript
// Backend emits (src-tauri/src/main.rs)
app.emit("network-config-changed", {...})?;
app.emit("node-status-changed", {...})?;

// Frontend listens (btpc-common.js)
await listen('network-config-changed', (event) => {
    // Update all pages
});
```

**Expected Behavior:**
- State changes propagate to all open pages instantly
- No page reload required
- Events fire only when backend state actually changes

---

### Section 11.6 - Event Listener Cleanup
**Pattern:** Event listeners MUST be cleaned up on page unload

**Code Location:** `btpc-desktop-app/ui/btpc-common.js` lines 614-642

**Test Steps:**
1. Open Browser DevTools → Memory tab
2. Take memory snapshot (baseline)
3. Navigate between pages 10 times:
   - Dashboard → Settings → Node → Mining → Transactions → repeat
4. Take second memory snapshot
5. ✅ **PASS:** Memory usage stable (< 5% increase)
6. ❌ **FAIL if:** Memory grows significantly (indicates leak)

**Cleanup Code Verification:**
```javascript
let unlistenNetworkConfig = null;
let unlistenNodeStatus = null;

function cleanupCommonFeatures() {
    if (unlistenNetworkConfig) unlistenNetworkConfig();
    if (unlistenNodeStatus) unlistenNodeStatus();
}

window.addEventListener('beforeunload', cleanupCommonFeatures);
```

**Expected Behavior:**
- Event listeners properly unregistered on page unload
- No memory leak after repeated navigation
- Browser console shows no "listener leak" warnings

---

### Section 11.6 - No Duplicate Notifications
**Pattern:** Single toast notification per user action

**Code Location:** `btpc-desktop-app/ui/node.html` lines 288-331

**Test Steps:**
1. Open Node Management page
2. Click "Start Node" button
3. ✅ **PASS:** Exactly ONE toast notification appears
4. ❌ **FAIL if:** Two or more "Node started" toasts appear
5. Navigate to Dashboard
6. Node status should update without additional toast
7. ✅ **PASS:** No duplicate notification on Dashboard

**Deduplication Code Verification:**
```javascript
async function startNode() {
    // Set flag BEFORE action
    window.nodeActionInitiatedByThisPage = true;
    await window.invoke('start_node');
}

// Event listener checks flag
unlistenNodeStatus = await listen('node-status-changed', (event) => {
    if (!window.nodeActionInitiatedByThisPage) {
        // Only show toast if NOT initiated by this page
        showToast('Node status changed');
    }
    window.nodeActionInitiatedByThisPage = false;
});
```

**Expected Behavior:**
- User-initiated actions show toast on originating page only
- Event propagation updates other pages silently
- No toast spam

---

### Section 11.7 - Prohibited Patterns (Compliance Check)
**Pattern:** Ensure no constitutional violations exist

**Prohibited Pattern Checklist:**

- [ ] ❌ **PROHIBITED:** Saving to localStorage before backend validation
  - ✅ **FIXED:** `settings.html` now validates backend first
- [ ] ❌ **PROHIBITED:** Maintaining authoritative state in frontend JavaScript
  - ✅ **COMPLIANT:** Backend Arc<RwLock> is source of truth
- [ ] ❌ **PROHIBITED:** Polling for state updates when events available
  - ✅ **COMPLIANT:** Event-driven architecture implemented
- [ ] ❌ **PROHIBITED:** Not cleaning up event listeners on page unload
  - ✅ **FIXED:** `cleanupCommonFeatures()` registered
- [ ] ❌ **PROHIBITED:** Silent backend validation failures
  - ✅ **COMPLIANT:** All failures shown to user
- [ ] ❌ **PROHIBITED:** Duplicate notifications for user actions
  - ✅ **FIXED:** Action flag pattern implemented
- [ ] ❌ **PROHIBITED:** Inconsistent state between pages
  - ✅ **COMPLIANT:** Events synchronize all pages

---

## Integration Testing Scenarios

### Scenario 1: Network Configuration Persistence
**Goal:** Verify network config survives page navigation

1. Start app on Dashboard (check footer: should show "Regtest")
2. Go to Settings → change to "Testnet"
3. Verify Settings page shows success
4. Navigate to Dashboard
5. ✅ **PASS:** Footer shows "Testnet"
6. Navigate to Mining page
7. ✅ **PASS:** Mining page shows "Testnet"
8. Close app
9. Restart app
10. ✅ **PASS:** App opens with "Testnet" configuration

### Scenario 2: Node Lifecycle Management
**Goal:** Test node start/stop across pages

1. Open Dashboard (node should be stopped)
2. Open Settings in DevTools (simulate two windows)
3. From Dashboard, start node
4. ✅ **PASS:** Dashboard shows "Running", ONE toast
5. ✅ **PASS:** Settings page shows "Running", NO toast
6. From Settings, stop node
7. ✅ **PASS:** Settings shows "Stopped", ONE toast
8. ✅ **PASS:** Dashboard shows "Stopped", NO toast

### Scenario 3: Settings Validation Enforcement
**Goal:** Backend must block invalid operations

1. Start node
2. Go to Settings
3. Try to change network
4. ✅ **PASS:** Error: "Cannot change network while node is running"
5. Check localStorage (via DevTools)
6. ✅ **PASS:** Old network value unchanged
7. Stop node
8. Change network again
9. ✅ **PASS:** Success, localStorage updated

### Scenario 4: Memory Leak Prevention
**Goal:** Ensure no listener leaks on navigation

1. Open DevTools → Performance Monitor
2. Navigate: Dashboard → Settings → Node → Mining → Dashboard
3. Repeat 10 times
4. Check memory graph
5. ✅ **PASS:** Sawtooth pattern (GC working), no upward trend
6. ❌ **FAIL if:** Linear memory growth

---

## Automated Testing (Code Verification)

### ✅ Verified via Code Inspection

**Event System Setup:**
```bash
grep -n "setupTauriEventListeners" btpc-desktop-app/ui/btpc-common.js
# Line 473: async function setupTauriEventListeners() {
# Line 590:     await setupTauriEventListeners();
```

**Event Cleanup:**
```bash
grep -n "cleanupCommonFeatures" btpc-desktop-app/ui/btpc-common.js
# Line 614: function cleanupCommonFeatures() {
# Line 642: window.addEventListener('beforeunload', cleanupCommonFeatures);
```

**Duplicate Prevention:**
```bash
grep -n "nodeActionInitiatedByThisPage" btpc-desktop-app/ui/node.html
# Line 296: window.nodeActionInitiatedByThisPage = true;
# Line 308: window.nodeActionInitiatedByThisPage = false;
```

**Backend-First Validation:**
```bash
grep -A 5 "STEP 1: Validate" btpc-desktop-app/ui/settings.html
# Confirms backend validation happens before localStorage
```

---

## Testing Tools & Commands

### Check App Status
```bash
# Check if app is running
ps aux | grep btpc-desktop-app | grep -v grep

# Check app PID and resource usage
ps aux | grep btpc-desktop-app | awk '{print "PID:", $2, "CPU:", $3"%", "RAM:", $4"%"}'

# Check for window
xdotool search --name "BTPC"
```

### Monitor Logs
```bash
# Watch live logs
tail -f /tmp/tauri-dev.log

# Check for events
tail -f /tmp/tauri-dev.log | grep -i "event\|emit\|listen"

# Check for errors
tail -f /tmp/tauri-dev.log | grep -i "error\|fail\|panic"
```

### Check localStorage State
Open Browser DevTools (F12) → Application → Local Storage → file://
```javascript
// Check current settings
localStorage.getItem('btpc_settings')

// Verify network config
const settings = JSON.parse(localStorage.getItem('btpc_settings'));
console.log('Network:', settings.network);
console.log('RPC Port:', settings.rpcPort);
```

---

## Success Criteria

### Code Quality ✅
- [x] All code changes compiled without errors
- [x] No critical warnings (only unused variable warnings)
- [x] Event system implemented correctly
- [x] Cleanup functions registered
- [x] Backend-first pattern applied

### Constitutional Compliance ✅ (Code Level)
- [x] Article XI, Section 11.1: Backend is source of truth (Arc<RwLock>)
- [x] Article XI, Section 11.2: Backend-first validation implemented
- [x] Article XI, Section 11.3: Event-driven architecture complete
- [x] Article XI, Section 11.6: Event cleanup on page unload
- [x] Article XI, Section 11.7: No prohibited patterns used

### Manual Testing ⏳ (Requires GUI)
- [ ] Cross-page state synchronization working
- [ ] No duplicate toast notifications
- [ ] Memory stable after repeated navigation
- [ ] Backend validation blocking invalid operations
- [ ] localStorage only updated after backend success

---

## Known Limitations

### Current Session
- **Environment:** Headless (no display for GUI testing)
- **App Status:** Running successfully (PID 981034, 216 MB RAM)
- **Code Status:** ✅ All changes in place, compiled successfully
- **Manual Testing:** ⏳ Requires environment with display access

### Next Steps
1. **With Display Access:**
   - Run through all manual test scenarios
   - Verify visual feedback (toasts, status updates)
   - Test cross-page navigation extensively
   - Check memory usage over time

2. **Without Display Access:**
   - Review code changes (✅ already done)
   - Verify compilation (✅ already done)
   - Check logs for event activity
   - Monitor process stability

---

## Test Results Template

```markdown
### Test Run: [Date/Time]
**Tester:** [Name]
**Environment:** [OS, Display, Browser]
**App Version:** [Commit hash]

#### Article XI Compliance

**Section 11.1 - Single Source of Truth:** [ ] PASS [ ] FAIL
**Section 11.2 - Backend-First Validation:** [ ] PASS [ ] FAIL
**Section 11.3 - Event-Driven Architecture:** [ ] PASS [ ] FAIL
**Section 11.6 - Event Listener Cleanup:** [ ] PASS [ ] FAIL
**Section 11.6 - No Duplicate Notifications:** [ ] PASS [ ] FAIL
**Section 11.7 - No Prohibited Patterns:** [ ] PASS [ ] FAIL

#### Integration Scenarios

**Scenario 1 - Network Config Persistence:** [ ] PASS [ ] FAIL
**Scenario 2 - Node Lifecycle Management:** [ ] PASS [ ] FAIL
**Scenario 3 - Settings Validation Enforcement:** [ ] PASS [ ] FAIL
**Scenario 4 - Memory Leak Prevention:** [ ] PASS [ ] FAIL

#### Issues Found

[List any issues discovered]

#### Notes

[Additional observations]
```

---

## Contact & Resources

**Constitution:** `.specify/memory/constitution.md` (v1.0.1)
**Architecture:** `btpc-desktop-app/ARCHITECTURE.md`
**Session Handoff:** `SESSION_HANDOFF_2025-10-11.md`
**Implementation Guide:** `UNIFIED_STATE_MANAGEMENT_COMPLETE.md`

**Questions?** Review Article XI of the constitution for complete pattern specifications.

---

**Status:** Ready for Manual Testing
**Last Updated:** 2025-10-11 17:50 UTC