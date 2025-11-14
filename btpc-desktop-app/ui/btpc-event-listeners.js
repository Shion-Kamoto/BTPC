/**
 * BTPC Event Listeners Module
 *
 * Handles Tauri backend events for authentication state changes.
 * Implements Article XI Section 11.3: Event-Driven Architecture
 *
 * Events:
 * - session:login: Emitted after successful authentication
 * - session:logout: Emitted after session termination
 *
 * Features:
 * - Event-driven UI updates
 * - Toast notifications for state changes
 * - Debug logging for development
 * - Auto-initialization on page load
 */

class BtpcEventManager {
    constructor() {
        this.listeners = new Map();
        this.toastContainer = null;
        this.debugMode = true; // Enable debug logging
    }

    /**
     * Initialize event listeners for authentication events
     */
    async initialize() {
        try {
            // Check if Tauri API is available
            if (!window.__TAURI__?.event) {
                console.warn('[Event Manager] Tauri event API not available');
                return;
            }

            const { listen } = window.__TAURI__.event;

            // Listen for session:login events
            const loginUnlisten = await listen('session:login', (event) => {
                this.handleSessionLogin(event.payload);
            });
            this.listeners.set('session:login', loginUnlisten);

            // Listen for session:logout events
            const logoutUnlisten = await listen('session:logout', (event) => {
                this.handleSessionLogout(event.payload);
            });
            this.listeners.set('session:logout', logoutUnlisten);

            if (this.debugMode) {
                console.log('[Event Manager] Event listeners initialized successfully');
            }
        } catch (error) {
            console.error('[Event Manager] Failed to initialize event listeners:', error);
        }
    }

    /**
     * Handle session:login event
     * @param {Object} payload - Event payload { session_token: string, timestamp: number }
     */
    handleSessionLogin(payload) {
        if (this.debugMode) {
            console.log('[Event Manager] session:login event received:', payload);
        }

        // Show success toast notification
        this.showToast('Login successful', 'success');

        // Update UI elements (if needed)
        this.updateAuthenticatedState(true);

        // Trigger custom event for other components
        window.dispatchEvent(new CustomEvent('btpc:authenticated', {
            detail: { sessionToken: payload.session_token }
        }));
    }

    /**
     * Handle session:logout event
     * @param {Object} payload - Event payload { timestamp: number }
     */
    handleSessionLogout(payload) {
        if (this.debugMode) {
            console.log('[Event Manager] session:logout event received:', payload);
        }

        // Show logout toast notification
        this.showToast('Logged out successfully', 'info');

        // Update UI elements (if needed)
        this.updateAuthenticatedState(false);

        // Trigger custom event for other components
        window.dispatchEvent(new CustomEvent('btpc:unauthenticated', {
            detail: { timestamp: payload.timestamp }
        }));
    }

    /**
     * Update UI based on authentication state
     * @param {boolean} authenticated - Whether user is authenticated
     */
    updateAuthenticatedState(authenticated) {
        // This can be extended to update specific UI elements
        // For now, we rely on navigation guard to handle redirects
        if (this.debugMode) {
            console.log(`[Event Manager] Authentication state: ${authenticated ? 'AUTHENTICATED' : 'NOT AUTHENTICATED'}`);
        }
    }

    /**
     * Show toast notification
     * @param {string} message - Toast message
     * @param {string} type - Toast type: 'success', 'error', 'info', 'warning'
     */
    showToast(message, type = 'info') {
        // Create toast container if it doesn't exist
        if (!this.toastContainer) {
            this.toastContainer = document.createElement('div');
            this.toastContainer.id = 'btpc-toast-container';
            this.toastContainer.style.cssText = `
                position: fixed;
                top: 20px;
                right: 20px;
                z-index: 10000;
                display: flex;
                flex-direction: column;
                gap: 10px;
                pointer-events: none;
            `;
            document.body.appendChild(this.toastContainer);
        }

        // Create toast element
        const toast = document.createElement('div');
        toast.className = `btpc-toast btpc-toast-${type}`;

        // Toast styles
        const colors = {
            success: { bg: 'rgba(16, 185, 129, 0.9)', border: '#10b981' },
            error: { bg: 'rgba(239, 68, 68, 0.9)', border: '#ef4444' },
            info: { bg: 'rgba(59, 130, 246, 0.9)', border: '#3b82f6' },
            warning: { bg: 'rgba(245, 158, 11, 0.9)', border: '#f59e0b' }
        };
        const color = colors[type] || colors.info;

        toast.style.cssText = `
            background: ${color.bg};
            border: 1px solid ${color.border};
            border-radius: 8px;
            padding: 12px 16px;
            color: white;
            font-size: 0.875rem;
            font-weight: 500;
            box-shadow: 0 10px 25px rgba(0, 0, 0, 0.3);
            pointer-events: auto;
            cursor: pointer;
            animation: slideInRight 0.3s ease-out;
            min-width: 200px;
            max-width: 400px;
        `;

        // Add animation keyframes if not already added
        if (!document.getElementById('btpc-toast-animations')) {
            const style = document.createElement('style');
            style.id = 'btpc-toast-animations';
            style.textContent = `
                @keyframes slideInRight {
                    from {
                        transform: translateX(400px);
                        opacity: 0;
                    }
                    to {
                        transform: translateX(0);
                        opacity: 1;
                    }
                }
                @keyframes slideOutRight {
                    from {
                        transform: translateX(0);
                        opacity: 1;
                    }
                    to {
                        transform: translateX(400px);
                        opacity: 0;
                    }
                }
            `;
            document.head.appendChild(style);
        }

        toast.textContent = message;

        // Add click to dismiss
        toast.addEventListener('click', () => {
            this.removeToast(toast);
        });

        // Add to container
        this.toastContainer.appendChild(toast);

        // Auto-remove after 3 seconds
        setTimeout(() => {
            this.removeToast(toast);
        }, 3000);
    }

    /**
     * Remove toast with animation
     * @param {HTMLElement} toast - Toast element to remove
     */
    removeToast(toast) {
        if (!toast || !toast.parentElement) return;

        toast.style.animation = 'slideOutRight 0.3s ease-in';
        setTimeout(() => {
            if (toast.parentElement) {
                toast.parentElement.removeChild(toast);
            }
        }, 300);
    }

    /**
     * Clean up event listeners (call on page unload if needed)
     */
    destroy() {
        this.listeners.forEach((unlisten, eventName) => {
            unlisten();
            if (this.debugMode) {
                console.log(`[Event Manager] Unregistered listener for ${eventName}`);
            }
        });
        this.listeners.clear();

        if (this.toastContainer && this.toastContainer.parentElement) {
            this.toastContainer.parentElement.removeChild(this.toastContainer);
        }
    }
}

// Global instance
window.btpcEventManager = new BtpcEventManager();

// Auto-initialize on DOM ready
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => {
        window.btpcEventManager.initialize();
    });
} else {
    // DOM already loaded
    window.btpcEventManager.initialize();
}

// Export for module usage
if (typeof module !== 'undefined' && module.exports) {
    module.exports = BtpcEventManager;
}