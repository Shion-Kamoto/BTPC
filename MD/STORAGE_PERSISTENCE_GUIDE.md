# BTPC Desktop App - Storage & Persistence Guide

**Last Updated**: 2025-10-06
**Status**: ✅ FULLY IMPLEMENTED

---

## Overview

The BTPC desktop application now has **complete local data persistence** for all app features and settings. Changes made in the UI are automatically saved to browser `localStorage` and persist across sessions.

---

## Storage Architecture

### Two-Layer Persistence

1. **Frontend (Browser localStorage)** - `btpc-storage.js`
   - UI settings and preferences
   - Cached blockchain data
   - User interface state
   - **Location**: Browser `localStorage` (survives page refresh)

2. **Backend (Rust/File System)** - Tauri backend
   - Wallet data (encrypted with AES-256-GCM)
   - UTXO database
   - Transaction history
   - **Location**: `~/.btpc/` directory

---

## Frontend Storage System (`btpc-storage.js`)

### Features

✅ **Automatic Persistence** - All changes saved instantly
✅ **Structured Storage** - Organized hierarchy for settings/cache
✅ **Type-Safe API** - Clean JavaScript methods
✅ **Backup/Restore** - Export/import configuration
✅ **Session Tracking** - Track last accessed wallets, favorites
✅ **Cache Management** - Smart blockchain data caching

### Storage Structure

```javascript
{
  version: "1.0.0",

  settings: {
    network: "testnet",           // mainnet/testnet/regtest
    theme: "dark",                // UI theme
    currency: "BTPC",
    language: "en",
    notifications: true,
    autoSync: true,
    syncInterval: 5000,           // 5 seconds
    defaultFee: 10000,            // 0.0001 BTPC
  },

  ui: {
    sidebarCollapsed: false,
    dashboardLayout: "grid",
    chartTimeframe: "24h",
    transactionFilter: "all",
    addressBookExpanded: false,
  },

  wallet: {
    selectedWalletId: null,
    defaultWalletId: null,
    favoriteWallets: [],          // Array of wallet IDs
    hiddenWallets: [],
    lastAccessedWallets: [],      // Last 10 accessed wallets
  },

  mining: {
    threads: 4,                   // CPU threads for mining
    autoStart: false,
    targetAddress: null,
    poolUrl: null,
  },

  node: {
    rpcHost: "127.0.0.1",
    rpcPort: 18350,               // Testnet RPC port
    p2pPort: 18351,
    autoConnect: true,
    trustedNodes: [],             // Peer addresses
  },

  cache: {
    lastBlockHeight: 0,
    lastSyncTime: null,
    balances: {},                 // address -> {balance, timestamp}
    transactions: [],             // Last 100 transactions
    addressBook: [],              // Saved addresses
  },

  timestamps: {
    created: Date.now(),
    lastUpdated: Date.now(),
    lastBackup: null,
  }
}
```

---

## Usage Examples

### Include in HTML Pages

```html
<!-- Load before other scripts -->
<script src="btpc-storage.js"></script>
<script src="btpc-common.js"></script>
```

The storage system initializes automatically as `window.btpcStorage`.

### Save Settings

```javascript
// Update general settings
window.btpcStorage.updateSettings({
    network: 'testnet',
    notifications: true,
    autoSync: true
});

// Update node configuration
window.btpcStorage.updateNodeConfig({
    rpcPort: 18350,
    autoConnect: true
});

// Update mining config
window.btpcStorage.updateMiningConfig({
    threads: 8,
    autoStart: false
});
```

### Load Settings

```javascript
// Get all settings
const settings = window.btpcStorage.getSettings();
console.log(settings.network);  // 'testnet'

// Get specific value with path notation
const rpcPort = window.btpcStorage.get('node.rpcPort', 18350);

// Get UI preferences
const uiPrefs = window.btpcStorage.getUIPreferences();
```

### Wallet Management

```javascript
// Set selected wallet
window.btpcStorage.setSelectedWallet('wallet-123');

// Add to favorites
window.btpcStorage.addFavoriteWallet('wallet-123');

// Remove from favorites
window.btpcStorage.removeFavoriteWallet('wallet-123');

// Get wallet preferences
const walletPrefs = window.btpcStorage.getWalletPreferences();
console.log(walletPrefs.favoriteWallets);
```

### Blockchain Data Caching

```javascript
// Cache balance for address
window.btpcStorage.cacheBalance('address123...', 1500000000);

// Get cached balance
const cached = window.btpcStorage.getCachedBalance('address123...');
console.log(cached.balance);  // 1500000000
console.log(cached.timestamp);

// Cache transaction
window.btpcStorage.cacheTransaction({
    txid: 'abc123...',
    amount: 500000000,
    timestamp: Date.now()
});

// Get cached transactions
const txs = window.btpcStorage.getCachedTransactions();
```

### Address Book

