/**
 * BTPC Common JavaScript - Shared utilities across all pages
 * Quantum-Resistant Cryptocurrency Desktop Application
 */

// NOTE: Tauri API initialization is handled by btpc-tauri-context.js
// window.invoke is set up before this script loads
// All functions in this file use window.invoke directly

/**
 * Show status message in a specified element
 */
function showStatus(elementId, message, type = 'info') {
    const element = document.getElementById(elementId);
    if (element) {
        element.className = `status ${type}`;
        element.textContent = message;
        element.style.display = 'block';
    }
}

/**
 * Format date for display
 */
function formatDate(dateString) {
    const date = new Date(dateString);
    return date.toLocaleDateString() + ' ' + date.toLocaleTimeString([], {hour: '2-digit', minute:'2-digit'});
}

/**
 * Format timestamp for detailed display
 */
function formatTimestamp(timestamp) {
    const date = new Date(timestamp);
    return date.toLocaleString('en-US', {
        year: 'numeric',
        month: 'short',
        day: '2-digit',
        hour: '2-digit',
        minute: '2-digit',
        second: '2-digit'
    });
}

/**
 * Format BTPC amount with 8 decimal places
 */
function formatBTPC(amount) {
    return parseFloat(amount).toFixed(8);
}

/**
 * Copy text to clipboard
 */
async function copyToClipboard(text) {
    try {
        if (navigator.clipboard && navigator.clipboard.writeText) {
            await navigator.clipboard.writeText(text);
            return true;
        } else {
            // Fallback for older browsers
            const textArea = document.createElement('textarea');
            textArea.value = text;
            textArea.style.position = 'fixed';
            textArea.style.left = '-999999px';
            document.body.appendChild(textArea);
            textArea.select();
            const success = document.execCommand('copy');
            document.body.removeChild(textArea);
            return success;
        }
    } catch (error) {
        console.error('Failed to copy to clipboard:', error);
        return false;
    }
}

/**
 * Show loading state in element
 */
function showLoading(elementId, message = 'Loading...') {
    const element = document.getElementById(elementId);
    if (element) {
        element.className = 'status info';
        element.innerHTML = `<span class="loading-spinner">⏳</span> ${message}`;
        element.style.display = 'block';
    }
}

/**
 * Create skeleton loading placeholder
 */
function createSkeleton(type = 'text', width = '100%') {
    const skeleton = document.createElement('div');
    skeleton.className = `skeleton skeleton-${type}`;
    skeleton.style.width = width;
    return skeleton;
}

/**
 * Show skeleton loading in container
 */
function showSkeletonLoading(containerId, count = 3, type = 'text') {
    const container = document.getElementById(containerId);
    if (container) {
        container.innerHTML = '';
        for (let i = 0; i < count; i++) {
            container.appendChild(createSkeleton(type));
        }
    }
}

/**
 * Toast notification system
 */
const Toast = {
    container: null,

    init() {
        if (!this.container) {
            this.container = document.createElement('div');
            this.container.id = 'toast-container';
            this.container.className = 'toast-container';
            document.body.appendChild(this.container);
        }
    },

    show(message, type = 'info', duration = 3000) {
        this.init();

        const toast = document.createElement('div');
        toast.className = `toast toast-${type}`;

        const icon = this.getIcon(type);
        toast.innerHTML = `
            <span class="toast-icon">${icon}</span>
            <span class="toast-message">${message}</span>
        `;

        this.container.appendChild(toast);

        // Trigger animation
        setTimeout(() => toast.classList.add('show'), 10);

        // Auto remove
        setTimeout(() => {
            toast.classList.remove('show');
            setTimeout(() => toast.remove(), 300);
        }, duration);
    },

    getIcon(type) {
        const icons = {
            success: '<svg width="16" height="16" viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M13.5 4.5L6 12L2.5 8.5" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>',
            error: '<svg width="16" height="16" viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M12 4L4 12M4 4L12 12" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>',
            warning: '<svg width="16" height="16" viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M8 2L14.9282 14H1.0718L8 2Z" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/><path d="M8 6V9" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/><circle cx="8" cy="11.5" r="0.5" fill="currentColor"/></svg>',
            info: '<svg width="16" height="16" viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg"><circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.5"/><path d="M8 7V11" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/><circle cx="8" cy="5" r="0.5" fill="currentColor"/></svg>'
        };
        return icons[type] || icons.info;
    },

    success(message, duration) {
        this.show(message, 'success', duration);
    },

    error(message, duration) {
        this.show(message, 'error', duration);
    },

    warning(message, duration) {
        this.show(message, 'warning', duration);
    },

    info(message, duration) {
        this.show(message, 'info', duration);
    }
};

