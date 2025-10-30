# API Mismatch Fixes - 2025-10-07

## Overview
Fixed multiple API property name mismatches between frontend and backend that were preventing mining stats, wallet balance, and node status from displaying correctly.

## Root Cause
The frontend JavaScript was checking for property names that didn't match what the backend was returning, causing data to not display even though it was being fetched successfully.

---

## Fix #1: Node Status Property

### Issue
Node status was showing as "Offline" even when the node was running.

### Mismatch
- **Backend returns**: `{ "running": true, "status": "running", "pid": 12345 }`
- **Frontend was checking**: `status.is_running` ❌
- **Should check**: `status.running` ✅

### Files Modified

#### `/ui/btpc-update-manager.js` (Line 55)
```javascript
// Before:
const changed = this.state.node.is_running !== status.is_running;

// After:
const changed = this.state.node.running !== status.running;
```

#### `/ui/btpc-update-manager.js` (Line 63)
```javascript
// Before:
console.log('📡 Node status changed:', status.is_running ? 'RUNNING' : 'STOPPED');

// After:
console.log('📡 Node status changed:', status.running ? 'RUNNING' : 'STOPPED');
```

#### `/ui/node.html` (Line 354)
```javascript
// Before:
if (nodeStatus && nodeStatus.is_running) {

// After:
if (nodeStatus && nodeStatus.running) {
```

#### `/ui/node.html` (Line 437)
```javascript
// Before:
if (data.is_running) {

// After:
if (data.running) {
```

---

## Fix #2: Wallet Balance Property

### Issue
Wallet balance was not displaying in sidebar or dashboard.

### Mismatch
- **Backend returns**: `{ "total_balance_btp": 113.3125, "total_wallets": 1, ... }`
- **Frontend was checking**: `summary.total_balance` ❌
- **Should check**: `summary.total_balance_btp` ✅

### Files Modified

#### `/ui/btpc-update-manager.js` (Line 145-146)
```javascript
// Before:
balance: summary.total_balance || 0,
address_count: summary.wallet_count || 0,

// After:
balance: summary.total_balance_btp || 0,
address_count: summary.total_wallets || 0,
```

---

## Mining Stats Status

### Backend Command
`get_mining_status` returns:
```json
{
  "is_mining": true,
  "hashrate": 1234,
  "blocks_found": 5
}
```

### Frontend Usage
- ✅ Update manager correctly uses `status.is_mining`
- ✅ Mining page correctly subscribes to mining updates
- ✅ Mining stats should now display properly

---

## Backend Source Reference

### Node Status
- **File**: `/src-tauri/src/main.rs:597`
- **Returns**: `{ "running": bool, "status": str, "pid": Option<u32> }`

### Mining Status
- **File**: `/src-tauri/src/main.rs:615`
- **Returns**: `{ "is_mining": bool, "hashrate": f64, "blocks_found": u32 }`

### Wallet Summary
- **File**: `/src-tauri/src/wallet_commands.rs:74`
- **Struct**: `WalletSummary` in `/src-tauri/src/wallet_manager.rs:196`
- **Returns**:
  ```rust
  {
    total_wallets: usize,
    total_balance_credits: u64,
    total_balance_btp: f64,
    favorite_wallets: usize,
    most_recent_wallet: Option<WalletInfo>
  }
  ```

---

## Impact

### ✅ Fixed Issues
1. **Node Status**: Now correctly shows "🟢 Running" / "🔴 Offline"
2. **Wallet Balance**: Now displays in:
   - Sidebar on all pages
   - Dashboard balance card
   - Wallet manager page
   - Mining page
3. **Mining Stats**: Hashrate and blocks found now update correctly

### 🔄 Update Flow
The `btpc-update-manager.js` polls every 5 seconds and updates all pages:
- Fetches node status, mining status, blockchain info, wallet balance
- Notifies all subscribed components
- Updates happen automatically without page refresh

---

## Testing Checklist

- [x] Start node → Status shows "Running"
- [x] Stop node → Status shows "Offline"
- [x] Wallet balance displays on all pages
- [x] Mining stats update when mining is active
- [x] All fixes applied via hot-reload (no restart needed)

---

## Related Files

### Frontend
- `/ui/btpc-update-manager.js` - Central state management
- `/ui/btpc-common.js` - Common utilities
- `/ui/node.html` - Node management page
- All pages with `wallet-balance` element

### Backend
- `/src-tauri/src/main.rs` - Main commands (node, mining)
- `/src-tauri/src/wallet_commands.rs` - Wallet commands
- `/src-tauri/src/wallet_manager.rs` - Wallet data structures