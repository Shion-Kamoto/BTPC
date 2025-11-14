# Network Configuration Persistence Fix

**Date:** 2025-10-11
**Status:** ✅ FIXED
**Priority:** HIGH

---

## Problem

Network selection (Mainnet/Testnet/Regtest) was **not persisting** across page navigation in the desktop app. Users would select a network in Settings, navigate to another page, then return to Settings to find their selection had reverted to the default value.

### Root Cause

The issue was caused by **conflicting sources of truth** for network configuration:

1. **Backend (Correct)**: `Arc<RwLock<NetworkType>>` in `main.rs` - properly stored network config in memory
2. **Frontend LocalStorage (Incorrect)**: `btpc-storage.js` had `network: 'testnet'` hardcoded in default settings

### The Bug Flow

1. User opens Settings page → Frontend loads network from localStorage (`'testnet'`)
2. User changes network to `mainnet` → Frontend calls `save_network_config()` → Backend updates `Arc<RwLock>` ✅
3. User navigates away from Settings
4. User returns to Settings → Frontend loads from localStorage AGAIN → Overwrites with `'testnet'` ❌

The backend state was correct, but the frontend kept overriding it with stale localStorage data.

---

## Solution

**Make the backend the single source of truth** for network configuration by removing it from frontend localStorage.

### Changes Made

#### 1. `btpc-storage.js` (lines 39-50)
**Before:**
```javascript
settings: {
    network: 'testnet',  // ❌ Hardcoded default conflicts with backend
    theme: 'dark',
    ...
}
```

**After:**
```javascript
settings: {
    // NOTE: 'network' is NOT stored here - backend (Arc<RwLock<NetworkType>>) is source of truth
    theme: 'dark',
    ...
}
```

#### 2. `settings.html` (line 341-347)
**Before:**
```javascript
window.btpcStorage.updateSettings({
    network: document.getElementById('network-type').value,  // ❌ Saved to localStorage
    dataDir: ...,
    ...
});
```

**After:**
```javascript
// Save general settings to browser storage (network is NOT stored here - backend is source of truth)
window.btpcStorage.updateSettings({
    dataDir: ...,  // ✅ Network removed from localStorage saves
    ...
});
```

### Why This Works

1. **Single Source of Truth**: Backend `Arc<RwLock<NetworkType>>` is the only place storing network config
2. **Always Fetch from Backend**: Settings page calls `get_network_config()` on load (line 274)
3. **Always Save to Backend**: Settings page calls `save_network_config()` on save (line 372)
4. **No localStorage Conflicts**: Network config is never read from or written to localStorage

---

## Architecture

### Backend State Management
```rust
// main.rs:347-350
pub struct AppState {
    config: LauncherConfig,
    active_network: Arc<RwLock<NetworkType>>,  // ✅ Mutable, in-memory state
    active_rpc_port: Arc<RwLock<u16>>,
    active_p2p_port: Arc<RwLock<u16>>,
    ...
}
```

### Frontend Update Flow
```
Settings Page Load:
  ├─ window.invoke('get_network_config')
  ├─ Backend reads from Arc<RwLock<NetworkType>>
  └─ Frontend updates dropdown

Settings Page Save:
  ├─ window.invoke('save_network_config', {network, rpcPort, p2pPort})
  ├─ Backend validates and updates Arc<RwLock<NetworkType>>
  └─ Frontend shows success message

Other Pages:
  ├─ btpcUpdateManager.updateNetworkConfig() (auto-refresh every 5s)
  ├─ window.invoke('get_network_config')
  └─ Footer displays current network
```

---

## Testing

### Manual Test Procedure

1. Start desktop app
2. Navigate to Settings → Network tab
3. Change network from Regtest to **Testnet**
4. Save settings (should see success message)
5. Navigate to Dashboard
6. Navigate back to Settings → Network tab
7. **Expected**: Network dropdown shows **Testnet** ✅
8. **Previous Bug**: Would show Regtest ❌

### Automated Test (Future)

```javascript
// Test network persistence across navigation
test('network config persists across page navigation', async () => {
    await settings.selectNetwork('mainnet');
    await settings.saveSettings();
    await dashboard.navigate();
    await settings.navigate();
    expect(await settings.getSelectedNetwork()).toBe('mainnet');
});
```

---

## Benefits

✅ **Single source of truth** - No conflicting state between frontend/backend
✅ **Persistence** - Network selection survives page navigation
✅ **Validation** - Backend validates ports and network type before saving
✅ **Consistency** - All pages show the same network config
✅ **Simplicity** - Removed redundant localStorage logic

---

## Related Files

- `btpc-desktop-app/src-tauri/src/main.rs` - Backend state management
- `btpc-desktop-app/ui/btpc-storage.js` - Frontend localStorage (network removed)
- `btpc-desktop-app/ui/settings.html` - Settings page (network save/load logic)
- `btpc-desktop-app/ui/btpc-update-manager.js` - Network config auto-refresh

---

## Migration Notes

### For Existing Users

Users with existing localStorage data containing `network: 'testnet'` will:
1. Have that value **ignored** on next app launch
2. Automatically use the backend default (`regtest`)
3. Can change to any network, and it will persist correctly

No data loss or migration script needed - the old localStorage value is simply not read anymore.

### For Developers

When adding new configuration options:
1. Decide if it needs **persistence across restarts** (use filesystem or database)
2. Decide if it needs **real-time updates** (use `Arc<RwLock>` in backend)
3. Decide if it's **UI preferences only** (use localStorage)

**Network config needs real-time updates** → Backend `Arc<RwLock>` is correct choice.

---

## Future Improvements

1. **Persist backend state to disk** - Save `Arc<RwLock<NetworkType>>` to `~/.btpc/config/network.toml` on change
2. **Restore on startup** - Read from file instead of hardcoded default
3. **Add network change event** - Notify all pages when network changes (WebSocket or Tauri events)

---

## Conclusion

The network configuration persistence issue is **fixed** by establishing the backend as the single source of truth and removing the conflicting localStorage logic. Users can now change networks in Settings, and their selection will persist across all page navigations.

**Status: RESOLVED** ✅