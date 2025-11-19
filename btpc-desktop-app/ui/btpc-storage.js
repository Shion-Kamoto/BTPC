/**
 * BTPC Desktop App - Local Storage Manager
 *
 * Provides persistent storage for UI settings, preferences, and cached data
 * Uses browser localStorage API with fallback mechanisms
 */

class BtpcStorage {
    constructor() {
        this.storageKey = 'btpc_app_data';
        this.version = '1.0.0';
        this.isAvailable = this.checkStorageAvailability();

        // Initialize storage if empty
        if (this.isAvailable && !this.getData()) {
            this.initializeStorage();
        }
    }

    /**
     * Check if localStorage is available
     */
    checkStorageAvailability() {
        try {
            const test = '__storage_test__';
            localStorage.setItem(test, test);
            localStorage.removeItem(test);
            return true;
        } catch (e) {
            console.warn('localStorage not available, using in-memory storage');
            return false;
        }
    }

    /**
     * Initialize storage with default structure
     */
    initializeStorage() {
        const defaultData = {
            version: this.version,
            settings: {
                // NOTE: 'network' is NOT stored here - backend (Arc<RwLock<NetworkType>>) is source of truth
                theme: 'dark',
                currency: 'BTPC',
                language: 'en',
                notifications: true,
                autoSync: true,
                syncInterval: 5000, // 5 seconds
                defaultFee: 10000, // 0.0000001 BTPC in credits
            },
            ui: {
                sidebarCollapsed: false,
                dashboardLayout: 'grid',
                chartTimeframe: '24h',
                transactionFilter: 'all',
                addressBookExpanded: false,
            },
            wallet: {
                selectedWalletId: null,
                defaultWalletId: null,
                favoriteWallets: [],
                hiddenWallets: [],
                lastAccessedWallets: [],
            },
            mining: {
                threads: navigator.hardwareConcurrency || 4,
                autoStart: false,
                targetAddress: null,
                poolUrl: null,
            },
            node: {
                rpcHost: '127.0.0.1',
                rpcPort: 18350,
                p2pPort: 18351,
                autoConnect: false,  // Disabled auto-start for better control during testing
                trustedNodes: [],
            },
            cache: {
                lastBlockHeight: 0,
                lastSyncTime: null,
                balances: {},
                transactions: [],
                addressBook: [],
            },
            timestamps: {
                created: Date.now(),
                lastUpdated: Date.now(),
                lastBackup: null,
            }
        };

        this.saveData(defaultData);
        return defaultData;
    }

    /**
     * Get all storage data
     */
    getData() {
        if (!this.isAvailable) {
            return this.memoryStorage || null;
        }

        try {
            const data = localStorage.getItem(this.storageKey);
            return data ? JSON.parse(data) : null;
        } catch (e) {
            console.error('Error reading from localStorage:', e);
            return null;
        }
    }

    /**
     * Save all storage data
     */
    saveData(data) {
        data.timestamps = data.timestamps || {};
        data.timestamps.lastUpdated = Date.now();

        if (!this.isAvailable) {
            this.memoryStorage = data;
            return true;
        }

        try {
            localStorage.setItem(this.storageKey, JSON.stringify(data));
            return true;
        } catch (e) {
            console.error('Error writing to localStorage:', e);
            return false;
        }
    }

    /**
     * Get a specific setting by path
     */
    get(path, defaultValue = null) {
        const data = this.getData();
        if (!data) return defaultValue;

        const keys = path.split('.');
        let value = data;

        for (const key of keys) {
            if (value && typeof value === 'object' && key in value) {
                value = value[key];
            } else {
                return defaultValue;
            }
        }

        return value;
    }

    /**
     * Set a specific setting by path
     */
    set(path, value) {
        const data = this.getData() || this.initializeStorage();
        const keys = path.split('.');
        let current = data;

        for (let i = 0; i < keys.length - 1; i++) {
            const key = keys[i];
            if (!(key in current) || typeof current[key] !== 'object') {
                current[key] = {};
            }
            current = current[key];
        }

        current[keys[keys.length - 1]] = value;
        return this.saveData(data);
    }

    /**
     * Get all settings
     */
    getSettings() {
        return this.get('settings', {});
    }

    /**
     * Update settings (merge with existing)
     */
    updateSettings(newSettings) {
        const currentSettings = this.getSettings();
        const merged = { ...currentSettings, ...newSettings };
        return this.set('settings', merged);
    }

    /**
     * Get UI preferences
     */
    getUIPreferences() {
        return this.get('ui', {});
    }

    /**
     * Update UI preferences
     */
    updateUIPreferences(newPrefs) {
        const current = this.getUIPreferences();
        const merged = { ...current, ...newPrefs };
        return this.set('ui', merged);
    }

    /**
     * Get wallet preferences
     */
    getWalletPreferences() {
        return this.get('wallet', {});
    }

    /**
     * Set selected wallet
     */
    setSelectedWallet(walletId) {
        this.set('wallet.selectedWalletId', walletId);

        // Update last accessed list
        const lastAccessed = this.get('wallet.lastAccessedWallets', []);
        const filtered = lastAccessed.filter(id => id !== walletId);
        filtered.unshift(walletId);
        this.set('wallet.lastAccessedWallets', filtered.slice(0, 10)); // Keep last 10
    }

    /**
     * Add wallet to favorites
     */
    addFavoriteWallet(walletId) {
        const favorites = this.get('wallet.favoriteWallets', []);
        if (!favorites.includes(walletId)) {
            favorites.push(walletId);
            this.set('wallet.favoriteWallets', favorites);
        }
    }

    /**
     * Remove wallet from favorites
     */
    removeFavoriteWallet(walletId) {
        const favorites = this.get('wallet.favoriteWallets', []);
        const filtered = favorites.filter(id => id !== walletId);
        this.set('wallet.favoriteWallets', filtered);
    }

