/**
 * BTPC Common JavaScript - Shared utilities across all pages
 * Quantum-Resistant Cryptocurrency Desktop Application
 */

// NOTE: Tauri API initialization is handled by btpc-tauri-context.js
// window.invoke is set up before this script loads
// All functions in this file use window.invoke directly

/**
 * FIX 2026-02-21 (H8): HTML escape to prevent XSS in innerHTML assignments
 * Use this for ANY user-supplied data (wallet nicknames, address book labels, etc.)
 */
function escapeHtml(str) {
    if (str === null || str === undefined) return '';
    return String(str)
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;')
        .replace(/'/g, '&#39;');
}

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
async function updateNetworkFooter(blockchainData) {
    try {
        // Update network name from backend config (not blockchain data)
        // This ensures the sidebar shows the user's selected network, not the chain's genesis network
        const networkNameEl = document.getElementById('network-name');
        if (networkNameEl && window.invoke) {
            try {
                const networkConfig = await window.invoke('get_network_config');
                if (networkConfig && networkConfig.network) {
                    const networkName = networkConfig.network.charAt(0).toUpperCase() + networkConfig.network.slice(1);
                    networkNameEl.textContent = networkName;
                } else {
                    networkNameEl.textContent = blockchainData.chain || 'Mainnet';
                }
            } catch (e) {
                // Fallback to blockchain data if config fetch fails
                networkNameEl.textContent = blockchainData.chain || 'Mainnet';
            }
        } else if (networkNameEl) {
            networkNameEl.textContent = blockchainData.chain || 'Mainnet';
        }

        // Update sync status - check if node is running first
        const syncStatusEl = document.getElementById('sync-status');
        if (syncStatusEl) {
            // Get node status from updateManager state
            const nodeRunning = window.btpcUpdateManager && window.btpcUpdateManager.state.node.running;

            if (!nodeRunning) {
                // Node is off - show offline status
                syncStatusEl.textContent = 'Offline';
                syncStatusEl.style.color = 'var(--text-muted)';
            } else if (blockchainData.is_synced) {
                syncStatusEl.textContent = 'Synced';
                syncStatusEl.style.color = 'var(--status-success)';
            } else {
                const progress = blockchainData.sync_progress || 0;
                syncStatusEl.textContent = `Syncing ${progress.toFixed(1)}%`;
                syncStatusEl.style.color = 'var(--status-warning)';
            }
        }

        // Update block height
        // For embedded node, headers always equal blocks (self-contained blockchain)
        // Only show sync progress if actually syncing from network (height < headers)
        const blockHeightEls = document.querySelectorAll('#chain-height, #chain-height-sidebar, .chain-height-display');
        blockHeightEls.forEach(el => {
            if (el) {
                const height = blockchainData.height || 0;
                const headers = blockchainData.headers || 0;

                // Show "height / headers" only when syncing (height < headers)
                // Otherwise just show height for cleaner display
                if (height < headers) {
                    el.textContent = `${height.toLocaleString()} / ${headers.toLocaleString()}`;
                } else {
                    el.textContent = height.toLocaleString();
                }
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

    // Insert into sidebar, before the network-status-footer
    const networkFooter = document.querySelector('.network-status-footer');
    if (networkFooter && networkFooter.parentNode) {
        networkFooter.parentNode.insertBefore(dateTimeDisplay, networkFooter);
    } else {
        // Fallback: append to sidebar nav
        const sidebar = document.querySelector('.sidebar');
        if (sidebar) {
            sidebar.appendChild(dateTimeDisplay);
        } else {
            document.body.appendChild(dateTimeDisplay);
        }
    }

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
        <span style="color: var(--border-color);">|</span>
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

/**
 * Show network warning banner on non-mainnet networks (or mainnet caution notice).
 * Called on DOMContentLoaded for all pages.
 */
function showNetworkBanner() {
    if (document.getElementById('network-warning-banner')) return;

    const networkEl = document.getElementById('network-name');
    if (!networkEl) return;

    const net = (networkEl.textContent || 'Mainnet').toLowerCase();
    if (net === 'mainnet') return; // No banner needed on mainnet

    const banner = document.createElement('div');
    banner.id = 'network-warning-banner';

    if (net === 'testnet') {
        banner.style.cssText = 'position:fixed;top:0;left:0;right:0;z-index:9999;padding:6px 16px;text-align:center;font-size:0.8125rem;font-weight:600;background:rgba(245,158,11,0.15);color:#f59e0b;border-bottom:1px solid rgba(245,158,11,0.3);pointer-events:none;';
        banner.textContent = 'TESTNET — Coins on this network have no real value';
    } else if (net === 'regtest') {
        banner.style.cssText = 'position:fixed;top:0;left:0;right:0;z-index:9999;padding:6px 16px;text-align:center;font-size:0.8125rem;font-weight:600;background:rgba(59,130,246,0.15);color:#3b82f6;border-bottom:1px solid rgba(59,130,246,0.3);pointer-events:none;';
        banner.textContent = 'REGTEST — Local development network';
    }

    if (banner.textContent) {
        document.body.prepend(banner);
        // Shift page content down to avoid overlap
        document.body.style.paddingTop = '32px';
    }
}

// Store unlisten functions for proper cleanup (prevents memory leaks)
let unlistenNetworkConfig = null;
let unlistenNodeStatus = null;
let unlistenReorgDetected = null;
let unlistenReorgInProgress = null;
let unlistenReorgCompleted = null;
let unlistenReorgFailed = null;
let unlistenDiskSpaceWarning = null;

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

            // Update chain info in Blockchain Info tab (node page)
            const infoChainEl = document.getElementById('info-chain');
            if (infoChainEl) {
                const networkName = network.charAt(0).toUpperCase() + network.slice(1);
                infoChainEl.textContent = networkName;
                console.log(`Updated info-chain display to: ${networkName}`);
            }

            // Show toast notification
            if (window.Toast) {
                Toast.info(`Network changed to ${network.charAt(0).toUpperCase() + network.slice(1)}`);
            }

            // FIX 2025-12-09: Reload wallets and transactions when network changes
            // Without this, pages still have cached wallet IDs from the previous network
            // which causes transaction queries to fail (wallet ID not found in new network)
            console.log('🔄 Network changed - triggering data reload...');

            // Dispatch custom event for pages to handle network change
            window.dispatchEvent(new CustomEvent('btpc:network-changed', {
                detail: { network, rpc_port, p2p_port }
            }));

            // If page has loadWallets function, call it to refresh wallet list
            if (typeof window.loadWallets === 'function') {
                console.log('🔄 Reloading wallets for new network...');
                window.loadWallets();
            }

            // If page has loadTransactions function, call it to refresh transactions
            if (typeof window.loadTransactions === 'function') {
                console.log('🔄 Reloading transactions for new network...');
                // Reset any cached wallet ID first
                if (typeof window.selectedWalletId !== 'undefined') {
                    window.selectedWalletId = null;
                }
                window.loadTransactions();
            }

            // If page has loadMiningAddresses function (mining.html), call it
            if (typeof window.loadMiningAddresses === 'function') {
                console.log('🔄 Reloading mining addresses for new network...');
                window.loadMiningAddresses();
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

        // FR-057: Listen for chain reorganization events
        unlistenReorgDetected = await listen('chain:reorg_detected', (event) => {
            console.log('Chain reorganization detected:', event.payload);
            showReorgBanner('detecting', event.payload);
            if (window.Toast) {
                Toast.warning(`Chain reorganization detected at height ${event.payload.fork_point_height}`);
            }
        });

        unlistenReorgInProgress = await listen('chain:reorg_in_progress', (event) => {
            console.log('Chain reorganization in progress:', event.payload);
            updateReorgProgress(event.payload);
        });

        unlistenReorgCompleted = await listen('chain:reorg_completed', (event) => {
            console.log('Chain reorganization completed:', event.payload);
            hideReorgBanner();
            if (window.Toast) {
                Toast.success(`Chain reorganization complete: ${event.payload.blocks_disconnected} blocks replaced`);
            }
        });

        unlistenReorgFailed = await listen('chain:reorg_failed', (event) => {
            console.error('Chain reorganization failed:', event.payload);
            hideReorgBanner();
            if (window.Toast) {
                Toast.error(`Chain reorganization failed: ${event.payload.error}`);
            }
        });

        // FR-058: Listen for disk space warning events
        unlistenDiskSpaceWarning = await listen('disk:space_warning', (event) => {
            console.warn('Low disk space warning:', event.payload);
            if (window.Toast) {
                Toast.warning(`Low disk space: ${event.payload.available_formatted} remaining`, 10000);
            }
        });

        await listen('disk:sync_paused', (event) => {
            console.warn('Sync paused due to low disk space:', event.payload);
            if (window.Toast) {
                Toast.error(`Blockchain sync paused: Only ${event.payload.available_formatted} disk space remaining. Free up space to continue.`, 0);
            }
        });

        await listen('disk:mining_prevented', (event) => {
            console.warn('Mining prevented due to low disk space:', event.payload);
            if (window.Toast) {
                Toast.error(`Mining disabled: Only ${event.payload.available_formatted} disk space remaining. Free up space to mine.`, 0);
            }
        });

        console.log('Tauri event listeners registered (including reorg and disk space events)');
    } catch (error) {
        console.error('Failed to set up Tauri event listeners:', error);
    }
}

/**
 * FR-057: Show chain reorganization banner
 */
function showReorgBanner(status, data) {
    // Remove existing banner if any
    hideReorgBanner();

    const banner = document.createElement('div');
    banner.id = 'reorg-banner';
    banner.className = 'reorg-banner';
    banner.innerHTML = `
        <div class="reorg-banner-content">
            <span class="reorg-icon">🔄</span>
            <span class="reorg-text">Chain Reorganization in Progress</span>
            <span class="reorg-details" id="reorg-details">
                Detecting fork at height ${data.fork_point_height || '...'}
            </span>
        </div>
    `;

    // Insert at top of main content area
    const mainContent = document.querySelector('.main-content') || document.body;
    mainContent.insertBefore(banner, mainContent.firstChild);
}

/**
 * FR-057: Update reorg progress indicator
 */
function updateReorgProgress(data) {
    const details = document.getElementById('reorg-details');
    if (details) {
        details.textContent = `Disconnecting ${data.blocks_to_disconnect} blocks, connecting ${data.blocks_to_connect} blocks (${data.current_progress}/${data.blocks_to_disconnect + data.blocks_to_connect})`;
    }
}

/**
 * FR-057: Hide chain reorganization banner
 */
function hideReorgBanner() {
    const banner = document.getElementById('reorg-banner');
    if (banner) {
        banner.classList.add('removing');
        setTimeout(() => banner.remove(), 300);
    }
}

/**
 * Check and execute auto-start settings (node and mining)
 * Only runs once per session to prevent multiple starts on page navigation
 */
/**
 * Get mining configuration for auto-start
 * Retrieves saved mining address or first available wallet address
 */
async function getAutoStartMiningConfig() {
    try {
        // Try to get saved mining address from miningConfig
        const miningConfig = window.btpcStorage.getMiningConfig();
        let miningAddress = miningConfig.address || '';

        // If no saved address, try to get first wallet
        if (!miningAddress) {
            const wallets = await window.invoke('list_wallets');
            if (wallets && wallets.length > 0) {
                miningAddress = wallets[0].address;
            }
        }

        if (!miningAddress) {
            console.warn('Auto-start mining: No wallet address available');
            return null;
        }

        const config = {
            enable_cpu: false,
            enable_gpu: true,
            cpu_threads: null,
            mining_address: miningAddress,
            mining_mode: miningConfig.mining_mode || 'solo',
            pool_config: null
        };

        // Include pool config if in pool mode
        if (config.mining_mode === 'pool' && miningConfig.pool_url) {
            config.pool_config = {
                url: miningConfig.pool_url,
                worker: miningConfig.pool_worker || 'default',
                password: miningConfig.pool_password || 'x'
            };
        }

        return config;
    } catch (e) {
        console.error('Failed to get mining config for auto-start:', e);
        return null;
    }
}

async function checkAutoStartSettings() {
    // FIX 2025-12-13: Use sessionStorage flag - persists for entire browser session
    // Only run auto-start ONCE per app launch, not on every page navigation
    const AUTOSTART_KEY = 'btpc_autostart_done';
    const NODE_START_TIME_KEY = 'btpc_node_start_time';

    // Check if already ran this session
    if (sessionStorage.getItem(AUTOSTART_KEY) === 'true') {
        console.log('[Auto-Start] Already ran this session, skipping');
        return;
    }

    // FIX 2025-12-27: Fresh app launch - clear stale node uptime data
    // This prevents the "3+ days uptime" bug when app restarts
    // The correct uptime will be set when node actually starts
    const storedUptime = localStorage.getItem(NODE_START_TIME_KEY);
    if (storedUptime) {
        console.log('[Auto-Start] Clearing stale node uptime from previous session');
        localStorage.removeItem(NODE_START_TIME_KEY);
    }

    // Mark as done for this session
    sessionStorage.setItem(AUTOSTART_KEY, 'true');
    console.log('[Auto-Start] Fresh app launch detected, checking settings...');

    if (!window.invoke) {
        console.warn('[Auto-Start] ❌ Tauri API (window.invoke) not ready');
        return;
    }

    // FIX 2025-12-13: Wait for btpcStorage to be ready (retry up to 5 times)
    let retryCount = 0;
    while (!window.btpcStorage && retryCount < 5) {
        console.log('[Auto-Start] Waiting for btpcStorage... attempt', retryCount + 1);
        await new Promise(resolve => setTimeout(resolve, 500));
        retryCount++;
    }

    if (!window.btpcStorage) {
        console.warn('[Auto-Start] ❌ btpcStorage not ready after 5 attempts');
        return;
    }
    console.log('[Auto-Start] ✅ btpcStorage ready');

    try {
        const nodeConfig = window.btpcStorage.getNodeConfig();
        const miningConfig = window.btpcStorage.getMiningConfig();

        // FIX 2025-12-08: Enhanced debugging for auto-start issues
        console.log('[Auto-Start] ========================================');
        console.log('[Auto-Start] Reading settings from localStorage...');
        console.log('[Auto-Start] Node config:', JSON.stringify(nodeConfig, null, 2));
        console.log('[Auto-Start] Mining config:', JSON.stringify(miningConfig, null, 2));
        console.log('[Auto-Start] autoConnect value:', nodeConfig.autoConnect, '(type:', typeof nodeConfig.autoConnect + ')');
        console.log('[Auto-Start] autoStart value:', miningConfig.autoStart, '(type:', typeof miningConfig.autoStart + ')');

        // Check if node should auto-start
        if (nodeConfig.autoConnect) {
            // Check if node is already running
            try {
                const nodeStatus = await window.invoke('get_node_status');
                if (!nodeStatus.running) {
                    console.log('Auto-starting node on application launch...');
                    // FIX 2025-12-27: Clear old uptime and set fresh start time
                    // This fixes the "3+ days uptime" bug when auto-starting
                    const NODE_START_TIME_KEY = 'btpc_node_start_time';
                    localStorage.setItem(NODE_START_TIME_KEY, Date.now().toString());

                    await window.invoke('start_node');
                    if (window.Toast) {
                        Toast.info('Node auto-started (Settings > Application)');
                    }

                    // If mining should also auto-start, wait for node then start mining
                    if (miningConfig.autoStart) {
                        // Wait a moment for node to initialize
                        setTimeout(async () => {
                            try {
                                const autoMiningConfig = await getAutoStartMiningConfig();
                                if (autoMiningConfig) {
                                    console.log('Auto-starting mining on node start...', autoMiningConfig);
                                    await window.invoke('start_mining', { config: autoMiningConfig });
                                    if (window.Toast) {
                                        Toast.info('Mining auto-started (Settings > Node)');
                                    }
                                } else {
                                    console.warn('Auto-start mining skipped: No wallet address available');
                                    if (window.Toast) {
                                        Toast.warning('Mining not started: Create a wallet first');
                                    }
                                }
                            } catch (miningErr) {
                                console.error('Failed to auto-start mining:', miningErr);
                            }
                        }, 2000);
                    }
                } else {
                    console.log('Node already running, skipping auto-start');
                    // But still check if mining should auto-start
                    if (miningConfig.autoStart) {
                        try {
                            const miningStatus = await window.invoke('get_mining_status');
                            if (!miningStatus.is_mining) {
                                const autoMiningConfig = await getAutoStartMiningConfig();
                                if (autoMiningConfig) {
                                    console.log('Auto-starting mining (node already running)...', autoMiningConfig);
                                    await window.invoke('start_mining', { config: autoMiningConfig });
                                    if (window.Toast) {
                                        Toast.info('Mining auto-started (Settings > Node)');
                                    }
                                } else {
                                    console.warn('Auto-start mining skipped: No wallet address available');
                                }
                            }
                        } catch (e) {
                            console.error('Failed to check/start mining:', e);
                        }
                    }
                }
            } catch (e) {
                console.error('Failed to check node status for auto-start:', e);
            }
        } else if (miningConfig.autoStart) {
            // Mining auto-start is enabled but node auto-start is not
            // Check if node is running and start mining if so
            try {
                const nodeStatus = await window.invoke('get_node_status');
                if (nodeStatus.running) {
                    const miningStatus = await window.invoke('get_mining_status');
                    if (!miningStatus.is_mining) {
                        const autoMiningConfig = await getAutoStartMiningConfig();
                        if (autoMiningConfig) {
                            console.log('Auto-starting mining (node already running)...', autoMiningConfig);
                            await window.invoke('start_mining', { config: autoMiningConfig });
                            if (window.Toast) {
                                Toast.info('Mining auto-started (Settings > Node)');
                            }
                        } else {
                            console.warn('Auto-start mining skipped: No wallet address available');
                        }
                    }
                }
            } catch (e) {
                console.error('Failed to auto-start mining:', e);
            }
        }
    } catch (error) {
        console.error('Error checking auto-start settings:', error);
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

    // Show network warning banner for non-mainnet networks
    showNetworkBanner();

    // Logout button is now in HTML (no longer dynamically created)
    // addLogoutButton(); // REMOVED: Caused duplicate buttons with HTML button

    // Set up Tauri event listeners for unified state management
    await setupTauriEventListeners();

    // Check and execute auto-start settings (node and mining)
    // Delayed slightly to ensure all APIs are ready
    setTimeout(checkAutoStartSettings, 1000);

    // Subscribe to updates from the update manager (Article XI compliant event-driven)
    if (window.btpcUpdateManager) {
        window.btpcUpdateManager.subscribe((type, data, fullState) => {
            if (type === 'blockchain') {
                updateNetworkFooter(data);
            } else if (type === 'node') {
                // When node status changes, update sync status display
                updateNetworkFooter(fullState.blockchain);
            } else if (type === 'wallet') {
                // Update sidebar balance when wallet state changes (event-driven)
                updateSidebarBalance();
            }
        });
        console.log('Subscribed to node, blockchain and wallet updates');

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

// ============================================
// Custom Dropdown Component - Purple Theme
// Converts native <select> to styled dropdown
// ============================================

/**
 * Initialize a custom dropdown from a native select element
 * @param {string} selectId - ID of the native select element
 * @param {Object} options - Configuration options
 * @param {Function} options.onChange - Callback when value changes
 * @param {Function} options.formatItem - Custom item formatter (receives option element, returns HTML)
 */
function initBtpcDropdown(selectId, options = {}) {
    const select = document.getElementById(selectId);
    if (!select) {
        console.warn(`[BtpcDropdown] Select element #${selectId} not found`);
        return null;
    }

    // Create dropdown container
    const container = document.createElement('div');
    container.className = 'btpc-dropdown';
    container.id = `${selectId}-dropdown`;

    // Create dropdown button
    const btn = document.createElement('button');
    btn.type = 'button';
    btn.className = 'btpc-dropdown-btn';
    btn.innerHTML = `<span class="selected-text">Select...</span><span class="chevron">▼</span>`;

    // Create dropdown menu
    const menu = document.createElement('div');
    menu.className = 'btpc-dropdown-menu';

    // Populate menu from select options
    function populateMenu() {
        menu.innerHTML = '';
        Array.from(select.options).forEach((option, index) => {
            if (option.value === '' && index === 0) {
                // Skip placeholder option
                return;
            }
            const item = document.createElement('div');
            item.className = 'btpc-dropdown-item';
            item.dataset.value = option.value;

            // FIX 2025-12-30: Apply wallet color as left border if data-color exists
            if (option.dataset.color) {
                item.style.borderLeft = `4px solid ${option.dataset.color}`;
                item.style.paddingLeft = '12px';
            }

            if (options.formatItem) {
                item.innerHTML = options.formatItem(option);
            } else {
                item.innerHTML = `<div class="item-label">${option.textContent}</div>`;
            }

            if (option.selected && option.value !== '') {
                item.classList.add('selected');
                btn.querySelector('.selected-text').textContent = option.textContent;
            }

            item.addEventListener('click', () => {
                selectItem(option.value, option.textContent);
            });

            menu.appendChild(item);
        });

        // Show placeholder if no selection
        if (!select.value) {
            const placeholder = select.options[0]?.textContent || 'Select...';
            btn.querySelector('.selected-text').textContent = placeholder;
        }
    }

    // Select an item
    function selectItem(value, text) {
        select.value = value;
        btn.querySelector('.selected-text').textContent = text;

        // Update selected state
        menu.querySelectorAll('.btpc-dropdown-item').forEach(item => {
            item.classList.toggle('selected', item.dataset.value === value);
        });

        // Close dropdown
        closeDropdown();

        // Trigger change event on original select
        select.dispatchEvent(new Event('change', { bubbles: true }));

        // Call onChange callback
        if (options.onChange) {
            options.onChange(value, text);
        }
    }

    // Toggle dropdown
    let isOpen = false;
    function toggleDropdown() {
        isOpen = !isOpen;
        btn.classList.toggle('open', isOpen);
        menu.classList.toggle('show', isOpen);
    }

    function closeDropdown() {
        isOpen = false;
        btn.classList.remove('open');
        menu.classList.remove('show');
    }

    // Event listeners
    btn.addEventListener('click', (e) => {
        e.stopPropagation();
        toggleDropdown();
    });

    // Close when clicking outside
    document.addEventListener('click', (e) => {
        if (!container.contains(e.target)) {
            closeDropdown();
        }
    });

    // Build dropdown
    container.appendChild(btn);
    container.appendChild(menu);

    // Hide original select and insert dropdown
    select.style.display = 'none';
    select.parentNode.insertBefore(container, select.nextSibling);

    // Initial population
    populateMenu();

    // Return API for external control
    return {
        refresh: populateMenu,
        setValue: (value) => {
            const option = select.querySelector(`option[value="${value}"]`);
            if (option) {
                selectItem(value, option.textContent);
            }
        },
        getValue: () => select.value,
        close: closeDropdown
    };
}

// Export dropdown utility
window.initBtpcDropdown = initBtpcDropdown;

console.log('✨ UI Polish utilities loaded: loading, toast, clipboard, dropdown');