```javascript
// Add address to address book
window.btpcStorage.addToAddressBook({
    address: 'f3a2b1c4...',
    label: 'Mining Rewards',
    category: 'Mining'
});

// Get address book
const addressBook = window.btpcStorage.getAddressBook();

// Remove address
window.btpcStorage.removeFromAddressBook('f3a2b1c4...');
```

### Backup & Restore

```javascript
// Create backup (returns JSON string)
const backup = window.btpcStorage.createBackup();
console.log(backup);  // Download or save

// Restore from backup
const success = window.btpcStorage.restoreFromBackup(backupString);

// Export settings only
const settingsJSON = window.btpcStorage.exportSettings();

// Import settings
window.btpcStorage.importSettings(settingsJSON);
```

### Clear Data

```javascript
// Clear all data (reset to defaults)
window.btpcStorage.clearAll();

// Clear only cache (preserve settings)
window.btpcStorage.clearCache();
```

### Get Storage Stats

```javascript
const stats = window.btpcStorage.getStats();
console.log(stats);
/* Output:
{
  version: "1.0.0",
  sizeBytes: 5432,
  sizeKB: "5.31",
  created: 1728234567890,
  lastUpdated: 1728234987654,
  lastBackup: null,
  walletCount: 3,
  cachedTransactions: 15,
  addressBookEntries: 5
}
*/
```

---

## Backend Storage (Rust/Tauri)

### Wallet Manager Persistence

**Location**: `~/.btpc/wallets/wallets_metadata.json`

The backend `WalletManager` automatically saves wallet metadata to disk:

```rust
// Auto-saves on every operation
wallet_manager.create_wallet(request)?;  // Saves immediately
wallet_manager.update_wallet(request)?;  // Saves immediately
wallet_manager.delete_wallet(id)?;       // Saves immediately
```

### Wallet Files

**Location**: `~/.btpc/wallets/wallet_{uuid}.json`

Each wallet is stored as an encrypted JSON file:
- Private keys encrypted with AES-256-GCM
- Argon2id key derivation from password
- Metadata includes nickname, category, balance cache

### UTXO Database

**Location**: `~/.btpc/data/wallet/utxos.json`

The UTXO manager persists:
- All unspent transaction outputs
- Spent UTXOs (for history)
- Balance caches per address

### Backups

**Location**: `~/.btpc/wallet-backups/`

Automatic backups when:
- Auto-backup enabled in wallet settings
- Manual backup requested
- Before risky operations

---

## Integration Example: Settings Page

The `settings.html` page demonstrates full integration:

```html
<!DOCTYPE html>
<html>
<head>
    <link rel="stylesheet" href="btpc-styles.css">
</head>
<body>
    <!-- Settings form -->
    <input type="text" id="network-type">
    <input type="number" id="rpc-port">
    <button onclick="saveSettings()">Save</button>

    <script src="btpc-storage.js"></script>
    <script>
        // Load settings on page load
        function loadSettings() {
            const settings = window.btpcStorage.getSettings();
            const nodeConfig = window.btpcStorage.getNodeConfig();

            document.getElementById('network-type').value = settings.network;
            document.getElementById('rpc-port').value = nodeConfig.rpcPort;
        }

        // Save settings
        function saveSettings() {
            window.btpcStorage.updateSettings({
                network: document.getElementById('network-type').value
            });

            window.btpcStorage.updateNodeConfig({
                rpcPort: parseInt(document.getElementById('rpc-port').value)
            });

            alert('✅ Settings saved!');
        }

        // Auto-load on page ready
        loadSettings();
    </script>
</body>
</html>
```

---

## File System Layout

```
~/.btpc/
├── config/
│   └── users/                    # User authentication data
├── data/
│   ├── node/                     # Blockchain data
│   └── wallet/
│       └── utxos.json            # UTXO database
├── wallets/
│   ├── wallets_metadata.json     # All wallet metadata
│   ├── wallet_uuid1.json         # Individual wallet files
│   ├── wallet_uuid2.json
│   └── ...
├── wallet-backups/
│   └── wallet_uuid1_backup_*.json
└── logs/
    ├── node.log
    ├── wallet.log
    └── mining.log
```

---

## Persistence Guarantees

### Frontend (Browser)

| Data Type | Persistence | Duration |
|-----------|-------------|----------|
| Settings | ✅ Saved | Until browser cache cleared |
| UI State | ✅ Saved | Until browser cache cleared |
| Wallet Preferences | ✅ Saved | Until browser cache cleared |
| Cached Data | ✅ Saved | Until manually cleared or expired |

### Backend (File System)

| Data Type | Persistence | Duration |
|-----------|-------------|----------|
| Wallet Files | ✅ Encrypted on disk | Permanent |
| UTXO Database | ✅ JSON file | Permanent |
| Wallet Metadata | ✅ JSON file | Permanent |
| Backups | ✅ Encrypted copies | Permanent (manual cleanup) |

---

