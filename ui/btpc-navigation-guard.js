/**
 * BTPC Navigation Guard
 *
 * Implements authentication-based routing for the BTPC desktop application.
 *
 * Features:
 * - Checks authentication status on page load
 * - Redirects unauthenticated users to login page
 * - Article XI compliant (backend-first authentication check)
 * - Fast performance (<50ms check via check_session command)
 *
 * Architecture:
 * - Backend Arc<RwLock<SessionState>> is single source of truth
 * - check_session() is read-only and fast (<50ms requirement)
 * - No localStorage for authentication state
 *
 * Usage:
 *   This script automatically runs on page load.
 *   No explicit initialization required.
 *
 * Pages:
 *   - login.html: No guard (public page)
 *   - All other pages: Require authentication
 */

const BtpcNavigationGuard = {
    /**
     * Check if current page is the login page
     * @returns {boolean}
     */
    isLoginPage() {
        const currentPath = window.location.pathname;
        return currentPath.endsWith('login.html') || currentPath === '/login.html';
    },

    /**
     * Check if current page is a public page that doesn't require authentication
     * @returns {boolean}
     */
    isPublicPage() {
        // Currently, only login.html is public
        // Add more pages here if needed (e.g., help.html, about.html)
        return this.isLoginPage();
    },

    /**
     * Check authentication status with backend
     * @returns {Promise<boolean>} true if authenticated, false otherwise
     */
    async checkAuthentication() {
        try {
            // Get Tauri invoke function
            const invoke = window.btpcInvoke || window.__TAURI__?.core?.invoke;

            if (!invoke) {
                console.warn('[Navigation Guard] Tauri API not available');
                // If Tauri API is not available, assume we're in development mode
                // In production, this should never happen
                return false;
            }

            // Call check_session command (backend-first, <50ms requirement)
            const response = await invoke('check_session');

            if (response && typeof response.authenticated === 'boolean') {
                console.log(`[Navigation Guard] Authentication check: ${response.authenticated ? 'AUTHENTICATED' : 'NOT AUTHENTICATED'}`);
                return response.authenticated;
            } else {
                console.warn('[Navigation Guard] Invalid check_session response:', response);
                return false;
            }
        } catch (error) {
            console.error('[Navigation Guard] Authentication check failed:', error);
            // On error, assume not authenticated (fail-safe)
            return false;
        }
    },

    /**
     * Redirect to login page
     */
    redirectToLogin() {
        console.log('[Navigation Guard] Redirecting to login page...');
        window.location.href = 'login.html';
    },

    /**
     * Redirect to dashboard (default authenticated page)
     */
    redirectToDashboard() {
        console.log('[Navigation Guard] Redirecting to dashboard...');
        window.location.href = 'index.html';
    },

    /**
     * Run navigation guard check
     * Called automatically on page load
     */
    async run() {
        try {
            // Check if this is a public page
            if (this.isPublicPage()) {
                console.log('[Navigation Guard] Public page, no authentication required');

                // If user is on login page but already authenticated, redirect to dashboard
                if (this.isLoginPage()) {
                    const isAuthenticated = await this.checkAuthentication();
                    if (isAuthenticated) {
                        console.log('[Navigation Guard] Already authenticated, redirecting to dashboard');
                        this.redirectToDashboard();
                        return;
                    }
                }

                return;
            }

            // For protected pages, check authentication
            console.log('[Navigation Guard] Protected page, checking authentication...');
            const isAuthenticated = await this.checkAuthentication();

            if (!isAuthenticated) {
                console.log('[Navigation Guard] Not authenticated, redirecting to login');
                this.redirectToLogin();
            } else {
                console.log('[Navigation Guard] Authentication verified, access granted');
            }
        } catch (error) {
            console.error('[Navigation Guard] Error during navigation guard check:', error);
            // On error, redirect to login for safety
            if (!this.isPublicPage()) {
                this.redirectToLogin();
            }
        }
    },

    /**
     * Initialize navigation guard
     * Sets up automatic authentication check on page load
     */
    init() {
        // Wait for Tauri API to be ready
        if (typeof window.__TAURI__ === 'undefined') {
            console.log('[Navigation Guard] Waiting for Tauri API...');
            setTimeout(() => this.init(), 100);
            return;
        }

        console.log('[Navigation Guard] Initializing...');
        this.run();
    }
};

// Auto-initialize navigation guard on page load
if (typeof window !== 'undefined') {
    // Run as soon as possible, but after Tauri API is loaded
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', () => {
            BtpcNavigationGuard.init();
        });
    } else {
        // DOM already loaded
        BtpcNavigationGuard.init();
    }

    // Export for use in other modules
    window.BtpcNavigationGuard = BtpcNavigationGuard;
}