/**
 * Hide element by ID
 */
function hideElement(elementId) {
    const element = document.getElementById(elementId);
    if (element) {
        element.style.display = 'none';
    }
}

/**
 * Show element by ID
 */
function showElement(elementId, displayStyle = 'block') {
    const element = document.getElementById(elementId);
    if (element) {
        element.style.display = displayStyle;
    }
}

/**
 * Toggle element visibility
 */
function toggleElement(elementId) {
    const element = document.getElementById(elementId);
    if (element) {
        if (element.style.display === 'none' || !element.style.display) {
            element.style.display = 'block';
        } else {
            element.style.display = 'none';
        }
    }
}

/**
 * Update network status footer using the centralized update manager
 * This function is called by the update manager when blockchain state changes
 */
function updateNetworkFooter(blockchainData) {
    try {
        // Update network name
        const networkNameEl = document.getElementById('network-name');
        if (networkNameEl) {
            networkNameEl.textContent = blockchainData.chain || 'Mainnet';
        }

        // Update sync status
        const syncStatusEl = document.getElementById('sync-status');
        if (syncStatusEl) {
            if (blockchainData.is_synced) {
                syncStatusEl.textContent = 'Synced';
                syncStatusEl.style.color = 'var(--status-success)';
            } else {
                const progress = blockchainData.sync_progress || 0;
                syncStatusEl.textContent = `Syncing ${progress.toFixed(1)}%`;
                syncStatusEl.style.color = 'var(--status-warning)';
            }
        }

        // Update block height (current / total)
        const blockHeightEls = document.querySelectorAll('#chain-height, #chain-height-sidebar, .chain-height-display');
        blockHeightEls.forEach(el => {
            if (el) {
                const height = (blockchainData.height || 0).toLocaleString();
                const headers = (blockchainData.headers || 0).toLocaleString();
                el.textContent = `${height} / ${headers}`;
            }
        });

        // Update progress bar
        const progressBarEls = document.querySelectorAll('.sync-progress-fill, #sync-progress-sidebar');
        progressBarEls.forEach(el => {
            if (el) {
                const progress = blockchainData.sync_progress || 0;
                el.style.width = `${progress}%`;
                // Change color based on sync status
                if (blockchainData.is_synced) {
                    el.style.backgroundColor = 'var(--status-success)';
                } else {
                    el.style.backgroundColor = 'var(--btpc-primary)';
                }
            }
        });

        // Update network status dot
        const statusDots = document.querySelectorAll('.network-status-dot');
        statusDots.forEach(dot => {
            if (blockchainData.height > 0) {
                dot.classList.remove('disconnected');
            } else {
                dot.classList.add('disconnected');
            }
        });

    } catch (error) {
        console.error('Failed to update network footer:', error);
    }
}

/**
 * Legacy function for backwards compatibility
 */
async function updateNetworkStatus() {
    // This is now handled by the update manager
    // Keep for backwards compatibility but do nothing
    console.debug('updateNetworkStatus() is deprecated, using update manager instead');
}

/**
 * Update sidebar balance display from update manager state
 * Article XI compliant: Uses state from update manager instead of polling backend
 */
function updateSidebarBalance() {
    // Get wallet state from update manager (no backend call)
    if (window.btpcUpdateManager) {
        const state = window.btpcUpdateManager.getState();
        const sidebarBalance = document.getElementById('sidebarBalance');
        if (sidebarBalance && state.wallet.balance !== null && state.wallet.balance !== undefined) {
            sidebarBalance.textContent = formatBTPC(state.wallet.balance);
        }
    }
}

/**
 * Set active navigation item based on current page
 */
function setActiveNavigation() {
    const currentPage = window.location.pathname.split('/').pop() || 'index.html';
    const navItems = document.querySelectorAll('.nav-item');

    navItems.forEach(item => {
        const href = item.getAttribute('href');
        if (href === currentPage) {
            item.classList.add('active');
        } else {
            item.classList.remove('active');
        }
    });
}

/**
 * Handle logout - clears session and redirects to login
 */
async function handleLogout() {
    if (confirm('Are you sure you want to logout?')) {
        try {
            // Clear any stored session data
            localStorage.removeItem('btpc_session');
            localStorage.removeItem('btpc_user');

            // If Tauri backend has a logout command, call it
            if (window.invoke) {
                try {
                    await window.invoke('logout');
                } catch (e) {
                    console.debug('Backend logout not available:', e);
                }
            }

            // Redirect to login page (or index if no login page exists)
            window.location.href = 'login.html';
        } catch (error) {
            console.error('Logout error:', error);
            alert('Failed to logout properly. Please try again.');
        }
    }
}

