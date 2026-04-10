# Mining Status Enhancement - 2025-10-07

## Executive Summary

Comprehensive audit of mining status properties revealed **NO API MISMATCHES** between frontend and backend. All property names match correctly. However, discovered and fixed **1 missing feature**: Estimated Reward display was never populated.

---

## Audit Results

### ✅ Backend API (get_mining_status)

**File**: `src-tauri/src/main.rs:615`

**Returns**:
```json
{
  "is_mining": bool,
  "hashrate": number,
  "blocks_found": number
}
```

### ✅ Frontend Property Usage

**Files Checked**:
- `mining.html` (lines 413-414, 566-569)
- `btpc-update-manager.js` (lines 84, 92)
- `index.html` (lines 220-221)

**Properties Used**:
- ✅ `is_mining` - Correctly used everywhere
- ✅ `hashrate` - Correctly used everywhere
- ✅ `blocks_found` - Correctly used everywhere

**Verdict**: **ALL property names match perfectly** ✅

---

## Missing Feature Identified

### Issue: Estimated Reward Field Not Populated

**Location**: `mining.html:177`

The UI contains an "Est. Reward" display field:
```html
<span class="status-item-label">Est. Reward</span>
<span class="status-item-value" id="est-reward">0.00 BTPC</span>
```

But this field was **never updated** by JavaScript - it remained at "0.00 BTPC" permanently.

### Root Cause

The backend `get_mining_status` command doesn't return an `estimated_reward` field. The frontend must calculate it client-side using:

```javascript
estimated_reward = blocks_found * 32.375 BTPC
```

**Block reward value** sourced from `src-tauri/src/main.rs:1143`:
```rust
let reward_btc = 32.375; // Constitutional reward
```

---

## Fix Applied

### File: `mining.html`

#### Fix #1: updateMiningStats Function (lines 409-420)

**Before**:
```javascript
async function updateMiningStats() {
    try {
        const status = await window.invoke('get_mining_status');
        if (status) {
            document.getElementById('hashrate').textContent = `${(status.hashrate || 0).toLocaleString()} H/s`;
            document.getElementById('blocks-found').textContent = status.blocks_found || 0;
        }
```

**After**:
```javascript
async function updateMiningStats() {
    try {
        const status = await window.invoke('get_mining_status');
        if (status) {
            const blocksFound = status.blocks_found || 0;
            const rewardPerBlock = 32.375; // BTPC reward per block
            const estimatedReward = blocksFound * rewardPerBlock;

            document.getElementById('hashrate').textContent = `${(status.hashrate || 0).toLocaleString()} H/s`;
            document.getElementById('blocks-found').textContent = blocksFound;
            document.getElementById('est-reward').textContent = `${estimatedReward.toFixed(8)} BTPC`;
        }
```

#### Fix #2: Update Manager Subscription (lines 569-594)

**Before**:
```javascript
updateManager.subscribe((type, data, fullState) => {
    if (type === 'mining') {
        if (data.is_mining) {
            document.getElementById('mining-status').innerHTML = '<span class="icon icon-pickaxe"></span> Active';
            document.getElementById('hashrate').textContent = `${(data.hashrate || 0).toLocaleString()} H/s`;
            document.getElementById('blocks-found').textContent = data.blocks_found || 0;
            // ...
        }
```

**After**:
```javascript
updateManager.subscribe((type, data, fullState) => {
    if (type === 'mining') {
        const blocksFound = data.blocks_found || 0;
        const rewardPerBlock = 32.375;
        const estimatedReward = blocksFound * rewardPerBlock;

        if (data.is_mining) {
            document.getElementById('mining-status').innerHTML = '<span class="icon icon-pickaxe"></span> Active';
            document.getElementById('hashrate').textContent = `${(data.hashrate || 0).toLocaleString()} H/s`;
            document.getElementById('blocks-found').textContent = blocksFound;
            document.getElementById('est-reward').textContent = `${estimatedReward.toFixed(8)} BTPC`;
            // ...
        } else {
            // Also show reward in inactive state (accumulated total)
            document.getElementById('est-reward').textContent = `${estimatedReward.toFixed(8)} BTPC`;
```

---

## Impact

### Before Fix
- Status: ⚠️ Inactive ✅ Hashrate: 0 H/s ✅ Blocks Found: 0 ❌ Est. Reward: 0.00 BTPC (hardcoded, never updates)

### After Fix
- Status: ⚠️ Inactive ✅ Hashrate: 0 H/s ✅ Blocks Found: 5 ✅ Est. Reward: 161.87500000 BTPC (calculated: 5 × 32.375)

---

## Calculation Logic

```javascript
const REWARD_PER_BLOCK = 32.375; // BTPC constitutional reward
const estimatedReward = blocks_found * REWARD_PER_BLOCK;
```

**Example Calculations**:
| Blocks Found | Calculation | Est. Reward |
|---|---|---|
| 0 | 0 × 32.375 | 0.00000000 BTPC |
| 1 | 1 × 32.375 | 32.37500000 BTPC |
| 5 | 5 × 32.375 | 161.87500000 BTPC |
| 100 | 100 × 32.375 | 3237.50000000 BTPC |

---

## Testing Checklist

- [x] Estimated reward displays as "0.00000000 BTPC" when no blocks mined
- [x] Estimated reward updates correctly when blocks are found
- [x] Reward persists after mining stops (shows accumulated total)
- [x] Reward calculation uses 8 decimal precision
- [x] Both update paths calculate reward (updateMiningStats + subscription)

---

## Summary

### API Mismatches Found: **0**

All mining status properties (`is_mining`, `hashrate`, `blocks_found`) match perfectly between frontend and backend.

### Missing Features Fixed: **1**

1. ✅ Estimated Reward calculation and display (mining.html lines 413-420, 571-585)

### Files Modified: **1**

1. ✅ `/ui/mining.html` - Added reward calculation in 2 locations

---

## Related Documentation

- Main audit: `COMPREHENSIVE_API_MISMATCH_FIXES_2025-10-07.md`
- Backend source: `src-tauri/src/main.rs:615` (get_mining_status)
- Backend reward: `src-tauri/src/main.rs:1143` (32.375 BTPC per block)
- Frontend file: `ui/mining.html`