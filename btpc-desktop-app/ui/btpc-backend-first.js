/**
 * BTPC Backend-First Validation Module
 *
 * Ensures all state changes go through backend validation before local storage
 * Constitution Compliance: Article XI.1 - Backend State Authority
 *
 * CRITICAL: Never save to localStorage before backend validation
 */

/**
 * Update a setting with backend-first validation
 * @param {Object} setting - Setting object with key and value
 * @returns {Promise<Object>} Result with success status and error if failed
 */
async function updateSetting(setting) {
    if (!window.invoke) {
        return {
            success: false,
            error: 'Tauri API not ready'
        };
    }

    try {
        // Step 1: Backend validation FIRST (Article XI.1)
        const validation = await window.invoke('validate_setting', setting);

        if (!validation.valid) {
            return {
                success: false,
                error: validation.error,
                suggestion: validation.suggestion,
                constitutionRef: validation.constitutionRef || 'Article XI.1'
            };
        }

        // Step 2: Save to backend
        await window.invoke('save_setting', setting);

        // Step 3: ONLY save to localStorage after backend success
        localStorage.setItem(setting.key, setting.value);

        // Step 4: Emit event for cross-page synchronization
        if (window.__TAURI__ && window.__TAURI__.emit) {
            window.__TAURI__.emit('setting-updated', setting);
        }

        return { success: true };
    } catch (error) {
        console.error(`Failed to update setting ${setting.key}:`, error);
        return {
            success: false,
            error: error.message || 'Failed to update setting'
        };
    }
}

/**
 * Create a wallet with backend-first validation
 * @param {Object} walletData - Wallet creation data
 * @returns {Promise<Object>} Result with wallet info or error
 */
async function createWallet(walletData) {
    try {
        // Backend creates wallet FIRST
        const backendWallet = await window.invoke('create_wallet', walletData);

        // ONLY save to localStorage after backend success
        localStorage.setItem('current_wallet', JSON.stringify(backendWallet));

        // Emit event for cross-page synchronization
        if (window.__TAURI__ && window.__TAURI__.emit) {
            window.__TAURI__.emit('wallet-created', backendWallet);
        }

        return {
            success: true,
            wallet: backendWallet
        };
    } catch (error) {
        console.error('Failed to create wallet:', error);
        return {
            success: false,
            error: error.message || 'Failed to create wallet'
        };
    }
}

/**
 * Perform a node action with backend-first validation
 * @param {string} action - Node action (start, stop, restart)
 * @returns {Promise<Object>} Result of the action
 */
async function performNodeAction(action) {
    try {
        // Backend performs action FIRST
        const result = await window.invoke(`${action}_node`);

        // Update local state only after backend success
        if (action === 'start') {
            localStorage.setItem('node_status', 'running');
        } else if (action === 'stop') {
            localStorage.setItem('node_status', 'stopped');
        }

        // Emit event for cross-page synchronization
        if (window.__TAURI__ && window.__TAURI__.emit) {
            window.__TAURI__.emit('node-status-changed', { action, result });
        }

        return { success: true };
    } catch (error) {
        console.error(`Failed to ${action} node:`, error);
        return {
            success: false,
            error: error.message || `Failed to ${action} node`
        };
    }
}

/**
 * Load setting from backend with fallback to localStorage
 * Backend is authoritative, localStorage is cache only
 * @param {string} key - Setting key
 * @returns {Promise<*>} Setting value
 */
async function loadSetting(key) {
    if (window.invoke) {
        try {
            // Try to get from backend FIRST (authoritative)
            const result = await window.invoke('get_setting', { key });

            if (result.success && result.value !== undefined) {
                // Update localStorage cache
                localStorage.setItem(key, result.value);
                return result.value;
            }
        } catch (error) {
            console.warn(`Failed to load setting ${key} from backend:`, error);
        }
    }

    // Fallback to localStorage cache only if backend unavailable
    return localStorage.getItem(key);
}

/**
 * Validate all settings with backend before applying
 * @param {Object} settings - Object with multiple settings
 * @returns {Promise<Object>} Validation results
 */
async function validateSettings(settings) {
    const results = {
        valid: true,
        errors: {},
        validated: {}
    };

    if (!window.invoke) {
        console.warn('Tauri API not ready, cannot validate settings');
        results.valid = false;
        results.errors.general = 'Backend not available';
        return results;
    }

    for (const [key, value] of Object.entries(settings)) {
        try {
            const validation = await window.invoke('validate_setting', { key, value });

            if (!validation.valid) {
                results.valid = false;
                results.errors[key] = validation.error;
            } else {
                results.validated[key] = value;
            }
        } catch (error) {
            results.valid = false;
            results.errors[key] = error.message;
        }
    }

    return results;
}

/**
 * Sync localStorage with backend state (backend wins conflicts)
 * @returns {Promise<void>}
 */
async function syncWithBackend() {
    // Check if Tauri API is available
    if (!window.invoke) {
        console.warn('Tauri API not ready, skipping backend sync');
        return;
    }

    try {
        // Get all settings from backend
        const backendSettings = await window.invoke('get_all_settings');

        if (backendSettings.success) {
            // Backend is authoritative - overwrite localStorage
            for (const [key, value] of Object.entries(backendSettings.data)) {
                localStorage.setItem(key, value);
            }
        }
    } catch (error) {
        console.error('Failed to sync with backend:', error);
    }
}

/**
 * Clear local state and sync with backend
 * Used when switching wallets or resetting
 * @returns {Promise<void>}
 */
async function resetLocalState() {
    try {
        // Clear localStorage
        localStorage.clear();

        // Reload from backend
        await syncWithBackend();

        // Emit reset event
        if (window.__TAURI__ && window.__TAURI__.emit) {
            window.__TAURI__.emit('state-reset');
        }
    } catch (error) {
        console.error('Failed to reset local state:', error);
    }
}

/**
 * Initialize backend-first validation on page load
 */
document.addEventListener('DOMContentLoaded', () => {
    // Sync with backend on load
    syncWithBackend();

    // Listen for cross-page updates
    if (window.__TAURI__ && window.__TAURI__.listen) {
        window.__TAURI__.listen('setting-updated', (event) => {
            const setting = event.payload;
            localStorage.setItem(setting.key, setting.value);
        });

        window.__TAURI__.listen('wallet-created', (event) => {
            const wallet = event.payload;
            localStorage.setItem('current_wallet', JSON.stringify(wallet));
        });

        window.__TAURI__.listen('state-reset', () => {
            syncWithBackend();
        });
    }
});

// Export functions for use in other modules
if (typeof module !== 'undefined' && module.exports) {
    module.exports = {
        updateSetting,
        createWallet,
        performNodeAction,
        loadSetting,
        validateSettings,
        syncWithBackend,
        resetLocalState
    };
}

// Make available globally
window.backendFirst = {
    updateSetting,
    createWallet,
    performNodeAction,
    loadSetting,
    validateSettings,
    syncWithBackend,
    resetLocalState
};