/**
 * Add date/time display to the page
 */
function addDateTimeDisplay() {
    // Check if display already exists
    if (document.getElementById('datetime-display')) {
        return;
    }

    const dateTimeDisplay = document.createElement('div');
    dateTimeDisplay.id = 'datetime-display';
    dateTimeDisplay.className = 'datetime-display';

    document.body.appendChild(dateTimeDisplay);

    // Update immediately
    updateDateTime();

    // Update every second
    setInterval(updateDateTime, 1000);
}

/**
 * Update the date/time display
 */
function updateDateTime() {
    const display = document.getElementById('datetime-display');
    if (!display) return;

    const now = new Date();
    const dateStr = now.toLocaleDateString(undefined, {
        year: 'numeric',
        month: '2-digit',
        day: '2-digit'
    });
    const timeStr = now.toLocaleTimeString(undefined, {
        hour: '2-digit',
        minute: '2-digit',
        second: '2-digit',
        hour12: true
    });

    display.innerHTML = `
        <span class="datetime-date">${dateStr}</span>
        <span class="datetime-time">${timeStr}</span>
    `;
}

/**
 * Add logout button to the page
 */
function addLogoutButton() {
    // Check if button already exists
    if (document.getElementById('logout-button')) {
        return;
    }

    const logoutBtn = document.createElement('button');
    logoutBtn.id = 'logout-button';
    logoutBtn.className = 'logout-button';
    logoutBtn.innerHTML = '<span class="icon icon-logout"></span> Logout';
    logoutBtn.onclick = handleLogout;

    document.body.appendChild(logoutBtn);
}

// Store unlisten functions for proper cleanup (prevents memory leaks)
let unlistenNetworkConfig = null;
let unlistenNodeStatus = null;

/**
 * Set up Tauri event listeners for unified state management
 */
async function setupTauriEventListeners() {
    if (!window.__TAURI__) {
        console.warn('Tauri API not available for event listeners');
        return;
    }

    try {
        // Get the event module
        const { listen } = window.__TAURI__.event;

        // Listen for network config changes (store unlisten function)
        unlistenNetworkConfig = await listen('network-config-changed', (event) => {
            console.log('Received network-config-changed event:', event.payload);
            const { network, rpc_port, p2p_port } = event.payload;

            // Update network name in footer on all pages
            const networkNameEl = document.getElementById('network-name');
            if (networkNameEl) {
                const networkName = network.charAt(0).toUpperCase() + network.slice(1);
                networkNameEl.textContent = networkName;
                console.log(`Updated network display to: ${networkName}`);
            }

            // Update network type in system info (dashboard)
            const networkTypeEl = document.getElementById('network-type');
            if (networkTypeEl) {
                const networkName = network.charAt(0).toUpperCase() + network.slice(1);
                networkTypeEl.textContent = networkName;
            }

            // Show toast notification
            if (window.Toast) {
                Toast.info(`Network changed to ${network.charAt(0).toUpperCase() + network.slice(1)}`);
            }
        });

        // Listen for node status changes (store unlisten function)
        unlistenNodeStatus = await listen('node-status-changed', (event) => {
            console.log('Received node-status-changed event:', event.payload);
            const { status, pid } = event.payload;

            // Update node status in dashboard if present
            const nodeStatusIcon = document.getElementById('node-status-icon');
            const nodeStatusText = document.getElementById('node-status-text');

            if (status === 'running') {
                if (nodeStatusIcon) {
                    nodeStatusIcon.innerHTML = '<span class="icon icon-link" style="width: 32px; height: 32px; color: var(--status-success);"></span>';
                }
                if (nodeStatusText) {
                    nodeStatusText.textContent = 'Running';
                    nodeStatusText.style.color = 'var(--status-success)';
                }
            } else {
                if (nodeStatusIcon) {
                    nodeStatusIcon.innerHTML = '<span class="icon icon-link" style="width: 32px; height: 32px; opacity: 0.3;"></span>';
                }
                if (nodeStatusText) {
                    nodeStatusText.textContent = 'Offline';
                    nodeStatusText.style.color = 'var(--text-muted)';
                }
            }

            // Update node controls on node.html page if present
            const startNodeBtn = document.getElementById('start-node-btn');
            const stopNodeBtn = document.getElementById('stop-node-btn');
            const nodeStatus = document.getElementById('node-status');

            if (status === 'running') {
                if (startNodeBtn) startNodeBtn.style.display = 'none';
                if (stopNodeBtn) stopNodeBtn.style.display = 'inline-flex';
                if (nodeStatus) nodeStatus.innerHTML = '<span class="icon icon-circle-check"></span> Running';

                // Only show toast if not on the page that initiated the action
                if (window.Toast && !window.nodeActionInitiatedByThisPage) {
                    Toast.success('Node started successfully');
                }
            } else {
                if (startNodeBtn) startNodeBtn.style.display = 'inline-flex';
                if (stopNodeBtn) stopNodeBtn.style.display = 'none';
                if (nodeStatus) nodeStatus.innerHTML = '<span class="icon icon-circle-x"></span> Offline';

                // Only show toast if not on the page that initiated the action
                if (window.Toast && !window.nodeActionInitiatedByThisPage) {
                    Toast.info('Node stopped');
                }
            }

            // Reset flag after handling event
            window.nodeActionInitiatedByThisPage = false;

            console.log(`Updated node status display: ${status} (PID: ${pid || 'none'})`);
        });

        console.log('Tauri event listeners registered');
    } catch (error) {
        console.error('Failed to set up Tauri event listeners:', error);
    }
}