    /**
     * Get mining configuration
     */
    getMiningConfig() {
        return this.get('mining', {});
    }

    /**
     * Update mining configuration
     */
    updateMiningConfig(config) {
        const current = this.getMiningConfig();
        const merged = { ...current, ...config };
        return this.set('mining', merged);
    }

    /**
     * Get node configuration
     */
    getNodeConfig() {
        return this.get('node', {});
    }

    /**
     * Update node configuration
     */
    updateNodeConfig(config) {
        const current = this.getNodeConfig();
        const merged = { ...current, ...config };
        return this.set('node', merged);
    }

    /**
     * Cache blockchain data
     */
    cacheBlockchainData(data) {
        const cache = this.get('cache', {});
        const updated = {
            ...cache,
            ...data,
            lastSyncTime: Date.now()
        };
        return this.set('cache', updated);
    }

    /**
     * Get cached blockchain data
     */
    getCachedBlockchainData() {
        return this.get('cache', {});
    }

    /**
     * Update balance cache
     */
    cacheBalance(address, balance) {
        const balances = this.get('cache.balances', {});
        balances[address] = {
            balance,
            timestamp: Date.now()
        };
        return this.set('cache.balances', balances);
    }

    /**
     * Get cached balance
     */
    getCachedBalance(address) {
        const balances = this.get('cache.balances', {});
        return balances[address] || null;
    }

    /**
     * Add transaction to cache
     */
    cacheTransaction(tx) {
        const transactions = this.get('cache.transactions', []);

        // Check if transaction already exists
        const existingIndex = transactions.findIndex(t => t.txid === tx.txid);
        if (existingIndex >= 0) {
            transactions[existingIndex] = tx;
        } else {
            transactions.unshift(tx);
        }

        // Keep only last 100 transactions
        const limited = transactions.slice(0, 100);
        return this.set('cache.transactions', limited);
    }

    /**
     * Get cached transactions
     */
    getCachedTransactions() {
        return this.get('cache.transactions', []);
    }

    /**
     * Add address to address book
     */
    addToAddressBook(entry) {
        const addressBook = this.get('cache.addressBook', []);

        // Check if address already exists
        const existingIndex = addressBook.findIndex(a => a.address === entry.address);
        if (existingIndex >= 0) {
            addressBook[existingIndex] = { ...entry, updatedAt: Date.now() };
        } else {
            addressBook.push({ ...entry, createdAt: Date.now() });
        }

        return this.set('cache.addressBook', addressBook);
    }

    /**
     * Get address book
     */
    getAddressBook() {
        return this.get('cache.addressBook', []);
    }

    /**
     * Remove address from address book
     */
    removeFromAddressBook(address) {
        const addressBook = this.get('cache.addressBook', []);
        const filtered = addressBook.filter(entry => entry.address !== address);
        return this.set('cache.addressBook', filtered);
    }

    /**
     * Create backup of storage
     */
    createBackup() {
        const data = this.getData();
        if (!data) return null;

        const backup = {
            ...data,
            backupTimestamp: Date.now()
        };

        // Store backup timestamp
        this.set('timestamps.lastBackup', Date.now());

        return JSON.stringify(backup, null, 2);
    }

    /**
     * Restore from backup
     */
    restoreFromBackup(backupString) {
        try {
            const backup = JSON.parse(backupString);

            // Validate backup structure
            if (!backup.version || !backup.settings) {
                throw new Error('Invalid backup format');
            }

            // Restore data
            delete backup.backupTimestamp;
            backup.timestamps.lastUpdated = Date.now();

            return this.saveData(backup);
        } catch (e) {
            console.error('Error restoring from backup:', e);
            return false;
        }
    }

    /**
     * Export settings as JSON
     */
    exportSettings() {
        const settings = this.getSettings();
        return JSON.stringify(settings, null, 2);
    }

    /**
     * Import settings from JSON
     */
    importSettings(settingsString) {
        try {
            const settings = JSON.parse(settingsString);
            return this.updateSettings(settings);
        } catch (e) {
            console.error('Error importing settings:', e);
            return false;
        }
    }

    /**
     * Clear all stored data
     */
    clearAll() {
        if (this.isAvailable) {
            localStorage.removeItem(this.storageKey);
        }
        this.memoryStorage = null;
        return this.initializeStorage();
    }

    /**
     * Clear only cache data
     */
    clearCache() {
        return this.set('cache', {
            lastBlockHeight: 0,
            lastSyncTime: null,
            balances: {},
            transactions: [],
            addressBook: this.get('cache.addressBook', []) // Preserve address book
        });
    }

    /**
     * Get storage statistics
     */
    getStats() {
        const data = this.getData();
        if (!data) return null;

        const dataString = JSON.stringify(data);
        const sizeInBytes = new Blob([dataString]).size;
        const sizeInKB = (sizeInBytes / 1024).toFixed(2);

        return {
            version: data.version,
            sizeBytes: sizeInBytes,
            sizeKB: sizeInKB,
            created: data.timestamps?.created,
            lastUpdated: data.timestamps?.lastUpdated,
            lastBackup: data.timestamps?.lastBackup,
            walletCount: data.wallet?.favoriteWallets?.length || 0,
            cachedTransactions: data.cache?.transactions?.length || 0,
            addressBookEntries: data.cache?.addressBook?.length || 0,
        };
    }
}

// Create global instance
window.btpcStorage = new BtpcStorage();

// Auto-save on page unload
window.addEventListener('beforeunload', () => {
    const data = window.btpcStorage.getData();
    if (data) {
        window.btpcStorage.saveData(data);
    }
});

console.log('BTPC Storage initialized:', window.btpcStorage.getStats());