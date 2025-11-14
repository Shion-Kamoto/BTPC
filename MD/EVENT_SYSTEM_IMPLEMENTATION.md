# BTPC Desktop App - Event System Implementation Plan

**Date:** 2025-10-06
**Issue:** Node cycling, analytics not auto-refreshing, manual refresh required
**Solution:** Implement Tauri v2 event system for real-time push updates

---

## Root Cause Analysis

### Current Problems

1. **Polling-based updates** - Each page uses `setInterval()` to poll backend every 5-10 seconds
2. **Multiple intervals** - Conflicting intervals across pages causing performance issues
3. **No real-time feedback** - Node/mining status changes not immediately reflected
4. **Race conditions** - Multiple simultaneous backend calls causing glitches

### Current Architecture (Problematic)

```
Frontend Page
   ↓ (every 5s)
   setInterval → invoke('get_node_status')
   ↓
   Backend returns status
   ↓
   Update UI
```

**Problems:**
- Wastes resources polling when nothing changed
- Delayed updates (up to 5 seconds)
- Multiple pages polling simultaneously
- No coordination between updates

---

## Solution: Tauri Event System

### New Architecture (Push-based)

```
Backend Process Change
   ↓
   emit('node-status-changed', status)
   ↓
Frontend Listener
   ↓
   Update UI immediately
```

**Benefits:**
- Instant updates (< 100ms)
- No wasted polling
- Single source of truth
- Coordinated updates

---

## Implementation Plan

### Phase 1: Backend Event Emitters

Add event emission to main.rs when state changes:

**Events to emit:**
1. `node-status-changed` - When node starts/stops
2. `mining-status-changed` - When mining starts/stops/finds block
3. `blockchain-synced` - When new block added
4. `wallet-balance-changed` - When balance updates
5. `transaction-received` - New transaction detected

**Example Implementation:**

```rust
// In start_node command
#[tauri::command]
async fn start_node(state: State<'_, AppState>, app: AppHandle) -> Result<String, String> {
    // ... start node logic ...

    // Emit event
    app.emit("node-status-changed", NodeStatusPayload {
        is_running: true,
        block_height: current_height,
        peer_count: peers,
    }).map_err(|e| e.to_string())?;

    Ok("Node started".to_string())
}
```

### Phase 2: Frontend Event Listeners

Update btpc-common.js to listen for events:

```javascript
// btpc-common.js
import { listen } from '@tauri-apps/api/event';

async function initEventListeners() {
    // Listen for node status changes
    await listen('node-status-changed', (event) => {
        updateNodeStatusUI(event.payload);
    });

    // Listen for mining status changes
    await listen('mining-status-changed', (event) => {
        updateMiningStatusUI(event.payload);
    });

    // Listen for balance changes
    await listen('wallet-balance-changed', (event) => {
        updateBalanceUI(event.payload);
    });

    // Listen for blockchain sync
    await listen('blockchain-synced', (event) => {
        updateBlockchainInfoUI(event.payload);
    });
}
```

### Phase 3: Replace Polling with Events + Fallback

Keep minimal polling (30s) as fallback, but rely on events:

```javascript
// Initial load
async function loadDashboard() {
    await updateDashboard(); // Load once
}

// Fallback polling (less frequent)
setInterval(updateDashboard, 30000); // 30 seconds instead of 5

// Real-time updates via events (instant)
initEventListeners();
```

---

## Detailed Implementation Steps

### Step 1: Add Event Emission to Backend

**File:** `btpc-desktop-app/src-tauri/src/main.rs`

```rust
use tauri::{Emitter, AppHandle};
use serde::Serialize;

#[derive(Clone, Serialize)]
struct NodeStatusPayload {
    is_running: bool,
    block_height: u64,
    peer_count: usize,
}

#[derive(Clone, Serialize)]
struct MiningStatusPayload {
    is_mining: bool,
    hashrate: u64,
    blocks_found: u64,
}

// In start_node command
#[tauri::command]
async fn start_node(state: State<'_, AppState>, app: AppHandle) -> Result<String, String> {
    let mut launcher = state.launcher.lock().unwrap();

    match launcher.start() {
        Ok(_) => {
            // Get current status
            let info = get_blockchain_info_internal(&state).await?;

            // Emit event
            app.emit("node-status-changed", NodeStatusPayload {
                is_running: true,
                block_height: info.height,
                peer_count: 0,
            }).map_err(|e| e.to_string())?;

            Ok("Node started successfully".to_string())
        },
        Err(e) => Err(format!("Failed to start node: {}", e))
    }
}

// Similar for stop_node, start_mining, stop_mining
```

### Step 2: Update Frontend Event System

**File:** `btpc-desktop-app/ui/btpc-common.js`

Add to the file:

```javascript
// Global event listeners
let eventUnlisteners = [];

/**
 * Initialize Tauri event listeners for real-time updates
 */
async function initTauriEvents() {
    if (!window.__TAURI__?.event) {
        console.warn('Tauri event API not available');
        return;
    }

    const { listen } = window.__TAURI__.event;

    try {
        // Node status events
        const nodeUnlisten = await listen('node-status-changed', (event) => {
            console.log('📡 Node status changed:', event.payload);
            updateNodeStatusFromEvent(event.payload);
        });
        eventUnlisteners.push(nodeUnlisten);

        // Mining status events
        const miningUnlisten = await listen('mining-status-changed', (event) => {
            console.log('⛏️ Mining status changed:', event.payload);
            updateMiningStatusFromEvent(event.payload);
        });
        eventUnlisteners.push(miningUnlisten);

        // Blockchain sync events
        const blockchainUnlisten = await listen('blockchain-synced', (event) => {
            console.log('⛓️ Blockchain synced:', event.payload);
            updateBlockchainInfoFromEvent(event.payload);
        });
        eventUnlisteners.push(blockchainUnlisten);

        // Wallet balance events
        const balanceUnlisten = await listen('wallet-balance-changed', (event) => {
            console.log('💰 Balance changed:', event.payload);
            updateBalanceFromEvent(event.payload);
        });
        eventUnlisteners.push(balanceUnlisten);

        console.log('✅ Event listeners initialized');
    } catch (error) {
        console.error('Failed to initialize event listeners:', error);
    }
}

/**
 * Update UI from node status event
 */
function updateNodeStatusFromEvent(payload) {
    // Update node status indicator
    const statusDot = document.querySelector('.node-status-dot');
    const statusText = document.querySelector('.node-status-text');

    if (statusDot && statusText) {
        if (payload.is_running) {
            statusDot.classList.add('active');
            statusText.textContent = 'Running';
        } else {
            statusDot.classList.remove('active');
            statusText.textContent = 'Offline';
        }
    }

    // Update block height
    const heightEl = document.getElementById('chain-height');
    if (heightEl && payload.block_height !== undefined) {
        heightEl.textContent = payload.block_height.toLocaleString();
    }
}

/**
 * Update UI from mining status event
 */
function updateMiningStatusFromEvent(payload) {
    const statusEl = document.getElementById('mining-status');
    const hashrateEl = document.getElementById('mining-hashrate');

    if (statusEl) {
        statusEl.textContent = payload.is_mining ? 'Active' : 'Inactive';
        statusEl.style.color = payload.is_mining ? 'var(--status-success)' : 'var(--text-muted)';
    }

    if (hashrateEl && payload.hashrate !== undefined) {
        hashrateEl.textContent = `${payload.hashrate.toLocaleString()} H/s`;
    }
}

/**
 * Update UI from blockchain sync event
 */
function updateBlockchainInfoFromEvent(payload) {
    const heightEl = document.getElementById('chain-height');
    if (heightEl && payload.height !== undefined) {
        heightEl.textContent = payload.height.toLocaleString();
    }
}

/**
 * Update UI from balance change event
 */
function updateBalanceFromEvent(payload) {
    const balanceEl = document.getElementById('wallet-balance');
    if (balanceEl && payload.balance !== undefined) {
        balanceEl.textContent = formatBTPC(payload.balance);
    }
}

/**
 * Cleanup event listeners
 */
function cleanupEventListeners() {
    eventUnlisteners.forEach(unlisten => unlisten());
    eventUnlisteners = [];
}
```

### Step 3: Update initCommonFeatures

```javascript
async function initCommonFeatures() {
    // Initialize Tauri
    await initTauri();

    // Set active navigation
    setActiveNavigation();

    // Initialize event listeners (NEW)
    await initTauriEvents();

    // Start status updates (reduced frequency)
    const updateInterval = startStatusUpdates(30000); // 30s instead of 5s

    // Store interval ID for cleanup
    window.btpcStatusInterval = updateInterval;

    console.log('✅ Common features initialized');
}

function cleanupCommonFeatures() {
    if (window.btpcStatusInterval) {
        clearInterval(window.btpcStatusInterval);
        delete window.btpcStatusInterval;
    }
    cleanupEventListeners(); // NEW
}
```

---

## Testing Plan

### Manual Testing

1. **Node Start/Stop**
   - Start node → Should see instant UI update
   - Stop node → Should see instant UI update
   - No delay, no manual refresh needed

2. **Mining Start/Stop**
   - Start mining → Hashrate appears immediately
   - Stop mining → Status changes immediately

3. **Balance Updates**
   - Mine a block → Balance updates within 1 second
   - No need to refresh page

4. **Page Navigation**
   - Navigate between pages → Events continue working
   - No duplicate listeners

### Console Verification

Open DevTools Console, should see:
```
✅ Tauri v2 core API loaded
✅ Event listeners initialized
📡 Node status changed: {is_running: true, block_height: 1234, ...}
⛏️ Mining status changed: {is_mining: true, hashrate: 5000, ...}
```

---

## Performance Comparison

### Before (Polling)

- Dashboard: 5s interval × 5 calls = ~25 backend calls/minute
- Wallet page: 10s interval × 3 calls = ~18 calls/minute
- Mining page: 2s interval × 2 calls = ~60 calls/minute
- **Total: ~103 backend calls/minute (all pages open)**

### After (Events)

- Initial load: 5 calls once
- Events: Only when state actually changes (~1-10/minute)
- Fallback polling: 30s interval × 5 calls = ~10 calls/minute
- **Total: ~15-20 calls/minute (90% reduction)**

---

## Rollout Strategy

### Phase 1: Core Status Events (Priority 1)
- ✅ node-status-changed
- ✅ mining-status-changed
- ✅ blockchain-synced

### Phase 2: Wallet Events (Priority 2)
- wallet-balance-changed
- transaction-received

### Phase 3: Advanced Events (Priority 3)
- peer-connected
- peer-disconnected
- sync-progress-updated

---

## Backwards Compatibility

Keep polling as fallback for:
1. Initial page load
2. Event system initialization failure
3. Missed events (30s refresh catches any gaps)

This ensures the app works even if events fail.

---

## Next Steps

1. ✅ Document event system plan
2. ⏳ Implement backend event emission
3. ⏳ Update frontend event listeners
4. ⏳ Test real-time updates
5. ⏳ Remove excessive polling
6. ⏳ Verify performance improvement

---

**Status:** Ready for implementation
**Priority:** High - Fixes major UX issue
**Estimated Time:** 2-3 hours