/**
 * Initialize common page features
 */
async function initCommonFeatures() {
    // NOTE: Tauri initialization is handled by btpc-tauri-context.js (loaded before this script)

    // Set active navigation
    setActiveNavigation();

    // Add date/time display
    addDateTimeDisplay();

    // Logout button is now in HTML (no longer dynamically created)
    // addLogoutButton(); // REMOVED: Caused duplicate buttons with HTML button

    // Set up Tauri event listeners for unified state management
    await setupTauriEventListeners();

    // Subscribe to updates from the update manager (Article XI compliant event-driven)
    if (window.btpcUpdateManager) {
        window.btpcUpdateManager.subscribe((type, data, fullState) => {
            if (type === 'blockchain') {
                updateNetworkFooter(data);
            } else if (type === 'wallet') {
                // Update sidebar balance when wallet state changes (event-driven)
                updateSidebarBalance();
            }
        });
        console.log('Subscribed to blockchain and wallet updates');

        // Article XI, Section 11.6: Start update manager globally once (singleton pattern)
        // All pages benefit from centralized polling - no page-specific intervals needed
        window.btpcUpdateManager.startAutoUpdate(5000);
        console.log('Global update manager initialized (Article XI compliance)');

        // Initial update from current state
        updateSidebarBalance();
    }

    console.log('Common features initialized');
}

/**
 * Cleanup function for page unload
 */
function cleanupCommonFeatures() {
    // Clean up event listeners (prevent memory leaks)
    if (unlistenNetworkConfig) {
        unlistenNetworkConfig();
        unlistenNetworkConfig = null;
        console.log('🧹 Cleaned up network-config-changed listener');
    }
    if (unlistenNodeStatus) {
        unlistenNodeStatus();
        unlistenNodeStatus = null;
        console.log('🧹 Cleaned up node-status-changed listener');
    }

    // Clean up interval
    if (window.btpcStatusInterval) {
        clearInterval(window.btpcStatusInterval);
        delete window.btpcStatusInterval;
    }
}

// Auto-initialize when DOM is loaded
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', initCommonFeatures);
} else {
    initCommonFeatures();
}

// Cleanup on page unload
window.addEventListener('beforeunload', cleanupCommonFeatures);
// ===== UI Polish: Loading & Toast System =====
// Added: 2025-10-22

/**
 * Show loading overlay with optional message
 * @param {string} message - Main loading message
 * @param {string} submessage - Optional secondary message
 * @returns {HTMLElement} The loading overlay element
 */
function showLoading(message = 'Processing...', submessage = '') {
    // Remove existing loading overlay if any
    hideLoading();

    const overlay = document.createElement('div');
    overlay.className = 'loading-overlay';
    overlay.id = 'btpc-loading-overlay';

    overlay.innerHTML = `
        <div class="loading-container">
            <div class="loading-spinner"></div>
            <div class="loading-text">${message}</div>
            ${submessage ? `<div class="loading-subtext">${submessage}</div>` : ''}
        </div>
    `;

    document.body.appendChild(overlay);
    return overlay;
}

/**
 * Hide loading overlay
 */
function hideLoading() {
    const overlay = document.getElementById('btpc-loading-overlay');
    if (overlay) {
        overlay.remove();
    }
}

