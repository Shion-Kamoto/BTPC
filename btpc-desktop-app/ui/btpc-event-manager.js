/**
 * BTPC Event Listener Management
 *
 * Prevents memory leaks by properly tracking and cleaning up event listeners
 * Constitution Compliance: Article XI.6 - Event Listener Cleanup
 */

/**
 * Manages event listeners with automatic cleanup
 */
class EventListenerManager {
    constructor() {
        this.listeners = new Map();
        this.nextId = 1;

        // Auto-cleanup on page unload
        this.unloadHandler = () => this.destroy();
        window.addEventListener('unload', this.unloadHandler);
        window.addEventListener('beforeunload', this.unloadHandler);
    }

    /**
     * Add a Tauri event listener with tracking
     * @param {string} event - Event name
     * @param {Function} handler - Event handler
     * @returns {Promise<number>} Listener ID
     */
    async listen(event, handler) {
        // Prevent duplicate listeners for same event/handler combo
        for (const [id, listener] of this.listeners) {
            if (listener.event === event && listener.handler === handler) {
                console.log(`Duplicate listener prevented for event: ${event}`);
                return id;
            }
        }

        try {
            if (!window.__TAURI__ || !window.__TAURI__.listen) {
                console.warn('Tauri API not available for event listening');
                return null;
            }

            const unlisten = await window.__TAURI__.listen(event, handler);
            const id = this.nextId++;

            this.listeners.set(id, {
                event,
                handler,
                unlisten,
                createdAt: new Date().toISOString()
            });

            console.log(`Event listener registered: ${event} (ID: ${id})`);
            return id;
        } catch (error) {
            console.error(`Failed to register listener for ${event}:`, error);
            return null;
        }
    }

    /**
     * Remove a specific listener
     * @param {number} listenerId - Listener ID to remove
     */
    removeListener(listenerId) {
        const listener = this.listeners.get(listenerId);

        if (listener) {
            try {
                if (typeof listener.unlisten === 'function') {
                    listener.unlisten();
                }
            } catch (error) {
                console.error(`Error cleaning up listener ${listenerId}:`, error);
            }

            this.listeners.delete(listenerId);
            console.log(`Event listener removed: ${listener.event} (ID: ${listenerId})`);
        }
    }

    /**
     * Check if a listener exists
     * @param {number} listenerId - Listener ID to check
     * @returns {boolean}
     */
    hasListener(listenerId) {
        return this.listeners.has(listenerId);
    }

    /**
     * Get the count of active listeners
     * @returns {number}
     */
    getListenerCount() {
        return this.listeners.size;
    }

    /**
     * Check if there are any active listeners
     * @returns {boolean}
     */
    hasActiveListeners() {
        return this.listeners.size > 0;
    }

    /**
     * Clean up all listeners
     */
    destroy() {
        console.log(`Cleaning up ${this.listeners.size} event listeners...`);

        for (const [id, listener] of this.listeners) {
            try {
                if (typeof listener.unlisten === 'function') {
                    listener.unlisten();
                }
            } catch (error) {
                console.error(`Error cleaning up listener ${id}:`, error);
            }
        }

        this.listeners.clear();

        // Remove unload handlers
        window.removeEventListener('unload', this.unloadHandler);
        window.removeEventListener('beforeunload', this.unloadHandler);

        console.log('All event listeners cleaned up');
    }
}

/**
 * Page Controller with automatic event listener cleanup
 */
class PageController {
    constructor() {
        this.listeners = [];
        this.eventManager = new EventListenerManager();

        // Initialize page-specific listeners
        this.initializeListeners();
    }

    async initializeListeners() {
        // Add common listeners that all pages need
        await this.addListener('blockchain-update', (event) => {
            this.handleBlockchainUpdate(event.payload);
        });

        await this.addListener('wallet-update', (event) => {
            this.handleWalletUpdate(event.payload);
        });

        await this.addListener('node-status', (event) => {
            this.handleNodeStatus(event.payload);
        });
    }

    /**
     * Add a listener through the event manager
     * @param {string} event - Event name
     * @param {Function} handler - Event handler
     */
    async addListener(event, handler) {
        const listenerId = await this.eventManager.listen(event, handler);
        if (listenerId) {
            this.listeners.push(listenerId);
        }
    }

    /**
     * Override these in page-specific controllers
     */
    handleBlockchainUpdate(data) {
        console.log('Blockchain update received:', data);
    }

    handleWalletUpdate(data) {
        console.log('Wallet update received:', data);
    }

    handleNodeStatus(data) {
        console.log('Node status received:', data);
    }

    /**
     * Clean up all listeners for this page
     */
    destroy() {
        console.log('Destroying page controller...');
        this.eventManager.destroy();
        this.listeners = [];
    }
}

/**
 * Manages cross-page event subscriptions
 */
class CrossPageEventManager {
    constructor() {
        this.subscriptions = new Map();
        this.eventManager = new EventListenerManager();
    }

    /**
     * Subscribe to a cross-page event
     * @param {string} event - Event name
     * @param {Function} handler - Event handler
     */
    async subscribe(event, handler) {
        const listenerId = await this.eventManager.listen(event, handler);

        if (listenerId) {
            this.subscriptions.set(event, {
                listenerId,
                handler
            });
        }
    }

    /**
     * Unsubscribe from a cross-page event
     * @param {string} event - Event name
     */
    unsubscribe(event) {
        const subscription = this.subscriptions.get(event);

        if (subscription) {
            this.eventManager.removeListener(subscription.listenerId);
            this.subscriptions.delete(event);
        }
    }

    /**
     * Get count of active subscriptions
     * @returns {number}
     */
    getActiveSubscriptions() {
        return this.subscriptions.size;
    }

    /**
     * Clean up all subscriptions
     */
    destroy() {
        this.eventManager.destroy();
        this.subscriptions.clear();
    }
}

/**
 * Global event manager instance (singleton)
 */
let globalEventManager = null;

/**
 * Get or create global event manager
 * @returns {EventListenerManager}
 */
function getGlobalEventManager() {
    if (!globalEventManager) {
        globalEventManager = new EventListenerManager();
    }
    return globalEventManager;
}

/**
 * Initialize event management for the current page
 */
function initializeEventManagement() {
    // Clean up any existing manager
    if (globalEventManager) {
        globalEventManager.destroy();
    }

    // Create new manager
    globalEventManager = new EventListenerManager();

    // Log initialization
    console.log('Event management initialized for page');

    return globalEventManager;
}

// Initialize on page load
document.addEventListener('DOMContentLoaded', () => {
    initializeEventManagement();

    // Monitor listener count in development
    if (window.location.hostname === 'localhost') {
        setInterval(() => {
            const manager = getGlobalEventManager();
            if (manager.getListenerCount() > 10) {
                console.warn(`⚠️ High listener count: ${manager.getListenerCount()} active listeners`);
            }
        }, 30000); // Check every 30 seconds
    }
});

// Export for use in other modules
if (typeof module !== 'undefined' && module.exports) {
    module.exports = {
        EventListenerManager,
        PageController,
        CrossPageEventManager,
        getGlobalEventManager,
        initializeEventManagement
    };
}

// Make available globally (renamed to avoid conflict with btpc-event-listeners.js)
window.btpcEventUtils = {
    EventListenerManager,
    PageController,
    CrossPageEventManager,
    getGlobalEventManager,
    initializeEventManagement
};