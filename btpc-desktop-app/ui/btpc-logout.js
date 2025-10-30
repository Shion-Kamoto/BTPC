/**
 * BTPC Logout Component
 *
 * Provides logout functionality for the application-level authentication system.
 *
 * Features:
 * - Calls the logout Tauri command
 * - Shows loading state during logout
 * - Handles errors gracefully
 * - Redirects to login page after successful logout
 * - Article XI compliant (backend-first)
 *
 * Usage:
 *   <button id="logout-btn" class="btn-logout">
 *     <span class="icon icon-logout"></span>
 *     Logout
 *   </button>
 *
 *   Then call: BtpcLogout.init('logout-btn');
 */

const BtpcLogout = {
    /**
     * Initialize logout button with the given ID
     * @param {string} buttonId - The ID of the logout button element
     */
    init(buttonId) {
        const logoutBtn = document.getElementById(buttonId);
        if (!logoutBtn) {
            console.error(`Logout button with ID "${buttonId}" not found`);
            return;
        }

        logoutBtn.addEventListener('click', () => this.performLogout(logoutBtn));
    },

    /**
     * Perform logout operation
     * @param {HTMLElement} button - The logout button element
     */
    async performLogout(button) {
        // Prevent multiple clicks
        if (button.disabled) return;

        // Store original button content
        const originalContent = button.innerHTML;

        // Disable button and show loading state
        button.disabled = true;
        button.innerHTML = `
            <span class="loading-spinner-small"></span>
            <span>Logging out...</span>
        `;

        try {
            // Get Tauri invoke function (from btpc-tauri-context.js or window.__TAURI__)
            const invoke = window.btpcInvoke || window.__TAURI__?.core?.invoke;

            if (!invoke) {
                throw new Error('Tauri API not available');
            }

            // Call logout Tauri command
            const response = await invoke('logout');

            if (response.success) {
                // Show success briefly before redirect
                button.innerHTML = `
                    <span style="color: var(--status-success);">✓</span>
                    <span>Logged out</span>
                `;

                // Redirect to login page after brief delay
                setTimeout(() => {
                    window.location.href = 'login.html';
                }, 500);
            } else {
                // This should never happen since logout always succeeds
                throw new Error('Logout failed unexpectedly');
            }
        } catch (error) {
            console.error('Logout error:', error);

            // Show error state
            button.innerHTML = `
                <span style="color: var(--status-error);">✕</span>
                <span>Logout failed</span>
            `;

            // Restore button after 2 seconds
            setTimeout(() => {
                button.disabled = false;
                button.innerHTML = originalContent;
            }, 2000);
        }
    },

    /**
     * Create and inject a logout button into the page header
     * This is a convenience method for pages that don't have a logout button yet
     *
     * @param {string} targetSelector - CSS selector for the element to append the button to
     */
    injectLogoutButton(targetSelector = '.page-header') {
        const target = document.querySelector(targetSelector);
        if (!target) {
            console.error(`Target element "${targetSelector}" not found`);
            return;
        }

        // Create button element
        const logoutBtn = document.createElement('button');
        logoutBtn.id = 'logout-btn';
        logoutBtn.className = 'btn-logout';
        logoutBtn.innerHTML = `
            <img src="src/assets/icons-svg/unlock.svg" alt="Logout" class="logout-icon">
            <span>Logout</span>
        `;

        // Wrap in a container div for positioning
        const container = document.createElement('div');
        container.className = 'logout-container';
        container.appendChild(logoutBtn);

        // Append to target
        target.appendChild(container);

        // Initialize the button
        this.init('logout-btn');
    }
};

// Auto-initialize if there's a logout button with id="logout-btn" on page load
if (typeof window !== 'undefined') {
    window.addEventListener('DOMContentLoaded', () => {
        const logoutBtn = document.getElementById('logout-btn');
        if (logoutBtn) {
            BtpcLogout.init('logout-btn');
        }
    });
}

// Export for use in other modules
if (typeof window !== 'undefined') {
    window.BtpcLogout = BtpcLogout;
}