/**
 * Show toast notification
 * @param {string} message - Toast message
 * @param {string} type - Toast type: 'success', 'error', 'warning', 'info'
 * @param {number} duration - Duration in ms (default: 5000, 0 = no auto-dismiss)
 * @param {string} title - Optional title
 */
function showToast(message, type = 'info', duration = 5000, title = '') {
    // Create toast container if it doesn't exist
    let container = document.getElementById('btpc-toast-container');
    if (!container) {
        container = document.createElement('div');
        container.id = 'btpc-toast-container';
        container.className = 'toast-container';
        document.body.appendChild(container);
    }

    // Generate unique ID for this toast
    const toastId = `toast-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;

    // Icon symbols for each type
    const icons = {
        success: '✓',
        error: '✕',
        warning: '⚠',
        info: 'ⓘ'
    };

    // Default titles
    const titles = {
        success: title || 'Success',
        error: title || 'Error',
        warning: title || 'Warning',
        info: title || 'Info'
    };

    // Create toast element
    const toast = document.createElement('div');
    toast.className = `toast ${type}`;
    toast.id = toastId;

    toast.innerHTML = `
        <div class="toast-icon">${icons[type] || icons.info}</div>
        <div class="toast-content">
            <div class="toast-title">${titles[type]}</div>
            <div class="toast-message">${message}</div>
        </div>
        <button class="toast-close" aria-label="Close">×</button>
    `;

    // Add to container
    container.appendChild(toast);

    // Close button handler
    const closeBtn = toast.querySelector('.toast-close');
    closeBtn.addEventListener('click', () => dismissToast(toastId));

    // Auto-dismiss if duration > 0
    if (duration > 0) {
        setTimeout(() => dismissToast(toastId), duration);
    }

    return toastId;
}

/**
 * Dismiss a specific toast
 * @param {string} toastId - Toast element ID
 */
function dismissToast(toastId) {
    const toast = document.getElementById(toastId);
    if (!toast) return;

    // Add removing class for exit animation
    toast.classList.add('removing');

    // Remove after animation completes
    setTimeout(() => {
        toast.remove();

        // Remove container if empty
        const container = document.getElementById('btpc-toast-container');
        if (container && container.children.length === 0) {
            container.remove();
        }
    }, 200);
}

/**
 * Convenience methods for common toast types
 */
const toast = {
    success: (message, duration = 5000, title = '') => showToast(message, 'success', duration, title),
    error: (message, duration = 120000, title = '') => showToast(message, 'error', duration, title),
    warning: (message, duration = 8000, title = '') => showToast(message, 'warning', duration, title),
    info: (message, duration = 5000, title = '') => showToast(message, 'info', duration, title)
};

/**
 * Format relative time (e.g., "2 minutes ago")
 * @param {Date|number} date - Date object or timestamp
 * @returns {string} Relative time string
 */
function formatRelativeTime(date) {
    const timestamp = date instanceof Date ? date.getTime() : date;
    const now = Date.now();
    const seconds = Math.floor((now - timestamp) / 1000);

    if (seconds < 60) return 'just now';
    if (seconds < 3600) return `${Math.floor(seconds / 60)} minutes ago`;
    if (seconds < 86400) return `${Math.floor(seconds / 3600)} hours ago`;
    if (seconds < 604800) return `${Math.floor(seconds / 86400)} days ago`;

    // For older dates, show formatted date
    return new Date(timestamp).toLocaleDateString();
}

/**
 * Copy text to clipboard with feedback
 * @param {string} text - Text to copy
 * @param {string} successMessage - Optional success message
 */
async function copyToClipboard(text, successMessage = 'Copied to clipboard') {
    try {
        if (navigator.clipboard && navigator.clipboard.writeText) {
            await navigator.clipboard.writeText(text);
            toast.success(successMessage);
        } else {
            // Fallback for older browsers
            const textarea = document.createElement('textarea');
            textarea.value = text;
            textarea.style.position = 'fixed';
            textarea.style.opacity = '0';
            document.body.appendChild(textarea);
            textarea.select();
            document.execCommand('copy');
            document.body.removeChild(textarea);
            toast.success(successMessage);
        }
    } catch (error) {
        console.error('Failed to copy:', error);
        toast.error('Failed to copy to clipboard');
    }
}

// Export utilities to global scope
window.showLoading = showLoading;
window.hideLoading = hideLoading;
window.showToast = showToast;
window.dismissToast = dismissToast;
window.toast = toast;
window.formatRelativeTime = formatRelativeTime;
window.copyToClipboard = copyToClipboard;

console.log('✨ UI Polish utilities loaded: loading, toast, clipboard');
