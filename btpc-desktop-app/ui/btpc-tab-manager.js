/**
 * BTPC Tab Manager Module
 * Provides tab switching functionality with localStorage persistence and ARIA compliance
 *
 * Article XI Compliance:
 * - Section 11.1: localStorage used only for non-critical UI preference (tab selection)
 * - Section 11.6: Event listeners cleaned up automatically via DOM element removal
 *
 * Requirements:
 * - FR-001 to FR-019: Tab button interaction, content visibility, state persistence, keyboard access
 * - NFR-001: < 50ms visual response to tab switch
 * - NFR-007, NFR-008: ARIA-compliant, screen reader friendly, keyboard navigation
 */

class TabManager {
    /**
     * Create a TabManager instance for a page
     * @param {Object} options - Configuration options
     * @param {string} options.page - Page identifier ('settings', 'transactions', 'mining')
     * @param {string} options.defaultTab - Default tab ID (first tab, fallback when no localStorage)
     * @param {Function} [options.onTabChange] - Optional callback when tab changes
     */
    constructor(options) {
        this.page = options.page;
        this.defaultTab = options.defaultTab;
        this.onTabChange = options.onTabChange || null;

        this.init();
    }

    /**
     * Initialize tab manager: load saved tab, set up event listeners
     */
    init() {
        // Load saved tab or default (FR-010, FR-011, FR-012)
        const savedTab = this.loadActiveTab();
        this.activateTab(savedTab, false); // Don't save on initial load

        // Set up event delegation on tab container (FR-001, Article XI.6)
        const tabNav = document.querySelector('.tab-nav');
        if (tabNav) {
            tabNav.addEventListener('click', this.handleTabClick.bind(this));
            tabNav.addEventListener('keydown', this.handleKeyDown.bind(this));
        } else {
            console.warn(`TabManager: .tab-nav container not found on page ${this.page}`);
        }
    }

    /**
     * Handle tab button click events (FR-001)
     * @param {Event} event - Click event
     */
    handleTabClick(event) {
        const tabBtn = event.target.closest('.tab-btn');
        if (!tabBtn) return;

        const tabId = tabBtn.dataset.tab || this.extractTabId(tabBtn);
        if (tabId) {
            this.activateTab(tabId, true); // Save to localStorage
        }
    }

    /**
     * Handle keyboard navigation events (FR-013 to FR-016)
     * @param {Event} event - Keydown event
     */
    handleKeyDown(event) {
        // Only handle tab-specific keys
        if (!['ArrowLeft', 'ArrowRight', 'Enter', ' '].includes(event.key)) return;

        const tabBtn = event.target.closest('.tab-btn');
        if (!tabBtn) return;

        event.preventDefault();

        // Enter or Space activates the focused tab (FR-014)
        if (event.key === 'Enter' || event.key === ' ') {
            const tabId = tabBtn.dataset.tab || this.extractTabId(tabBtn);
            if (tabId) {
                this.activateTab(tabId, true);
            }
        } else {
            // Arrow keys navigate between tabs (FR-015)
            this.navigateTabs(event.key, tabBtn);
        }
    }