## Security Considerations

### Frontend Storage

- ⚠️ **NOT encrypted** - Browser localStorage is plain text
- ✅ **No private keys** - Never store private keys in localStorage
- ✅ **Safe for preferences** - Settings, UI state, cache only

### Backend Storage

- ✅ **Fully encrypted** - All wallet files use AES-256-GCM
- ✅ **Password-protected** - Argon2id key derivation
- ✅ **Post-quantum ready** - ML-DSA (Dilithium5) signatures

---

## Testing Persistence

### Test Storage Save/Load

```javascript
// Open browser console (F12)

// 1. Save test data
window.btpcStorage.updateSettings({ network: 'testnet' });

// 2. Check it saved
console.log(window.btpcStorage.getSettings().network);  // 'testnet'

// 3. Refresh page (Ctrl+R)

// 4. Verify persistence
console.log(window.btpcStorage.getSettings().network);  // Still 'testnet'
```

### Test Wallet Metadata Persistence

```bash
# Save wallet via UI (creates metadata file)

# Check file exists
cat ~/.btpc/wallets/wallets_metadata.json

# Restart app

# Verify wallets still loaded
```

---

## Migration Notes

### From Old localStorage (v0.x)

The new storage system is backwards compatible but uses a new structure.

Old settings at `localStorage.getItem('btpc_settings')` are automatically migrated on first load.

### Manual Migration

```javascript
// Export old settings
const oldSettings = JSON.parse(localStorage.getItem('btpc_settings') || '{}');

// Import to new system
window.btpcStorage.updateSettings({
    network: oldSettings.network || 'mainnet',
    // ... map other fields
});

// Remove old key (optional)
localStorage.removeItem('btpc_settings');
```

---

## Troubleshooting

### Settings not persisting across page reload

**Solution**:
```javascript
// Check if localStorage is available
console.log(window.btpcStorage.isAvailable);  // Should be true

// Check storage stats
console.log(window.btpcStorage.getStats());

// Manually save
window.btpcStorage.saveData(window.btpcStorage.getData());
```

### Storage quota exceeded

**Solution**:
```javascript
// Clear cache (preserves settings)
window.btpcStorage.clearCache();

// Or limit transaction cache
const limited = window.btpcStorage.getCachedTransactions().slice(0, 50);
window.btpcStorage.set('cache.transactions', limited);
```

### Backend wallet not saving

**Check permissions**:
```bash
ls -la ~/.btpc/wallets/
# Should be owned by current user

# Fix permissions if needed
chmod 700 ~/.btpc/wallets
chmod 600 ~/.btpc/wallets/*.json
```

---

## API Reference

### Core Methods

| Method | Description |
|--------|-------------|
| `getData()` | Get all storage data |
| `saveData(data)` | Save all storage data |
| `get(path, default)` | Get value by path (e.g., 'settings.network') |
| `set(path, value)` | Set value by path |

### Settings Methods

| Method | Description |
|--------|-------------|
| `getSettings()` | Get all settings |
| `updateSettings(obj)` | Update settings (merge) |
| `getNodeConfig()` | Get node configuration |
| `updateNodeConfig(obj)` | Update node config |
| `getMiningConfig()` | Get mining configuration |
| `updateMiningConfig(obj)` | Update mining config |

### Wallet Methods

| Method | Description |
|--------|-------------|
| `setSelectedWallet(id)` | Set current wallet |
| `addFavoriteWallet(id)` | Add to favorites |
| `removeFavoriteWallet(id)` | Remove from favorites |
| `getWalletPreferences()` | Get wallet prefs |

### Cache Methods

| Method | Description |
|--------|-------------|
| `cacheBalance(addr, bal)` | Cache balance for address |
| `getCachedBalance(addr)` | Get cached balance |
| `cacheTransaction(tx)` | Cache transaction |
| `getCachedTransactions()` | Get all cached txs |
| `addToAddressBook(entry)` | Add address with label |
| `getAddressBook()` | Get all saved addresses |

### Utility Methods

| Method | Description |
|--------|-------------|
| `createBackup()` | Export full backup JSON |
| `restoreFromBackup(json)` | Restore from backup |
| `exportSettings()` | Export settings only |
| `importSettings(json)` | Import settings |
| `clearAll()` | Clear all data |
| `clearCache()` | Clear cache only |
| `getStats()` | Get storage statistics |

---

## Conclusion

The BTPC desktop app now has **full persistence** for all app features:

✅ **Settings saved locally** - Network, RPC, preferences
✅ **UI state preserved** - Sidebar, layout, filters
✅ **Wallet data encrypted** - Backend AES-256-GCM
✅ **Blockchain cache** - Balances, transactions
✅ **Backup/Restore** - Export/import configuration

**All changes are automatically saved** and will persist across app restarts and page reloads.

---

**Last Updated**: 2025-10-06
**Version**: 1.0.0
**Status**: ✅ Production Ready