    /**
     * Activate a specific tab (FR-002, FR-005, FR-006, FR-008)
     * @param {string} tabId - Tab identifier to activate
     * @param {boolean} save - Whether to save to localStorage (default: true)
     */
    activateTab(tabId, save = true) {
        // Remove active state from all tab buttons (FR-003)
        document.querySelectorAll('.tab-btn').forEach(btn => {
            btn.classList.remove('active');
            btn.setAttribute('aria-selected', 'false'); // NFR-007
            btn.setAttribute('tabindex', '-1'); // Only active tab in tab order
        });

        // Remove active state from all tab content panels (FR-006)
        document.querySelectorAll('.tab-content').forEach(content => {
            content.classList.remove('active');
        });

        // Activate target tab button (FR-002)
        const targetBtn = document.querySelector(`[data-tab="${tabId}"]`) ||
                         document.querySelector(`.tab-btn[onclick*="${tabId}"]`);

        if (targetBtn) {
            targetBtn.classList.add('active');
            targetBtn.setAttribute('aria-selected', 'true'); // NFR-007
            targetBtn.setAttribute('tabindex', '0'); // Make focusable
        } else {
            console.warn(`TabManager: Tab button for "${tabId}" not found on page ${this.page}`);
        }

        // Activate target tab content panel (FR-005, FR-008)
        const targetContent = document.getElementById(`tab-panel-${tabId}`) ||
                             document.getElementById(`tab-${tabId}`);

        if (targetContent) {
            targetContent.classList.add('active');
        } else {
            console.warn(`TabManager: Tab content #tab-panel-${tabId} not found on page ${this.page}`);
        }

        // Save to localStorage (FR-009, FR-010, FR-011)
        if (save) {
            this.saveActiveTab(tabId);
        }

        // Optional callback for custom logic
        if (this.onTabChange) {
            this.onTabChange(tabId);
        }
    }

    /**
     * Save active tab ID to localStorage (FR-009)
     * @param {string} tabId - Tab identifier to save
     */
    saveActiveTab(tabId) {
        try {
            localStorage.setItem(`btpc_active_tab_${this.page}`, tabId);
        } catch (e) {
            // Graceful degradation: localStorage disabled or unavailable (Edge Case 1)
            console.warn('localStorage unavailable, tab state will not persist:', e);
        }
    }

    /**
     * Load active tab ID from localStorage (FR-010, FR-011, FR-012)
     * @returns {string} Saved tab ID or defaultTab if none saved
     */
    loadActiveTab() {
        try {
            const saved = localStorage.getItem(`btpc_active_tab_${this.page}`);
            return saved || this.defaultTab; // FR-012: default to first tab if no saved state
        } catch (e) {
            // Graceful degradation: localStorage disabled or unavailable
            console.warn('localStorage unavailable, using default tab:', e);
            return this.defaultTab;
        }
    }

    /**
     * Navigate between tabs using arrow keys (FR-015)
     * @param {string} key - 'ArrowLeft' or 'ArrowRight'
     * @param {HTMLElement} currentBtn - Currently focused tab button
     */
    navigateTabs(key, currentBtn) {
        const allTabs = Array.from(document.querySelectorAll('.tab-btn'));
        const currentIndex = allTabs.indexOf(currentBtn);

        let nextIndex;
        if (key === 'ArrowRight') {
            // Move to next tab, wrap to first if at end
            nextIndex = (currentIndex + 1) % allTabs.length;
        } else { // ArrowLeft
            // Move to previous tab, wrap to last if at beginning
            nextIndex = (currentIndex - 1 + allTabs.length) % allTabs.length;
        }

        const nextBtn = allTabs[nextIndex];
        const nextTabId = nextBtn.dataset.tab || this.extractTabId(nextBtn);

        if (nextTabId) {
            this.activateTab(nextTabId, true);
            nextBtn.focus(); // Move keyboard focus to new tab (FR-016)
        }
    }

    /**
     * Extract tab ID from button's onclick attribute (fallback for legacy buttons)
     * @param {HTMLElement} button - Tab button element
     * @returns {string|null} Extracted tab ID or null if not found
     */
    extractTabId(button) {
        // Fallback: Extract from onclick attribute (e.g., onclick="switchTab('node')")
        const onclick = button.getAttribute('onclick');
        if (onclick) {
            const match = onclick.match(/switchTab\(['"](.+?)['"]\)/);
            if (match) return match[1];
        }

        // Additional fallback: Extract from aria-controls
        const ariaControls = button.getAttribute('aria-controls');
        if (ariaControls) {
            // Remove 'tab-panel-' or 'tab-' prefix
            return ariaControls.replace(/^tab-panel-/, '').replace(/^tab-/, '');
        }

        return null;
    }
}

// Export for use in pages (make available globally)
window.TabManager = TabManager;
